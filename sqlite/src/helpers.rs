use crate::values::SQLiteValue;
use drizzle_core::{
    Join, OrderBy, SQL, SQLSchema, SQLSchemaType, SQLTable, ToSQL, helpers as core_helpers,
    traits::{SQLModel, SQLParam},
};

// Re-export core helpers with SQLiteValue type for convenience
pub(crate) use core_helpers::{
    delete, from, group_by, having, limit, offset, order_by, select, set, update, where_clause,
};

/// Helper function to create a JOIN clause using table generic - SQLite-specific wrapper
pub(crate) fn join<'a, T>(
    join_type: Join,
    condition: SQL<'a, SQLiteValue<'a>>,
) -> SQL<'a, SQLiteValue<'a>>
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    core_helpers::join::<T, SQLiteValue>(join_type, condition)
}

/// Helper function to create an INNER JOIN clause using table generic - SQLite wrapper
pub(crate) fn inner_join<'a, T>(condition: SQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>>
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    core_helpers::inner_join::<T, SQLiteValue>(condition)
}

/// Helper function to create a LEFT JOIN clause using table generic - SQLite wrapper
pub(crate) fn left_join<'a, T>(condition: SQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>>
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    core_helpers::left_join::<T, SQLiteValue>(condition)
}

/// Helper function to create a RIGHT JOIN clause using table generic - SQLite wrapper
pub(crate) fn right_join<'a, T>(condition: SQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>>
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    core_helpers::right_join::<T, SQLiteValue>(condition)
}

/// Helper function to create a FULL JOIN clause - SQLite specific (not actually supported in SQLite)
/// This exists for API compatibility but will generate invalid SQL in SQLite
pub(crate) fn full_join<'a, T>(condition: SQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>>
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    // SQLite doesn't actually support FULL JOIN, but we'll generate it anyway
    // This should probably emit a warning or error in real usage
    core_helpers::join::<T, SQLiteValue>(Join::Full, condition)
}

/// Creates an INSERT INTO statement with the specified table - SQLite specific
pub(crate) fn insert<'a, T>() -> SQL<'a, SQLiteValue<'a>>
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    let sql = SQL::raw("INSERT INTO");
    sql.append_raw(T::Schema::NAME)
}

/// Helper function to create VALUES clause for INSERT
pub(crate) fn values<'a, Table, V>(
    rows: impl IntoIterator<Item = <Table as SQLTable<'a, V>>::Insert>,
) -> SQL<'a, V>
where
    Table: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    let rows_vec: Vec<_> = rows.into_iter().collect();
    if rows_vec.is_empty() {
        return SQL::raw("VALUES");
    }

    // Get column names from the first row
    let columns = rows_vec[0].columns();

    // Generate value rows, each row uses values() method
    let value_rows: Vec<SQL<'a, V>> = rows_vec
        .iter()
        .map(|row| SQL::raw("(").append(row.values()).append_raw(")"))
        .collect();

    // Return: (col1, col2) VALUES (?, ?), (?, ?)
    SQL::raw("(")
        .append(columns)
        .append_raw(") VALUES ")
        .append(SQL::join(value_rows, ", "))
}

/// Helper function to create a RETURNING clause - SQLite specific
pub(crate) fn returning<'a>(columns: Vec<SQL<'a, SQLiteValue<'a>>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("RETURNING");

    if columns.is_empty() {
        return sql.append_raw("*");
    }

    sql.append(SQL::join(columns, ", "))
}
