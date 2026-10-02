//! `SQLite` JSON functions and conditions.
//!
//! These helpers build untyped [`SQL`] fragments for JSON stored in TEXT or
//! BLOB columns. JSON paths, keys and values are sent as bound parameters.
//! For standard, typed expressions use `drizzle_core::expr`.

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::values::SQLiteValue;
use drizzle_core::{SQL, ToSQL};

/// Wraps `value` in `json(..)`, which checks that it is valid JSON and
/// returns it as minified JSON text.
///
/// # Examples
///
/// ```
/// # use drizzle_sqlite::expr::json;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// let expr = json(SQL::<SQLiteValue>::raw("metadata"));
/// assert_eq!(expr.sql(), "json (metadata)");
/// ```
pub fn json<'a>(value: impl ToSQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    SQL::func("json", value.to_sql())
}

/// Wraps `value` in `jsonb(..)`, which checks that it is valid JSON and
/// returns it in `SQLite`'s binary JSONB format.
///
/// # Examples
///
/// ```
/// # use drizzle_sqlite::expr::jsonb;
/// # use drizzle_core::SQL;
/// # use drizzle_sqlite::values::SQLiteValue;
/// let expr = jsonb(SQL::<SQLiteValue>::raw("metadata"));
/// assert_eq!(expr.sql(), "jsonb (metadata)");
/// ```
pub fn jsonb<'a>(value: impl ToSQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    SQL::func("jsonb", value.to_sql())
}

/// Builds `left ->> field = value`: the JSON field `field` equals `value`.
///
/// `field` may be a key name (`"theme"`) or a JSON path (`"$.theme"`).
///
/// # Examples
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

/// Builds `left ->> field != value`: the JSON field `field` does not equal
/// `value`.
///
/// `field` may be a key name or a JSON path. Like any SQL comparison, this
/// is not true when the field is missing (`NULL`).
///
/// # Examples
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

/// Builds `json_extract(left, path) = value`: the value at `path` equals
/// `value`.
///
/// Despite the name, this is an equality test, not a substring or
/// array-membership test. Use [`json_array_contains`] or
/// [`json_text_contains`] for those.
///
/// # Examples
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

/// Builds `json_type(left, path) IS NOT NULL`: the JSON document has a
/// value at `path` (a JSON `null` counts as present).
///
/// # Examples
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

/// Builds `json_type(left, path) IS NULL`: the JSON document has no value
/// at `path`.
///
/// # Examples
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

/// Builds an `EXISTS` test that is true when the JSON array at `path`
/// has an element equal to `value`.
///
/// # Examples
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

/// Builds a test that is true when the JSON object at `path` has the key
/// `key`.
///
/// The key is appended to the path (`"$"` and `""` mean the root object),
/// and the result is checked with `json_type(..) IS NOT NULL`. `key` is not
/// quoted, so it must be a plain key name.
///
/// # Examples
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

/// Builds a case-insensitive substring test: the text at `path` contains
/// `value`.
///
/// Uses `instr(lower(..), lower(..)) > 0`, so case folding only applies to
/// ASCII letters.
///
/// # Examples
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

/// Builds `CAST(json_extract(left, path) AS NUMERIC) > value`: the number
/// at `path` is greater than `value`.
///
/// # Examples
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

/// Builds `left ->> path`, which returns the value at `path` as an SQL
/// value (TEXT, INTEGER, REAL or NULL).
///
/// `path` may be a key name or a JSON path. Use [`json_extract_text`] to
/// get JSON text instead.
///
/// # Examples
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

/// Builds `left -> path`, which returns the value at `path` as JSON text
/// (strings stay quoted, objects and arrays stay JSON).
///
/// Use [`json_extract`] to get a plain SQL value instead.
///
/// # Examples
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
