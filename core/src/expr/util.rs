//! Helpers: aliases (`AS`), `CAST`, `TYPEOF`, raw typed SQL and `EXCLUDED`.

use crate::dialect::{DialectSupports, feature};
use crate::dialect::{MySQLDialect, PostgresDialect, SQLiteDialect};
use crate::sql::{SQL, Token};
use crate::traits::{SQLColumnInfo, SQLParam, ToSQL};
use crate::types::{Compatible, DataType, Textual};

use super::{AggregateKind, Expr, NonNull, Null, Nullability, SQLExpr, Scalar};
use crate::scope::ScopeOnly;

// =============================================================================
// ALIAS
// =============================================================================

/// An expression renamed with `AS "name"`; created by [`alias`] or
/// [`AliasExt::alias`].
///
/// Keeps the inner expression's SQL type, nullability and decoded Rust type,
/// so an aliased column in a SELECT list still decodes to the right type.
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

impl<E: super::ExprSources> super::ExprSources for AliasedExpr<E> {
    type Sources = E::Sources;
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

/// Method syntax for naming a selected expression.
///
/// - `.alias("name")` sets a name at run time ([`AliasedExpr`]).
/// - `.named::<Tag>()` sets a name in the type ([`NamedExpr`]), so a derived
///   table built from the query can refer to the column.
///
/// Implemented for every expression with a known Rust type. On raw [`SQL`],
/// the inherent [`SQL::alias`] method is called instead and returns `SQL`.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, ToSQL, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let n = count(users.id).alias("user_count");
/// assert_eq!(n.to_sql().sql(), r#"COUNT ("users"."id") AS "user_count""#);
/// ```
pub trait AliasExt: Sized {
    /// Renames this expression: `expr AS "name"`.
    fn alias(self, name: &'static str) -> AliasedExpr<Self> {
        AliasedExpr { expr: self, name }
    }

    /// Names this expression with a type-level [`Tag`](crate::Tag), rendered
    /// as `expr AS "<Tag::NAME>"`.
    fn named<Name: crate::Tag>(self) -> NamedExpr<Self, Name> {
        NamedExpr {
            expr: self,
            name: core::marker::PhantomData,
        }
    }
}

impl<T: crate::row::ExprValueType> AliasExt for T {}

/// Renames an expression: `expr AS "name"`.
///
/// The result keeps the expression's SQL type, nullability and decoded Rust
/// type. Same as [`AliasExt::alias`].
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, ToSQL, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let label = alias(upper(users.name), "label");
/// assert_eq!(label.to_sql().sql(), r#"UPPER ("users"."name") AS "label""#);
/// ```
pub const fn alias<E>(expr: E, name: &'static str) -> AliasedExpr<E> {
    AliasedExpr { expr, name }
}

/// An expression whose output name is a type-level [`Tag`](crate::Tag);
/// created by [`AliasExt::named`].
///
/// Unlike [`AliasedExpr`], the name is part of the type, so a derived table
/// built from the query can select this column by name.
#[derive(Clone, Copy, Debug)]
pub struct NamedExpr<E, Name> {
    pub(crate) expr: E,
    pub(crate) name: core::marker::PhantomData<Name>,
}

impl<E, Name> NamedExpr<E, Name> {
    /// Returns the wrapped expression.
    pub const fn expression(&self) -> &E {
        &self.expr
    }

    /// Returns the wrapped expression by value.
    pub fn into_expression(self) -> E {
        self.expr
    }
}

impl<'a, V, E, Name> ToSQL<'a, V> for NamedExpr<E, Name>
where
    V: SQLParam + 'a,
    E: ToSQL<'a, V>,
    Name: crate::Tag,
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.expr.to_sql().alias(Name::NAME)
    }

    fn into_sql(self) -> SQL<'a, V> {
        self.expr.into_sql().alias(Name::NAME)
    }
}

impl<E: super::ExprSources, Name> super::ExprSources for NamedExpr<E, Name> {
    type Sources = E::Sources;
}

