//! Utility SQL functions (alias, cast, distinct, typeof, concat, excluded).

use crate::dialect::{MySQLDialect, PostgresDialect, SQLiteDialect};
use crate::sql::{SQL, Token};
use crate::traits::{SQLColumnInfo, SQLParam, ToSQL};
use crate::types::{Compatible, DataType, Textual};

use super::{AggOr, AggregateKind, Expr, NonNull, Null, NullOr, Nullability, SQLExpr, Scalar};

// =============================================================================
// ALIAS
// =============================================================================

/// An expression aliased with `AS "name"`.
///
/// Preserves the original expression's type information (`ExprValueType`,
/// `Expr`, etc.) so that aliased columns in SELECT tuples still infer
/// the correct row type.
#[derive(Clone, Copy, Debug)]
pub struct AliasedExpr<E> {
    pub(crate) expr: E,
    pub(crate) name: &'static str,
}

impl<'a, V, E> ToSQL<'a, V> for AliasedExpr<E>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.expr.to_sql().alias(self.name)
    }

    fn into_sql(self) -> SQL<'a, V> {
        self.expr.into_sql().alias(self.name)
    }
}

impl<'a, V, E> Expr<'a, V> for AliasedExpr<E>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
{
    type SQLType = E::SQLType;
    type Nullable = E::Nullable;
    type Aggregate = E::Aggregate;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        self.expr.to_expr_sql().alias(self.name)
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        self.expr.into_expr_sql().alias(self.name)
    }
}

impl<E: super::HasAggStatus> super::HasAggStatus for AliasedExpr<E> {
    type Status = E::Status;
}

impl<E: crate::row::ExprValueType> crate::row::ExprValueType for AliasedExpr<E> {
    type ValueType = E::ValueType;
}

impl<E> crate::row::IntoSelectTarget for AliasedExpr<E>
where
    E: crate::row::ExprValueType,
{
    type Marker = crate::row::SelectCols<(Self,)>;
}

/// Extension trait providing `.alias()` method syntax on any expression.
///
/// This is a blanket impl on all `Sized` types. The `AliasedExpr` it creates
/// is only useful when the inner type implements `ToSQL`/`Expr`/`ExprValueType`,
/// so calling `.alias()` on non-SQL types is harmless but useless.
///
/// For `SQL<'a, V>` values, the inherent `SQL::alias()` method takes
/// precedence and returns `SQL<'a, V>` (no type preservation needed for raw SQL).
pub trait AliasExt: Sized {
    fn alias(self, name: &'static str) -> AliasedExpr<Self> {
        AliasedExpr { expr: self, name }
    }
}

impl<T: Sized> AliasExt for T {}

/// Create an aliased expression.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::alias;
///
/// // SELECT users.first_name || users.last_name AS full_name
/// let full_name = alias(string_concat(users.first_name, users.last_name), "full_name");
/// # "####;
/// ```
pub const fn alias<E>(expr: E, name: &'static str) -> AliasedExpr<E> {
    AliasedExpr { expr, name }
}

// =============================================================================
// TYPEOF
// =============================================================================

#[diagnostic::on_unimplemented(
    message = "TYPEOF is not available for this dialect",
    label = "use a dialect-specific type inspection expression"
)]
pub trait TypeofSupport {}

impl TypeofSupport for SQLiteDialect {}

/// Get the SQL type of an expression.
///
/// Returns the data type name as text.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::typeof_;
///
/// // SELECT TYPEOF(users.age) -- returns "integer"
/// let age_type = typeof_(users.age);
/// # "####;
/// ```
pub fn typeof_<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as crate::dialect::DialectTypes>::Text, NonNull, E::Aggregate>
where
    V: SQLParam + 'a,
    V::DialectMarker: TypeofSupport,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("TYPEOF", expr.into_expr_sql()))
}

/// Alias for typeof_ (uses Rust raw identifier syntax).
pub fn r#typeof<'a, V, E>(
    expr: E,
) -> SQLExpr<'a, V, <V::DialectMarker as crate::dialect::DialectTypes>::Text, NonNull, E::Aggregate>
where
    V: SQLParam + 'a,
    V::DialectMarker: TypeofSupport,
    E: Expr<'a, V>,
{
    typeof_(expr)
}

// =============================================================================
// CAST
// =============================================================================

/// Default SQL cast type name for a type marker.
pub trait DefaultCastTypeName: DataType {
    const CAST_TYPE_NAME: &'static str;
}

