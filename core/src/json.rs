//! Drizzle-owned wrapper for JSON column values.
//!
//! [`Json<T>`] marks a value as a JSON document. Each dialect crate implements
//! its value, expression, and row-decoding traits for `Json<T>` once,
//! generically, so table macros never implement traits on a payload type and
//! never name `serde_json` in generated code.

use core::ops::{Deref, DerefMut};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize, Serializer};

use crate::error::DrizzleError;
use crate::expr::{AggregateKind, ColumnBinOp, ColumnNeg, Excluded, Nullability, SQLExpr};
use crate::placeholder::{Placeholder, TypedPlaceholder};
use crate::prelude::{String, Vec};
use crate::traits::SQLParam;
use crate::types::DataType;

/// A JSON document stored in a JSON column.
///
/// `Json<T>` wraps any `T: Serialize` (to write) or `T: DeserializeOwned` (to
/// read), including foreign types such as `Vec<String>`, `BTreeMap<String,
/// i64>`, or `serde_json::Value`. It serializes transparently: `Json(value)`
/// produces the same JSON as `value`.
///
/// Generated table models keep the payload type on JSON fields. A field
/// declared as `#[column(json)] meta: Meta` is `Meta` in the select, insert,
/// update, and partial-select models; the table macro converts through
/// `Json<Meta>` internally. Wrap a value in `Json` yourself when you use it as
/// an SQL expression, for example `eq(documents.meta, Json(meta))`.
///
/// # Encoding
///
/// - SQLite stores JSON text and binds values through `json(?)`.
/// - PostgreSQL binds `json` or `jsonb` to match the column. Expressions such
///   as `eq(t.meta, Json(value))` bind `jsonb`, the only JSON type with an
///   equality operator.
/// - MySQL binds the JSON text and compares through `CAST(? AS JSON)`.
///
/// # Panics
///
/// Serialization fails only when `T`'s `Serialize` implementation fails, for
/// example for a map whose keys are not strings. Conversions with an error
/// channel report that failure: [`Json::to_json_string`], decoding from rows,
/// and relational query decoding. Conversions without one panic with a message
/// that starts with `drizzle: failed to serialize JSON value`:
///
/// - insert-model constructors and `with_*` setters of JSON fields,
/// - update-model `with_*` setters of JSON fields,
/// - rendering `Json<T>` as an SQL expression (`ToSQL`/`Expr`),
/// - MySQL's `DrizzleMySQLColumn::encode` and `From<Json<T>> for MySQLValue`.
///
/// Call [`Json::to_json_string`] first when a payload may not serialize.
///
/// # Examples
///
/// ```
/// use drizzle_core::Json;
///
/// #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
/// struct Settings {
///     theme: String,
/// }
///
/// let settings = Json(Settings { theme: "dark".into() });
/// // `Json` dereferences to the payload.
/// assert_eq!(settings.theme, "dark");
///
/// let text = settings.to_json_string()?;
/// assert_eq!(text, r#"{"theme":"dark"}"#);
/// assert_eq!(Json::<Settings>::from_json_str(&text)?, settings);
/// # Ok::<(), drizzle_core::error::DrizzleError>(())
/// ```
///
/// With a table, keep the payload type on the field and wrap comparison values:
///
/// ```rust
/// # let _ = r####"
/// use drizzle::core::Json;
/// use drizzle::core::expr::eq;
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Documents {
///     #[column(primary)]
///     id: i64,
///     #[column(json)]
///     settings: Settings,
/// }
///
/// db.insert(documents)
///     .values([InsertDocuments::new(1, settings.clone())])
///     .execute()?;
///
/// let matching: Vec<SelectDocuments> = db
///     .select(())
///     .from(documents)
///     .r#where(eq(documents.settings, Json(settings)))
///     .all()?;
/// # "####;
/// ```
#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    /// Unwraps the JSON payload.
    #[inline]
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> From<T> for Json<T> {
    #[inline]
    fn from(value: T) -> Self {
        Self(value)
    }
}

impl<T> Deref for Json<T> {
    type Target = T;

    #[inline]
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> DerefMut for Json<T> {
    #[inline]
    fn deref_mut(&mut self) -> &mut T {
        &mut self.0
    }
}

impl<T: Serialize> Serialize for Json<T> {
    #[inline]
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Deserialize<'de>> Deserialize<'de> for Json<T> {
    #[inline]
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Self)
    }
}

impl<T: Serialize> Json<T> {
    /// Serializes the payload as compact JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::JsonError`] when `T`'s `Serialize`
    /// implementation fails.
    pub fn to_json_string(&self) -> Result<String, DrizzleError> {
        serde_json::to_string(&self.0).map_err(DrizzleError::from)
    }

    /// Serializes the payload as compact JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::JsonError`] when `T`'s `Serialize`
    /// implementation fails.
    pub fn to_json_vec(&self) -> Result<Vec<u8>, DrizzleError> {
        serde_json::to_vec(&self.0).map_err(DrizzleError::from)
    }

    /// Serializes the payload as a `serde_json::Value`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::JsonError`] when `T`'s `Serialize`
    /// implementation fails.
    pub fn to_json_value(&self) -> Result<serde_json::Value, DrizzleError> {
        serde_json::to_value(&self.0).map_err(DrizzleError::from)
    }

    /// Serializes the payload as JSON text for a conversion with no error
    /// channel.
    ///
    /// # Panics
    ///
    /// Panics when `T`'s `Serialize` implementation fails; see
    /// [`Json`](Json#panics).
    #[doc(hidden)]
    #[track_caller]
    pub fn encode_json_text(&self) -> String {
        self.to_json_string()
            .unwrap_or_else(|error| serialization_failed(&error))
    }

    /// Serializes the payload as JSON bytes for a conversion with no error
    /// channel.
    ///
    /// # Panics
    ///
    /// Panics when `T`'s `Serialize` implementation fails; see
    /// [`Json`](Json#panics).
    #[doc(hidden)]
    #[track_caller]
    pub fn encode_json_bytes(&self) -> Vec<u8> {
        self.to_json_vec()
            .unwrap_or_else(|error| serialization_failed(&error))
    }

    /// Serializes the payload as a `serde_json::Value` for a conversion with
    /// no error channel.
    ///
    /// # Panics
    ///
    /// Panics when `T`'s `Serialize` implementation fails; see
    /// [`Json`](Json#panics).
    #[doc(hidden)]
    #[track_caller]
    pub fn encode_json_value(&self) -> serde_json::Value {
        self.to_json_value()
            .unwrap_or_else(|error| serialization_failed(&error))
    }
}

#[cold]
#[track_caller]
fn serialization_failed(error: &DrizzleError) -> ! {
    panic!("drizzle: failed to serialize JSON value for a JSON column: {error}")
}

impl<T: DeserializeOwned> Json<T> {
    /// Deserializes a payload from JSON text.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::JsonError`] when `json` is not valid JSON or
    /// does not match `T`.
    pub fn from_json_str(json: &str) -> Result<Self, DrizzleError> {
        serde_json::from_str(json)
            .map(Self)
            .map_err(DrizzleError::from)
    }

    /// Deserializes a payload from JSON bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::JsonError`] when `json` is not valid JSON or
    /// does not match `T`.
    pub fn from_json_slice(json: &[u8]) -> Result<Self, DrizzleError> {
        serde_json::from_slice(json)
            .map(Self)
            .map_err(DrizzleError::from)
    }

    /// Deserializes a payload from a `serde_json::Value`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::JsonError`] when `value` does not match `T`.
    pub fn from_json_value(value: serde_json::Value) -> Result<Self, DrizzleError> {
        serde_json::from_value(value)
            .map(Self)
            .map_err(DrizzleError::from)
    }
}

// =============================================================================
// Model field conversions
// =============================================================================

/// A model field value that can hold a JSON payload.
///
/// Dialect crates implement this for their insert and update value types.
/// Table macros call it to convert the payload type of a JSON field.
pub trait JsonColumnValue<T>: Sized {
    /// Converts `value` into the SQL value of a JSON column.
    ///
    /// # Panics
    ///
    /// Panics when `T`'s `Serialize` implementation fails, because the model
    /// setters that call this have no error channel; see
    /// [`Json`](Json#panics).
    fn from_json(value: Json<T>) -> Self;
}

/// SQL operands accepted by a JSON field's update setter in place of a
/// payload.
///
/// Implemented for placeholders, typed SQL expressions, `EXCLUDED` column
/// references, and each dialect's model value types. A payload type never
/// implements it, which keeps the argument conversions of [`JsonColumnArg`]
/// unambiguous.
pub trait JsonColumnOperand {}

/// Marker types that select an implementation of [`JsonColumnArg`].
///
/// Type inference picks the marker; callers never name these types.
pub mod arg {
    /// Selects the conversion of a bare payload value.
    #[derive(Debug)]
    pub enum Payload {}

    /// Selects the conversion of a [`Json`](super::Json)-wrapped payload.
    #[derive(Debug)]
    pub enum Wrapped {}

    /// Selects the conversion of an SQL operand, such as a placeholder.
    #[derive(Debug)]
    pub enum Operand {}
}

/// A value accepted by the update setter of a JSON field.
///
/// Setters of a field declared as `#[column(json)] meta: Meta` accept a bare
/// `Meta`, a `Json<Meta>`, or an SQL operand such as a placeholder, an
/// `EXCLUDED` reference, or the dialect's update value (for example
/// `SQLiteUpdateValue::Null`). `Marker` is inferred and keeps the three
/// conversions from overlapping.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be assigned to this JSON column",
    note = "pass the column's payload type, `drizzle::core::Json(value)`, a placeholder, or an SQL expression"
)]
pub trait JsonColumnArg<Out, Marker>: Sized {
    /// Converts this argument into the model field value.
    fn into_json_column(self) -> Out;
}

