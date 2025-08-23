use crate::{SQL, SQLChunk, SQLComparable, SQLParam, ToSQL};

/// Format a SQL comparison with the given operator
fn internal_format_sql_comparison<'a, V, L, R>(
    left: L,
    operator: &'static str,
    right: R,
) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    left.to_sql().append_raw(operator).append(right.to_sql())
}

/// Create an equality condition (=)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::eq;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let left = SQL::<&str>::raw("name");
/// let right = SQL::<&str>::raw("'Item A'");
/// let condition = eq(left, right);
/// assert_eq!(condition.sql(), "name='Item A'");
/// # }
/// ```
pub fn eq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "=", right)
}

/// Create a not-equal condition (<>)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::neq;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let left = SQL::<&str>::raw("name");
/// let right = SQL::<&str>::raw("'Item A'");
/// let condition = neq(left, right);
/// assert_eq!(condition.sql(), "name<>'Item A'");
/// # }
/// ```
pub fn neq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<>", right)
}

/// Create a greater-than condition (>)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::gt;
/// # use drizzle_core::SQL;
/// let left = SQL::raw("id");
/// let condition = gt(left, 1);
/// assert_eq!(condition.sql(), "id>?");
/// ```
pub fn gt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, ">", right)
}

/// Create a greater-than-or-equal condition (>=)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::gte;
/// # use drizzle_core::SQL;
/// let left = SQL::raw("id");
/// let condition = gte(left, 2);
/// assert_eq!(condition.sql(), "id>=?");
/// ```
pub fn gte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, ">=", right)
}

/// Create a less-than condition (<)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::lt;
/// # use drizzle_core::SQL;
/// let left = SQL::raw("id");
/// let condition = lt(left, 3);
/// assert_eq!(condition.sql(), "id<?");
/// ```
pub fn lt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<", right)
}

/// Create a less-than-or-equal condition (<=)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::lte;
/// # use drizzle_core::SQL;
/// let left = SQL::raw("id");
/// let condition = lte(left, 2);
/// assert_eq!(condition.sql(), "id<=?");
/// ```
pub fn lte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<=", right)
}

/// Create an IN condition with an array of values
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::in_array;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let left = SQL::<&str>::raw("name");
/// let values = [SQL::<&str>::raw("'Apple'"), SQL::<&str>::raw("'Cherry'")];
/// let condition = in_array(left, values);
/// assert_eq!(condition.sql(), "name IN ('Apple','Cherry')");
/// # }
/// ```
pub fn in_array<'a, V, L, I, R>(left: L, values: I) -> SQL<'a, V>
where
    V: SQLParam,
    L: ToSQL<'a, V>,
    I: IntoIterator<Item = R>,
    R: Into<V> + ToSQL<'a, V>,
{
    let left_sql = left.to_sql();
    let mut values_iter = values.into_iter();

    match values_iter.next() {
        None => left_sql.append_raw("IN (NULL)"),
        Some(first_value) => {
            let mut result = left_sql.append_raw("IN (").append(first_value.to_sql());
            for value in values_iter {
                result = result.append_raw(",").append(value.to_sql());
            }
            result.append_raw(")")
        }
    }
}

/// Create a NOT IN condition with an array of values
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::not_in_array;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let left = SQL::<&str>::raw("name");
/// let values = [SQL::<&str>::raw("'Apple'")];
/// let condition = not_in_array(left, values);
/// assert_eq!(condition.sql(), "name NOT IN ('Apple')");
/// # }
/// ```
pub fn not_in_array<'a, V, L, I, R>(left: L, values: I) -> SQL<'a, V>
where
    V: SQLParam,
    L: ToSQL<'a, V>,
    I: IntoIterator<Item = R>,
    R: Into<V> + ToSQL<'a, V>,
{
    let left_sql = left.to_sql();
    let mut values_iter = values.into_iter();

    match values_iter.next() {
        None => left_sql.append_raw("NOT IN (NULL)"),
        Some(first_value) => {
            let mut result = left_sql.append_raw("NOT IN (").append(first_value.to_sql());
            for value in values_iter {
                result = result.append_raw(",").append(value.to_sql());
            }
            result.append_raw(")")
        }
    }
}

/// Create an IS NULL condition
///
/// # Example
/// ```
/// # use drizzle_core::expressions::conditions::is_null;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("email");
/// let condition = is_null(column);
/// assert_eq!(condition.sql(), "email IS NULL");
/// # }
/// ```
pub fn is_null<'a, V, R>(right: R) -> SQL<'a, V>
where
    V: SQLParam,
    R: ToSQL<'a, V>,
{
    right.to_sql().append_raw("IS NULL")
}