impl<'a, V, E, Name> Expr<'a, V> for NamedExpr<E, Name>
where
    V: SQLParam + 'a,
    E: Expr<'a, V>,
    Name: crate::Tag,
{
    type SQLType = E::SQLType;
    type Nullable = E::Nullable;
    type Aggregate = E::Aggregate;

    fn to_expr_sql(&self) -> SQL<'a, V> {
        self.expr.to_expr_sql().alias(Name::NAME)
    }

    fn into_expr_sql(self) -> SQL<'a, V> {
        self.expr.into_expr_sql().alias(Name::NAME)
    }
}

impl<E, Name> super::HasAggStatus for NamedExpr<E, Name>
where
    E: super::HasAggStatus,
{
    type Status = E::Status;
}

impl<E, Name> crate::row::ExprValueType for NamedExpr<E, Name>
where
    E: crate::row::ExprValueType,
{
    type ValueType = E::ValueType;
}

impl<E, Name> crate::row::IntoSelectTarget for NamedExpr<E, Name>
where
    E: crate::row::ExprValueType,
{
    type Marker = crate::row::SelectCols<(Self,)>;
}

impl<E, Name> crate::row::GroupByIdentity for NamedExpr<E, Name>
where
    E: crate::row::GroupByIdentity,
{
    type Identity = E::Identity;
}

// =============================================================================
// TYPEOF
// =============================================================================

impl DialectSupports<feature::Typeof> for SQLiteDialect {}

/// The storage class of a value as text (`TYPEOF`), on SQLite.
///
/// Returns `'null'`, `'integer'`, `'real'`, `'text'` or `'blob'`. Accepts any
/// expression. The result is text and never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// assert_eq!(typeof_(users.age).sql(), r#"TYPEOF ("users"."age")"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn typeof_<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as crate::dialect::DialectTypes>::Text,
    NonNull,
    E::Aggregate,
    ScopeOnly<E::Sources>,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Typeof>,
    E: Expr<'a, V>,
{
    SQLExpr::new(SQL::func("TYPEOF", expr.into_expr_sql()))
}

/// Same as [`typeof_`], spelled with a raw identifier.
#[allow(clippy::type_complexity)]
pub fn r#typeof<'a, V, E>(
    expr: E,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as crate::dialect::DialectTypes>::Text,
    NonNull,
    E::Aggregate,
    ScopeOnly<E::Sources>,
>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Typeof>,
    E: Expr<'a, V>,
{
    typeof_(expr)
}

// =============================================================================
// CAST
// =============================================================================

/// The SQL type name [`cast`] writes when given a type marker, such as
/// `"INTEGER"` for SQLite's `Integer`.
pub trait DefaultCastTypeName: DataType {
    /// The type name used in `CAST(expr AS <name>)`.
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

/// The target argument of [`cast`]: a SQL type name such as `"VARCHAR(255)"`,
/// or a type marker value whose [`DefaultCastTypeName`] is used.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a cast target for `{T}` here",
    label = "a type marker target must be the cast's result type, from this dialect",
    note = "or pass the SQL type name as a string, such as `\"VARCHAR(255)\"`"
)]
pub trait CastTarget<'a, T: DataType, D> {
    /// The SQL type name to cast to.
    fn cast_type_name(self) -> &'a str;
}

/// Casts from `Source` to `Target` that dialect `D` allows.
///
/// On SQLite and PostgreSQL the two types must be compatible. On MySQL,
/// `CAST` accepts a fixed set of targets (`SIGNED`, `DOUBLE`, `CHAR`,
/// `DATE`, ...).
#[diagnostic::on_unimplemented(
    message = "cannot cast `{Source}` to `{Target}` for this dialect",
    label = "cast target is incompatible with source type",
    note = "use a supported target marker, or raw SQL when the conversion is intentionally dialect-specific"
)]
pub trait CastTypePolicy<D, Source: DataType, Target: DataType> {}

/// Nullability of a cast to `Self` on dialect `D`. MySQL temporal casts give
/// NULL for invalid input, so they are nullable.
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

