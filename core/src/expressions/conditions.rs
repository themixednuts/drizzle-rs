use crate::{SQL, SQLChunk, SQLParam, ToSQL};
use std::borrow::Cow;

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
    let left_sql = left.to_sql();
    let right_sql = right.to_sql();

    // Pre-calculate capacity for all chunks
    let total_capacity = left_sql.chunks.len() + 1 + right_sql.chunks.len();
    let mut chunks = Vec::with_capacity(total_capacity);

    // Add left expression chunks
    chunks.extend(left_sql.chunks);

    // Add operator as a Text chunk
    chunks.push(SQLChunk::Text(Cow::Borrowed(operator)));

    // Add right expression chunks
    chunks.extend(right_sql.chunks);

    SQL { chunks }
}

pub fn eq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "=", right)
}

pub fn neq<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<>", right)
}

pub fn gt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, ">", right)
}

pub fn gte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, ">=", right)
}

pub fn lt<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<", right)
}

pub fn lte<'a, V, L, R>(left: L, right: R) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V> + Into<V>,
{
    internal_format_sql_comparison(left, "<=", right)
}

// in_array - column IN (values)
pub fn in_array<'a, V, L, R>(left: L, values: Vec<R>) -> SQL<'a, V>
where
    V: SQLParam,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    let left_sql = left.to_sql();

    // Early optimization: empty values case
    if values.is_empty() {
        return SQL::raw(format!("{}IN(NULL)", left_sql.sql()));
    }

    let mut result = SQL::raw("");

    // Add left expression
    result = result.append(left_sql);

    // Add "IN(" text
    result = result.append_raw("IN(");

    // Join the values with commas
    let mut first = true;
    for value in values {
        if !first {
            result = result.append_raw(",");
        }
        first = false;

        result = result.append(value.to_sql());
    }

    // Add closing parenthesis
    result = result.append_raw(")");

    result
}

// not_in_array - column NOT IN (values)
pub fn not_in_array<'a, V, L, R>(left: L, values: Vec<R>) -> SQL<'a, V>
where
    V: SQLParam,
    L: ToSQL<'a, V>,
    R: Into<V> + ToSQL<'a, V>,
{
    let left_sql = left.to_sql();

    // Early optimization: empty values case
    if values.is_empty() {
        return SQL::raw(format!("{} NOT IN(NULL)", left_sql.sql()));
    }

    let mut result = SQL::raw("");

    // Add left expression
    result = result.append(left_sql);

    // Add "NOT IN(" text
    result = result.append_raw("NOT IN(");

    // Join the values with commas
    let mut first = true;
    for value in values {
        if !first {
            result = result.append_raw(",");
        }
        first = false;

        result = result.append(value.to_sql());
    }

    // Add closing parenthesis
    result = result.append_raw(")");

    result
}

// is_null - column IS NULL
pub fn is_null<'a, V, R>(right: R) -> SQL<'a, V>
where
    V: SQLParam,
    R: ToSQL<'a, V>,
{
    let right_sql = right.to_sql();

    let mut result = SQL::raw("");
    result = result.append(right_sql);
    result = result.append_raw("IS NULL");

    result
}

// is_not_null - column IS NOT NULL
pub fn is_not_null<'a, V, R>(right: R) -> SQL<'a, V>
where
    V: SQLParam,
    R: ToSQL<'a, V>,
{
    let right_sql = right.to_sql();

    let mut result = SQL::raw("");
    result = result.append(right_sql);
    result = result.append_raw("IS NOT NULL");

    result
}

// exists - EXISTS (subquery)
pub fn exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    let subquery_sql = subquery.to_sql();

    let mut result = SQL::raw("");
    result = result.append_raw("EXISTS(");
    result = result.append(subquery_sql);
    result = result.append_raw(")");

    result
}