/// Create an IS NOT NULL condition
///
/// # Example
/// ```
/// # use drizzle_core::expressions::conditions::is_not_null;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let column = SQL::<&str>::raw("email");
/// let condition = is_not_null(column);
/// assert_eq!(condition.sql(), "email IS NOT NULL");
/// # }
/// ```
pub fn is_not_null<'a, V, R>(right: R) -> SQL<'a, V>
where
    V: SQLParam,
    R: ToSQL<'a, V>,
{
    right.to_sql().append_raw("IS NOT NULL")
}

/// Create an EXISTS condition with a subquery
///
/// # Example
/// ```
/// # use drizzle_core::expressions::conditions::exists;
/// # use drizzle_core::SQL;
/// let subquery = SQL::<&'_ str>::raw("SELECT 1 FROM users WHERE active = 1");
/// let condition = exists(subquery);
/// assert_eq!(condition.sql(), "EXISTS (SELECT 1 FROM users WHERE active = 1)");
/// ```
pub fn exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    SQL::raw("EXISTS (")
        .append(subquery.to_sql())
        .append_raw(")")
}

/// Create a NOT EXISTS condition with a subquery
///
/// # Example
/// ```
/// # use drizzle_core::expressions::conditions::not_exists;
/// # use drizzle_core::SQL;
/// let subquery = SQL::<&'_ str>::raw("SELECT 1 FROM inactive_users");
/// let condition = not_exists(subquery);
/// assert_eq!(condition.sql(), "NOT EXISTS (SELECT 1 FROM inactive_users)");
/// ```
pub fn not_exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    SQL::raw("NOT EXISTS (")
        .append(subquery.to_sql())
        .append_raw(")")
}

/// Create a BETWEEN condition
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::between;
/// # use drizzle_core::SQL;
/// let left = SQL::raw("age");
/// let condition = between(left, 22, 28);
/// assert_eq!(condition.sql(), "(age BETWEEN ? AND ?)");
/// ```
pub fn between<'a, V, L, R1, R2>(left: L, start: R1, end: R2) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R1: ToSQL<'a, V>,
    R2: ToSQL<'a, V>,
{
    SQL::raw("(")
        .append(left.to_sql())
        .append_raw("BETWEEN")
        .append(start.to_sql())
        .append_raw("AND")
        .append(end.to_sql())
        .append_raw(")")
}

/// Create a NOT BETWEEN condition
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::not_between;
/// # use drizzle_core::SQL;
/// let left = SQL::raw("age");
/// let condition = not_between(left, 22, 28);
/// assert_eq!(condition.sql(), "(age NOT BETWEEN ? AND ?)");
/// ```
pub fn not_between<'a, V, L, R1, R2>(left: L, lower: R1, upper: R2) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R1: ToSQL<'a, V>,
    R2: ToSQL<'a, V>,
{
    SQL::raw("(")
        .append(left.to_sql())
        .append_raw("NOT BETWEEN")
        .append(lower.to_sql())
        .append_raw("AND")
        .append(upper.to_sql())
        .append_raw(")")
}

/// Create a LIKE condition for pattern matching
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::like;
/// # use drizzle_core::SQL;
/// let left = SQL::<&str>::raw("name");
/// let condition = like(left, "Apple%");
/// assert_eq!(condition.sql(), "name LIKE ?");
/// ```
pub fn like<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    left.to_sql().append_raw("LIKE").append(pattern.to_sql())
}

/// Create a NOT LIKE condition for pattern matching
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::not_like;
/// # use drizzle_core::SQL;
/// let left = SQL::<&str>::raw("name");
/// let condition = not_like(left, "Apple%");
/// assert_eq!(condition.sql(), "name NOT LIKE ?");
/// ```
pub fn not_like<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    left.to_sql()
        .append_raw("NOT LIKE")
        .append(pattern.to_sql())
}

/// Create a NOT condition
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::{not, eq};
/// # use drizzle_core::SQL;
/// let column = SQL::<&str>::raw("active");
/// let condition = not(eq(column, "true"));
/// assert_eq!(condition.sql(), "NOT(active='true')");
/// ```
pub fn not<'a, V, T>(expression: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    let expr_sql = expression.to_sql();
    // Check if the expression needs wrapping in parentheses
    // Simple heuristic: wrap if it contains more than one chunk or the chunk isn't simple text
    let needs_paren = expr_sql.chunks.len() > 1
        || (expr_sql.chunks.len() == 1 && !matches!(expr_sql.chunks[0], SQLChunk::Text(_)));

    if needs_paren {
        SQL::raw("NOT(").append(expr_sql).append_raw(")")
    } else {
        SQL::raw("NOT").append(expr_sql)
    }
}

