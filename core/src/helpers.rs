use crate::{SQL, SQLSchemaType, SQLTable, ToSQL, Token, traits::SQLParam};

/// Helper function to create a SELECT statement with the given columns
pub fn select<'a, Value, T>(columns: T) -> SQL<'a, Value>
where
    Value: SQLParam,
    T: ToSQL<'a, Value>,
{
    SQL::from(Token::SELECT).append(&columns)
}

/// Helper function to create a SELECT DISTINCT statement with the given columns
pub fn select_distinct<'a, Value, T>(columns: T) -> SQL<'a, Value>
where
    Value: SQLParam,
    T: ToSQL<'a, Value>,
{
    SQL::from_iter([Token::SELECT, Token::DISTINCT]).append(&columns)
}

fn set_op<'a, Value, L, R>(left: L, op: Token, all: bool, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    let left = left.to_sql().parens();
    let right = right.to_sql().parens();
    let op_sql = if all {
        SQL::from(op).push(Token::ALL)
    } else {
        SQL::from(op)
    };

    left.append(op_sql).append(right)
}

/// Helper function to create a UNION statement
pub fn union<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::UNION, false, right)
}

/// Helper function to create a UNION ALL statement
pub fn union_all<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::UNION, true, right)
}

/// Helper function to create an INTERSECT statement
pub fn intersect<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::INTERSECT, false, right)
}

/// Helper function to create an INTERSECT ALL statement
pub fn intersect_all<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::INTERSECT, true, right)
}

/// Helper function to create an EXCEPT statement
pub fn except<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::EXCEPT, false, right)
}

/// Helper function to create an EXCEPT ALL statement
pub fn except_all<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::EXCEPT, true, right)
}

/// Creates an INSERT INTO statement with the specified table
pub fn insert<'a, Table, Type, Value>(table: Table) -> SQL<'a, Value>
where
    Type: SQLSchemaType,
    Value: SQLParam,
    Table: SQLTable<'a, Type, Value>,
{
    SQL::from_iter([Token::INSERT, Token::INTO]).append(&table)
}

/// Helper function to create a FROM clause
pub fn from<'a, T, Value>(query: T) -> SQL<'a, Value>
where
    T: ToSQL<'a, Value>,
    Value: SQLParam,
{
    SQL::from(Token::FROM).append(&query)
}

/// Helper function to create a WHERE clause
pub fn r#where<'a, V>(condition: impl ToSQL<'a, V>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    SQL::from(Token::WHERE).append(&condition)
}

/// Helper function to create a GROUP BY clause
pub fn group_by<'a, V, I, T>(expressions: I) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    I: IntoIterator<Item = T>,
    T: ToSQL<'a, V>,
{
    SQL::from_iter([Token::GROUP, Token::BY]).append(SQL::join(
        expressions.into_iter().map(|e| e.to_sql()),
        Token::COMMA,
    ))
}

/// Helper function to create a HAVING clause
pub fn having<'a, V>(condition: impl ToSQL<'a, V>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    SQL::from(Token::HAVING).append(&condition)
}

/// Helper function to create an ORDER BY clause
pub fn order_by<'a, T, V>(expressions: T) -> SQL<'a, V>
where
    T: ToSQL<'a, V>,
    V: SQLParam + 'a,
{
    SQL::from_iter([Token::ORDER, Token::BY]).append(expressions.to_sql())
}

/// Helper function to create a LIMIT clause
pub fn limit<'a, V>(value: usize) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    SQL::from(Token::LIMIT).append(SQL::number(value))
}

/// Helper function to create an OFFSET clause
pub fn offset<'a, V>(value: usize) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    SQL::from(Token::OFFSET).append(SQL::number(value))
}

/// Helper function to create an UPDATE statement
pub fn update<'a, Table, Type, Value>(table: Table) -> SQL<'a, Value>
where
    Table: SQLTable<'a, Type, Value>,
    Type: SQLSchemaType,
    Value: SQLParam + 'a,
{
    SQL::from(Token::UPDATE).append(&table)
}

/// Helper function to create a SET clause for UPDATE
pub fn set<'a, Table, Type, Value>(assignments: Table::Update) -> SQL<'a, Value>
where
    Value: SQLParam + 'a,
    Table: SQLTable<'a, Type, Value>,
    Type: SQLSchemaType,
{
    SQL::from(Token::SET).append(assignments.to_sql())
}

/// Helper function to create a DELETE FROM statement
pub fn delete<'a, Table, Type, Value>(table: Table) -> SQL<'a, Value>
where
    Table: SQLTable<'a, Type, Value>,
    Type: SQLSchemaType,
    Value: SQLParam + 'a,
{
    SQL::from_iter([Token::DELETE, Token::FROM]).append(&table)
}
