// SQLite specific condition functions, particularly for JSON

use crate::{traits::SQLiteSQL, values::SQLiteValue};
use drizzle_core::{SQL, ToSQL};

/// Create a JSON field equality condition using SQLite ->> operator
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_eq;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_eq(column, "theme", "dark");
/// assert_eq!(condition.sql(), "metadata ->>'theme' = ?");
/// # }
/// ```
pub fn json_eq<'a, L, R>(left: L, field: &'a str, value: R) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(format!("->>'{}' = ", field)))
        .append(value.to_sql())
}

/// Create a JSON field inequality condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_ne;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_ne(column, "theme", "light");
/// assert_eq!(condition.sql(), "metadata ->>'theme' != ?");
/// # }
/// ```
pub fn json_ne<'a, L, R>(left: L, field: &'a str, value: R) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(format!("->>'{}' != ", field)))
        .append(value.to_sql())
}

/// Create a JSON field contains condition using json_extract
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_contains;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_contains(column, "$.preferences[0]", "dark_theme");
/// assert_eq!(condition.sql(), "json_extract( metadata , '$.preferences[0]') = ?");
/// # }
/// ```
pub fn json_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') = ", path)))
        .append(value.to_sql())
}

/// Create a JSON field exists condition using json_type
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_exists;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_exists(column, "$.theme");
/// assert_eq!(condition.sql(), "json_type( metadata , '$.theme') IS NOT NULL");
/// # }
/// ```
pub fn json_exists<'a, L>(left: L, path: &'a str) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("json_type(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') IS NOT NULL", path)))
}

/// Create a JSON field does not exist condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_not_exists;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_not_exists(column, "$.theme");
/// assert_eq!(condition.sql(), "json_type( metadata , '$.theme') IS NULL");
/// # }
/// ```
pub fn json_not_exists<'a, L>(left: L, path: &'a str) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("json_type(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') IS NULL", path)))
}

/// Create a JSON array contains value condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_array_contains;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_array_contains(column, "$.preferences", "dark_theme");
/// assert_eq!(condition.sql(), "EXISTS(SELECT 1 FROM json_each( metadata , '$.preferences') WHERE value = ? )");
/// # }
/// ```
pub fn json_array_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("EXISTS(SELECT 1 FROM json_each(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') WHERE value = ", path)))
        .append(value.to_sql())
        .append(SQL::raw(")"))
}

/// Create a JSON object contains key condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_object_contains_key;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_object_contains_key(column, "$", "theme");
/// assert_eq!(condition.sql(), "json_type( metadata , '$.theme') IS NOT NULL");
/// # }
/// ```
pub fn json_object_contains_key<'a, L>(left: L, path: &'a str, key: &'a str) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    let full_path = if path.ends_with('$') || path.is_empty() {
        format!("$.{}", key)
    } else {
        format!("{}.{}", path, key)
    };

    SQL::raw("json_type(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') IS NOT NULL", full_path)))
}

/// Create a JSON text search condition using case-insensitive matching
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_text_contains;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_text_contains(column, "$.description", "user");
/// assert_eq!(condition.sql(), "instr(lower(json_extract( metadata , '$.description'))), lower( ? )) > 0");
/// # }
/// ```
pub fn json_text_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("instr(lower(json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}'))), lower(", path)))
        .append(value.to_sql())
        .append(SQL::raw(")) > 0"))
}

/// Create a JSON numeric greater-than condition
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_gt;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let condition = json_gt(column, "$.score", 85.0);
/// assert_eq!(condition.sql(), "CAST(json_extract( metadata , '$.score') AS NUMERIC) > ?");
/// ```
pub fn json_gt<'a, L, R>(left: L, path: &'a str, value: R) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("CAST(json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') AS NUMERIC) > ", path)))
        .append(value.to_sql())
}

/// Helper function for JSON extraction using ->> operator
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_extract;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let extract_expr = json_extract(column, "theme");
/// assert_eq!(extract_expr.sql(), "metadata ->>'theme'");
/// # }
/// ```
pub fn json_extract<'a, L>(left: L, path: impl AsRef<str>) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(format!("->>'{}'", path.as_ref())))
}

/// Helper function for JSON extraction as JSON text using -> operator
///
/// # Example
/// ```
/// # use drizzle_sqlite::conditions::json_extract_text;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// # fn main() {
/// let column = SQL::<SQLiteValue>::raw("metadata");
/// let extract_expr = json_extract_text(column, "preferences");
/// assert_eq!(extract_expr.sql(), "metadata ->'preferences'");
/// # }
/// ```
pub fn json_extract_text<'a, L>(left: L, path: &'a str) -> SQLiteSQL<'a>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql().append(SQL::raw(format!("->'{}'", path)))
}