/// Create an IN condition (alias for in_array)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::is_in;
/// # use drizzle_core::SQL;
/// let left = SQL::raw("id");
/// let condition = is_in(left, [1, 3]);
/// assert_eq!(condition.sql(), "id IN (?,?)");
/// ```
pub fn is_in<'a, V, L, I, T>(left: L, values: I) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    I: IntoIterator<Item = T>,
    T: Clone + Into<V> + ToSQL<'a, V>,
{
    let left_sql = left.to_sql();
    let mut values_iter = values.into_iter();

    match values_iter.next() {
        None => left_sql.append_raw("IN (NULL)"),
        Some(first_value) => {
            let mut result = left_sql.append_raw("IN (").append(first_value.to_sql());
            for value in values_iter {
                result = result.append_raw(",").append(value.to_sql());
            }
            result.append_raw(")")
        }
    }
}

/// Create a NOT IN condition (alias for not_in_array)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::not_in;
/// # use drizzle_core::SQL;
/// let left = SQL::raw("id");
/// let condition = not_in(left, [1]);
/// assert_eq!(condition.sql(), "id NOT IN (?)");
/// ```
pub fn not_in<'a, V, L, I, T>(left: L, values: I) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    I: IntoIterator<Item = T>,
    T: Clone + Into<V> + ToSQL<'a, V>,
{
    let left_sql = left.to_sql();
    let mut values_iter = values.into_iter();

    match values_iter.next() {
        None => left_sql.append_raw("NOT IN (NULL)"),
        Some(first_value) => {
            let mut result = left_sql.append_raw("NOT IN (").append(first_value.to_sql());
            for value in values_iter {
                result = result.append_raw(",").append(value.to_sql());
            }
            result.append_raw(")")
        }
    }
}

/// Combine multiple conditions with AND
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::{and, eq};
/// # use drizzle_core::SQL;
/// let col1 = SQL::<&str>::raw("active");
/// let col2 = SQL::<&str>::raw("role");
/// let condition = and([
///     eq(col1, "true"),
///     eq(col2, "admin")
/// ]);
/// assert_eq!(condition.sql(), "(active='true' AND role='admin')");
/// ```
pub fn and<'a, V, T>(conditions: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: IntoIterator<Item = SQL<'a, V>>,
{
    let mut iter = conditions.into_iter();

    match iter.next() {
        None => SQL::empty(), // No conditions = empty
        Some(first) => {
            let Some(second) = iter.next() else {
                return first;
            };
            // Multiple conditions - rebuild iterator and wrap in parentheses
            let all_conditions = std::iter::once(first)
                .chain(std::iter::once(second))
                .chain(iter);
            SQL::raw("(")
                .append(SQL::join(all_conditions, "AND"))
                .append_raw(")")
        }
    }
}

/// Combine multiple conditions with OR
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::{or, eq};
/// # use drizzle_core::SQL;
/// let col1 = SQL::<&str>::raw("role");
/// let col2 = SQL::<&str>::raw("status");
/// let condition = or([
///     eq(col1, "admin"),
///     eq(col2, "premium")
/// ]);
/// assert_eq!(condition.sql(), "(role='admin' OR status='premium')");
/// ```
pub fn or<'a, V, T>(conditions: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: IntoIterator<Item = SQL<'a, V>>,
{
    let mut iter = conditions.into_iter();

    match iter.next() {
        None => SQL::empty(), // No conditions = empty
        Some(first) => {
            let Some(second) = iter.next() else {
                return first;
            };
            // Single condition doesn't need parentheses
            // Multiple conditions - rebuild iterator and wrap in parentheses
            let all_conditions = std::iter::once(first)
                .chain(std::iter::once(second))
                .chain(iter);
            SQL::raw("(")
                .append(SQL::join(all_conditions, "OR"))
                .append_raw(")")
        }
    }
}

/// Create a string concatenation expression using || operator
///
/// # Example
/// ```
/// # use drizzle_core::expressions::conditions::string_concat;
/// # use drizzle_core::SQL;
/// # fn main() {
/// let left = SQL::<&str>::raw("name");
/// let right = SQL::<&str>::raw("' - Suffix'");
/// let concat_expr = string_concat(left, right);
/// assert_eq!(concat_expr.sql(), "name || ' - Suffix'");
/// # }
/// ```
pub fn string_concat<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    left.to_sql().append_raw("||").append(right.to_sql())
}

/// Create a case-insensitive LIKE condition (PostgreSQL specific)
///
/// # Example
/// ```ignore
/// # use drizzle_core::expressions::conditions::ilike;
/// # use drizzle_core::SQL;
/// let left = SQL::<&str>::raw("name");
/// let condition = ilike(left, "apple%");
/// assert_eq!(condition.sql(), "name ILIKE ?");
/// ```
pub fn ilike<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    left.to_sql().append_raw("ILIKE").append(pattern.to_sql())
}
