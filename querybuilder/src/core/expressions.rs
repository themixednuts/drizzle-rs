pub mod conditions;

use crate::core::{SQL, SQLParam, ToSQL};
use std::borrow::Cow;

// Helper function to format SQL comparison expressions
pub(crate) fn format_sql_comparison<'a, V, L, R>(left: L, op: &'static str, right: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    let SQL(left_sql, mut left_params) = left.to_sql();
    let SQL(right_sql, mut right_params) = right.to_sql();

    let mut params = Vec::with_capacity(left_params.len() + right_params.len());
    params.append(&mut left_params);
    params.append(&mut right_params);

    let sql = format!("{} {} {}", left_sql, op, right_sql);

    SQL(Cow::Owned(sql), params)
}

// Helper function to format SQL unary expressions
pub(crate) fn format_sql_unary<'a, V, T>(expr: T, op: &'static str) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    let SQL(expr_sql, params) = expr.to_sql();

    // If expr_sql is already an owned value, we can append directly without allocating a new string
    if let Cow::Owned(mut owned_sql) = expr_sql {
        owned_sql.push_str(" ");
        owned_sql.push_str(op);
        return SQL(Cow::Owned(owned_sql), params);
    }

    // Otherwise, we need to format a new string
    SQL(Cow::Owned(format!("{} {}", expr_sql, op)), params)
}
