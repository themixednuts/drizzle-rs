//! Free functions that render single SQL clauses (`SELECT ...`,
//! `WHERE ...`, `LIMIT ...`, `UNION`, ...).
//!
//! Dialect builders assemble statements from these. They do no type or
//! scope checking beyond their bounds; prefer the builders in application
//! code.
//!
//! # Examples
//!
//! ```
//! use drizzle_core::SQL;
//! use drizzle_core::helpers::{from, limit, select};
//! # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
//! # use std::borrow::Cow;
//! # #[derive(Debug, Clone, PartialEq)]
//! # struct Value(i64);
//! # impl SQLParam for Value {
//! #     const DIALECT: Dialect = Dialect::SQLite;
//! #     type DialectMarker = SQLiteDialect;
//! # }
//! # impl From<Value> for Cow<'_, Value> {
//! #     fn from(value: Value) -> Self { Cow::Owned(value) }
//! # }
//!
//! let sql: SQL<'_, Value> = select(SQL::ident("id"))
//!     .append(from(SQL::ident("users")))
//!     .append(limit(10));
//! assert_eq!(sql.sql(), r#"SELECT "id" FROM "users" LIMIT 10"#);
//! ```

use crate::prelude::{Cow, Vec};
use crate::{
    ColumnRef, PaginationArg, SQL, SQLChunk, SQLSchemaType, SQLTable, ToSQL, Token, expr::Expr,
    traits::SQLParam, types::BooleanLike,
};

/// The `LIMIT` MySQL renders before an `OFFSET` that has no limit of its own.
///
/// MySQL has no bare `OFFSET`. Its manual suggests `18446744073709551615`
/// (`u64::MAX`) for "every remaining row", but for a `UNION` MySQL adds the
/// offset to the limit, and `u64::MAX + offset` wraps: the query returns no
/// rows (MySQL 8.0 and 8.4). `i64::MAX` is just as unbounded in practice and
/// leaves room for any offset.
#[doc(hidden)]
pub const MYSQL_UNBOUNDED_LIMIT: &str = "9223372036854775807";

/// Renders `SELECT <columns>`.
///
/// When `columns` is empty and a table follows in `FROM`, rendering expands
/// the projection into that table's columns.
pub fn select<'a, Value, T>(columns: T) -> SQL<'a, Value>
where
    Value: SQLParam,
    T: ToSQL<'a, Value>,
{
    SQL::from(Token::SELECT).append(columns.into_sql())
}

/// Renders `SELECT DISTINCT <columns>`.
pub fn select_distinct<'a, Value, T>(columns: T) -> SQL<'a, Value>
where
    Value: SQLParam,
    T: ToSQL<'a, Value>,
{
    SQL::from_iter([Token::SELECT, Token::DISTINCT]).append(columns.into_sql())
}

/// Clauses found outside any parentheses of a query used as a set-operation
/// operand.
#[derive(Debug, Default, Clone, Copy)]
struct OperandShape {
    /// `ORDER BY`, `LIMIT`, `OFFSET` or a locking `FOR` clause, which would
    /// otherwise apply to the whole compound (or be rejected before it).
    has_tail: bool,
    /// A `UNION` or `EXCEPT` operator.
    has_union_or_except: bool,
    /// An `INTERSECT` operator.
    has_intersect: bool,
    /// The query opens with a `WITH` clause.
    starts_with_cte: bool,
}

impl OperandShape {
    fn of<V: SQLParam>(sql: &SQL<'_, V>) -> Self {
        let mut shape = Self::default();
        let mut depth = 0usize;
        let mut leading = true;

        for chunk in &sql.chunks {
            match chunk {
                // A sqlcommenter comment may precede the query.
                SQLChunk::Raw(text) if leading && text.trim_start().starts_with("/*") => {
                    continue;
                }
                SQLChunk::Token(Token::LPAREN) => depth += 1,
                SQLChunk::Token(Token::RPAREN) => depth = depth.saturating_sub(1),
                SQLChunk::Token(Token::WITH) if leading => shape.starts_with_cte = true,
                SQLChunk::Token(Token::ORDER | Token::LIMIT | Token::OFFSET | Token::FOR)
                    if depth == 0 =>
                {
                    shape.has_tail = true;
                }
                SQLChunk::Token(Token::UNION | Token::EXCEPT) if depth == 0 => {
                    shape.has_union_or_except = true;
                }
                SQLChunk::Token(Token::INTERSECT) if depth == 0 => shape.has_intersect = true,
                _ => {}
            }
            leading = false;
        }

        shape
    }