/// Converts an expression to another SQL type (`CAST(expr AS type)`).
///
/// The target is either a type marker value, such as
/// `drizzle::sqlite::types::Text`, whose SQL name is used, or a SQL type
/// name such as `"VARCHAR(255)"`. With a name, give the result type
/// explicitly: `cast::<_, _, Text>(expr, "VARCHAR(255)")`.
///
/// On SQLite and PostgreSQL, source and target types must be compatible (see
/// [`CastTypePolicy`]). The result has the target type and keeps the
/// expression's aggregate kind. It keeps the expression's nullability,
/// except for MySQL casts to temporal types, which can give NULL for invalid
/// input and so are nullable.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// // Integer to SQLite REAL, using the type marker.
/// let real = cast(users.age, Real::default());
/// assert_eq!(real.sql(), r#"CAST ("users"."age" AS REAL)"#);
///
/// // The same with an explicit type name.
/// let real = cast::<_, _, Real>(users.age, "DOUBLE");
/// assert_eq!(real.sql(), r#"CAST ("users"."age" AS DOUBLE)"#);
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
    E::Sources,
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

/// Joins two text values; same as [`concat`](super::concat).
///
/// Renders `left || right` on SQLite and PostgreSQL and `CONCAT(left, right)`
/// on MySQL. Both arguments must be text. The result is text, nullable if
/// either argument is.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let label = string_concat(users.name, "!");
/// assert_eq!(label.sql(), r#""users"."name" || ?"#);
/// ```
#[allow(clippy::type_complexity)]
pub fn string_concat<'a, V, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    V,
    <V::DialectMarker as crate::dialect::DialectTypes>::Text,
    <L::Nullable as Nullability>::Or<R::Nullable>,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    (L::Sources, R::Sources),
>
where
    V: SQLParam + 'a,
    L: Expr<'a, V>,
    R: Expr<'a, V>,
    L::SQLType: Textual,
    R::SQLType: Textual,
    R::Nullable: Nullability,
    R::Aggregate: AggregateKind,
{
    super::concat(left, right)
}

// =============================================================================
// RAW SQL Expression
// =============================================================================

/// A raw SQL fragment with a declared SQL type, typed as nullable.
///
/// The text is inserted into the query as is, so never build it from user
/// input. The type system trusts the declared type `T` and cannot check the
/// SQL. The result is nullable; use [`raw_non_null`] when the SQL can never be
/// NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let one = raw::<Value, Int>("1");
/// assert_eq!(eq(users.id, one).sql(), r#""users"."id" = 1"#);
/// ```
#[must_use]
pub fn raw<'a, V, T>(sql: &'a str) -> SQLExpr<'a, V, T, Null, Scalar, ()>
where
    V: SQLParam + 'a,
    T: DataType,
{
    SQLExpr::new(SQL::raw(sql))
}

/// A raw SQL fragment with a declared SQL type, typed as nullable; same as
/// [`raw`].
#[must_use]
pub fn raw_nullable<'a, V, T>(sql: &'a str) -> SQLExpr<'a, V, T, Null, Scalar, ()>
where
    V: SQLParam + 'a,
    T: DataType,
{
    SQLExpr::new(SQL::raw(sql))
}

/// A raw SQL fragment with a declared SQL type, typed as non-null.
///
/// Like [`raw`], but the result is non-null. The type system trusts both the
/// declared type `T` and the claim that the SQL is never NULL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, DialectTypes, SQLiteDialect as D};
/// # use drizzle_core::{ColumnRef, SQL, SQLParam, expr::*};
/// # #[derive(Clone, Debug)] struct Value(String);
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = D; }
/// # impl<X: ToString> From<X> for Value { fn from(v: X) -> Self { Value(v.to_string()) } }
/// # impl From<Value> for std::borrow::Cow<'_, Value> { fn from(v: Value) -> Self { Self::Owned(v) } }
/// # type C<X, N = NonNull> = &'static SQLExpr<'static, Value, X, N>;
/// # fn col<X: drizzle_core::types::DataType, N: Nullability>(c: &'static str) -> C<X, N> { Box::leak(Box::new(SQLExpr::new(SQL::column(ColumnRef::sql("users", c))))) }
/// # type Int = <D as DialectTypes>::Int; type Text = <D as DialectTypes>::Text; type Real = <D as DialectTypes>::Double;
/// # struct Users { id: C<Int>, age: C<Int>, name: C<Text>, email: C<Text, Null>, score: C<Real, Null>, active: C<<D as DialectTypes>::Bool>, created_at: C<<D as DialectTypes>::Timestamp> }
/// # let users = Users { id: col("id"), age: col("age"), name: col("name"), email: col("email"), score: col("score"), active: col("active"), created_at: col("created_at") };
/// let now = raw_non_null::<Value, Text>("CURRENT_TIMESTAMP");
/// assert_eq!(now.sql(), "CURRENT_TIMESTAMP");
/// ```
#[must_use]
pub fn raw_non_null<'a, V, T>(sql: &'a str) -> SQLExpr<'a, V, T, NonNull, Scalar, ()>
where
    V: SQLParam + 'a,
    T: DataType,
{
    SQLExpr::new(SQL::raw(sql))
}

