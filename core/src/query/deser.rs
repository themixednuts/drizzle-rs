//! Deserialization from JSON columns in query results.

use core::fmt;
use core::marker::PhantomData;

use serde::Deserialize;
use serde::de::{self, IgnoredAny, MapAccess, SeqAccess, Visitor};

use crate::error::DrizzleError;
use crate::prelude::*;
use crate::relation::RelationDef;

use super::row::QueryRow;
use super::store::RelEntry;

/// Decodes one field from a JSON object map.
///
/// Generated table models implement this so relation rows can be decoded in a
/// single serde pass without building an intermediate JSON tree.
pub trait JsonObjectDecoder<'de>: Sized {
    /// Scratch state used while a JSON object is being read.
    type State;

    /// Creates an empty decode state.
    fn begin() -> Self::State;

    /// Attempts to decode `key` from `map` into `state`.
    ///
    /// Returns `true` when the key was consumed. If this returns `false`, the
    /// caller remains responsible for consuming the map value.
    ///
    /// # Errors
    /// Returns the serde map error when a matched field fails to decode.
    fn decode_field<A>(state: &mut Self::State, key: &str, map: &mut A) -> Result<bool, A::Error>
    where
        A: MapAccess<'de>;

    /// Builds the decoded value from its completed state.
    ///
    /// # Errors
    /// Returns a serde error when required fields were not present.
    fn finish<E>(state: Self::State) -> Result<Self, E>
    where
        E: de::Error;
}

/// Deserializes a value directly from JSON text.
pub trait FromJsonObject: Sized {
    /// Reads `Self` from a JSON string.
    ///
    /// # Errors
    /// Returns `DrizzleError` if the JSON text cannot be decoded as `Self`.
    fn from_json_str(json: &str, context: &str) -> Result<Self, DrizzleError>;
}

impl<T> FromJsonObject for T
where
    T: for<'de> Deserialize<'de>,
{
    #[inline]
    fn from_json_str(json: &str, context: &str) -> Result<Self, DrizzleError> {
        serde_json::from_str(json)
            .map_err(|e| DrizzleError::Other(format!("failed to parse {context} JSON: {e}").into()))
    }
}

/// Deserializes a `RelEntry` chain from relation JSON columns.
pub trait DeserializeStore: Sized {
    /// Reads relation JSON columns from a row reader.
    ///
    /// Each relation column is parsed when its matching relation field is
    /// decoded.
    ///
    /// # Errors
    /// Returns `DrizzleError` if a JSON column is missing or fails to decode.
    fn from_json_columns<F>(next: &mut F) -> Result<Self, DrizzleError>
    where
        F: FnMut() -> Result<Option<String>, DrizzleError>;
}

impl DeserializeStore for () {
    #[inline]
    fn from_json_columns<F>(_next: &mut F) -> Result<Self, DrizzleError>
    where
        F: FnMut() -> Result<Option<String>, DrizzleError>,
    {
        Ok(())
    }
}

impl<'de> JsonObjectDecoder<'de> for () {
    type State = ();

    #[inline]
    fn begin() -> Self::State {}

    #[inline]
    fn decode_field<A>(_state: &mut Self::State, _key: &str, _map: &mut A) -> Result<bool, A::Error>
    where
        A: MapAccess<'de>,
    {
        Ok(false)
    }

    #[inline]
    fn finish<E>(_state: Self::State) -> Result<Self, E>
    where
        E: de::Error,
    {
        Ok(())
    }
}

impl<Rel, Data, Rest> DeserializeStore for RelEntry<Rel, Data, Rest>
where
    Rel: RelationDef,
    Data: FromJsonColumn,
    Rest: DeserializeStore,
{
    fn from_json_columns<F>(next: &mut F) -> Result<Self, DrizzleError>
    where
        F: FnMut() -> Result<Option<String>, DrizzleError>,
    {
        let json = next()?;
        let data = Data::from_json_column(json.as_deref(), Rel::NAME)
            .map_err(|e| DrizzleError::Other(format!("relation '{}': {e}", Rel::NAME).into()))?;
        let rest = Rest::from_json_columns(next)?;
        Ok(Self::new(data, rest))
    }
}

