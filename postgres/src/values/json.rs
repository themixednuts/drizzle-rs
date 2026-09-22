//! [`Json<T>`] support for PostgreSQL `json` and `jsonb` columns.
//!
//! PostgreSQL has two JSON column types, so the column decides whether a
//! payload binds as `json` or `jsonb`: generated models pass the column's SQL
//! type marker ([`PostgresJsonType`]) when they convert a payload. `Json<T>`
//! used directly as an expression binds `jsonb`, the only JSON type with an
//! equality operator. Row decoding goes through the drivers' JSON codec
//! (`drizzle::postgres::driver_types::Json`) and the AWS Data API row leaves.

use drizzle_core::error::DrizzleError;
use drizzle_core::expr::{Expr, NonNull, Nullability, Scalar};
use drizzle_core::json::{Json, JsonColumnOperand, JsonColumnValue};
use drizzle_core::types::DataType;
use drizzle_core::{SQL, SQLParam, ToSQL};
use serde::Serialize;

use super::{PostgresInsertValue, PostgresUpdateValue, PostgresValue, ValueWrapper};

mod private {
    pub trait Sealed {}
}

/// SQL type markers of PostgreSQL's JSON column types.
///
/// Selects whether a JSON payload binds as `json` or `jsonb`.
pub trait PostgresJsonType: DataType + private::Sealed {
    /// Whether values bind as `jsonb` rather than `json`.
    const JSONB: bool;
}

impl private::Sealed for crate::types::Json {}

impl PostgresJsonType for crate::types::Json {
    const JSONB: bool = false;
}

impl private::Sealed for crate::types::Jsonb {}

impl PostgresJsonType for crate::types::Jsonb {
    const JSONB: bool = true;
}

const fn document<'a, Target: PostgresJsonType>(value: serde_json::Value) -> PostgresValue<'a> {
    if Target::JSONB {
        PostgresValue::Jsonb(value)
    } else {
        PostgresValue::Json(value)
    }
}

impl PostgresValue<'_> {
    /// Serializes `value` as a parameter for a JSON column of type `Target`:
    /// [`PostgresValue::Jsonb`] for `jsonb`, [`PostgresValue::Json`] for
    /// `json`.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::JsonError`] when `T`'s `Serialize`
    /// implementation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use drizzle_core::Json;
    /// use drizzle_postgres::types::Jsonb;
    /// use drizzle_postgres::values::PostgresValue;
    ///
    /// let value = PostgresValue::try_from_json::<Jsonb, _>(&Json(vec!["a", "b"]))?;
    /// assert_eq!(value, PostgresValue::Jsonb(serde_json::json!(["a", "b"])));
    /// # Ok::<(), drizzle_core::error::DrizzleError>(())
    /// ```
    pub fn try_from_json<Target: PostgresJsonType, T: Serialize>(
        value: &Json<T>,
    ) -> Result<Self, DrizzleError> {
        value.to_json_value().map(document::<Target>)
    }
}

/// A `jsonb` value, like `Json<T>` used as an expression. A failing
/// `Serialize` implementation is an error here, not a silent NULL.
impl<'a, T: Serialize> TryFrom<Json<T>> for PostgresValue<'a> {
    type Error = DrizzleError;

    fn try_from(value: Json<T>) -> Result<Self, Self::Error> {
        Self::try_from_json::<crate::types::Jsonb, T>(&value)
    }
}

/// Binds a `jsonb` parameter.
///
/// # Panics
///
/// Panics when `T`'s `Serialize` implementation fails; see
/// [`Json`](drizzle_core::Json#panics).
impl<'a, T: Serialize> ToSQL<'a, PostgresValue<'a>> for Json<T> {
    fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
        SQL::param(PostgresValue::Jsonb(self.encode_json_value()))
    }
}

/// `Json<T>` is a `jsonb` expression; `json` and `jsonb` columns both accept
/// it, and PostgreSQL defines `=` only for `jsonb`.
impl<'a, T: Serialize> Expr<'a, PostgresValue<'a>> for Json<T> {
    type SQLType = crate::types::Jsonb;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

impl<'a, T: Serialize> PostgresInsertValue<'a, PostgresValue<'a>, T> {
    /// Insert value for a JSON column of type `Target` (`json` or `jsonb`).
    ///
    /// # Panics
    ///
    /// Panics when `T`'s `Serialize` implementation fails; see
    /// [`Json`](drizzle_core::Json#panics).
    #[must_use]
    pub fn json<Target: PostgresJsonType>(value: Json<T>) -> Self {
        Self::Value(ValueWrapper::<PostgresValue<'a>, T>::new(SQL::param(
            document::<Target>(value.encode_json_value()),
        )))
    }
}

/// The column's SQL type marker (`Target`) selects `json` or `jsonb`.
impl<'a, T, Target, TargetNull> JsonColumnValue<T>
    for PostgresUpdateValue<'a, PostgresValue<'a>, T, Target, TargetNull>
where
    T: Serialize,
    Target: PostgresJsonType,
    TargetNull: Nullability,
{
    fn from_json(value: Json<T>) -> Self {
        Self::Value(ValueWrapper::<PostgresValue<'a>, T>::new(SQL::param(
            document::<Target>(value.encode_json_value()),
        )))
    }
}

impl<V: SQLParam, T> JsonColumnOperand for PostgresInsertValue<'_, V, T> {}

impl<V, T, Target, TargetNull> JsonColumnOperand
    for PostgresUpdateValue<'_, V, T, Target, TargetNull>
where
    V: SQLParam,
    Target: DataType,
    TargetNull: Nullability,
{
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Json as JsonType, Jsonb};

    #[test]
    fn column_type_selects_the_json_variant() {
        let payload = Json(serde_json::json!({ "a": 1 }));
        assert_eq!(
            PostgresValue::try_from_json::<JsonType, _>(&payload).unwrap(),
            PostgresValue::Json(serde_json::json!({ "a": 1 }))
        );
        assert_eq!(
            PostgresValue::try_from_json::<Jsonb, _>(&payload).unwrap(),
            PostgresValue::Jsonb(serde_json::json!({ "a": 1 }))
        );
    }

    #[test]
    fn expressions_bind_jsonb() {
        let sql = Json(vec![1, 2]).to_sql();
        let params: Vec<_> = sql.params().collect();
        assert_eq!(params, [&PostgresValue::Jsonb(serde_json::json!([1, 2]))]);
    }

    #[test]
    fn serialization_failures_are_errors() {
        let mut map = std::collections::BTreeMap::new();
        map.insert((1, 2), 3);
        assert!(matches!(
            PostgresValue::try_from_json::<Jsonb, _>(&Json(map)),
            Err(DrizzleError::JsonError(_))
        ));
    }
}
