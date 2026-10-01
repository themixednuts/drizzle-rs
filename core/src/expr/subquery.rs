//! The SQL type a subquery produces, for use in comparisons and `IN`.
//!
//! A one-column select has that column's SQL type; a select of several
//! columns has a tuple of their SQL types.

use crate::traits::SQLParam;
use crate::types::DataType;

use super::Expr;

/// Maps a select marker (the type-level record of what a query selects) to
/// the SQL type the query produces as a subquery.
pub trait SubqueryType<'a, V: SQLParam> {
    /// SQL type of the subquery: the column's type, or a tuple of types.
    type SQLType: DataType;
}

impl<'a, V, E> SubqueryType<'a, V> for crate::row::SelectCols<(E,)>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    type SQLType = E::SQLType;
}

macro_rules! impl_subquery_type_tuple {
    ($($E:ident),+; $($idx:tt),+) => {
        impl<'a, V, $($E),+> SubqueryType<'a, V> for crate::row::SelectCols<($($E,)+)>
        where
            V: SQLParam + 'a,
            $($E: Expr<'a, V>,)+
        {
            type SQLType = ($($E::SQLType,)+);
        }
    };
}

macro_rules! with_col_sizes_2_to_8 {
    ($callback:ident) => {
        seq_tuples!(@from $callback
            [E0]
            [0];
            (E1,1) (E2,2) (E3,3)
            (E4,4) (E5,5) (E6,6) (E7,7)
        );
    };
}

with_col_sizes_2_to_8!(impl_subquery_type_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_subquery_type_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_subquery_type_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_subquery_type_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_subquery_type_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_subquery_type_tuple);

impl<'a, V, M, Scope, Used> SubqueryType<'a, V> for crate::row::Scoped<M, Scope, Used>
where
    V: SQLParam + 'a,
    M: SubqueryType<'a, V>,
{
    type SQLType = M::SQLType;
}