impl<'de, Rel, Data, Rest> JsonObjectDecoder<'de> for RelEntry<Rel, Data, Rest>
where
    Rel: RelationDef,
    Data: FromJsonField<'de>,
    Rest: JsonObjectDecoder<'de>,
{
    type State = (Option<Data>, Rest::State);

    fn begin() -> Self::State {
        (None, Rest::begin())
    }

    fn decode_field<A>(state: &mut Self::State, key: &str, map: &mut A) -> Result<bool, A::Error>
    where
        A: MapAccess<'de>,
    {
        if key == Rel::NAME {
            state.0 = Some(Data::decode_json_field(map, Rel::NAME)?);
            return Ok(true);
        }

        Rest::decode_field(&mut state.1, key, map)
    }

    fn finish<E>(state: Self::State) -> Result<Self, E>
    where
        E: de::Error,
    {
        let data = match state.0 {
            Some(data) => data,
            None => Data::missing_json_field(Rel::NAME)?,
        };
        let rest = Rest::finish(state.1)?;
        Ok(Self::new(data, rest))
    }
}

/// Parses a relation's wrapped data from JSON text.
///
/// Implemented for `Vec<T>` (Many), `Option<T>` (`OptionalOne`), and
/// `QueryRow<Base, Store>` (One).
pub trait FromJsonColumn: Sized {
    /// Converts an optional JSON column into this relation data type.
    ///
    /// # Errors
    /// Returns `DrizzleError` when the JSON text does not match the expected
    /// shape for this type.
    fn from_json_column(json: Option<&str>, context: &str) -> Result<Self, DrizzleError>;
}

/// Decodes a relation field from a parent JSON object.
pub trait FromJsonField<'de>: Sized {
    /// Reads this value from a serde map entry.
    ///
    /// # Errors
    /// Returns the serde map error if the value fails to decode.
    fn decode_json_field<A>(map: &mut A, context: &str) -> Result<Self, A::Error>
    where
        A: MapAccess<'de>;

    /// Supplies the value for a missing relation field.
    ///
    /// # Errors
    /// Returns a serde error when the relation is required.
    fn missing_json_field<E>(context: &str) -> Result<Self, E>
    where
        E: de::Error;
}

impl<T> FromJsonColumn for Vec<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn from_json_column(json: Option<&str>, context: &str) -> Result<Self, DrizzleError> {
        match json {
            Some(json) => serde_json::from_str::<JsonVec<T>>(json)
                .map(|items| items.0)
                .map_err(|e| {
                    DrizzleError::Other(format!("failed to parse {context} JSON: {e}").into())
                }),
            None => Ok(Self::new()),
        }
    }
}

impl<'de, T> FromJsonField<'de> for Vec<T>
where
    T: Deserialize<'de>,
{
    fn decode_json_field<A>(map: &mut A, _context: &str) -> Result<Self, A::Error>
    where
        A: MapAccess<'de>,
    {
        map.next_value::<JsonVec<T>>().map(|items| items.0)
    }

    fn missing_json_field<E>(_context: &str) -> Result<Self, E>
    where
        E: de::Error,
    {
        Ok(Self::new())
    }
}

impl<T> FromJsonColumn for Option<T>
where
    T: for<'de> Deserialize<'de>,
{
    fn from_json_column(json: Option<&str>, context: &str) -> Result<Self, DrizzleError> {
        match json {
            Some(json) => serde_json::from_str(json).map_err(|e| {
                DrizzleError::Other(format!("failed to parse {context} JSON: {e}").into())
            }),
            None => Ok(None),
        }
    }
}

impl<'de, T> FromJsonField<'de> for Option<T>
where
    T: Deserialize<'de>,
{
    fn decode_json_field<A>(map: &mut A, _context: &str) -> Result<Self, A::Error>
    where
        A: MapAccess<'de>,
    {
        map.next_value()
    }

    fn missing_json_field<E>(_context: &str) -> Result<Self, E>
    where
        E: de::Error,
    {
        Ok(None)
    }
}

impl<Base, Store> FromJsonColumn for QueryRow<Base, Store>
where
    Self: for<'de> Deserialize<'de>,
{
    fn from_json_column(json: Option<&str>, context: &str) -> Result<Self, DrizzleError> {
        let Some(json) = json else {
            return Err(DrizzleError::Other(
                format!("missing JSON column for {context}").into(),
            ));
        };

        let row: Option<Self> = serde_json::from_str(json).map_err(|e| {
            DrizzleError::Other(format!("failed to parse {context} JSON: {e}").into())
        })?;
        row.ok_or_else(|| {
            DrizzleError::Other(format!("expected non-null relation '{context}'").into())
        })
    }
}

