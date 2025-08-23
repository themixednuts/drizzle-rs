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
///
/// # Example
/// ```
/// # use drizzle_core::expressions::cte;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let cte_query = SQL::<&str>::raw("SELECT id, name FROM users WHERE id = 42");
/// let sq = cte("my_cte").r#as(cte_query);
/// assert_eq!(sq.name(), "my_cte");
/// # }
/// ```
pub fn cte(name: &'static str) -> CTE {
    CTE { name }
}

/// Combine a CTE with a main query
///
/// # Example
/// ```
/// # use drizzle_core::expressions::{cte, with};
/// # use drizzle_core::SQL;
/// # fn main() {
/// let cte_query = SQL::<&str>::raw("SELECT id, name FROM users WHERE id = 42");
/// let main_query = SQL::<&str>::raw("SELECT * FROM sq");
/// let sq = cte("sq").r#as(cte_query);
/// let result = with(sq, main_query);
/// let sql_output = result.sql();
/// assert_eq!(sql_output, "WITH sq AS (SELECT id, name FROM users WHERE id = 42) SELECT * FROM sq");
/// # }
/// ```
pub fn with<'a, V: SQLParam + 'a, Q: ToSQL<'a, V>, M: ToSQL<'a, V>>(
    cte: DefinedCTE<'a, V, Q>,
    main_query: M,
) -> SQL<'a, V> {
    SQL::raw("WITH")
        .append(cte.definition())
        .append(main_query.to_sql())
}

/// Create an alias for a column or expression
///
/// # Arguments
/// * `col` - The column or expression to alias
/// * `alias` - The alias name
///
/// # Example
/// ```
/// # use drizzle_core::expressions::alias;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("name");
/// let aliased = alias(column, "item_name");
/// assert_eq!(aliased.sql(), "name AS item_name");
/// # }
/// ```
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
///
/// # Example
/// ```
/// # use drizzle_core::expressions::count;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("id");
/// let count_expr = count(column);
/// assert_eq!(count_expr.sql(), "COUNT (id)");
/// # }
/// ```
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
///
/// # Example
/// ```
/// # use drizzle_core::expressions::sum;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("amount");
/// let sum_expr = sum(column);
/// assert_eq!(sum_expr.sql(), "SUM (amount)");
/// # }
/// ```
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
///
/// # Example
/// ```
/// # use drizzle_core::expressions::avg;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("score");
/// let avg_expr = avg(column);
/// assert_eq!(avg_expr.sql(), "AVG (score)");
/// # }
/// ```
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
///
/// # Example
/// ```
/// # use drizzle_core::expressions::min;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("age");
/// let min_expr = min(column);
/// assert_eq!(min_expr.sql(), "MIN (age)");
/// # }
/// ```
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
///
/// # Example
/// ```
/// # use drizzle_core::expressions::max;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("price");
/// let max_expr = max(column);
/// assert_eq!(max_expr.sql(), "MAX (price)");
/// # }
/// ```
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
///
/// # Example
/// ```
/// # use drizzle_core::expressions::distinct;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("name");
/// let distinct_expr = distinct(column);
/// assert_eq!(distinct_expr.sql(), "DISTINCT name");
/// # }
/// ```
pub fn distinct<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::raw("DISTINCT").append(expr.to_sql())
}

/// Helper function to create a COALESCE expression
///
/// # Arguments
/// * `expr` - The primary expression
/// * `default` - The default value to use if expr is NULL
///
/// # Returns
/// An `SQL` fragment representing COALESCE(expr, default)
///
/// # Example
/// ```
/// # use drizzle_core::expressions::coalesce;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("email");
/// let default_val = SQL::<&str>::raw("'no-email@example.com'");
/// let coalesce_expr = coalesce(column, default_val);
/// assert_eq!(coalesce_expr.sql(), "COALESCE(email, 'no-email@example.com')");
/// # }
/// ```
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

/// Helper function to create a TYPEOF expression
///
/// # Arguments
/// * `expr` - The expression to get the type of
///
/// # Returns
/// An `SQL` fragment representing TYPEOF(expr)
///
/// # Example
/// ```
/// # use drizzle_core::expressions::r#typeof;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("value");
/// let typeof_expr = r#typeof(column);
/// assert_eq!(typeof_expr.sql(), "TYPEOF(value)");
/// # }
/// ```
pub fn r#typeof<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::raw("TYPEOF(").append(expr.to_sql()).append_raw(")")
}

/// Helper function to create a CAST expression
///
/// # Arguments
/// * `expr` - The expression to cast
/// * `target_type` - The target type to cast to (e.g., "INTEGER", "TEXT", "REAL")
///
/// # Returns
/// An `SQL` fragment representing CAST(expr AS target_type)
///
/// # Example
/// ```
/// # use drizzle_core::expressions::cast;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("value");
/// let cast_expr = cast(column, "INTEGER");
/// assert_eq!(cast_expr.sql(), "CAST(value AS INTEGER)");
/// # }
/// ```
pub fn cast<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E, target_type: &'a str) -> SQL<'a, V> {
    SQL::raw("CAST(")
        .append(expr.to_sql())
        .append_raw(" AS ")
        .append_raw(target_type)
        .append_raw(")")
}

// Helper function to create SQL aggregate function expressions
fn create_aggregate_function<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(
    expr: E,
    function_name: &'a str,
) -> SQL<'a, V> {
    SQL::raw(function_name).append(expr.to_sql().subquery())
}
