use crate::values::SQLiteValue;
use drizzle_core::{Join, SQL, SQLTable, SortDirection, ToSQL, traits::SQLParam};

/// Helper function to create a SELECT statement with the given columns
pub(crate) fn select<'a, const N: usize>(
    columns: [impl ToSQL<'a, SQLiteValue<'a>>; N],
) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("SELECT");
    if N == 0 {
        sql.append_raw("*")
    } else {
        sql.append(SQL::join(&columns, ","))
    }
}

/// Helper function to create a FROM clause
pub(crate) fn from<'a>(table: impl ToSQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("FROM");
    sql.append(table.to_sql())
}

/// Helper function to create a WHERE clause
pub(crate) fn where_clause<'a>(condition: SQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("WHERE");
    sql.append(condition)
}

/// Helper function to create a JOIN clause
pub(crate) fn join<'a>(
    join_type: Join,
    table: impl ToSQL<'a, SQLiteValue<'a>>,
    condition: SQL<'a, SQLiteValue<'a>>,
) -> SQL<'a, SQLiteValue<'a>> {
    let sql = join_type.to_sql();
    let sql = sql.append_raw(" ");
    let sql = sql.append(table.to_sql());
    let sql = sql.append_raw(" ON ");
    sql.append(condition)
}

/// Helper function to create an INNER JOIN clause
pub(crate) fn inner_join<'a>(
    table: impl ToSQL<'a, SQLiteValue<'a>>,
    condition: SQL<'a, SQLiteValue<'a>>,
) -> SQL<'a, SQLiteValue<'a>> {
    join(Join::Inner, table, condition)
}

/// Helper function to create a LEFT JOIN clause
pub(crate) fn left_join<'a>(
    table: impl ToSQL<'a, SQLiteValue<'a>>,
    condition: SQL<'a, SQLiteValue<'a>>,
) -> SQL<'a, SQLiteValue<'a>> {
    join(Join::Left, table, condition)
}

/// Helper function to create a RIGHT JOIN clause
pub(crate) fn right_join<'a>(
    table: impl ToSQL<'a, SQLiteValue<'a>>,
    condition: SQL<'a, SQLiteValue<'a>>,
) -> SQL<'a, SQLiteValue<'a>> {
    join(Join::Right, table, condition)
}

/// Helper function to create a FULL JOIN clause
pub(crate) fn full_join<'a>(
    table: impl ToSQL<'a, SQLiteValue<'a>>,
    condition: SQL<'a, SQLiteValue<'a>>,
) -> SQL<'a, SQLiteValue<'a>> {
    join(Join::Full, table, condition)
}

/// Helper function to create a GROUP BY clause
pub(crate) fn group_by<'a>(expressions: Vec<SQL<'a, SQLiteValue<'a>>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("GROUP BY");
    sql.append(SQL::join(&expressions, ", "))
}

/// Helper function to create a HAVING clause
pub(crate) fn having<'a>(condition: SQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("HAVING");
    sql.append(condition)
}

/// Helper function to create an ORDER BY clause
pub(crate) fn order_by<'a>(
    expressions: Vec<(SQL<'a, SQLiteValue<'a>>, SortDirection)>,
) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("ORDER BY ");

    let order_sqls: Vec<SQL<'a, SQLiteValue<'a>>> = expressions
        .into_iter()
        .map(|(expr, direction)| {
            let mut expr_sql = expr;
            expr_sql = expr_sql.append_raw(" ");
            match direction {
                SortDirection::Asc => expr_sql.append_raw("ASC"),
                SortDirection::Desc => expr_sql.append_raw("DESC"),
            }
        })
        .collect();

    sql.append(SQL::join(&order_sqls, ", "))
}

/// Helper function to create a LIMIT clause
pub(crate) fn limit<'a>(limit_val: usize) -> SQL<'a, SQLiteValue<'a>> {
    SQL::raw(format!("LIMIT {}", limit_val))
}

/// Helper function to create an OFFSET clause
pub(crate) fn offset<'a>(offset_val: usize) -> SQL<'a, SQLiteValue<'a>> {
    SQL::raw(format!("OFFSET {}", offset_val))
}

/// Creates an INSERT INTO statement with the specified table
pub(crate) fn insert_into<'a>(table: impl ToSQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("INSERT INTO");
    sql.append(table.to_sql())
}

/// Helper function to create VALUES clause for INSERT
pub(crate) fn values<'a, Table, V>(
    rows: impl IntoIterator<Item = <Table as SQLTable<'a, V>>::Insert>,
) -> SQL<'a, SQLiteValue<'a>>
where
    Table: SQLTable<'a, V>,
    V: SQLParam + 'a,
    <Table as SQLTable<'a, V>>::Insert: ToSQL<'a, V>,
{
    let sql = SQL::raw("VALUES ");

    let value_rows: Vec<SQL<'a, SQLiteValue<'a>>> = rows
        .into_iter()
        .map(|row| {
            let mut row_sql = SQL::raw("(");
            row_sql = row_sql.append_raw(")");
            row_sql
        })
        .collect();

    sql.append(SQL::join(&value_rows, ", "))
}

/// Helper function to create an UPDATE statement
pub(crate) fn update<'a>(table: impl ToSQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("UPDATE ");
    sql.append(table.to_sql())
}

/// Helper function to create a SET clause for UPDATE
pub(crate) fn set<'a>(
    assignments: Vec<(String, SQL<'a, SQLiteValue<'a>>)>,
) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("SET ");

    let set_sqls: Vec<SQL<'a, SQLiteValue<'a>>> = assignments
        .into_iter()
        .map(|(column, value)| {
            let mut assign_sql = SQL::raw(column);
            assign_sql = assign_sql.append_raw(" = ");
            assign_sql.append(value)
        })
        .collect();

    sql.append(SQL::join(&set_sqls, ", "))
}

/// Helper function to create a DELETE FROM statement
pub(crate) fn delete_from<'a>(table: impl ToSQL<'a, SQLiteValue<'a>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("DELETE FROM ");
    sql.append(table.to_sql())
}

/// Helper function to create a RETURNING clause
pub(crate) fn returning<'a>(columns: Vec<SQL<'a, SQLiteValue<'a>>>) -> SQL<'a, SQLiteValue<'a>> {
    let sql = SQL::raw("RETURNING ");

    if columns.is_empty() {
        return sql.append_raw("*");
    }

    sql.append(SQL::join(&columns, ", "))
}
