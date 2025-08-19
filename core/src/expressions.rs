pub mod conditions;

use crate::{SQL, SQLParam, ToSQL};

/// A Common Table Expression (CTE) builder
pub struct CTE {
    name: &'static str,
}

/// A defined CTE with its query
pub struct DefinedCTE<'a, V: SQLParam, Q: ToSQL<'a, V>> {
    name: &'a str,
    query: Q,
    _phantom: std::marker::PhantomData<V>,
}

impl CTE {
    /// Define the CTE with a query
    pub fn r#as<'a, V: SQLParam, Q: ToSQL<'a, V>>(self, query: Q) -> DefinedCTE<'a, V, Q> {
        DefinedCTE {
            name: self.name,
            query,
            _phantom: std::marker::PhantomData,
        }
    }
}

impl<'a, V: SQLParam, Q: ToSQL<'a, V>> DefinedCTE<'a, V, Q> {
    /// Get the CTE name for referencing in queries
    pub fn name(&self) -> &'a str {
        self.name
    }
}

impl<'a, V: SQLParam + 'a, Q: ToSQL<'a, V>> ToSQL<'a, V> for DefinedCTE<'a, V, Q> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::raw(self.name)
    }
}

impl<'a, V: SQLParam, Q: ToSQL<'a, V>> DefinedCTE<'a, V, Q> {
    /// Get the full CTE definition for WITH clauses
    pub fn definition(&self) -> SQL<'a, V> {
        SQL::raw(self.name)
            .append_raw("AS")
            .append(self.query.to_sql().subquery())
    }
}

impl<'a, V: SQLParam, Q: ToSQL<'a, V>> AsRef<DefinedCTE<'a, V, Q>> for DefinedCTE<'a, V, Q> {
    fn as_ref(&self) -> &DefinedCTE<'a, V, Q> {
        self
    }
}

/// Create a new CTE with the given name
pub fn cte(name: &'static str) -> CTE {
    CTE { name }
}

/// Combine a CTE with a main query
pub fn with<'a, V: SQLParam + 'a, Q: ToSQL<'a, V>, M: ToSQL<'a, V>>(
    cte: DefinedCTE<'a, V, Q>,
    main_query: M,
) -> SQL<'a, V> {
    SQL::raw("WITH")
        .append(cte.definition())
        .append(main_query.to_sql())
}

pub fn alias<'a, V: SQLParam + 'a, C: ToSQL<'a, V>>(col: C, alias: &'a str) -> SQL<'a, V> {
    col.to_sql()
        .append_raw(" AS ")
        .append(SQL::<'a, V>::raw(alias))
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