// not_exists - NOT EXISTS (subquery)
pub fn not_exists<'a, V, T>(subquery: T) -> SQL<'a, V>
where
    V: SQLParam,
    T: ToSQL<'a, V>,
{
    let subquery_sql = subquery.to_sql();

    let mut result = SQL::raw("");
    result = result.append_raw("NOT EXISTS(");
    result = result.append(subquery_sql);
    result = result.append_raw(")");

    result
}

// between - column BETWEEN lower AND upper
pub fn between<'a, V, L, R1, R2>(left: L, start: R1, end: R2) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R1: ToSQL<'a, V>,
    R2: ToSQL<'a, V>,
{
    let left_sql = left.to_sql();
    let start_sql = start.to_sql();
    let end_sql = end.to_sql();

    let mut result = SQL::raw("");

    // Add opening parenthesis and left expression
    result = result.append_raw("(");
    result = result.append(left_sql);

    // Add BETWEEN clause
    result = result.append_raw("BETWEEN");

    // Add start value
    result = result.append(start_sql);

    // Add AND clause
    result = result.append_raw("AND");

    // Add end value
    result = result.append(end_sql);

    // Add closing parenthesis
    result = result.append_raw(")");

    result
}

// not_between - column NOT BETWEEN lower AND upper
pub fn not_between<'a, V, L, R1, R2>(left: L, lower: R1, upper: R2) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R1: ToSQL<'a, V>,
    R2: ToSQL<'a, V>,
{
    let left_sql = left.to_sql();
    let lower_sql = lower.to_sql();
    let upper_sql = upper.to_sql();

    let mut result = SQL::raw("");

    // Add opening parenthesis and left expression
    result = result.append_raw("(");
    result = result.append(left_sql);

    // Add NOT BETWEEN clause
    result = result.append_raw("NOT BETWEEN");

    // Add lower value
    result = result.append(lower_sql);

    // Add AND clause
    result = result.append_raw("AND");

    // Add upper value
    result = result.append(upper_sql);

    // Add closing parenthesis
    result = result.append_raw(")");

    result
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
    let value_sqls: Vec<SQL<'a, V>> = values.into_iter().map(|v| v.to_sql()).collect();

    if value_sqls.is_empty() {
        // SQL standard requires at least one value in IN, but different DBs handle it differently.
        // Returning `left IN (NULL)` is often safe but might not be desired.
        // Returning a universally false condition like `1=0` might be safer depending on context.
        // For now, stick to `IN (NULL)` as per original code, but this might need revisiting.
        return left_sql.append_raw("IN(NULL)");
    }

    left_sql
        .append_raw("IN(")
        .append(SQL::join(&value_sqls, ",")) // Join values with comma
        .append_raw(")")
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
    let value_sqls: Vec<SQL<'a, V>> = values.into_iter().map(|v| v.to_sql()).collect();

    if value_sqls.is_empty() {
        // Similar to `is_in`, empty `NOT IN` is tricky. `NOT IN (NULL)` is often true.
        // Returning `1=1` might be safer. Sticking to original `NOT IN (NULL)` for now.
        return left_sql.append_raw("NOT IN(NULL)");
    }

    left_sql
        .append_raw("NOT IN(")
        .append(SQL::join(&value_sqls, ",")) // Join values with comma
        .append_raw(")")
}

// Combine conditions with AND
pub fn and<'a, V>(conditions: Vec<Option<SQL<'a, V>>>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    let conditions: Vec<SQL<'a, V>> = conditions.into_iter().filter_map(|c| c).collect();

    if conditions.is_empty() {
        return SQL::raw("");
    }
    if conditions.len() == 1 {
        return conditions.into_iter().next().unwrap();
    }

    SQL::raw("(")
        .append(SQL::join(&conditions, "AND"))
        .append_raw(")")
}

// Combine conditions with OR
pub fn or<'a, V>(conditions: Vec<SQL<'a, V>>) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    if conditions.is_empty() {
        return SQL::raw("");
    }
    if conditions.len() == 1 {
        return conditions.into_iter().next().unwrap();
    }

    SQL::raw("(")
        .append(SQL::join(&conditions, "OR"))
        .append_raw(")")
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
