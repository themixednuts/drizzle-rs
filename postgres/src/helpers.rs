use crate::{traits::PostgresTable, values::PostgresValue};
use drizzle_core::{
    Join, SQL, ToSQL, Token, helpers,
    traits::{SQLColumnInfo, SQLModel},
};

// Re-export core helpers with PostgresValue type for convenience
pub(crate) use helpers::{
    delete, except, except_all, from, group_by, having, intersect, intersect_all, limit, offset,
    order_by, select, select_distinct, set, union, union_all, update, r#where,
};

/// Helper to convert column info to SQL for joining (column names only for INSERT)
fn columns_info_to_sql<'a>(columns: &[&'static dyn SQLColumnInfo]) -> SQL<'a, PostgresValue<'a>> {
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
    table_trait: PostgresTable<'a>,
    condition_trait: ToSQL<'a, PostgresValue<'a>>,
    sql_type: SQL<'a, PostgresValue<'a>>,
);

/// Helper function to create a SELECT DISTINCT ON statement (PostgreSQL-specific)
pub(crate) fn select_distinct_on<'a, On, Columns>(
    on: On,
    columns: Columns,
) -> SQL<'a, PostgresValue<'a>>
where
    On: ToSQL<'a, PostgresValue<'a>>,
    Columns: ToSQL<'a, PostgresValue<'a>>,
{
    SQL::from_iter([Token::SELECT, Token::DISTINCT, Token::ON, Token::LPAREN])
        .append(on.to_sql())
        .push(Token::RPAREN)
        .append(columns.to_sql())
}

//------------------------------------------------------------------------------
// USING clause internal helper (PostgreSQL-specific)
//------------------------------------------------------------------------------

fn join_using_internal<'a, Table>(
    table: Table,
    join: Join,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
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

//------------------------------------------------------------------------------
// USING clause versions of JOIN functions (PostgreSQL-specific)
//------------------------------------------------------------------------------

pub fn join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new(), columns)
}

pub fn inner_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().inner(), columns)
}

pub fn left_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().left(), columns)
}

pub fn left_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().left().outer(), columns)
}

pub fn right_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().right(), columns)
}

pub fn right_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().right().outer(), columns)
}

pub fn full_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
where
    Table: PostgresTable<'a>,
{
    join_using_internal(table, Join::new().full(), columns)
}

pub fn full_outer_join_using<'a, Table>(
    table: Table,
    columns: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>>
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

/// Helper function to create a RETURNING clause - PostgreSQL specific
pub(crate) fn returning<'a, 'b, I>(columns: I) -> SQL<'a, PostgresValue<'a>>
where
    I: ToSQL<'a, PostgresValue<'a>>,
{
    SQL::from(Token::RETURNING).append(columns.to_sql())
}

/// Helper function to create an UPSERT (ON CONFLICT) clause - PostgreSQL specific
#[allow(dead_code)]
pub(crate) fn on_conflict<'a>(
    conflict_target: Option<SQL<'a, PostgresValue<'a>>>,
    action: impl ToSQL<'a, PostgresValue<'a>>,
) -> SQL<'a, PostgresValue<'a>> {
    let mut sql = SQL::from_iter([Token::ON, Token::CONFLICT]);

    if let Some(target) = conflict_target {
        sql = sql.push(Token::LPAREN).append(target).push(Token::RPAREN);
    }

    sql.append(action.to_sql())
}

//------------------------------------------------------------------------------
// FOR UPDATE/SHARE row locking (PostgreSQL-specific)
//------------------------------------------------------------------------------

/// Helper function to create a FOR UPDATE clause
pub(crate) fn for_update<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::UPDATE])
}

/// Helper function to create a FOR SHARE clause
pub(crate) fn for_share<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::SHARE])
}

/// Helper function to create a FOR NO KEY UPDATE clause
pub(crate) fn for_no_key_update<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::NO, Token::KEY, Token::UPDATE])
}

/// Helper function to create a FOR KEY SHARE clause
pub(crate) fn for_key_share<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::KEY, Token::SHARE])
}

/// Helper function to create a FOR UPDATE OF table clause
/// Note: Uses UNQUALIFIED table name per drizzle-orm beta-12 fix (#4950)
pub(crate) fn for_update_of<'a>(table_name: &str) -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::UPDATE, Token::OF]).append(SQL::ident(table_name.to_owned()))
}

/// Helper function to create a FOR SHARE OF table clause
/// Note: Uses UNQUALIFIED table name per drizzle-orm beta-12 fix (#4950)
pub(crate) fn for_share_of<'a>(table_name: &str) -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::FOR, Token::SHARE, Token::OF]).append(SQL::ident(table_name.to_owned()))
}

/// Helper function to add NOWAIT to a FOR clause
pub(crate) fn nowait<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from(Token::NOWAIT)
}

/// Helper function to add SKIP LOCKED to a FOR clause
pub(crate) fn skip_locked<'a>() -> SQL<'a, PostgresValue<'a>> {
    SQL::from_iter([Token::SKIP, Token::LOCKED])
}
