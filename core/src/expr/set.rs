//! Set operations (IN, NOT IN, EXISTS, NOT EXISTS).

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{Compatible, DataType};

use super::{AggregateKind, Expr, NonNull, SQLExpr, Scalar};

#[inline]
fn operand_sql<'a, V, T>(value: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    value.into_sql().parens_if_subquery()
}

// =============================================================================
// InSubqueryLhs — marker-parameterized trait for single exprs and tuples
// =============================================================================

/// Marker for a single `Expr` used as `IN (subquery)` LHS.
#[doc(hidden)]
pub enum Single {}

/// Marker for a tuple of `Expr`s used as `IN (subquery)` LHS.
#[doc(hidden)]
pub enum Multi {}

/// Left-hand side of an `IN (subquery)` expression.
///
/// Accepts single expressions (`col`) or tuples (`(col_a, col_b)`).
/// The marker `M` is inferred — callers never specify it.
pub trait InSubqueryLhs<'a, V: SQLParam, M>: Sized {
    type SQLType: DataType;
    type Aggregate: AggregateKind;
    fn into_lhs_sql(self) -> SQL<'a, V>;
}

/// Single expression: `in_subquery(users.id, sub)`
impl<'a, V, E> InSubqueryLhs<'a, V, Single> for E
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    type SQLType = E::SQLType;
    type Aggregate = E::Aggregate;
    fn into_lhs_sql(self) -> SQL<'a, V> {
        self.into_sql().parens_if_subquery()
    }
}

/// Tuple impls: `in_subquery((users.id, users.name), sub)`
macro_rules! impl_in_subquery_lhs_tuple {
    ($($E:ident),+; $($idx:tt),+) => {
        impl<'a, V, $($E),+> InSubqueryLhs<'a, V, Multi> for ($($E,)+)
        where
            V: SQLParam + 'a,
            $($E: Expr<'a, V>,)+
        {
            type SQLType = ($($E::SQLType,)+);
            type Aggregate = Scalar;
            fn into_lhs_sql(self) -> SQL<'a, V> {
                ToSQL::into_sql(self).parens()
            }
        }
    };
}

with_col_sizes_8!(impl_in_subquery_lhs_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_in_subquery_lhs_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_in_subquery_lhs_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_in_subquery_lhs_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_in_subquery_lhs_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_in_subquery_lhs_tuple);

// =============================================================================
// IN Array
// =============================================================================

/// IN array check.
///
/// Returns true if the expression's value is in the provided array.
/// Requires the expression type to be compatible with the array element type.
pub fn in_array<'a, V, E, I, R>(
    expr: E,
    values: I,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator<Item = R>,
    R: Expr<'a, V>,
    E::SQLType: Compatible<R::SQLType>,
{
    SQLExpr::new(in_array_impl(expr, values, false))
}

/// NOT IN array check.
///
/// Returns true if the expression's value is NOT in the provided array.
/// Requires the expression type to be compatible with the array element type.
pub fn not_in_array<'a, V, E, I, R>(
    expr: E,
    values: I,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator<Item = R>,
    R: Expr<'a, V>,
    E::SQLType: Compatible<R::SQLType>,
{
    SQLExpr::new(in_array_impl(expr, values, true))
}

fn in_array_impl<'a, V, E, I, R>(expr: E, values: I, negated: bool) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    I: IntoIterator<Item = R>,
    R: Expr<'a, V>,
    E::SQLType: Compatible<R::SQLType>,
{
    let left_sql = operand_sql(expr);
    let mut values_iter = values.into_iter();

    match values_iter.next() {
        None => {
            if negated {
                left_sql.append(SQL::raw("NOT IN (SELECT NULL WHERE 1=0)"))
            } else {
                left_sql.append(SQL::raw("IN (SELECT NULL WHERE 1=0)"))
            }
        }
        Some(first_value) => {
            let mut result = left_sql;
            if negated {
                result = result.push(Token::NOT);
            }

            result = result
                .push(Token::IN)
                .push(Token::LPAREN)
                .append(operand_sql(first_value));

            for value in values_iter {
                result = result.push(Token::COMMA).append(operand_sql(value));
            }
            result.push(Token::RPAREN)
        }
    }
}

/// IN subquery check.
///
/// Returns true if the expression's value is in the subquery results.
/// Accepts a single expression or a tuple of expressions as the LHS:
///
/// ```ignore
/// in_subquery(users.id, sub)                      // single column
/// in_subquery((users.id, users.name), sub)        // multi-column
/// ```
pub fn in_subquery<'a, V, L, S, M>(
    lhs: L,
    subquery: S,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, L::Aggregate>
where
    V: SQLParam + 'a,
    L: InSubqueryLhs<'a, V, M>,
    S: Expr<'a, V>,
    L::SQLType: Compatible<S::SQLType>,
{
    SQLExpr::new(
        lhs.into_lhs_sql()
            .push(Token::IN)
            .append(subquery.into_sql().parens()),
    )
}

/// NOT IN subquery check.
///
/// Accepts a single expression or a tuple of expressions as the LHS:
///
/// ```ignore
/// not_in_subquery(users.id, sub)                  // single column
/// not_in_subquery((users.id, users.name), sub)    // multi-column
/// ```
pub fn not_in_subquery<'a, V, L, S, M>(
    lhs: L,
    subquery: S,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, L::Aggregate>
where
    V: SQLParam + 'a,
    L: InSubqueryLhs<'a, V, M>,
    S: Expr<'a, V>,
    L::SQLType: Compatible<S::SQLType>,
{
    SQLExpr::new(
        lhs.into_lhs_sql()
            .push(Token::NOT)
            .push(Token::IN)
            .append(subquery.into_sql().parens()),
    )
}

// =============================================================================
// EXISTS
// =============================================================================

/// EXISTS subquery check.
///
/// Returns true if the subquery returns any rows.
pub fn exists<'a, V, S>(
    subquery: S,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    S: ToSQL<'a, V>,
{
    SQLExpr::new(
        SQL::from_iter([Token::EXISTS, Token::LPAREN])
            .append(subquery.into_sql())
            .push(Token::RPAREN),
    )
}

/// NOT EXISTS subquery check.
///
/// Returns true if the subquery returns no rows.
pub fn not_exists<'a, V, S>(
    subquery: S,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, Scalar>
where
    V: SQLParam + 'a,
    S: ToSQL<'a, V>,
{
    SQLExpr::new(
        SQL::from_iter([Token::NOT, Token::EXISTS, Token::LPAREN])
            .append(subquery.into_sql())
            .push(Token::RPAREN),
    )
}