impl<T, Out: JsonColumnValue<T>> JsonColumnArg<Out, arg::Payload> for T {
    #[inline]
    fn into_json_column(self) -> Out {
        Out::from_json(Json(self))
    }
}

impl<T, Out: JsonColumnValue<T>> JsonColumnArg<Out, arg::Wrapped> for Json<T> {
    #[inline]
    fn into_json_column(self) -> Out {
        Out::from_json(self)
    }
}

impl<V, Out> JsonColumnArg<Out, arg::Operand> for V
where
    V: JsonColumnOperand + Into<Out>,
{
    #[inline]
    fn into_json_column(self) -> Out {
        self.into()
    }
}

impl JsonColumnOperand for Placeholder {}

impl<T: DataType, N: Nullability> JsonColumnOperand for TypedPlaceholder<T, N> {}

impl<C> JsonColumnOperand for Excluded<C> {}

impl<V, T, N, A> JsonColumnOperand for SQLExpr<'_, V, T, N, A>
where
    V: SQLParam,
    T: DataType,
    N: Nullability,
    A: AggregateKind,
{
}

impl<L, R, Op, D, T, N> JsonColumnOperand for ColumnBinOp<L, R, Op, D, T, N> {}

impl<E, D, T, N> JsonColumnOperand for ColumnNeg<E, D, T, N> {}

