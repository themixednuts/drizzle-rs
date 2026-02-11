use crate::traits::SQLiteTable;
use crate::values::SQLiteValue;
use drizzle_core::{
    SQL, Token, helpers as core_helpers,
    traits::{SQLColumnInfo, SQLModel, ToSQL},
};

// Re-export core helpers with SQLiteValue type for convenience
pub(crate) use core_helpers::{
    delete, except, except_all, from, group_by, having, insert, intersect, intersect_all, limit,
    offset, order_by, select, select_distinct, set, union, union_all, update, r#where,
};

// Re-export Join from core
pub use drizzle_core::Join;

/// Helper to convert column info to SQL for joining (column names only for INSERT)
fn columns_info_to_sql<'a>(columns: &[&'static dyn SQLColumnInfo]) -> SQL<'a, SQLiteValue<'a>> {
    let mut sql = SQL::with_capacity_chunks(columns.len().saturating_mul(2));
    for (idx, col) in columns.iter().enumerate() {
        if idx > 0 {
            sql.push_mut(Token::COMMA);
        }
        sql.append_mut(SQL::ident(col.name()));
    }
    sql
}

// Generate all join helper functions using the shared macro
drizzle_core::impl_join_helpers!(
    table_trait: SQLiteTable<'a>,
    condition_trait: ToSQL<'a, SQLiteValue<'a>>,
    sql_type: SQL<'a, SQLiteValue<'a>>,
);

/// Helper function to create VALUES clause for INSERT with pattern validation
/// All rows must have the same column pattern (enforced by type parameter)
pub(crate) fn values<'a, Table, T>(
    rows: impl IntoIterator<Item = Table::Insert<T>>,
) -> SQL<'a, SQLiteValue<'a>>
where
    Table: SQLiteTable<'a> + Default,
{
    let rows: Vec<Table::Insert<T>> = rows.into_iter().collect();

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
    let mut values_sql = SQL::with_capacity_chunks(rows.len().saturating_mul(4));
    for (idx, row) in rows.iter().enumerate() {
        if idx > 0 {
            values_sql.push_mut(Token::COMMA);
        }
        values_sql.push_mut(Token::LPAREN);
        values_sql.append_mut(row.values());
        values_sql.push_mut(Token::RPAREN);
    }

    columns_sql.parens().push(Token::VALUES).append(values_sql)
}

/// Helper function to create a RETURNING clause - SQLite specific
pub(crate) fn returning<'a, 'b, I>(columns: I) -> SQL<'a, SQLiteValue<'a>>
where
    I: ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::from(Token::RETURNING).append(&columns)
}
