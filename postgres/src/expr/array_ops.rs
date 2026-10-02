//! `PostgreSQL` array operators. Documented in [`crate::expr`].

#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::values::PostgresValue;
use drizzle_core::ToSQL;
use drizzle_core::expr::{AggregateKind, Expr, ExprSources, NonNull, SQLExpr, Scalar};
use drizzle_core::scope::Arg;
use drizzle_core::sql::{SQL, SQLChunk};
use drizzle_types::postgres::types::{Any, Boolean};
use drizzle_types::{Array, Compatible, DataType, Placeholder};

/// SQL type pairs that the array operators (`@>`, `<@`, `&&`) accept.
///
/// `Self` is the SQL type of the left operand and `Rhs` the SQL type of the
/// right operand. The pair is accepted when:
///
/// - both are arrays, `Array<T>` and `Array<U>`, and `T` is [`Compatible`]
///   with `U` (so `int4[]` works with `int8[]`, but not with `text[]`);
/// - the left side is an array and the right side is a [`Placeholder`] or
///   untyped SQL ([`Any`]);
/// - the left side is untyped SQL ([`Any`]), which accepts any right side.
///
/// Anything else, such as a single value or a plain `Vec`, is rejected at
/// compile time. Wrap a Rust list in [`PgArray`] to bind it as one array.
///
/// # Type safety
///
/// ```compile_fail
/// use drizzle_core::expr::raw_non_null;
/// use drizzle_postgres::expr::array_contains;
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::{Array, postgres::types::Text};
///
/// let tags = raw_non_null::<PostgresValue, Array<Text>>("tags");
/// // A single value is not an array: wrap it as `PgArray(vec!["rust"])`.
/// let _ = array_contains(tags, "rust");
/// ```
///
/// ```compile_fail
/// use drizzle_core::expr::raw_non_null;
/// use drizzle_postgres::expr::{array_overlaps, PgArray};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::{Array, postgres::types::Int4};
///
/// let ids = raw_non_null::<PostgresValue, Array<Int4>>("ids");
/// // `int4[]` cannot be compared with `text[]`.
/// let _ = array_overlaps(ids, PgArray(vec!["1"]));
/// ```
#[diagnostic::on_unimplemented(
    message = "PostgreSQL array operators cannot combine `{Self}` with `{Rhs}`",
    label = "both operands must be arrays with compatible element types",
    note = "pass a bound array with `PgArray(vec![...])`; a bare value is not an array"
)]
pub trait ArrayOperand<Rhs> {}

impl<T: DataType, U: DataType> ArrayOperand<Array<U>> for Array<T> where T: Compatible<U> {}
impl<T: DataType> ArrayOperand<Any> for Array<T> {}
impl<T: DataType> ArrayOperand<Placeholder> for Array<T> {}
impl<R> ArrayOperand<R> for Any {}

/// Binds a `Vec<T>` as a single `PostgreSQL` array parameter.
///
/// A plain `Vec<T>` renders one parameter per element (`$1, $2, $3`), which
/// suits `IN (...)` lists. The array operators need one array value instead,
/// so wrap the list: `PgArray(vec![1, 2])` renders as a single `$1`.
///
/// Its SQL type is `Array<T's SQL type>`, so `PgArray(vec!["a"])` is an
/// `Array<Text>` and `PgArray(vec![1_i32])` an `Array<Int4>`. It is never NULL.
///
/// # Examples
///
/// ```
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::expr::{array_contains, PgArray};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::{Array, postgres::types::Text};
///
/// let tags = raw_non_null::<PostgresValue, Array<Text>>("tags");
/// let condition = array_contains(tags, PgArray(vec!["rust", "python"]));
/// let sql = condition.to_sql();
/// assert_eq!(sql.sql(), "tags @> $1");
/// assert_eq!(sql.params().count(), 1); // one array parameter
/// ```
pub struct PgArray<T>(pub Vec<T>);

impl<'a, T> ToSQL<'a, PostgresValue<'a>> for PgArray<T>
where
    T: Into<PostgresValue<'a>> + Clone,
{
    fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
        let array: Vec<PostgresValue<'a>> = self.0.iter().map(|v| v.clone().into()).collect();
        SQL::param(PostgresValue::Array(array))
    }
}

impl<T> ExprSources for PgArray<T> {
    type Sources = ();
}

impl<'a, T> Expr<'a, PostgresValue<'a>> for PgArray<T>
where
    T: Expr<'a, PostgresValue<'a>> + Into<PostgresValue<'a>> + Clone,
{
    type SQLType = Array<T::SQLType>;
    type Nullable = NonNull;
    type Aggregate = Scalar;
}