#[cfg(all(test, feature = "std"))]
mod tests {
    use super::*;
    use crate::prelude::ToString;

    #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
    struct Payload {
        name: String,
        count: i64,
    }

    #[test]
    fn serializes_and_deserializes_transparently() {
        let value = Json(Payload {
            name: "a".into(),
            count: 2,
        });
        let text = value.to_json_string().unwrap();
        assert_eq!(text, r#"{"name":"a","count":2}"#);
        assert_eq!(value.to_json_vec().unwrap(), text.as_bytes());
        assert_eq!(Json::<Payload>::from_json_str(&text).unwrap(), value);
        assert_eq!(
            Json::<Payload>::from_json_slice(text.as_bytes()).unwrap(),
            value
        );
        assert_eq!(
            Json::<Payload>::from_json_value(value.to_json_value().unwrap()).unwrap(),
            value
        );
        // The wrapper adds no JSON structure of its own.
        assert_eq!(
            serde_json::to_string(&value).unwrap(),
            serde_json::to_string(&value.0).unwrap()
        );
    }

    #[test]
    fn wrapper_conversions_reach_the_payload() {
        let mut value = Json::from(Payload::default());
        value.count += 5;
        assert_eq!(value.count, 5);
        assert_eq!(value.into_inner().count, 5);
    }

    #[test]
    fn serialization_failures_are_errors() {
        let mut map = std::collections::BTreeMap::new();
        map.insert((1, 2), "tuple keys are not JSON object keys");
        let error = Json(map).to_json_string().unwrap_err();
        assert!(matches!(error, DrizzleError::JsonError(_)));
        assert!(error.to_string().contains("key must be a string"));
    }

    #[test]
    #[should_panic(expected = "drizzle: failed to serialize JSON value")]
    fn infallible_encoders_panic_with_a_drizzle_message() {
        let mut map = std::collections::BTreeMap::new();
        map.insert((1, 2), 3);
        let _ = Json(map).encode_json_text();
    }

    #[test]
    fn decoding_failures_are_errors() {
        assert!(matches!(
            Json::<Payload>::from_json_str("{\"name\":1}"),
            Err(DrizzleError::JsonError(_))
        ));
        assert!(Json::<Payload>::from_json_slice(b"not json").is_err());
    }
}