// =============================================================================
// EXCLUDED (for ON CONFLICT DO UPDATE)
// =============================================================================

/// A column of the row that failed to insert (`EXCLUDED."column"`); created
/// by [`excluded`].
#[derive(Clone, Copy, Debug)]
pub struct Excluded<C> {
    column: C,
}

impl DialectSupports<feature::Excluded> for SQLiteDialect {}
impl DialectSupports<feature::Excluded> for PostgresDialect {}

/// Refers to the value a conflicting insert tried to write (`EXCLUDED."column"`).
///
/// Use it in `ON CONFLICT ... DO UPDATE SET` to copy the new value into the
/// existing row. The argument must be a table column. The result has the
/// column's SQL type and nullability. Available on SQLite and PostgreSQL.
///
/// # Examples
///
/// ```rust
/// # use drizzle_core::dialect::{Dialect, SQLiteDialect};
/// # use drizzle_core::{SQL, SQLColumnInfo, SQLParam, SQLTableInfo, ToSQL, expr::excluded};
/// # #[derive(Clone, Debug)] struct Value;
/// # impl SQLParam for Value { const DIALECT: Dialect = Dialect::SQLite; type DialectMarker = SQLiteDialect; }
/// # struct NameColumn;
/// # impl SQLColumnInfo for NameColumn {
/// #     fn name(&self) -> &'static str { "name" }
/// #     fn is_not_null(&self) -> bool { true }
/// #     fn is_primary_key(&self) -> bool { false }
/// #     fn is_unique(&self) -> bool { false }
/// #     fn r#type(&self) -> &'static str { "TEXT" }
/// #     fn has_default(&self) -> bool { false }
/// #     fn table(&self) -> &'static dyn SQLTableInfo { unimplemented!() }
/// # }
/// // `NameColumn` stands in for a generated column such as `users.name`.
/// let new_name: SQL<'_, Value> = excluded(NameColumn).to_sql();
/// assert_eq!(new_name.sql(), r#"EXCLUDED."name""#);
/// ```
///
/// In an upsert built with a dialect crate:
///
/// ```text
/// // INSERT INTO "simple" ("id", "name") VALUES (?, ?)
/// //   ON CONFLICT ("id") DO UPDATE SET "name" = EXCLUDED."name"
/// db.insert(simple)
///     .values([InsertSimple::new("test").with_id(1)])
///     .on_conflict(simple.id)
///     .do_update(UpdateSimple::default().with_name(excluded(simple.name)));
/// ```
pub const fn excluded<C>(column: C) -> Excluded<C> {
    Excluded { column }
}

/// `EXCLUDED` is the proposed insert row, not a FROM source.
impl<C> super::ExprSources for Excluded<C> {
    type Sources = ();
}

impl<'a, V, C> Expr<'a, V> for Excluded<C>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Excluded>,
    C: Expr<'a, V> + SQLColumnInfo,
{
    type SQLType = C::SQLType;
    type Nullable = C::Nullable;
    type Aggregate = C::Aggregate;
}

impl<'a, V, C> ToSQL<'a, V> for Excluded<C>
where
    V: SQLParam + 'a,
    V::DialectMarker: DialectSupports<feature::Excluded>,
    C: SQLColumnInfo,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::empty()
            .push(Token::EXCLUDED)
            .push(Token::DOT)
            .append(SQL::ident(self.column.name()))
    }
}