impl DefaultCastTypeName for drizzle_types::sqlite::types::Integer {
    const CAST_TYPE_NAME: &'static str = "INTEGER";
}
impl DefaultCastTypeName for drizzle_types::sqlite::types::Text {
    const CAST_TYPE_NAME: &'static str = "TEXT";
}
impl DefaultCastTypeName for drizzle_types::sqlite::types::Real {
    const CAST_TYPE_NAME: &'static str = "REAL";
}
impl DefaultCastTypeName for drizzle_types::sqlite::types::Blob {
    const CAST_TYPE_NAME: &'static str = "BLOB";
}
impl DefaultCastTypeName for drizzle_types::sqlite::types::Numeric {
    const CAST_TYPE_NAME: &'static str = "NUMERIC";
}
impl DefaultCastTypeName for drizzle_types::sqlite::types::Any {
    const CAST_TYPE_NAME: &'static str = "ANY";
}

impl DefaultCastTypeName for drizzle_types::postgres::types::Int2 {
    const CAST_TYPE_NAME: &'static str = "SMALLINT";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Int4 {
    const CAST_TYPE_NAME: &'static str = "INTEGER";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Int8 {
    const CAST_TYPE_NAME: &'static str = "BIGINT";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Float4 {
    const CAST_TYPE_NAME: &'static str = "REAL";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Float8 {
    const CAST_TYPE_NAME: &'static str = "DOUBLE PRECISION";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Varchar {
    const CAST_TYPE_NAME: &'static str = "VARCHAR";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Text {
    const CAST_TYPE_NAME: &'static str = "TEXT";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Char {
    const CAST_TYPE_NAME: &'static str = "CHAR";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Bytea {
    const CAST_TYPE_NAME: &'static str = "BYTEA";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Boolean {
    const CAST_TYPE_NAME: &'static str = "BOOLEAN";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Timestamptz {
    const CAST_TYPE_NAME: &'static str = "TIMESTAMPTZ";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Timestamp {
    const CAST_TYPE_NAME: &'static str = "TIMESTAMP";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Date {
    const CAST_TYPE_NAME: &'static str = "DATE";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Time {
    const CAST_TYPE_NAME: &'static str = "TIME";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Timetz {
    const CAST_TYPE_NAME: &'static str = "TIMETZ";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Numeric {
    const CAST_TYPE_NAME: &'static str = "NUMERIC";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Uuid {
    const CAST_TYPE_NAME: &'static str = "UUID";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Json {
    const CAST_TYPE_NAME: &'static str = "JSON";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Jsonb {
    const CAST_TYPE_NAME: &'static str = "JSONB";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Any {
    const CAST_TYPE_NAME: &'static str = "ANY";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Interval {
    const CAST_TYPE_NAME: &'static str = "INTERVAL";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Inet {
    const CAST_TYPE_NAME: &'static str = "INET";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Cidr {
    const CAST_TYPE_NAME: &'static str = "CIDR";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::MacAddr {
    const CAST_TYPE_NAME: &'static str = "MACADDR";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::MacAddr8 {
    const CAST_TYPE_NAME: &'static str = "MACADDR8";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Point {
    const CAST_TYPE_NAME: &'static str = "POINT";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::LineString {
    const CAST_TYPE_NAME: &'static str = "PATH";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Rect {
    const CAST_TYPE_NAME: &'static str = "BOX";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::BitString {
    const CAST_TYPE_NAME: &'static str = "BIT VARYING";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Line {
    const CAST_TYPE_NAME: &'static str = "LINE";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::LineSegment {
    const CAST_TYPE_NAME: &'static str = "LSEG";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Polygon {
    const CAST_TYPE_NAME: &'static str = "POLYGON";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Circle {
    const CAST_TYPE_NAME: &'static str = "CIRCLE";
}
impl DefaultCastTypeName for drizzle_types::postgres::types::Enum {
    const CAST_TYPE_NAME: &'static str = "TEXT";
}

impl DefaultCastTypeName for drizzle_types::mysql::types::BigInt {
    const CAST_TYPE_NAME: &'static str = "SIGNED";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::BigIntUnsigned {
    const CAST_TYPE_NAME: &'static str = "UNSIGNED";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Float {
    const CAST_TYPE_NAME: &'static str = "FLOAT";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Double {
    const CAST_TYPE_NAME: &'static str = "DOUBLE";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Decimal {
    const CAST_TYPE_NAME: &'static str = "DECIMAL";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Varchar {
    const CAST_TYPE_NAME: &'static str = "CHAR";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Varbinary {
    const CAST_TYPE_NAME: &'static str = "BINARY";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Json {
    const CAST_TYPE_NAME: &'static str = "JSON";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Date {
    const CAST_TYPE_NAME: &'static str = "DATE";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Time {
    const CAST_TYPE_NAME: &'static str = "TIME";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::DateTime {
    const CAST_TYPE_NAME: &'static str = "DATETIME";
}
impl DefaultCastTypeName for drizzle_types::mysql::types::Year {
    const CAST_TYPE_NAME: &'static str = "YEAR";
}

