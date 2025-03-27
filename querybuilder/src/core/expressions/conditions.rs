use std::borrow::Cow;
use std::fmt::Display;

use super::{format_sql_comparison, format_sql_unary};
use crate::core::{IntoValue, SQL, SQLParam, ToSQL};

// eq - column = value
pub fn eq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    format_sql_comparison(left, "=", right)
}

// ne - column != value
pub fn ne<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    format_sql_comparison(left, "!=", right)
}

// gt - column > value
pub fn gt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    format_sql_comparison(left, ">", right)
}

// gte - column >= value
pub fn gte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    format_sql_comparison(left, ">=", right)
}

// lt - column < value
pub fn lt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    format_sql_comparison(left, "<", right)
}

// lte - column <= value
pub fn lte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    format_sql_comparison(left, "<=", right)
}

// in_array - column IN (values)
pub fn in_array<'a, V, L, R>(left: L, values: Vec<R>) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Clone,
{
    let params = vec!["?"; values.len()].join(", ");
    let sql = format!("{} IN ({})", left, params);
    let values = values.into_iter().map(|v| v.into_value()).collect();
    SQL(Cow::Owned(sql), values)
}

// not_in_array - column NOT IN (values)
pub fn not_in_array<'a, V, L, R>(left: L, values: Vec<R>) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Clone,
{
    let params = vec!["?"; values.len()].join(", ");
    let sql = format!("{} NOT IN ({})", left, params);
    let values = values.into_iter().map(|v| v.into_value()).collect();
    SQL(Cow::Owned(sql), values)
}

// is_null - column IS NULL
pub fn is_null<'a, V, T>(expr: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
{
    format_sql_unary(expr, "IS NULL")
}

// is_not_null - column IS NOT NULL
pub fn is_not_null<'a, V, T>(expr: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
{
    format_sql_unary(expr, "IS NOT NULL")
}

// exists - EXISTS (subquery)
pub fn exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
{
    let SQL(subquery_sql, subquery_params) = subquery.to_sql();
    let sql_str = format!("EXISTS ({})", subquery_sql);
    SQL(Cow::Owned(sql_str), subquery_params)
}

// not_exists - NOT EXISTS (subquery)
pub fn not_exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
{
    let SQL(subquery_sql, subquery_params) = subquery.to_sql();
    let sql_str = format!("NOT EXISTS ({})", subquery_sql);
    SQL(Cow::Owned(sql_str), subquery_params)
}

// between - column BETWEEN lower AND upper
pub fn between<'a, V, T, L, U>(expr: T, lower: L, upper: U) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
    L: IntoValue<V>,
    U: IntoValue<V>,
{
    let sql_str = format!("{} BETWEEN ? AND ?", expr);
    SQL(
        Cow::Owned(sql_str),
        vec![lower.into_value(), upper.into_value()],
    )
}

// not_between - column NOT BETWEEN lower AND upper
pub fn not_between<'a, V, T, L, U>(expr: T, lower: L, upper: U) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
    L: IntoValue<V>,
    U: IntoValue<V>,
{
    let sql_str = format!("{} NOT BETWEEN ? AND ?", expr);
    SQL(
        Cow::Owned(sql_str),
        vec![lower.into_value(), upper.into_value()],
    )
}

// like - column LIKE pattern
pub fn like<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    format_sql_comparison(left, "LIKE", pattern)
}

// not_like - column NOT LIKE pattern
pub fn not_like<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam,
    L: Display + ToSQL<'a, V>,
    R: IntoValue<V> + Display + ToSQL<'a, V>,
{
    format_sql_comparison(left, "NOT LIKE", pattern)
}

// Combine multiple expressions with AND, filtering out None values
pub fn and<'a, V, T>(expressions: Vec<Option<T>>) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
{
    let filtered: Vec<_> = expressions.into_iter().filter_map(|e| e).collect();

    if filtered.is_empty() {
        return SQL(Cow::Borrowed(""), Vec::new());
    }

    if filtered.len() == 1 {
        return filtered[0].to_sql();
    }

    let mut combined_sql = String::from("(");
    let mut combined_params = Vec::new();

    for (i, expr) in filtered.iter().enumerate() {
        let SQL(sql, mut params) = expr.to_sql();

        if i > 0 {
            combined_sql.push_str(" AND ");
        }

        combined_sql.push_str(&sql);
        combined_params.append(&mut params);
    }

    combined_sql.push(')');
    SQL(Cow::Owned(combined_sql), combined_params)
}

// Combine multiple expressions with OR, filtering out None values
pub fn or<'a, V, T>(expressions: Vec<Option<T>>) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
{
    let filtered: Vec<_> = expressions.into_iter().filter_map(|e| e).collect();

    if filtered.is_empty() {
        return SQL(Cow::Borrowed(""), Vec::new());
    }

    if filtered.len() == 1 {
        return filtered[0].to_sql();
    }

    let mut combined_sql = String::from("(");
    let mut combined_params = Vec::new();

    for (i, expr) in filtered.iter().enumerate() {
        let SQL(sql, mut params) = expr.to_sql();

        if i > 0 {
            combined_sql.push_str(" OR ");
        }

        combined_sql.push_str(&sql);
        combined_params.append(&mut params);
    }

    combined_sql.push(')');
    SQL(Cow::Owned(combined_sql), combined_params)
}

// Create a NOT condition
pub fn not<'a, V, T>(expression: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: Display + ToSQL<'a, V>,
{
    let SQL(sql, params) = expression.to_sql();
    let sql_str = format!("NOT ({})", sql);
    SQL(Cow::Owned(sql_str), params)
}
