use crate::traits::{SQLiteTable, SQLiteTableInfo};
use crate::values::SQLiteValue;
use drizzle_core::{
    Joinable, SQL, Token, helpers as core_helpers,
    traits::{SQLColumnInfo, SQLModel, ToSQL},
};

// Re-export core helpers with SQLiteValue type for convenience
pub(crate) use core_helpers::{
    delete, except, except_all, from, group_by, having, insert, intersect, intersect_all, limit,
    offset, order_by, select, select_distinct, set, union, union_all, update, r#where,
};

// Re-export Join from core
pub use drizzle_core::Join;

// =============================================================================
// JoinArg trait — dispatch between bare table (auto-FK) and (table, condition)
// =============================================================================

/// Trait for arguments accepted by `.join()` and related join methods.
///
/// Implemented for:
/// - **`(table, condition)`** — explicit ON condition (always available)
/// - **bare table** — auto-derives ON condition from FK metadata (requires `Joinable` bound)
pub trait JoinArg<'a, FromTable> {
    type JoinedTable;
    fn into_join_sql(self, join: Join) -> SQL<'a, SQLiteValue<'a>>;
}

/// Bare table: derives the ON condition from `Joinable::fk_columns()`.
impl<'a, U, T> JoinArg<'a, T> for U
where
    U: SQLiteTable<'a> + Joinable<T>,
    T: SQLiteTableInfo + Default,
{
    type JoinedTable = U;
    fn into_join_sql(self, join: Join) -> SQL<'a, SQLiteValue<'a>> {
        use drizzle_core::ToSQL;
        let from = T::default();
        let cols = <U as Joinable<T>>::fk_columns();
        let join_name = self.name();
        let from_name = from.name();

        let mut condition = SQL::with_capacity_chunks(cols.len() * 7);
        for (idx, (self_col, target_col)) in cols.iter().enumerate() {
            if idx > 0 {
                condition.push_mut(Token::AND);
            }
            condition.append_mut(
                SQL::ident(join_name.to_string())
                    .push(Token::DOT)
                    .append(SQL::ident(self_col.to_string())),
            );
            condition.push_mut(Token::EQ);
            condition.append_mut(
                SQL::ident(from_name.to_string())
                    .push(Token::DOT)
                    .append(SQL::ident(target_col.to_string())),
            );
        }

        join.to_sql()
            .append(&self)
            .push(Token::ON)
            .append(&condition)
    }
}

/// Tuple `(table, condition)`: explicit ON condition.
impl<'a, U, C, T> JoinArg<'a, T> for (U, C)
where
    U: SQLiteTable<'a>,
    C: ToSQL<'a, SQLiteValue<'a>>,
{
    type JoinedTable = U;
    fn into_join_sql(self, join: Join) -> SQL<'a, SQLiteValue<'a>> {
        let (table, condition) = self;
        join_internal(table, join, condition)
    }
}

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
