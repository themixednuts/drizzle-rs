use crate::{
    common::Join,
    traits::{SQLiteSQL, SQLiteTable, ToSQLiteSQL},
};
use drizzle_core::{
    SQL, ToSQL, Token, helpers as core_helpers,
    traits::{SQLColumnInfo, SQLModel},
};

// Re-export core helpers with SQLiteValue type for convenience
pub(crate) use core_helpers::{
    delete, from, group_by, having, insert, limit, offset, order_by, select, set, update, r#where,
};

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

fn join_internal<'a, Table>(
    table: Table,
    join: Join,
    condition: impl ToSQLiteSQL<'a>,
) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join.to_sql()
        .append(&table)
        .push(Token::ON)
        .append(&condition)
}

/// Helper function to create a JOIN clause using table generic
pub fn natural_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::default().natural(), condition)
}

/// Helper function to create a JOIN clause using table generic
pub fn join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::default(), condition)
}

pub fn natural_left_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().natural().left(), condition)
}

/// Helper function to create a LEFT JOIN clause using table generic
pub fn left_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().left(), condition)
}

pub fn left_outer_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().left().outer(), condition)
}

pub fn natural_left_outer_join<'a, Table>(
    table: Table,
    condition: impl ToSQLiteSQL<'a>,
) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().natural().left().outer(), condition)
}

pub fn natural_right_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().natural().right(), condition)
}

/// Helper function to create a RIGHT JOIN clause using table generic
pub fn right_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().right(), condition)
}

pub fn right_outer_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().right().outer(), condition)
}

pub fn natural_right_outer_join<'a, Table>(
    table: Table,
    condition: impl ToSQLiteSQL<'a>,
) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().natural().right().outer(), condition)
}

pub fn natural_full_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().natural().full(), condition)
}

/// Helper function to create a FULL JOIN clause using table generic
pub fn full_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().full(), condition)
}

pub fn full_outer_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().full().outer(), condition)
}

pub fn natural_full_outer_join<'a, Table>(
    table: Table,
    condition: impl ToSQLiteSQL<'a>,
) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().natural().full().outer(), condition)
}

pub fn natural_inner_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().natural().inner(), condition)
}

/// Helper function to create an INNER JOIN clause using table generic
pub fn inner_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().inner(), condition)
}

/// Helper function to create a CROSS JOIN clause using table generic
pub fn cross_join<'a, Table>(table: Table, condition: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a>
where
    Table: SQLiteTable<'a>,
{
    join_internal(table, Join::new().cross(), condition)
}

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

    // Check if this is a DEFAULT VALUES case (no columns)
    if columns_info.is_empty() {
        return SQL::from_iter([Token::DEFAULT, Token::VALUES]);
    }

    let columns_sql = columns_info_to_sql(&columns_info);
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
