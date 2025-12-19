//! SQL expressions and conditions
//!
//! This module provides standard SQL expressions and conditions that work across all dialects.
//! For dialect-specific expressions, see the respective crate (e.g., `drizzle_postgres::expressions`).

use crate::{
    sql::{SQL, SQLChunk, Token},
    traits::{SQLComparable, SQLParam, ToSQL},
};

// =============================================================================
// Aggregate Functions
// =============================================================================

pub fn alias<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E, alias: &'a str) -> SQL<'a, V> {
    expr.to_sql().alias(alias)
}

pub fn count<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("COUNT", expr.to_sql())
}

pub fn sum<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("SUM", expr.to_sql())
}

pub fn avg<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("AVG", expr.to_sql())
}

pub fn min<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("MIN", expr.to_sql())
}

pub fn max<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("MAX", expr.to_sql())
}

pub fn distinct<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::raw("DISTINCT").append(&expr)
}

pub fn coalesce<'a, V: SQLParam + 'a, E: ToSQL<'a, V>, D: ToSQL<'a, V>>(
    expr: E,
    default: D,
) -> SQL<'a, V> {
    SQL::func(
        "COALESCE",
        expr.to_sql().push(Token::COMMA).append(default.to_sql()),
    )
}

pub fn r#typeof<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("TYPEOF", expr.to_sql())
}

pub fn cast<'a, V: SQLParam + 'a, E: ToSQL<'a, V>, Type: ToSQL<'a, V>>(
    expr: E,
    target_type: Type,
) -> SQL<'a, V> {
    SQL::func("CAST", expr.to_sql().push(Token::AS).append(&target_type))
}

pub fn r#in<'a, V: SQLParam + 'a, E: ToSQL<'a, V>, S: ToSQL<'a, V>>(
    expr: E,
    values: S,
) -> SQL<'a, V> {
    expr.to_sql()
        .push(Token::IN)
        .append(values.to_sql().parens())
}

// =============================================================================
// Comparison Conditions
// =============================================================================

fn binary_op<'a, V, L, R>(left: L, operator: Token, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    let right_sql = right.to_sql();
    // Wrap subqueries (starting with SELECT) in parentheses
    let right_sql = if right_sql.is_subquery() {
        right_sql.parens()
    } else {
        right_sql
    };
    left.to_sql().push(operator).append(right_sql)
}

pub fn eq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    binary_op(left, Token::EQ, right)
}

pub fn neq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    binary_op(left, Token::NE, right)
}

pub fn gt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    binary_op(left, Token::GT, right)
}

pub fn gte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    binary_op(left, Token::GE, right)
}

pub fn lt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    binary_op(left, Token::LT, right)
}

pub fn lte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    binary_op(left, Token::LE, right)
}

// =============================================================================
// Array/Set Conditions
// =============================================================================

pub fn in_array<'a, V, L, I, R>(left: L, values: I) -> SQL<'a, V>
where
    V: SQLParam,
    L: ToSQL<'a, V>,
    I: IntoIterator<Item = R>,
    R: Into<V> + ToSQL<'a, V>,
{
    let left_sql = left.to_sql();
    let mut values_iter = values.into_iter();

    match values_iter.next() {
        // Empty array: use FALSE condition (1=0) since IN (NULL) behaves inconsistently
        None => left_sql.append(SQL::raw("IN (SELECT NULL WHERE 1=0)")),
        Some(first_value) => {
            let mut result = left_sql
                .push(Token::IN)
                .push(Token::LPAREN)
                .append(first_value.to_sql());
            for value in values_iter {
                result = result.push(Token::COMMA).append(value.to_sql());
            }
            result.push(Token::RPAREN)
        }
    }
}

pub fn not_in_array<'a, V, L, I, R>(left: L, values: I) -> SQL<'a, V>
where
    V: SQLParam,
    L: ToSQL<'a, V>,
    I: IntoIterator<Item = R>,
    R: Into<V> + ToSQL<'a, V>,
{
    let left_sql = left.to_sql();
    let mut values_iter = values.into_iter();

    match values_iter.next() {
        None => left_sql
            .push(Token::NOT)
            .push(Token::IN)
            .push(Token::LPAREN)
            .push(Token::NULL)
            .push(Token::RPAREN),
        Some(first_value) => {
            let mut result = left_sql
                .push(Token::NOT)
                .push(Token::IN)
                .push(Token::LPAREN)
                .append(first_value.to_sql());
            for value in values_iter {
                result = result.push(Token::COMMA).append(value.to_sql());
            }
            result.push(Token::RPAREN)
        }
    }
}

