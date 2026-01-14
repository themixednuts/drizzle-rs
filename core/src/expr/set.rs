//! Set operations (IN, NOT IN, EXISTS, NOT EXISTS).

use crate::sql::{Token, SQL};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{Bool, Compatible};

use super::{Expr, NonNull, Scalar, SQLExpr};

// =============================================================================
// IN Array
// =============================================================================

/// IN array check.
///
/// Returns true if the expression's value is in the provided array.
/// Requires the expression type to be compatible with the array element type.
pub fn in_array<'a, V, E, I, R>(expr: E, values: I) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator<Item = R>,
    R: Expr<'a, V>,
    E::SQLType: Compatible<R::SQLType>,
{
    let left_sql = expr.to_sql();
    let mut values_iter = values.into_iter();

    let sql = match values_iter.next() {
        // Empty array: use FALSE condition since IN (NULL) behaves inconsistently
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
    };
    SQLExpr::new(sql)
}

/// NOT IN array check.
///
/// Returns true if the expression's value is NOT in the provided array.
/// Requires the expression type to be compatible with the array element type.
pub fn not_in_array<'a, V, E, I, R>(expr: E, values: I) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator<Item = R>,
    R: Expr<'a, V>,
    E::SQLType: Compatible<R::SQLType>,
{
    let left_sql = expr.to_sql();
    let mut values_iter = values.into_iter();

    let sql = match values_iter.next() {
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
    };
    SQLExpr::new(sql)
}

/// IN subquery check.
///
/// Returns true if the expression's value is in the subquery results.
pub fn in_subquery<'a, V, E, S>(expr: E, subquery: S) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
    S: ToSQL<'a, V>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(Token::IN)
            .append(subquery.to_sql().parens()),
    )
}

/// NOT IN subquery check.
pub fn not_in_subquery<'a, V, E, S>(expr: E, subquery: S) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
    S: ToSQL<'a, V>,
{
    SQLExpr::new(
        expr.to_sql()
            .push(Token::NOT)
            .push(Token::IN)
            .append(subquery.to_sql().parens()),
    )
}

// =============================================================================
// EXISTS
// =============================================================================

/// EXISTS subquery check.
///
/// Returns true if the subquery returns any rows.
pub fn exists<'a, V, S>(subquery: S) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    S: ToSQL<'a, V>,
{
    SQLExpr::new(
        SQL::from_iter([Token::EXISTS, Token::LPAREN])
            .append(subquery.to_sql())
            .push(Token::RPAREN),
    )
}

/// NOT EXISTS subquery check.
///
/// Returns true if the subquery returns no rows.
pub fn not_exists<'a, V, S>(subquery: S) -> SQLExpr<'a, V, Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    S: ToSQL<'a, V>,
{
    SQLExpr::new(
        SQL::from_iter([Token::NOT, Token::EXISTS, Token::LPAREN])
            .append(subquery.to_sql())
            .push(Token::RPAREN),
    )
}