    const fn is_compound(self) -> bool {
        self.has_union_or_except || self.has_intersect
    }
}

/// Makes `operand` a single set-operation operand.
///
/// `PostgreSQL` and `MySQL` accept a parenthesized query there. `SQLite` does
/// not, so the operand becomes a derived table instead; its columns keep their
/// names and order.
fn group_set_operand<'a, V: SQLParam>(operand: SQL<'a, V>) -> SQL<'a, V> {
    match V::DIALECT {
        crate::Dialect::SQLite => {
            SQL::from_iter([Token::SELECT, Token::STAR, Token::FROM]).append(operand.parens())
        }
        crate::Dialect::PostgreSQL | crate::Dialect::MySQL => operand.parens(),
    }
}

/// Joins two queries with a set operator, grouping an operand whenever it
/// would not otherwise parse as one operand of this operator:
///
/// - an operand with its own `ORDER BY` / `LIMIT` / `OFFSET`, which would
///   otherwise limit the whole compound or be rejected before the operator;
/// - a compound right operand, so `a.union(b.except(c))` is `A ∪ (B − C)`;
/// - a right operand that opens with `WITH`;
/// - on `PostgreSQL` and `MySQL`, a left `UNION` / `EXCEPT` compound joined by
///   `INTERSECT`, which binds tighter there, so chains apply left to right.
///
/// Plain operands and left-to-right chains render unchanged.
fn set_op<'a, Value, L, R>(left: L, op: Token, all: bool, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    let left = left.into_sql();
    let right = right.into_sql();

    let left_shape = OperandShape::of(&left);
    let intersect_binds_tighter = !matches!(Value::DIALECT, crate::Dialect::SQLite);
    let left = if left_shape.has_tail
        || (intersect_binds_tighter
            && matches!(op, Token::INTERSECT)
            && left_shape.has_union_or_except)
    {
        group_set_operand(left)
    } else {
        left
    };

    let right_shape = OperandShape::of(&right);
    let right = if right_shape.has_tail || right_shape.is_compound() || right_shape.starts_with_cte
    {
        group_set_operand(right)
    } else {
        right
    };

    let op_sql = if all {
        SQL::from(op).push(Token::ALL)
    } else {
        SQL::from(op)
    };

    left.append(op_sql).append(right)
}

/// Renders `<left> UNION <right>`.
///
/// An operand that would not parse as one operand on its own (it has an
/// `ORDER BY` / `LIMIT`, or is itself compound) is grouped first: in
/// parentheses on PostgreSQL and MySQL, as `SELECT * FROM (...)` on SQLite.
///
/// # Examples
///
/// ```
/// use drizzle_core::SQL;
/// use drizzle_core::helpers::union;
/// # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
/// # use std::borrow::Cow;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct Value(i64);
/// # impl SQLParam for Value {
/// #     const DIALECT: Dialect = Dialect::SQLite;
/// #     type DialectMarker = SQLiteDialect;
/// # }
/// # impl From<Value> for Cow<'_, Value> {
/// #     fn from(value: Value) -> Self { Cow::Owned(value) }
/// # }
///
/// let sql: SQL<'_, Value> = union(SQL::raw("SELECT 1"), SQL::raw("SELECT 2"));
/// assert_eq!(sql.sql(), "SELECT 1 UNION SELECT 2");
/// ```
pub fn union<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::UNION, false, right)
}

/// Renders `<left> UNION ALL <right>`, grouping operands like [`union`].
pub fn union_all<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::UNION, true, right)
}

/// Renders `<left> INTERSECT <right>`, grouping operands like [`union`].
pub fn intersect<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::INTERSECT, false, right)
}

/// Renders `<left> INTERSECT ALL <right>`, grouping operands like [`union`].
pub fn intersect_all<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::INTERSECT, true, right)
}

