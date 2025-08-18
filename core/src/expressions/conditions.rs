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

pub fn eq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "=", right)
}

pub fn neq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<>", right)
}

pub fn gt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, ">", right)
}

pub fn gte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, ">=", right)
}

pub fn lt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<", right)
}

pub fn lte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: SQLComparable<'a, V, R> + ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<=", right)
}

// in_array - column IN (values)
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

// not_in_array - column NOT IN (values)
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

// is_null - column IS NULL
pub fn is_null<'a, V, R>(right: R) -> SQL<'a, V>
where
    V: SQLParam,
    R: ToSQL<'a, V>,
{
    right.to_sql().append_raw("IS NULL")
}

// is_not_null - column IS NOT NULL
pub fn is_not_null<'a, V, R>(right: R) -> SQL<'a, V>
where
    V: SQLParam,
    R: ToSQL<'a, V>,
{
    right.to_sql().append_raw("IS NOT NULL")
}

// exists - EXISTS (subquery)
pub fn exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    SQL::raw("EXISTS (")
        .append(subquery.to_sql())
        .append_raw(")")
}

// not_exists - NOT EXISTS (subquery)
pub fn not_exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    SQL::raw("NOT EXISTS (")
        .append(subquery.to_sql())
        .append_raw(")")
}

// between - column BETWEEN lower AND upper
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

// not_between - column NOT BETWEEN lower AND upper
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

// String LIKE - column LIKE pattern
pub fn like<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    left.to_sql().append_raw("LIKE").append(pattern.to_sql())
}

// String NOT LIKE - column NOT LIKE pattern
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

// Create a NOT condition
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

// is_in - column IN (values)
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

// not_in - column NOT IN (values)
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

// Combine conditions with AND
pub fn and<'a, V, T>(conditions: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: IntoIterator<Item = SQL<'a, V>>,
{
    let mut iter = conditions.into_iter();

    match iter.next() {
        None => SQL::empty(), // No conditions = empty
        Some(first) => {
            let second = iter.next();
            if second.is_none() {
                // Single condition doesn't need parentheses
                first
            } else {
                // Multiple conditions - rebuild iterator and wrap in parentheses
                let all_conditions = std::iter::once(first)
                    .chain(std::iter::once(second.unwrap()))
                    .chain(iter);
                SQL::raw("(")
                    .append(SQL::join(all_conditions, "AND"))
                    .append_raw(")")
            }
        }
    }
}

// Combine conditions with OR
pub fn or<'a, V, T>(conditions: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: IntoIterator<Item = SQL<'a, V>>,
{
    let mut iter = conditions.into_iter();

    match iter.next() {
        None => SQL::empty(), // No conditions = empty
        Some(first) => {
            let second = iter.next();
            if second.is_none() {
                // Single condition doesn't need parentheses
                first
            } else {
                // Multiple conditions - rebuild iterator and wrap in parentheses
                let all_conditions = std::iter::once(first)
                    .chain(std::iter::once(second.unwrap()))
                    .chain(iter);
                SQL::raw("(")
                    .append(SQL::join(all_conditions, "OR"))
                    .append_raw(")")
            }
        }
    }
}

// String concatenation (using || operator)
pub fn string_concat<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
    left.to_sql().append_raw("||").append(right.to_sql())
}

// Case-insensitive LIKE (PostgreSQL specific, might need conditional compilation)
pub fn ilike<'a, V, L, R>(left: L, pattern: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    left.to_sql().append_raw("ILIKE").append(pattern.to_sql())
}