/// Input accepted by [`cast`].
///
/// You can pass:
/// - a SQL type string (dialect-specific), or
/// - a type marker value (uses that marker's default SQL cast name).
pub trait CastTarget<'a, T: DataType, D> {
    fn cast_type_name(self) -> &'a str;
}

/// Additional cast safety policy by dialect.
#[diagnostic::on_unimplemented(
    message = "cannot cast `{Source}` to `{Target}` for this dialect",
    label = "cast target is incompatible with source type",
    note = "use a supported target marker, or raw SQL when the conversion is intentionally dialect-specific"
)]
pub trait CastTypePolicy<D, Source: DataType, Target: DataType> {}

/// Dialect policy for casts that may produce `NULL` from a non-NULL input.
#[doc(hidden)]
pub trait CastNullabilityPolicy<D, Input: Nullability>: DataType {
    type Output: Nullability;
}

macro_rules! mysql_cast_policy {
    (
        preserving: [$($preserving:ty),+ $(,)?],
        nullable: [$($nullable:ty),+ $(,)?],
    ) => {
        $(
            impl<Source: DataType> CastTypePolicy<MySQLDialect, Source, $preserving> for () {}

            impl<Input: Nullability> CastNullabilityPolicy<MySQLDialect, Input> for $preserving {
                type Output = Input;
            }
        )+
        $(
            impl<Source: DataType> CastTypePolicy<MySQLDialect, Source, $nullable> for () {}

            impl<Input: Nullability> CastNullabilityPolicy<MySQLDialect, Input> for $nullable {
                type Output = Null;
            }
        )+
    };
}

mysql_cast_policy! {
    preserving: [
        drizzle_types::mysql::types::BigInt,
        drizzle_types::mysql::types::BigIntUnsigned,
        drizzle_types::mysql::types::Float,
        drizzle_types::mysql::types::Double,
        drizzle_types::mysql::types::Decimal,
        drizzle_types::mysql::types::Varchar,
        drizzle_types::mysql::types::Varbinary,
        drizzle_types::mysql::types::Json,
    ],
    nullable: [
        drizzle_types::mysql::types::Date,
        drizzle_types::mysql::types::Time,
        drizzle_types::mysql::types::DateTime,
        drizzle_types::mysql::types::Year,
    ],
}

impl<Source: DataType + Compatible<Target>, Target: DataType>
    CastTypePolicy<PostgresDialect, Source, Target> for ()
{
}

impl<Input: Nullability, Target: DataType> CastNullabilityPolicy<PostgresDialect, Input>
    for Target
{
    type Output = Input;
}

impl<Source: DataType + Compatible<Target>, Target: DataType>
    CastTypePolicy<SQLiteDialect, Source, Target> for ()
{
}

impl<Input: Nullability, Target: DataType> CastNullabilityPolicy<SQLiteDialect, Input> for Target {
    type Output = Input;
}

impl<'a, T: DataType, D> CastTarget<'a, T, D> for &'a str {
    fn cast_type_name(self) -> &'a str {
        self
    }
}

impl<'a, T, D> CastTarget<'a, T, D> for T
where
    T: DataType + DefaultCastTypeName,
{
    fn cast_type_name(self) -> &'a str {
        T::CAST_TYPE_NAME
    }
}

/// Cast an expression to a different type.
///
/// The target type marker specifies the result type for the type system.
/// The cast target may be:
/// - a SQL type string (`"INTEGER"`, `"int4"`, `"VARCHAR(255)"`), or
/// - a type marker value (`Int`, `Text`, `drizzle::sqlite::types::Integer`, ...).
///
/// Preserves the aggregate marker. Nullability follows the dialect and target
/// type because MySQL temporal casts can return `NULL` for invalid non-NULL
/// input.
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::cast;
/// use drizzle_core::types::{Int, Text};
///
/// // SELECT CAST(users.age AS TEXT)
/// let age_text = cast::<_, _, Text>(users.age, Text);
///
/// // Explicit SQL type name (dialect-specific)
/// let age_text = cast::<_, _, Text>(users.age, "VARCHAR(255)");
/// let age_int = cast::<_, _, Int>(users.age, "int4");
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn cast<'a, V, E, Target>(
    expr: E,
    target_type: impl CastTarget<'a, Target, V::DialectMarker>,
) -> SQLExpr<
    'a,
    V,
    Target,
    <Target as CastNullabilityPolicy<V::DialectMarker, E::Nullable>>::Output,
    E::Aggregate,