/// Renders `<left> EXCEPT <right>`, grouping operands like [`union`].
pub fn except<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::EXCEPT, false, right)
}

/// Renders `<left> EXCEPT ALL <right>`, grouping operands like [`union`].
pub fn except_all<'a, Value, L, R>(left: L, right: R) -> SQL<'a, Value>
where
    Value: SQLParam,
    L: ToSQL<'a, Value>,
    R: ToSQL<'a, Value>,
{
    set_op(left, Token::EXCEPT, true, right)
}

/// Renders `INSERT INTO <table>`.
pub fn insert<'a, Table, Type, Value>(table: &Table) -> SQL<'a, Value>
where
    Type: SQLSchemaType,
    Value: SQLParam,
    Table: SQLTable<'a, Type, Value>,
{
    SQL::from_iter([Token::INSERT, Token::INTO]).append(table)
}

/// Renders the `(columns) VALUES (..), (..)` of a multi-row `INSERT` whose
/// rows set different columns.
///
/// The column list holds every column any row sets, in the order the rows
/// first name them, and a row that leaves one of those columns unset gets
/// `DEFAULT` in that cell: what omitting the column means for a single-row
/// insert. `PostgreSQL` and `MySQL` accept `DEFAULT` there; `SQLite` does not.
///
/// `rows` pairs each row's `SQLModel::columns()` with its `values()`, which
/// holds one value per column, joined by commas. Returns `None` when a row's
/// values do not split into one value per column.
#[doc(hidden)]
pub fn insert_values_with_defaults<'a, V: SQLParam>(
    rows: Vec<(Cow<'static, [ColumnRef]>, SQL<'a, V>)>,
) -> Option<SQL<'a, V>> {
    let mut columns: Vec<ColumnRef> = Vec::new();
    for (row_columns, _) in &rows {
        for column in row_columns.iter() {
            if !columns.contains(column) {
                columns.push(*column);
            }
        }
    }

    let mut values = SQL::with_capacity_chunks(rows.len().saturating_mul(columns.len() * 2 + 2));
    for (index, (row_columns, row_values)) in rows.into_iter().enumerate() {
        let mut cells = split_top_level_commas(row_values);
        if cells.len() != row_columns.len() {
            return None;
        }
        if index > 0 {
            values.push_mut(Token::COMMA);
        }
        values.push_mut(Token::LPAREN);
        for (position, column) in columns.iter().enumerate() {
            if position > 0 {
                values.push_mut(Token::COMMA);
            }
            match row_columns.iter().position(|set| set == column) {
                Some(cell) => values.append_mut(core::mem::take(&mut cells[cell])),
                None => values.push_mut(Token::DEFAULT),
            }
        }
        values.push_mut(Token::RPAREN);
    }

    Some(
        SQL::columns(&columns)
            .parens()
            .push(Token::VALUES)
            .append(values),
    )
}

/// Splits `sql` at the commas outside any parentheses.
fn split_top_level_commas<'a, V: SQLParam>(sql: SQL<'a, V>) -> Vec<SQL<'a, V>> {
    let mut parts = Vec::new();
    let mut current = SQL::empty();
    let mut depth = 0usize;
    for chunk in sql.chunks {
        match chunk {
            SQLChunk::Token(Token::LPAREN) => depth += 1,
            SQLChunk::Token(Token::RPAREN) => depth = depth.saturating_sub(1),
            SQLChunk::Token(Token::COMMA) if depth == 0 => {
                parts.push(core::mem::take(&mut current));
                continue;
            }
            _ => {}
        }
        current.chunks.push(chunk);
    }
    if !current.chunks.is_empty() || !parts.is_empty() {
        parts.push(current);
    }
    parts
}

/// Renders `FROM <source>`.
pub fn from<'a, T, Value>(query: T) -> SQL<'a, Value>
where
    T: ToSQL<'a, Value>,
    Value: SQLParam,
{
    SQL::from(Token::FROM).append(query.into_sql())
}

/// Renders `WHERE <condition>`. The condition must be boolean.
pub fn r#where<'a, V, E>(condition: E) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: BooleanLike,
{
    SQL::from(Token::WHERE).append(condition.into_expr_sql())
}

