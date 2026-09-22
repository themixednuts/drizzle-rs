//! [`Json<T>`] support for SQLite JSON columns.
//!
//! SQLite stores JSON as TEXT. Values are bound through `json(?)`, which
//! validates and minifies the document, and are decoded from TEXT (or from
//! JSON bytes in a BLOB) through [`FromSQLiteValue`], so every SQLite driver
//! reads them through the same codec.

use drizzle_core::error::DrizzleError;
use drizzle_core::expr::{Expr, NonNull, Nullability, Scalar};
use drizzle_core::json::{Json, JsonColumnOperand, JsonColumnValue};
use drizzle_core::types::DataType;
use drizzle_core::{SQL, SQLParam, ToSQL};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{SQLiteInsertValue, SQLiteUpdateValue, SQLiteValue, ValueWrapper};
use crate::prelude::*;
use crate::traits::FromSQLiteValue;

/// Renders `json(?)` around the serialized payload.
fn json_param<'a, T: Serialize>(value: &Json<T>) -> SQL<'a, SQLiteValue<'a>> {
    crate::expr::json(SQL::param(SQLiteValue::Text(Cow::Owned(
        value.encode_json_text(),
    ))))
}

impl<T: DeserializeOwned> FromSQLiteValue for Json<T> {
    fn from_sqlite_integer(value: i64) -> Result<Self, DrizzleError> {
        Self::from_json_value(serde_json::Value::from(value))
    }

    fn from_sqlite_text(value: &str) -> Result<Self, DrizzleError> {
        Self::from_json_str(value)
    }

    fn from_sqlite_real(value: f64) -> Result<Self, DrizzleError> {
        let number = serde_json::Number::from_f64(value).ok_or_else(|| {
            DrizzleError::ConversionError(
                format!("cannot convert non-finite REAL {value} to JSON").into(),
            )
        })?;
        Self::from_json_value(serde_json::Value::Number(number))
    }

    fn from_sqlite_blob(value: &[u8]) -> Result<Self, DrizzleError> {
        Self::from_json_slice(value)
    }
}

/// The JSON text of the payload. A failing `Serialize` implementation is an
/// error here, not a silent NULL.
impl<'a, T: Serialize> TryFrom<Json<T>> for SQLiteValue<'a> {
    type Error = DrizzleError;

    fn try_from(value: Json<T>) -> Result<Self, Self::Error> {
        value
            .to_json_string()
            .map(|text| SQLiteValue::Text(Cow::Owned(text)))
    }
}

/// Renders `json(?)`.
///
/// # Panics
///
/// Panics when `T`'s `Serialize` implementation fails; see
/// [`Json`](drizzle_core::Json#panics).
impl<'a, T: Serialize> ToSQL<'a, SQLiteValue<'a>> for Json<T> {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        json_param(self)
    }
}

/// A JSON document is TEXT to SQLite, so `Json<T>` compares against JSON (and
/// other TEXT) columns.
impl<'a, T: Serialize> Expr<'a, SQLiteValue<'a>> for Json<T> {
    type SQLType = crate::types::Text;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, T: Serialize> JsonColumnValue<T> for SQLiteInsertValue<'a, SQLiteValue<'a>, T> {
    fn from_json(value: Json<T>) -> Self {
        Self::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(json_param(&value)))
    }
}

impl<'a, T, Target, TargetNull> JsonColumnValue<T>
    for SQLiteUpdateValue<'a, SQLiteValue<'a>, T, Target, TargetNull>
where
    T: Serialize,
    Target: DataType,
    TargetNull: Nullability,
{
    fn from_json(value: Json<T>) -> Self {
        Self::Value(ValueWrapper::<SQLiteValue<'a>, T>::new(json_param(&value)))
    }
}

impl<V: SQLParam, T> JsonColumnOperand for SQLiteInsertValue<'_, V, T> {}

impl<V, T, Target, TargetNull> JsonColumnOperand for SQLiteUpdateValue<'_, V, T, Target, TargetNull>
where
    V: SQLParam,
    Target: DataType,
    TargetNull: Nullability,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, Debug, PartialEq)]
    struct Payload {
        name: String,
    }

    #[test]
    fn binds_json_text_through_the_json_function() {
        let sql = Json(Payload { name: "a".into() }).to_sql();
        assert_eq!(sql.sql(), "json (?)");
        let params: Vec<_> = sql.params().collect();
        assert_eq!(params, [&SQLiteValue::Text(r#"{"name":"a"}"#.into())]);
    }

    #[test]
    fn decodes_every_json_storage_class() {
        assert_eq!(
            Json::<Payload>::from_sqlite_text(r#"{"name":"t"}"#)
                .unwrap()
                .0,
            Payload { name: "t".into() }
        );
        assert_eq!(
            Json::<Payload>::from_sqlite_blob(br#"{"name":"b"}"#)
                .unwrap()
                .0,
            Payload { name: "b".into() }
        );
        assert_eq!(Json::<i64>::from_sqlite_integer(7).unwrap().0, 7);
        assert_eq!(Json::<f64>::from_sqlite_real(1.5).unwrap().0, 1.5);
        assert!(Json::<f64>::from_sqlite_real(f64::NAN).is_err());
        assert!(Json::<Payload>::from_sqlite_text("not json").is_err());
        assert!(Json::<Payload>::from_sqlite_null().is_err());
    }
}
