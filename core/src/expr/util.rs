//! Utility SQL functions (alias, cast, distinct, typeof, concat).

use crate::sql::{SQL, Token};
use crate::traits::{SQLParam, ToSQL};
use crate::types::{DataType, Textual};

use super::{Expr, NonNull, Null, SQLExpr, Scalar};

// =============================================================================
// ALIAS
// =============================================================================

/// Create an aliased expression.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::alias;
///
/// // SELECT users.first_name || users.last_name AS full_name
/// let full_name = alias(string_concat(users.first_name, users.last_name), "full_name");
/// ```
pub fn alias<'a, V, E>(expr: E, name: &'a str) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
{
    expr.to_sql().alias(name)
}

// =============================================================================
// TYPEOF
// =============================================================================

/// Get the SQL type of an expression.
///
/// Returns the data type name as text.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::typeof_;
///
/// // SELECT TYPEOF(users.age) -- returns "integer"
/// let age_type = typeof_(users.age);
/// ```
pub fn typeof_<'a, V, E>(expr: E) -> SQLExpr<'a, V, crate::types::Text, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
{
    SQLExpr::new(SQL::func("TYPEOF", expr.to_sql()))
}

/// Alias for typeof_ (uses Rust raw identifier syntax).
pub fn r#typeof<'a, V, E>(expr: E) -> SQLExpr<'a, V, crate::types::Text, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
{
    typeof_(expr)
}

// =============================================================================
// CAST
// =============================================================================

/// Cast an expression to a different type.
///
/// The target type marker specifies the result type for the type system,
/// while the SQL type string specifies the actual SQL type name (dialect-specific).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::cast;
/// use drizzle_core::types::Text;
///
/// // SELECT CAST(users.age AS TEXT)
/// let age_text = cast::<_, _, _, Text>(users.age, "TEXT");
///
/// // PostgreSQL-specific
/// let age_text = cast::<_, _, _, Text>(users.age, "VARCHAR(255)");
/// ```
pub fn cast<'a, V, E, Target>(expr: E, target_type: &'a str) -> SQLExpr<'a, V, Target, Null, Scalar>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
    Target: DataType,
{
    SQLExpr::new(SQL::func(
        "CAST",
        expr.to_sql().push(Token::AS).append(SQL::raw(target_type)),
    ))
}

/// Cast an expression to a different type, preserving non-null status.
///
/// Use this when you know the input expression is non-null and the cast
/// will always succeed (e.g., widening conversions).
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::cast_non_null;
/// use drizzle_core::types::BigInt;
///
/// // Cast a non-null integer to bigint
/// let big_id = cast_non_null::<_, _, _, BigInt>(users.id, "BIGINT");
/// ```
pub fn cast_non_null<'a, V, E, Target>(
    expr: E,
    target_type: &'a str,
) -> SQLExpr<'a, V, Target, NonNull, Scalar>
where
    V: SQLParam + 'a,
    E: Expr<'a, V, Nullable = NonNull>,
    Target: DataType,
{
    SQLExpr::new(SQL::func(
        "CAST",
        expr.to_sql().push(Token::AS).append(SQL::raw(target_type)),
    ))
}

// =============================================================================
// STRING CONCATENATION
// =============================================================================

/// Concatenate two string expressions using || operator.
///
/// Requires both operands to be `Textual` (Text or VarChar).
///
/// # Type Safety
///
/// ```ignore
/// // ✅ OK: Both are Text
/// string_concat(users.first_name, users.last_name);
///
/// // ✅ OK: Text with string literal
/// string_concat(users.first_name, " ");
///
/// // ❌ Compile error: Int is not Textual
/// string_concat(users.id, users.name);
/// ```
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::string_concat;
///
/// // SELECT users.first_name || ' ' || users.last_name
/// let full_name = string_concat(string_concat(users.first_name, " "), users.last_name);
/// ```
pub fn string_concat<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<'a, V, crate::types::Text, NonNull, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Textual,
    R::SQLType: Textual,
{
    SQLExpr::new(left.to_sql().push(Token::CONCAT).append(right.to_sql()))
}

// =============================================================================
// RAW SQL Expression
// =============================================================================

/// Create a raw SQL expression with a specified type.
///
/// Use this for dialect-specific features or when the type system
/// can't infer the correct type.
///
/// # Safety
///
/// This bypasses type checking. Use sparingly and only when necessary.
///
/// # Example
///
/// ```ignore
/// use drizzle_core::expr::raw;
/// use drizzle_core::types::Int;
///
/// let expr = raw::<_, Int>("RANDOM()");
/// ```
pub fn raw<'a, V, T>(sql: &'a str) -> SQLExpr<'a, V, T, Null, Scalar>
where
    V: SQLParam + 'a,
    T: DataType,
{
    SQLExpr::new(SQL::raw(sql))
}

/// Create a raw SQL expression with explicit nullability.
pub fn raw_non_null<'a, V, T>(sql: &'a str) -> SQLExpr<'a, V, T, NonNull, Scalar>
where
    V: SQLParam + 'a,
    T: DataType,
{
    SQLExpr::new(SQL::raw(sql))
}
