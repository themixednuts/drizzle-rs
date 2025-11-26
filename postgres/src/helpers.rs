use crate::{
    PostgresSQL, ToPostgresSQL, common::Join, traits::PostgresTable, values::PostgresValue,
};
use drizzle_core::{
    SQL, ToSQL, Token, helpers,
    traits::{SQLColumnInfo, SQLModel},
};

// Re-export core helpers with PostgresValue type for convenience
pub(crate) use helpers::{
    delete, from, group_by, having, limit, offset, order_by, select, set, update, r#where,
};

/// Helper to convert column info to SQL for joining (column names only for INSERT)
fn columns_info_to_sql<'a>(columns: &[&'static dyn SQLColumnInfo]) -> PostgresSQL<'a> {
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
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join.to_sql()
        .append(table.to_sql())
        .push(Token::ON)
        .append(condition.to_sql())
}

fn join_using_internal<'a, Table>(
    table: Table,
    join: Join,
    columns: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join.to_sql()
        .append(table.to_sql())
        .push(Token::USING)
        .push(Token::LPAREN)
        .append(columns.to_sql())
        .push(Token::RPAREN)
}

/// Helper function to create a JOIN clause using table generic
pub fn natural_join<'a, Table>(table: Table, condition: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::default().natural(), condition)
}

/// Helper function to create a JOIN clause using table generic
pub fn join<'a, Table>(table: Table, condition: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::default(), condition)
}

pub fn natural_left_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().natural().left(), condition)
}

/// Helper function to create a LEFT JOIN clause using table generic
pub fn left_join<'a, Table>(table: Table, condition: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().left(), condition)
}

pub fn left_outer_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().left().outer(), condition)
}

pub fn natural_left_outer_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().natural().left().outer(), condition)
}

pub fn natural_right_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().natural().right(), condition)
}

/// Helper function to create a RIGHT JOIN clause using table generic
pub fn right_join<'a, Table>(table: Table, condition: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().right(), condition)
}

pub fn right_outer_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().right().outer(), condition)
}

pub fn natural_right_outer_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().natural().right().outer(), condition)
}

pub fn natural_full_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().natural().full(), condition)
}

/// Helper function to create a FULL JOIN clause using table generic
pub fn full_join<'a, Table>(table: Table, condition: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().full(), condition)
}

pub fn full_outer_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().full().outer(), condition)
}

pub fn natural_full_outer_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().natural().full().outer(), condition)
}

pub fn natural_inner_join<'a, Table>(
    table: Table,
    condition: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().natural().inner(), condition)
}

/// Helper function to create an INNER JOIN clause using table generic
pub fn inner_join<'a, Table>(table: Table, condition: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().inner(), condition)
}

/// Helper function to create a CROSS JOIN clause using table generic
pub fn cross_join<'a, Table>(table: Table, condition: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_internal(table, Join::new().cross(), condition)
}

//------------------------------------------------------------------------------
// USING clause versions of JOIN functions (PostgreSQL-specific)
//------------------------------------------------------------------------------

pub fn join_using<'a, Table>(table: Table, columns: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new(), columns)
}

pub fn inner_join_using<'a, Table>(table: Table, columns: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().inner(), columns)
}

pub fn left_join_using<'a, Table>(table: Table, columns: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().left(), columns)
}

pub fn left_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().left().outer(), columns)
}

pub fn right_join_using<'a, Table>(table: Table, columns: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().right(), columns)
}

pub fn right_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().right().outer(), columns)
}

pub fn full_join_using<'a, Table>(table: Table, columns: impl ToPostgresSQL<'a>) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().full(), columns)
}

pub fn full_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().full().outer(), columns)
}

// Note: NATURAL JOINs don't use USING clause as they automatically match column names
// CROSS JOIN also doesn't use USING clause as it produces Cartesian product

/// Creates an INSERT INTO statement with the specified table - PostgreSQL specific
pub(crate) fn insert<'a, Table>(table: Table) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    SQL::from_iter([Token::INSERT, Token::INTO]).append(&table)
}

/// Helper function to create VALUES clause for INSERT with pattern validation
/// All rows must have the same column pattern (enforced by type parameter)
pub(crate) fn values<'a, Table, T>(
    rows: impl IntoIterator<Item = Table::Insert<T>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
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
    let value_clauses: Vec<_> = rows.iter().map(|row| row.values().parens()).collect();

    columns_sql
        .parens()
        .push(Token::VALUES)
        .append(SQL::join(value_clauses, Token::COMMA))
}

/// Helper function to create a RETURNING clause - PostgreSQL specific
pub(crate) fn returning<'a, 'b, I>(columns: I) -> PostgresSQL<'a>
where
    I: ToPostgresSQL<'a>,
{
    SQL::from(Token::RETURNING).append(columns.to_sql())
}

/// Helper function to create an UPSERT (ON CONFLICT) clause - PostgreSQL specific
pub(crate) fn on_conflict<'a, T>(
    conflict_target: Option<PostgresSQL<'a>>,
    action: impl ToPostgresSQL<'a>,
) -> PostgresSQL<'a> {
    let mut sql = SQL::from_iter([Token::ON, Token::CONFLICT]);

    if let Some(target) = conflict_target {
        sql = sql.push(Token::LPAREN).append(target).push(Token::RPAREN);
    }

    sql.append(action.to_sql())
}
