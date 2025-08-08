use crate::{common::Join, values::SQLiteValue};
use drizzle_core::{
    SQL, SQLSchema, SQLTable, ToSQL, helpers as core_helpers,
    traits::{SQLModel, SQLParam},
};

// Re-export core helpers with SQLiteValue type for convenience
pub(crate) use core_helpers::{
    delete, from, group_by, having, limit, offset, order_by, select, set, update, r#where,
};

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