// =============================================================================
// NULL Conditions
// =============================================================================

pub fn is_null<'a, V, R>(right: R) -> SQL<'a, V>
where
    V: SQLParam,
    R: ToSQL<'a, V>,
{
    right.to_sql().push(Token::IS).push(Token::NULL)
}

pub fn is_not_null<'a, V, R>(right: R) -> SQL<'a, V>
where
    V: SQLParam,
    R: ToSQL<'a, V>,
{
    right
        .to_sql()
        .push(Token::IS)
        .push(Token::NOT)
        .push(Token::NULL)
}

// =============================================================================
// Subquery Conditions
// =============================================================================

pub fn exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    SQL::from_iter([Token::EXISTS, Token::LPAREN])
        .append(subquery.to_sql())
        .push(Token::RPAREN)
}

pub fn not_exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    SQL::from_iter([Token::NOT, Token::EXISTS, Token::LPAREN])
        .append(subquery.to_sql())
        .push(Token::RPAREN)
}

// =============================================================================
// Range Conditions
// =============================================================================

pub fn between<'a, V, L, R1, R2>(left: L, start: R1, end: R2) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R1: ToSQL<'a, V>,
    R2: ToSQL<'a, V>,
{
    SQL::from(Token::LPAREN)
        .append(left.to_sql())
        .push(Token::BETWEEN)
        .append(start.to_sql())
        .push(Token::AND)
        .append(end.to_sql())
        .push(Token::RPAREN)
}

pub fn not_between<'a, V, L, R1, R2>(left: L, lower: R1, upper: R2) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R1: ToSQL<'a, V>,
    R2: ToSQL<'a, V>,
{
    SQL::from(Token::LPAREN)
        .append(left.to_sql())
        .push(Token::NOT)
        .push(Token::BETWEEN)
        .append(lower.to_sql())
        .push(Token::AND)
        .append(upper.to_sql())
        .push(Token::RPAREN)
}

// =============================================================================
// Pattern Matching
// =============================================================================

pub fn like<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    left.to_sql().push(Token::LIKE).append(pattern.to_sql())
}

pub fn not_like<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    left.to_sql()
        .push(Token::NOT)
        .push(Token::LIKE)
        .append(pattern.to_sql())
}

// =============================================================================
// Logical Operators
// =============================================================================

pub fn not<'a, V, T>(expression: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    let expr_sql = expression.to_sql();
    let needs_paren = expr_sql.chunks.len() > 1
        || (expr_sql.chunks.len() == 1
            && !matches!(expr_sql.chunks[0], SQLChunk::Raw(_) | SQLChunk::Ident(_)));

    if needs_paren {
        SQL::from_iter([Token::NOT, Token::LPAREN])
            .append(expr_sql)
            .push(Token::RPAREN)
    } else {
        SQL::from(Token::NOT).append(expr_sql)
    }
}

pub fn and<'a, V, T>(conditions: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: IntoIterator<Item = SQL<'a, V>>,
{
    let mut iter = conditions.into_iter();

    match iter.next() {
        None => SQL::empty(),
        Some(first) => {
            let Some(second) = iter.next() else {
                return first;
            };
            let all_conditions = core::iter::once(first)
                .chain(core::iter::once(second))
                .chain(iter);
            SQL::from(Token::LPAREN)
                .append(SQL::join(all_conditions, Token::AND))
                .push(Token::RPAREN)
        }
    }
}

pub fn or<'a, V, T>(conditions: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: IntoIterator<Item = SQL<'a, V>>,
{
    let mut iter = conditions.into_iter();

    match iter.next() {
        None => SQL::empty(),
        Some(first) => {
            let Some(second) = iter.next() else {
                return first;
            };
            let all_conditions = core::iter::once(first)
                .chain(core::iter::once(second))
                .chain(iter);
            SQL::from(Token::LPAREN)
                .append(SQL::join(all_conditions, Token::OR))
                .push(Token::RPAREN)
        }
    }
}

// =============================================================================
// String Operations
// =============================================================================

pub fn string_concat<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    left.to_sql().push(Token::CONCAT).append(right.to_sql())
}
