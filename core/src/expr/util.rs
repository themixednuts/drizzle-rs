//! Utility SQL functions (alias, cast, distinct, typeof, concat, excluded).

use crate::sql::{SQL, Token};
use crate::traits::{SQLColumnInfo, SQLParam, ToSQL};
use crate::types::{DataType, Textual};

use super::{Expr, NonNull, Null, NullOr, Nullability, SQLExpr, Scalar};

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
    expr.into_sql().alias(name)
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
    SQLExpr::new(SQL::func("TYPEOF", expr.into_sql()))
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
/// Preserves the input expression's nullability and aggregate marker.
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
pub fn cast<'a, V, E, Target>(
    expr: E,
    target_type: &'a str,
) -> SQLExpr<'a, V, Target, E::Nullable, E::Aggregate>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    Target: DataType,
{
    SQLExpr::new(SQL::func(
        "CAST",
        expr.into_sql()
            .push(Token::AS)
            .append(SQL::raw(target_type)),
    ))
}

// =============================================================================
// STRING CONCATENATION
// =============================================================================

/// Concatenate two string expressions using || operator.
///
/// Requires both operands to be `Textual` (Text or VarChar).
/// Nullability follows SQL concatenation rules: nullable input -> nullable output.
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
) -> SQLExpr<'a, V, crate::types::Text, <L::Nullable as NullOr<R::Nullable>>::Output, Scalar>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Textual,
    R::SQLType: Textual,
    L::Nullable: NullOr<R::Nullable>,
    R::Nullable: Nullability,
{
    super::concat(left, right)
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

// =============================================================================
// EXCLUDED (for ON CONFLICT DO UPDATE)
// =============================================================================

/// Wraps a column to reference its value from the proposed insert row
/// (the EXCLUDED row in ON CONFLICT DO UPDATE SET).
#[derive(Clone, Copy, Debug)]
pub struct Excluded<C> {
    column: C,
}

/// Reference a column's value from the proposed insert row (EXCLUDED).
///
/// Used in ON CONFLICT DO UPDATE SET to reference the value that would
/// have been inserted.
///
/// # Example
/// ```ignore
/// db.insert(simple)
///     .values([InsertSimple::new("test").with_id(1)])
///     .on_conflict(simple.id)
///     .do_update(UpdateSimple::default().with_name(excluded(simple.name)));
/// // Generates: ... ON CONFLICT ("id") DO UPDATE SET "name" = EXCLUDED."name"
/// ```
pub fn excluded<C>(column: C) -> Excluded<C> {
    Excluded { column }
}

impl<'a, V, C> Expr<'a, V> for Excluded<C>
where
    V: SQLParam + 'a,
    C: Expr<'a, V> + SQLColumnInfo,
{
    type SQLType = C::SQLType;
    type Nullable = C::Nullable;
    type Aggregate = C::Aggregate;
}

impl<'a, V, C> ToSQL<'a, V> for Excluded<C>
where
    V: SQLParam + 'a,
    C: SQLColumnInfo,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::empty()
            .push(Token::EXCLUDED)
            .push(Token::DOT)
            .append(SQL::ident(self.column.name().to_string()))
    }
}
