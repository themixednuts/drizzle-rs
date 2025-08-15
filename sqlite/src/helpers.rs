use crate::{common::Join, values::SQLiteValue};
use drizzle_core::{
    SQL, SQLTable, ToSQL, helpers as core_helpers,
    traits::{SQLColumnInfo, SQLModel, SQLParam},
};

// Re-export core helpers with SQLiteValue type for convenience
pub(crate) use core_helpers::{
    delete, from, group_by, having, limit, offset, order_by, select, set, update, r#where,
};

/// Helper to convert column info to SQL for joining (column names only for INSERT)
fn columns_info_to_sql<'a, V>(columns: &[&'static dyn SQLColumnInfo]) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    // For INSERT statements, we need just column names, not fully qualified names
    let joined_names = columns
        .iter()
        .map(|col| col.name())
        .collect::<Vec<_>>()
        .join(", ");
    SQL::raw(joined_names)
}

fn join_internal<'a, T, V>(table: T, join: Join, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    let sql = join.to_sql();
    let sql = sql.append_raw(" ");
    let sql = sql.append(table.to_sql());
    let sql = sql.append_raw(" ON ");
    sql.append(condition)
}

/// Helper function to create a JOIN clause using table generic
pub fn natural_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::default().natural(), condition)
}

/// Helper function to create a JOIN clause using table generic
pub fn join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::default(), condition)
}

pub fn natural_left_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().natural().left(), condition)
}

/// Helper function to create a LEFT JOIN clause using table generic
pub fn left_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().left(), condition)
}

pub fn left_outer_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().left().outer(), condition)
}

pub fn natural_left_outer_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().natural().left().outer(), condition)
}

pub fn natural_right_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().natural().right(), condition)
}

/// Helper function to create a RIGHT JOIN clause using table generic
pub fn right_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().right(), condition)
}

pub fn right_outer_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().right().outer(), condition)
}

pub fn natural_right_outer_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().natural().right().outer(), condition)
}

pub fn natural_full_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().natural().full(), condition)
}

/// Helper function to create a FULL JOIN clause using table generic
pub fn full_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().full(), condition)
}

pub fn full_outer_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().full().outer(), condition)
}

pub fn natural_full_outer_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().natural().full().outer(), condition)
}

pub fn natural_inner_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().natural().inner(), condition)
}

/// Helper function to create an INNER JOIN clause using table generic
pub fn inner_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().inner(), condition)
}

/// Helper function to create a CROSS JOIN clause using table generic
pub fn cross_join<'a, T, V>(table: T, condition: SQL<'a, V>) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    join_internal(table, Join::new().cross(), condition)
}

/// Creates an INSERT INTO statement with the specified table - SQLite specific
pub(crate) fn insert<'a, T>(table: T) -> SQL<'a, SQLiteValue<'a>>
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    SQL::raw("INSERT INTO").append(&table)
}

/// Helper function to create VALUES clause for INSERT with smart batching
/// Groups rows by their column patterns and generates separate batch INSERT statements
pub(crate) fn values<'a, Table, V>(
    rows: impl IntoIterator<Item = <Table as SQLTable<'a, V>>::Insert>,
) -> SQL<'a, V>
where
    Table: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    let mut iter = rows.into_iter();

    match iter.next() {
        None => SQL::raw("VALUES"),
        Some(row) => {
            let rows: Box<_> = std::iter::once(row).chain(iter).collect();

            SQL::join(
                group_rows_by_columns(&rows)
                    .into_iter()
                    .map(|(columns, batch_rows)| generate_batch_insert(columns, batch_rows)),
                "; ",
            )
        }
    }
}

/// Groups rows by their column patterns for efficient batching
fn group_rows_by_columns<'a, V, Row>(rows: &[Row]) -> Vec<(SQL<'a, V>, Vec<&Row>)>
where
    V: SQLParam + 'a,
    Row: SQLModel<'a, V>,
{
    use std::collections::BTreeMap;

    // Use BTreeMap for consistent ordering of column patterns
    let mut groups: BTreeMap<String, (SQL<'a, V>, Vec<&Row>)> = BTreeMap::new();

    for row in rows {
        let columns_info = row.columns();
        let columns_sql = columns_info_to_sql(&columns_info);
        let columns_key = columns_sql.sql(); // Use SQL string as grouping key

        groups
            .entry(columns_key)
            .or_insert_with(|| (columns_sql, Vec::new()))
            .1
            .push(row);
    }

    groups.into_values().collect()
}

/// Generates a batched INSERT statement for rows with the same column pattern  
fn generate_batch_insert<'a, V, Row>(columns: SQL<'a, V>, batch_rows: Vec<&Row>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    Row: SQLModel<'a, V>,
{
    // Check if this batch has no columns (all fields are Omit) - use DEFAULT VALUES
    if batch_rows
        .first()
        .map_or(true, |row| row.columns().is_empty())
    {
        return batch_rows
            .iter()
            .map(|_| SQL::raw("DEFAULT VALUES"))
            .collect::<Vec<_>>()
            .into_iter()
            .reduce(|acc, sql| acc.append_raw("; ").append(sql))
            .unwrap_or(SQL::raw("DEFAULT VALUES"));
    }

    // Generate standard INSERT with columns and values
    let value_clauses = batch_rows
        .iter()
        .map(|row| SQL::raw("(").append(row.values()).append_raw(")"))
        .collect::<Vec<_>>();

    SQL::raw("(")
        .append(columns)
        .append_raw(") VALUES ")
        .append(SQL::join(value_clauses, ", "))
}

/// Helper function to create a RETURNING clause - SQLite specific
pub(crate) fn returning<'a, I>(columns: I) -> SQL<'a, SQLiteValue<'a>>
where
    I: ToSQL<'a, SQLiteValue<'a>>,
{
    SQL::raw("RETURNING").append(columns.to_sql())
}
