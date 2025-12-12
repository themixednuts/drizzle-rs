use crate::traits::{SQLiteSQL, SQLiteTable, ToSQLiteSQL};
use drizzle_core::{
    SQL, Token, helpers as core_helpers,
    traits::{SQLColumnInfo, SQLModel},
};

// Re-export core helpers with SQLiteValue type for convenience
pub(crate) use core_helpers::{
    delete, from, group_by, having, insert, limit, offset, order_by, select, set, update, r#where,
};

// Re-export Join from core
pub use drizzle_core::Join;

/// Helper to convert column info to SQL for joining (column names only for INSERT)
fn columns_info_to_sql<'a>(columns: &[&'static dyn SQLColumnInfo]) -> SQLiteSQL<'a> {
    // For INSERT statements, we need just column names, not fully qualified names
    let joined_names = columns
        .iter()
        .map(|col| col.name())
        .collect::<Vec<_>>()
        .join(", ");
    SQL::raw(joined_names)
}

// Generate all join helper functions using the shared macro
drizzle_core::impl_join_helpers!(
    table_trait: SQLiteTable<'a>,
    condition_trait: ToSQLiteSQL<'a>,
    sql_type: SQLiteSQL<'a>,
);

/// Helper function to create VALUES clause for INSERT with pattern validation
/// All rows must have the same column pattern (enforced by type parameter)
pub(crate) fn values<'a, Table, T>(
    rows: impl IntoIterator<Item = Table::Insert<T>>,
) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a> + Default,
{
    let rows: Vec<_> = rows.into_iter().collect();

    if rows.is_empty() {
        return SQL::from(Token::VALUES);
    }

    // Since all rows have the same PATTERN, they all have the same columns
    // Get column info from the first row (all rows will have the same columns)
    let columns_info = rows[0].columns();
    let columns_slice = columns_info.as_ref();

    // Check if this is a DEFAULT VALUES case (no columns)
    if columns_slice.is_empty() {
        return SQL::from_iter([Token::DEFAULT, Token::VALUES]);
    }

    let columns_sql = columns_info_to_sql(columns_slice);
    let value_clauses = rows.iter().map(|row| {
        SQL::from(Token::LPAREN)
            .append(row.values())
            .push(Token::RPAREN)
    });

    columns_sql
        .parens()
        .push(Token::VALUES)
        .append(SQL::join(value_clauses, Token::COMMA))
}

/// Helper function to create a RETURNING clause - SQLite specific
pub(crate) fn returning<'a, 'b, I>(columns: I) -> SQLiteSQL<'a>
where
    I: ToSQLiteSQL<'a>,
{
    SQL::from(Token::RETURNING).append(&columns)
}
