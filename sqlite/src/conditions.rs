// SQLite specific condition functions, particularly for JSON

use crate::values::SQLiteValue;
use drizzle_core::{SQL, ToSQL};

// JSON field equality - column->>'field' = value (using SQLite ->> operator for text comparison)
pub fn json_eq<'a, L, R>(left: L, field: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(format!("->>'{}' = ", field)))
        .append(value.to_sql())
}

// JSON field inequality - column->>'field' != value
pub fn json_ne<'a, L, R>(left: L, field: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(format!("->>'{}' != ", field)))
        .append(value.to_sql())
}

// JSON field contains - json_extract(column, '$.field') = value
pub fn json_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') = ", path)))
        .append(value.to_sql())
}

// JSON field exists - json_type(column, '$.field') IS NOT NULL
pub fn json_exists<'a, L>(left: L, path: &'a str) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("json_type(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') IS NOT NULL", path)))
}

// JSON field does not exist - json_type(column, '$.field') IS NULL
pub fn json_not_exists<'a, L>(left: L, path: &'a str) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("json_type(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') IS NULL", path)))
}

// JSON array contains value - EXISTS(SELECT 1 FROM json_each(column, '$.path') WHERE value = ?)
pub fn json_array_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("EXISTS(SELECT 1 FROM json_each(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') WHERE value = ", path)))
        .append(value.to_sql())
        .append_raw(")")
}

// JSON object contains key - json_type(column, '$.path.key') IS NOT NULL
pub fn json_object_contains_key<'a, L>(
    left: L,
    path: &'a str,
    key: &'a str,
) -> SQL<'a, SQLiteValue<'a>>
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

// JSON text search in value - instr(lower(json_extract(column, '$.path')), lower(?)) > 0
pub fn json_text_contains<'a, L, R>(left: L, path: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("instr(lower(json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}'))), lower(", path)))
        .append(value.to_sql())
        .append_raw(")) > 0")
}

// JSON comparison functions for numbers
pub fn json_gt<'a, L, R>(left: L, path: &'a str, value: R) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
    R: Into<SQLiteValue<'a>> + ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("CAST(json_extract(")
        .append(left.to_sql())
        .append(SQL::raw(format!(", '{}') AS NUMERIC) > ", path)))
        .append(value.to_sql())
}

// Helper function for the JSON arrow-arrow operators (extract as Value)
pub fn json_extract<'a, L>(left: L, path: impl AsRef<str>) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql()
        .append(SQL::raw(format!("->>'{}'", path.as_ref())))
}

// Helper function for the JSON arrow operators (extract as JSON text)
pub fn json_extract_text<'a, L>(left: L, path: &'a str) -> SQL<'a, SQLiteValue<'a>>
where
    L: ToSQL<'a, SQLiteValue<'a>>,
{
    left.to_sql().append(SQL::raw(format!("->'{}'", path)))
}
