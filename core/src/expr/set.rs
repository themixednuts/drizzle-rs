//! Set operations (IN, NOT IN, EXISTS, NOT EXISTS).

use crate::dialect::DialectTypes;
use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{Compatible, DataType};

use super::{Expr, NonNull, Null, SQLExpr, Scalar};

#[inline]
fn operand_sql<'a, V, T>(value: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    value.into_sql().parens_if_subquery()
}

#[derive(Clone, Copy, Debug)]
pub struct RowValue<T>(T);

pub trait RowValueType<'a, V: SQLParam> {
    type SQLType: DataType;
}

macro_rules! impl_row_value_type_tuple {
    ($($E:ident),+; $($idx:tt),+) => {
        impl<'a, V, $($E),+> RowValueType<'a, V> for ($($E,)+)
        where
            V: SQLParam + 'a,
            $($E: Expr<'a, V>,)+
        {
            type SQLType = ($($E::SQLType,)+);
        }
    };
}

with_col_sizes_8!(impl_row_value_type_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_row_value_type_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_row_value_type_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_row_value_type_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_row_value_type_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_row_value_type_tuple);

impl<'a, V, T> ToSQL<'a, V> for RowValue<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.0.to_sql().parens()
    }

    fn into_sql(self) -> SQL<'a, V> {
        self.0.into_sql().parens()
    }
}

impl<'a, V, T> Expr<'a, V> for RowValue<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V> + RowValueType<'a, V>,
{
    type SQLType = T::SQLType;
    type Nullable = Null;
    type Aggregate = Scalar;
}

/// Wrap an expression or tuple into a row-value expression: `(a, b, ...)`.
pub fn row<T>(value: T) -> RowValue<T> {
    RowValue(value)
}

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
pub fn in_subquery<'a, V, E, S>(
    expr: E,
    subquery: S,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    S: Expr<'a, V>,
    E::SQLType: Compatible<S::SQLType>,
{
    SQLExpr::new(
        operand_sql(expr)
            .push(Token::IN)
            .append(subquery.into_sql().parens()),
    )
}

/// NOT IN subquery check.
pub fn not_in_subquery<'a, V, E, S>(
    expr: E,
    subquery: S,
) -> SQLExpr<'a, V, <V::DialectMarker as DialectTypes>::Bool, NonNull, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    S: Expr<'a, V>,
    E::SQLType: Compatible<S::SQLType>,
{
    SQLExpr::new(
        operand_sql(expr)
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
