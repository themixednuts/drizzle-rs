pub mod conditions;

use crate::{SQL, SQLParam, ToSQL};

/// Creates an aliased SQL expression (e.g., "column AS alias").
///
/// # Arguments
/// * `col` - The column or expression to alias (must implement `ToSQL`).
/// * `alias` - The desired alias name.
///
/// # Returns
/// An `SQL` fragment representing the aliased expression.
///
/// # Example
/// ```
/// # use querybuilder::core::{SQL, SQLParam, ToSQL};
/// # use querybuilder::core::expressions::alias;
/// # use std::borrow::Cow;
/// # #[derive(Clone)] pub struct MockValue;
/// # impl SQLParam for MockValue {}
/// # struct MyColumn;
/// # impl<'a> ToSQL<'a, MockValue> for MyColumn {
/// #     fn to_sql(&self) -> SQL<'a, MockValue> { SQL { fragments: vec![Cow::Borrowed("my_col")], params: vec![] } }
/// # }
/// let aliased_col = alias(MyColumn, "mc");
/// assert_eq!(aliased_col.to_string(), "my_col AS mc");
/// ```
pub fn alias<'a, V: SQLParam + 'a, C: ToSQL<'a, V>>(col: C, alias: &'a str) -> SQL<'a, V> {
    col.to_sql().append_raw(" AS ").append(SQL::raw(alias))
}

/// Helper function to create a COUNT expression
///
/// # Arguments
/// * `expr` - The expression to count
///
/// # Returns
/// An `SQL` fragment representing COUNT(expr)
pub fn count<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    create_aggregate_function(expr, "COUNT")
}

/// Helper function to create a SUM expression
///
/// # Arguments
/// * `expr` - The expression to sum
///
/// # Returns
/// An `SQL` fragment representing SUM(expr)
pub fn sum<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    create_aggregate_function(expr, "SUM")
}

/// Helper function to create an AVG expression
///
/// # Arguments
/// * `expr` - The expression to average
///
/// # Returns
/// An `SQL` fragment representing AVG(expr)
pub fn avg<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    create_aggregate_function(expr, "AVG")
}

/// Helper function to create a MIN expression
///
/// # Arguments
/// * `expr` - The expression to find the minimum of
///
/// # Returns
/// An `SQL` fragment representing MIN(expr)
pub fn min<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    create_aggregate_function(expr, "MIN")
}

/// Helper function to create a MAX expression
///
/// # Arguments
/// * `expr` - The expression to find the maximum of
///
/// # Returns
/// An `SQL` fragment representing MAX(expr)
pub fn max<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    create_aggregate_function(expr, "MAX")
}

/// Helper function to create a DISTINCT expression
///
/// # Arguments
/// * `expr` - The expression to apply DISTINCT to
///
/// # Returns
/// An `SQL` fragment representing DISTINCT expr
pub fn distinct<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::raw("DISTINCT ").append(expr.to_sql())
}

/// Helper function to create a COALESCE expression
///
/// # Arguments
/// * `expr` - The primary expression
/// * `default` - The default value to use if expr is NULL
///
/// # Returns
/// An `SQL` fragment representing COALESCE(expr, default)
pub fn coalesce<'a, V: SQLParam + 'a, E: ToSQL<'a, V>, D: ToSQL<'a, V>>(
    expr: E,
    default: D,
) -> SQL<'a, V> {
    SQL::raw("COALESCE(")
        .append(expr.to_sql())
        .append_raw(", ")
        .append(default.to_sql())
        .append_raw(")")
}

// Helper function to create SQL aggregate function expressions
fn create_aggregate_function<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(
    expr: E,
    function_name: &'a str,
) -> SQL<'a, V> {
    SQL::raw(function_name)
        .append_raw("(")
        .append(expr.to_sql())
        .append_raw(")")
}