/// Tests whether the left array contains every element of the right array (`@>`).
///
/// Both operands must be arrays with compatible element types; see
/// [`ArrayOperand`]. The result is `boolean`, and NULL when either operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::expr::{array_contains, PgArray};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::{Array, postgres::types::Text};
///
/// let tags = raw_non_null::<PostgresValue, Array<Text>>("tags");
/// // Rows tagged with both "rust" and "sql" (and possibly more).
/// let condition = array_contains(tags, PgArray(vec!["rust", "sql"]));
/// assert_eq!(condition.to_sql().sql(), "tags @> $1");
/// ```
#[allow(clippy::type_complexity)]
pub fn array_contains<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    (Arg<L::Nullable, L::Sources>, Arg<R::Nullable, R::Sources>),
>
where
    L: Expr<'a, PostgresValue<'a>>,
    R: Expr<'a, PostgresValue<'a>>,
    L::SQLType: ArrayOperand<R::SQLType>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("@>".into()))
            .append(right.to_sql()),
    )
}

/// Tests whether every element of the left array is in the right array (`<@`).
///
/// Both operands must be arrays with compatible element types; see
/// [`ArrayOperand`]. The result is `boolean`, and NULL when either operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::expr::{array_contained, PgArray};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::{Array, postgres::types::Text};
///
/// let tags = raw_non_null::<PostgresValue, Array<Text>>("tags");
/// // Rows whose tags all come from the allowed list.
/// let condition = array_contained(tags, PgArray(vec!["rust", "sql", "web"]));
/// assert_eq!(condition.to_sql().sql(), "tags <@ $1");
/// ```
#[allow(clippy::type_complexity)]
pub fn array_contained<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    (Arg<L::Nullable, L::Sources>, Arg<R::Nullable, R::Sources>),
>
where
    L: Expr<'a, PostgresValue<'a>>,
    R: Expr<'a, PostgresValue<'a>>,
    L::SQLType: ArrayOperand<R::SQLType>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("<@".into()))
            .append(right.to_sql()),
    )
}

/// Tests whether two arrays share at least one element (`&&`).
///
/// Both operands must be arrays with compatible element types; see
/// [`ArrayOperand`]. The result is `boolean`, and NULL when either operand is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::expr::{array_overlaps, PgArray};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::{Array, postgres::types::Int4};
///
/// let team_ids = raw_non_null::<PostgresValue, Array<Int4>>("team_ids");
/// // Rows in team 1 or team 2.
/// let condition = array_overlaps(team_ids, PgArray(vec![1_i32, 2]));
/// assert_eq!(condition.to_sql().sql(), "team_ids && $1");
/// ```
#[allow(clippy::type_complexity)]
pub fn array_overlaps<'a, L, R>(
    left: L,
    right: R,
) -> SQLExpr<
    'a,
    PostgresValue<'a>,
    Boolean,
    NonNull,
    <L::Aggregate as AggregateKind>::Or<R::Aggregate>,
    (Arg<L::Nullable, L::Sources>, Arg<R::Nullable, R::Sources>),
>
where
    L: Expr<'a, PostgresValue<'a>>,
    R: Expr<'a, PostgresValue<'a>>,
    L::SQLType: ArrayOperand<R::SQLType>,
{
    SQLExpr::new(
        left.to_sql()
            .push(SQLChunk::Raw("&&".into()))
            .append(right.to_sql()),
    )
}

/// Method forms of the array operators, available on every `PostgreSQL` expression.
///
/// Each method calls the free function of the same name and has the same
/// operand rules ([`ArrayOperand`]).
///
/// # Examples
///
/// ```
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::expr::{ArrayExprExt, PgArray};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::{Array, postgres::types::Text};
///
/// let tags = raw_non_null::<PostgresValue, Array<Text>>("tags");
/// let condition = tags.array_overlaps(PgArray(vec!["rust"]));
/// assert_eq!(condition.to_sql().sql(), "tags && $1");
/// ```
pub trait ArrayExprExt<'a>: Expr<'a, PostgresValue<'a>> + Sized {
    /// Tests whether `self` contains every element of `other` (`@>`).
    ///
    /// See [`array_contains`].
    #[allow(clippy::type_complexity)]
    fn array_contains<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<R::Aggregate>,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        R: Expr<'a, PostgresValue<'a>>,
        Self::SQLType: ArrayOperand<R::SQLType>,
    {
        array_contains(self, other)
    }

    /// Tests whether every element of `self` is in `other` (`<@`).
    ///
    /// See [`array_contained`].
    #[allow(clippy::type_complexity)]
    fn array_contained<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<R::Aggregate>,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        R: Expr<'a, PostgresValue<'a>>,
        Self::SQLType: ArrayOperand<R::SQLType>,
    {
        array_contained(self, other)
    }

    /// Tests whether `self` and `other` share at least one element (`&&`).
    ///
    /// See [`array_overlaps`].
    #[allow(clippy::type_complexity)]
    fn array_overlaps<R>(
        self,
        other: R,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        <Self::Aggregate as AggregateKind>::Or<R::Aggregate>,
        (
            Arg<Self::Nullable, Self::Sources>,
            Arg<R::Nullable, R::Sources>,
        ),
    >
    where
        R: Expr<'a, PostgresValue<'a>>,
        Self::SQLType: ArrayOperand<R::SQLType>,
    {
        array_overlaps(self, other)
    }
}

impl<'a, E: Expr<'a, PostgresValue<'a>>> ArrayExprExt<'a> for E {}
