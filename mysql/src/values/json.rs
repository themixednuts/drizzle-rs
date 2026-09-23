//! [`Json<T>`] support for MySQL `JSON` columns.
//!
//! `Json<T>` owns the MySQL column codec for JSON payloads: it binds the JSON
//! text as bytes and decodes the bytes MySQL returns, so table macros never
//! implement [`DrizzleMySQLColumn`] on a payload type.

use drizzle_core::error::DrizzleError;
use drizzle_core::expr::{Expr, NonNull, Nullability, Scalar};
use drizzle_core::json::{Json, JsonColumnOperand, JsonColumnValue};
use drizzle_core::types::DataType;
use drizzle_core::{FromDrizzleRow, SQL, SQLParam, ToSQL, Token};
use serde::Serialize;
use serde::de::DeserializeOwned;

use super::{MySQLInsertValue, MySQLUpdateValue, MySQLValue, OwnedMySQLValue, ValueWrapper};
use crate::driver::{MySQLRow, MySQLRowAccess};
use crate::prelude::*;
use crate::traits::DrizzleMySQLColumn;

fn json_bytes<'a, T: Serialize>(value: &Json<T>) -> MySQLValue<'a> {
    MySQLValue::Bytes(Cow::Owned(value.encode_json_bytes()))
}

/// Stores the payload as a MySQL `JSON` document.
///
/// # Panics
///
/// [`encode`](DrizzleMySQLColumn::encode) and
/// [`encode_owned`](DrizzleMySQLColumn::encode_owned) have no error channel
/// and panic when `T`'s `Serialize` implementation fails; see
/// [`Json`](drizzle_core::Json#panics).
impl<T: Serialize + DeserializeOwned> DrizzleMySQLColumn for Json<T> {
    type SQLType = crate::types::Json;

    const SQL_TYPE: &'static str = "JSON";

    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
        match value {
            MySQLValue::Bytes(bytes) => Self::from_json_slice(bytes.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                "expected MySQL JSON bytes".into(),
            )),
        }
    }

    fn encode(&self) -> MySQLValue<'_> {
        json_bytes(self)
    }

    fn encode_owned(self) -> OwnedMySQLValue {
        OwnedMySQLValue::Bytes(self.encode_json_bytes())
    }
}

impl<'row, R, T> FromDrizzleRow<MySQLRow<'row, R>> for Json<T>
where
    R: MySQLRowAccess + ?Sized,
    T: Serialize + DeserializeOwned,
{
    const COLUMN_COUNT: usize = 1;

    fn from_row_at(row: &MySQLRow<'row, R>, offset: usize) -> Result<Self, DrizzleError> {
        row.decode_column::<Self>(offset)
    }
}

/// Renders `CAST(? AS JSON)` so MySQL compares JSON documents rather than
/// strings.
///
/// # Panics
///
/// Panics when `T`'s `Serialize` implementation fails; see
/// [`Json`](drizzle_core::Json#panics).
impl<'a, T: Serialize> ToSQL<'a, MySQLValue<'a>> for Json<T> {
    fn to_sql(&self) -> SQL<'a, MySQLValue<'a>> {
        SQL::func(
            "CAST",
            SQL::param(json_bytes(self))
                .push(Token::AS)
                .append(SQL::raw("JSON")),
        )
    }
}

impl<'a, T: Serialize> Expr<'a, MySQLValue<'a>> for Json<T> {
    type SQLType = crate::types::Json;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, T: Serialize> JsonColumnValue<T> for MySQLInsertValue<'a, MySQLValue<'a>, T> {
    fn from_json(value: Json<T>) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(SQL::param(
            json_bytes(&value),
        )))
    }
}

impl<'a, T, Target, TargetNull> JsonColumnValue<T>
    for MySQLUpdateValue<'a, MySQLValue<'a>, T, Target, TargetNull>
where
    T: Serialize,
    Target: DataType,
    TargetNull: Nullability,
{
    fn from_json(value: Json<T>) -> Self {
        Self::Value(ValueWrapper::<MySQLValue<'a>, T>::new(SQL::param(
            json_bytes(&value),
        )))
    }
}

impl<V: SQLParam, T> JsonColumnOperand for MySQLInsertValue<'_, V, T> {}

impl<V, T, Target, TargetNull> JsonColumnOperand for MySQLUpdateValue<'_, V, T, Target, TargetNull>
where
    V: SQLParam,
    Target: DataType,
    TargetNull: Nullability,
{
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
    struct Payload {
        tags: Vec<String>,
    }

    #[test]
    fn column_codec_round_trips_json_bytes() {
        let value = Json(Payload {
            tags: vec!["a".into()],
        });
        let encoded = value.encode();
        assert_eq!(encoded.as_bytes(), Some(br#"{"tags":["a"]}"#.as_slice()));
        assert_eq!(Json::<Payload>::decode(encoded).unwrap(), value);
        assert!(Json::<Payload>::decode(MySQLValue::Int(1)).is_err());
        assert!(Json::<Payload>::decode(MySQLValue::from("not json")).is_err());
    }

    #[test]
    fn expressions_cast_to_json() {
        let sql = Json(vec![1, 2]).to_sql();
        assert_eq!(sql.sql(), "CAST(? AS JSON)");
    }
}