/// Renders `GROUP BY <expr>, <expr>, ...`.
pub fn group_by<'a, V, I, T>(expressions: I) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    I: IntoIterator<Item = T>,
    T: ToSQL<'a, V>,
{
    SQL::from_iter([Token::GROUP, Token::BY]).append(SQL::join(
        expressions.into_iter().map(ToSQL::into_sql),
        Token::COMMA,
    ))
}

/// Renders `GROUP BY <expr>` from a single item.
///
/// Unlike [`group_by`], this takes one value, which may be a column or a
/// tuple of columns that already renders as a comma-separated list.
pub fn group_by_expr<'a, V, T>(expr: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    SQL::from_iter([Token::GROUP, Token::BY]).append(expr.into_sql())
}

/// Renders `HAVING <condition>`. The condition must be boolean.
pub fn having<'a, V, E>(condition: E) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    E::SQLType: BooleanLike,
{
    SQL::from(Token::HAVING).append(condition.into_expr_sql())
}

/// Renders `ORDER BY <expressions>`.
pub fn order_by<'a, T, V>(expressions: T) -> SQL<'a, V>
where
    T: ToSQL<'a, V>,
    V: SQLParam + 'a,
{
    SQL::from_iter([Token::ORDER, Token::BY]).append(expressions.into_sql())
}

/// Renders `ORDER BY <expressions>` for a compound query (`UNION`,
/// `INTERSECT`, `EXCEPT`).
///
/// A compound result has no table scope, so PostgreSQL and turso reject
/// `ORDER BY "table"."column"` there; only output column names are allowed.
/// Column references are therefore written as bare names.
pub fn set_order_by<'a, T, V>(expressions: T) -> SQL<'a, V>
where
    T: ToSQL<'a, V>,
    V: SQLParam + 'a,
{
    SQL::from_iter([Token::ORDER, Token::BY]).append(unqualified_columns(expressions.into_sql()))
}

/// Replaces every column reference in `sql` with its bare column name.
pub fn unqualified_columns<'a, V>(mut sql: SQL<'a, V>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    for chunk in &mut sql.chunks {
        if let SQLChunk::Column(column) = chunk {
            *chunk = SQLChunk::ident_static(column.name);
        }
    }
    sql
}

/// Renders `LIMIT <value>` (see [`PaginationArg`]).
///
/// # Panics
///
/// Panics when an integer argument is negative or does not fit in `usize`.
#[must_use]
#[track_caller]
pub fn limit<'a, V, P>(value: P) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    P: PaginationArg<'a, V>,
{
    SQL::from(Token::LIMIT).append(value.into_pagination_sql())
}

/// Renders `OFFSET <value>` (see [`PaginationArg`]).
///
/// # Panics
///
/// Panics when an integer argument is negative or does not fit in `usize`.
#[must_use]
#[track_caller]
pub fn offset<'a, V, P>(value: P) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    P: PaginationArg<'a, V>,
{
    SQL::from(Token::OFFSET).append(value.into_pagination_sql())
}

/// Renders `UPDATE <table>`.
pub fn update<'a, Table, Type, Value>(table: &Table) -> SQL<'a, Value>
where
    Table: SQLTable<'a, Type, Value>,
    Type: SQLSchemaType,
    Value: SQLParam + 'a,
{
    SQL::from(Token::UPDATE).append(table)
}

/// Renders `SET <assignments>` from a table's update model.
pub fn set<'a, Table, Type, Value>(assignments: &Table::Update) -> SQL<'a, Value>
where
    Value: SQLParam + 'a,
    Table: SQLTable<'a, Type, Value>,
    Type: SQLSchemaType,
{
    SQL::from(Token::SET).append(assignments.to_sql())
}

/// Renders `DELETE FROM <table>`.
pub fn delete<'a, Table, Type, Value>(table: &Table) -> SQL<'a, Value>
where
    Table: SQLTable<'a, Type, Value>,
    Type: SQLSchemaType,
    Value: SQLParam + 'a,
{
    SQL::from_iter([Token::DELETE, Token::FROM]).append(table)
}