>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    Target: DataType + CastNullabilityPolicy<V::DialectMarker, E::Nullable>,
    (): CastTypePolicy<V::DialectMarker, E::SQLType, Target>,
{
    SQLExpr::new(SQL::func(
        "CAST",
        expr.into_expr_sql()
            .push(Token::AS)
            .append(SQL::raw(target_type.cast_type_name())),
    ))
}

// =============================================================================
// STRING CONCATENATION
// =============================================================================

/// Concatenate two string expressions using dialect-appropriate SQL.
///
/// Requires both operands to be `Textual` (Text or `VarChar`).
/// Nullability follows SQL concatenation rules: nullable input -> nullable output.
/// SQLite and PostgreSQL render `left || right`; MySQL renders
/// `CONCAT(left, right)` because `||` is logical OR under its default SQL mode.
///
/// # Type Safety
///
/// ```rust
/// # let _ = r####"
/// // ✅ OK: Both are Text
/// string_concat(users.first_name, users.last_name);
///
/// // ✅ OK: Text with string literal
/// string_concat(users.first_name, " ");
///
/// // ❌ Compile error: Int is not Textual
/// string_concat(users.id, users.name);
/// # "####;
/// ```
///
/// # Example
///
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::string_concat;
///
/// // SQLite/PostgreSQL: first_name || ' ' || last_name
/// // MySQL: CONCAT(CONCAT(first_name, ' '), last_name)
/// let full_name = string_concat(string_concat(users.first_name, " "), users.last_name);
/// # "####;
/// ```
#[allow(clippy::type_complexity)]
pub fn string_concat<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as crate::dialect::DialectTypes>::Text,
    <L::Nullable as NullOr<R::Nullable>>::Output,
    <L::Aggregate as AggOr<R::Aggregate>>::Output,
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Textual,
    R::SQLType: Textual,
    L::Nullable: NullOr<R::Nullable>,
    R::Nullable: Nullability,
    L::Aggregate: AggOr<R::Aggregate>,
    R::Aggregate: AggregateKind,
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
/// ```rust
/// # let _ = r####"
/// use drizzle_core::expr::raw;
/// use drizzle_core::types::Int;
///
/// let expr = raw::<_, Int>("RANDOM()");
/// # "####;
/// ```
#[must_use]
pub fn raw<'a, V, T>(sql: &'a str) -> SQLExpr<'a, V, T, Null, Scalar>
where
    V: SQLParam + 'a,
    T: DataType,
{
    SQLExpr::new(SQL::raw(sql))
}

/// Create a raw SQL expression with explicit nullable nullability.
#[must_use]
pub fn raw_nullable<'a, V, T>(sql: &'a str) -> SQLExpr<'a, V, T, Null, Scalar>
where
    V: SQLParam + 'a,
    T: DataType,
{
    SQLExpr::new(SQL::raw(sql))
}

/// Create a raw SQL expression with explicit non-null nullability.
#[must_use]
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

/// Dialects whose upsert syntax exposes the proposed row as `EXCLUDED`.
pub trait ExcludedSupport {}

impl ExcludedSupport for SQLiteDialect {}
impl ExcludedSupport for PostgresDialect {}

/// Reference a column's value from the proposed insert row (EXCLUDED).
///
/// Used in ON CONFLICT DO UPDATE SET to reference the value that would
/// have been inserted.
///
/// # Example
/// ```rust
/// # let _ = r####"
/// db.insert(simple)
///     .values([InsertSimple::new("test").with_id(1)])
///     .on_conflict(simple.id)
///     .do_update(UpdateSimple::default().with_name(excluded(simple.name)));
/// // Generates: ... ON CONFLICT ("id") DO UPDATE SET "name" = EXCLUDED."name"
/// # "####;
/// ```
pub const fn excluded<C>(column: C) -> Excluded<C> {
    Excluded { column }
}

impl<'a, V, C> Expr<'a, V> for Excluded<C>
where
    V: SQLParam + 'a,
    V::DialectMarker: ExcludedSupport,
    C: Expr<'a, V> + SQLColumnInfo,
{
    type SQLType = C::SQLType;
    type Nullable = C::Nullable;
    type Aggregate = C::Aggregate;
}

impl<'a, V, C> ToSQL<'a, V> for Excluded<C>
where
    V: SQLParam + 'a,
    V::DialectMarker: ExcludedSupport,
    C: SQLColumnInfo,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::empty()
            .push(Token::EXCLUDED)
            .push(Token::DOT)
            .append(SQL::ident(self.column.name()))
    }
}
