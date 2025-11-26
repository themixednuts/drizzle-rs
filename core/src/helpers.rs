use crate::prelude::*;
use crate::{SQL, SQLSchemaType, SQLTable, ToSQL, Token, traits::SQLParam};

/// Helper function to create a SELECT statement with the given columns
pub fn select<'a, Value, T>(columns: T) -> SQL<'a, Value>
where
    Value: SQLParam,
    T: ToSQL<'a, Value>,
{
    SQL::from(Token::SELECT).append(&columns)
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
pub fn group_by<'a, V, I>(expressions: I) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    I: IntoIterator<Item = SQL<'a, V>>,
{
    SQL::from_iter([Token::GROUP, Token::BY]).append(SQL::join(expressions, Token::COMMA))
}

/// Helper function to create a HAVING clause
pub fn having<'a, V>(condition: SQL<'a, V>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    SQL::from(Token::HAVING).append(condition)
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
    SQL::from(Token::LIMIT).append(SQL::raw(value.to_string()))
}

/// Helper function to create an OFFSET clause
pub fn offset<'a, V>(value: usize) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    SQL::from(Token::OFFSET).append(SQL::raw(value.to_string()))
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
