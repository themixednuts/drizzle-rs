//! SQLite-specific SQL expressions and JSON helpers.
//!
//! This module provides `SQLite` dialect functions and JSON expressions.
//! For standard SQL expressions, use `drizzle_core::expr`.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::values::SQLiteValue;
use drizzle_core::{SQL, ToSQL};

/// Wraps a value with the `SQLite` `json()` function, validating and returning JSON text.
pub fn json<'a>(value: impl ToSQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    SQL::func("json", value.to_sql())
}

/// Wraps a value with the `SQLite` `jsonb()` function, validating and returning JSON in binary format.
pub fn jsonb<'a>(value: impl ToSQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    SQL::func("jsonb", value.to_sql())
}

/// Create a JSON field equality condition using `SQLite` ->> operator
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_eq;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_eq(column, "theme", "dark");
/// assert_eq!(condition.sql(), "metadata ->> ? = ?");
/// # }
/// ```
pub fn json_eq<'a, L, R>(left: L, field: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(" ->> "))
        .append(SQL::param(SQLiteValue::from(field)))
        .append(SQL::raw(" = "))
        .append(SQL::param(value.into()))
}

/// Create a JSON field inequality condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_ne;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_ne(column, "theme", "light");
/// assert_eq!(condition.sql(), "metadata ->> ? != ?");
/// # }
/// ```
pub fn json_ne<'a, L, R>(left: L, field: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(" ->> "))
        .append(SQL::param(SQLiteValue::from(field)))
        .append(SQL::raw(" != "))
        .append(SQL::param(value.into()))
}

/// Create a JSON field contains condition using `json_extract`
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_contains;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_contains(column, "$.preferences[0]", "dark_theme");
/// assert_eq!(condition.sql(), "json_extract( metadata , ? ) = ?");
/// # }
/// ```
pub fn json_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>>,
{
    SQL::raw("json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(", "))
        .append(SQL::param(SQLiteValue::from(path)))
        .append(SQL::raw(") = "))
        .append(SQL::param(value.into()))
}

/// Create a JSON field exists condition using `json_type`
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_exists;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_exists(column, "$.theme");
/// assert_eq!(condition.sql(), "json_type( metadata , ? ) IS NOT NULL");
/// # }
/// ```
pub fn json_exists<'a, L>(left: L, path: &'a str) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("json_type(")
        .append(left.to_sql())
        .append(SQL::raw(", "))
        .append(SQL::param(SQLiteValue::from(path)))
        .append(SQL::raw(") IS NOT NULL"))
}

/// Create a JSON field does not exist condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_not_exists;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_not_exists(column, "$.theme");
/// assert_eq!(condition.sql(), "json_type( metadata , ? ) IS NULL");
/// # }
/// ```
pub fn json_not_exists<'a, L>(left: L, path: &'a str) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("json_type(")
        .append(left.to_sql())
        .append(SQL::raw(", "))
        .append(SQL::param(SQLiteValue::from(path)))
        .append(SQL::raw(") IS NULL"))
}

/// Create a JSON array contains value condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_array_contains;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_array_contains(column, "$.preferences", "dark_theme");
/// assert_eq!(
///     condition.sql(),
///     "EXISTS(SELECT 1 FROM json_each( metadata , ? ) WHERE value = ? )"
/// );
/// # }
/// ```
pub fn json_array_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>>,
{
    SQL::raw("EXISTS(SELECT 1 FROM json_each(")
        .append(left.to_sql())
        .append(SQL::raw(", "))
        .append(SQL::param(SQLiteValue::from(path)))
        .append(SQL::raw(") WHERE value = "))
        .append(SQL::param(value.into()))
        .append(SQL::raw(")"))
}

/// Create a JSON object contains key condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_object_contains_key;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_object_contains_key(column, "$", "theme");
/// assert_eq!(condition.sql(), "json_type( metadata , ? ) IS NOT NULL");
/// # }
/// ```
pub fn json_object_contains_key<'a, L>(
    left: L,
    path: &'a str,
    key: &'a str,
) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    let full_path = if path.ends_with('$') || path.is_empty() {
        format!("$.{key}")
    } else {
        format!("{path}.{key}")
    };

    SQL::raw("json_type(")
        .append(left.to_sql())
        .append(SQL::raw(", "))
        .append(SQL::param(SQLiteValue::from(full_path)))
        .append(SQL::raw(") IS NOT NULL"))
}

/// Create a JSON text search condition using case-insensitive matching
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_text_contains;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_text_contains(column, "$.description", "user");
/// assert_eq!(
///     condition.sql(),
///     "instr(lower(json_extract( metadata , ? )), lower( ? )) > 0"
/// );
/// # }
/// ```
pub fn json_text_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>>,
{
    SQL::raw("instr(lower(json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(", "))
        .append(SQL::param(SQLiteValue::from(path)))
        .append(SQL::raw(")), lower("))
        .append(SQL::param(value.into()))
        .append(SQL::raw(")) > 0"))
}

/// Create a JSON numeric greater-than condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_gt;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_gt(column, "$.score", 85.0);
/// assert_eq!(condition.sql(), "CAST(json_extract( metadata , ? ) AS NUMERIC) > ?");
/// ```
pub fn json_gt<'a, L, R>(left: L, path: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>>,
{
    SQL::raw("CAST(json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(", "))
        .append(SQL::param(SQLiteValue::from(path)))
        .append(SQL::raw(") AS NUMERIC) > "))
        .append(SQL::param(value.into()))
}

/// Helper function for JSON extraction using ->> operator
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_extract;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let extract_expr = json_extract(column, "theme");
/// assert_eq!(extract_expr.sql(), "metadata ->> ?");
/// # }
/// ```
pub fn json_extract<'a, L>(left: L, path: impl AsRef<str>) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(" ->> "))
        .append(SQL::param(SQLiteValue::from(path.as_ref().to_owned())))
}

/// Helper function for JSON extraction as JSON text using -> operator
///
/// # Example
/// ```
/// # use drizzle_sqlite::expr::json_extract_text;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let extract_expr = json_extract_text(column, "preferences");
/// assert_eq!(extract_expr.sql(), "metadata -> ?");
/// # }
/// ```
pub fn json_extract_text<'a, L>(left: L, path: &'a str) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(" -> "))
        .append(SQL::param(SQLiteValue::from(path)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_paths_are_parameters_in_stable_order() {
        let expression = json_contains(
            SQL::param(SQLiteValue::from("document")),
            "$.preferences['quoted']",
            "dark",
        );

        assert_eq!(expression.sql(), "json_extract( ? , ? ) = ?");
        let document = SQLiteValue::from("document");
        let path = SQLiteValue::from("$.preferences['quoted']");
        let value = SQLiteValue::from("dark");
        assert_eq!(
            expression.params().collect::<Vec<_>>(),
            vec![&document, &path, &value]
        );
    }

    #[test]
    fn json_object_key_is_bound_as_data() {
        let expression =
            json_object_contains_key(SQL::<SQLiteValue>::raw("metadata"), "$", "quote'key");

        assert_eq!(expression.sql(), "json_type( metadata , ? ) IS NOT NULL");
        let path = SQLiteValue::from("$.quote'key");
        assert_eq!(expression.params().collect::<Vec<_>>(), vec![&path]);
    }
}
