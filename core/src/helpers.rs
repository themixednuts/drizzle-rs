use crate::{OrderBy, SQL, SQLTable, ToSQL, traits::SQLParam};

/// Helper function to create a SELECT statement with the given columns
pub fn select<'a, Value, T>(columns: T) -> SQL<'a, Value>
where
    Value: SQLParam + 'a,
    T: ToSQL<'a, Value>,
{
    SQL::raw("SELECT").append(columns.to_sql())
}

/// Helper function to create a FROM clause using table generic
pub fn from<'a, T, V>(table: T) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    SQL::raw("FROM").append(&table)
}

/// Helper function to create a WHERE clause
pub fn r#where<'a, V>(condition: SQL<'a, V>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    let sql = SQL::raw("WHERE");
    sql.append(condition)
}

/// Helper function to create a GROUP BY clause
pub fn group_by<'a, V, I>(expressions: I) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    I: IntoIterator<Item = SQL<'a, V>>,
{
    let sql = SQL::raw("GROUP BY");
    sql.append(SQL::join(expressions, ", "))
}

/// Helper function to create a HAVING clause
pub fn having<'a, V>(condition: SQL<'a, V>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    let sql = SQL::raw("HAVING");
    sql.append(condition)
}

/// Helper function to create an ORDER BY clause
pub fn order_by<'a, TSQL, T, V>(expressions: T) -> SQL<'a, V>
where
    TSQL: ToSQL<'a, V>,
    T: IntoIterator<Item = (TSQL, OrderBy)>,
    V: SQLParam + 'a,
{
    let sql = SQL::raw("ORDER BY");

    sql.append(SQL::join(
        expressions.into_iter().map(|(expr, direction)| {
            let mut expr_sql = expr.to_sql();
            expr_sql = expr_sql.append_raw(" ");
            match direction {
                OrderBy::Asc => expr_sql.append_raw("ASC"),
                OrderBy::Desc => expr_sql.append_raw("DESC"),
            }
        }),
        ", ",
    ))
}

/// Helper function to create a LIMIT clause
pub fn limit<'a, V>(value: usize) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    SQL::raw(format!("LIMIT {}", value))
}

/// Helper function to create an OFFSET clause
pub fn offset<'a, V>(value: usize) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    SQL::raw(format!("OFFSET {}", value))
}

/// Helper function to create an UPDATE statement using table generic
pub fn update<'a, T, V>(table: T) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    SQL::raw("UPDATE").append(&table)
}

/// Helper function to create a SET clause for UPDATE
pub fn set<'a, Table, Value>(assignments: Table::Update) -> SQL<'a, Value>
where
    Value: SQLParam + 'a,
    Table: SQLTable<'a, Value>,
{
    SQL::raw("SET").append(assignments.to_sql())
}

/// Helper function to create a DELETE FROM statement using table generic
pub fn delete<'a, T, V>(table: T) -> SQL<'a, V>
where
    T: SQLTable<'a, V>,
    V: SQLParam + 'a,
{
    SQL::raw("DELETE FROM").append(&table)
}
