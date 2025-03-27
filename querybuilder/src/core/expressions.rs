pub mod conditions;

use crate::core::{IntoValue, SQL, SQLParam, ToSQL};
use std::borrow::Cow;
use std::fmt::Display;

// Helper function to format SQL comparison expressions
pub(crate) fn format_sql_comparison<'a, V, L, R>(left: L, op: &'static str, right: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    let SQL(left_sql, mut left_params) = left.to_sql();
    let SQL(right_sql, mut right_params) = right.to_sql();

    let sql = format!("{} {} {}", left_sql, op, right_sql);
    let mut params = Vec::new();
    params.append(&mut left_params);
    params.append(&mut right_params);

    SQL(Cow::Owned(sql), params)
}

// Helper function to format SQL unary expressions
pub(crate) fn format_sql_unary<'a, V, T>(expr: T, op: &'static str) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
{
    let SQL(expr_sql, params) = expr.to_sql();
    SQL(Cow::Owned(format!("{} {}", expr_sql, op)), params)
}
