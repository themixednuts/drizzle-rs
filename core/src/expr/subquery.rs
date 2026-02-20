//! Typed subquery SQL-type extraction.
//!
//! This module maps a `SELECT` marker to the SQL type produced by the subquery:
//! - single-column selects map to the column SQL type
//! - multi-column selects map to a tuple of SQL types

use crate::traits::SQLParam;
use crate::types::DataType;

use super::Expr;

/// Maps a select marker to the SQL type produced by that subquery.
pub trait SubqueryType<'a, V: SQLParam> {
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

impl<'a, V, M, Scope> SubqueryType<'a, V> for crate::row::Scoped<M, Scope>
where
    V: SQLParam + 'a,
    M: SubqueryType<'a, V>,
{
    type SQLType = M::SQLType;
}