impl<'de, Base, Store> FromJsonField<'de> for QueryRow<Base, Store>
where
    Self: Deserialize<'de>,
{
    fn decode_json_field<A>(map: &mut A, context: &str) -> Result<Self, A::Error>
    where
        A: MapAccess<'de>,
    {
        let row: Option<Self> = map.next_value()?;
        row.ok_or_else(|| de::Error::custom(format!("expected non-null relation '{context}'")))
    }

    fn missing_json_field<E>(context: &str) -> Result<Self, E>
    where
        E: de::Error,
    {
        Err(de::Error::custom(format!(
            "missing non-null relation '{context}'"
        )))
    }
}

impl<'de, Base, Store> Deserialize<'de> for QueryRow<Base, Store>
where
    Base: JsonObjectDecoder<'de>,
    Store: JsonObjectDecoder<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct QueryRowVisitor<Base, Store>(PhantomData<(Base, Store)>);

        impl<'de, Base, Store> Visitor<'de> for QueryRowVisitor<Base, Store>
        where
            Base: JsonObjectDecoder<'de>,
            Store: JsonObjectDecoder<'de>,
        {
            type Value = QueryRow<Base, Store>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a relation row JSON object")
            }

            fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
            where
                A: MapAccess<'de>,
            {
                let mut base = Base::begin();
                let mut store = Store::begin();

                while let Some(key) = map.next_key::<Cow<'de, str>>()? {
                    let key = key.as_ref();
                    if Base::decode_field(&mut base, key, &mut map)? {
                        continue;
                    }
                    if Store::decode_field(&mut store, key, &mut map)? {
                        continue;
                    }
                    map.next_value::<IgnoredAny>()?;
                }

                Ok(QueryRow::new(Base::finish(base)?, Store::finish(store)?))
            }
        }

        deserializer.deserialize_map(QueryRowVisitor::<Base, Store>(PhantomData))
    }
}

/// Deserializes JSON booleans that may be represented as `0` or `1`.
pub struct JsonBool(pub bool);

impl<'de> Deserialize<'de> for JsonBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct BoolVisitor;

        impl Visitor<'_> for BoolVisitor {
            type Value = JsonBool;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a boolean or integer")
            }

            fn visit_bool<E>(self, value: bool) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonBool(value))
            }

            fn visit_i64<E>(self, value: i64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonBool(value != 0))
            }

            fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonBool(value != 0))
            }
        }

        deserializer.deserialize_any(BoolVisitor)
    }
}

/// Deserializes nullable JSON booleans that may be represented as `0` or `1`.
pub struct JsonOptionalBool(pub Option<bool>);

impl<'de> Deserialize<'de> for JsonOptionalBool {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct OptionalBoolVisitor;

        impl<'de> Visitor<'de> for OptionalBoolVisitor {
            type Value = JsonOptionalBool;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a nullable boolean or integer")
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonOptionalBool(None))
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonOptionalBool(None))
            }

            fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
            where
                D: serde::Deserializer<'de>,
            {
                JsonBool::deserialize(deserializer).map(|value| JsonOptionalBool(Some(value.0)))
            }
        }

        deserializer.deserialize_option(OptionalBoolVisitor)
    }
}

struct JsonVec<T>(Vec<T>);

impl<'de, T> Deserialize<'de> for JsonVec<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        struct JsonVecVisitor<T>(PhantomData<T>);

        impl<'de, T> Visitor<'de> for JsonVecVisitor<T>
        where
            T: Deserialize<'de>,
        {
            type Value = JsonVec<T>;

            fn expecting(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str("a JSON array or null")
            }

            fn visit_unit<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonVec(Vec::new()))
            }

            fn visit_none<E>(self) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(JsonVec(Vec::new()))
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: SeqAccess<'de>,
            {
                let mut out = Vec::with_capacity(seq.size_hint().unwrap_or(0));
                while let Some(value) = seq.next_element::<Option<T>>()? {
                    if let Some(value) = value {
                        out.push(value);
                    }
                }
                Ok(JsonVec(out))
            }
        }

        deserializer.deserialize_any(JsonVecVisitor::<T>(PhantomData))
    }
}
