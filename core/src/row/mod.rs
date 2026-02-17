//! Type-safe row inference for query builders.
//!
//! Provides type-level machinery to infer the Rust return type from a query's
//! selected columns, table, and joins — so `.all()` and `.get()` return the
//! correct type without turbofish annotations.
//!
//! # Architecture
//!
//! ```text
//! .select(cols)    → Marker  (SelectStar | SelectCols<C> | SelectExpr)
//! .from(table)     → R       (Marker + Table → row type via ResolveRow)
//! .join(t2)        → R'      (Marker + R + JoinedTable → new R via AfterJoin)
//! .all()           → Vec<R>  (R: FromDrizzleRow)
//! .all_as::<T>()   → Vec<T>  (user override)
//! ```

// Driver-specific leaf FromDrizzleRow implementations
#[cfg(feature = "libsql")]
mod libsql;
#[cfg(any(feature = "tokio-postgres", feature = "postgres-sync"))]
mod postgres;
#[cfg(feature = "rusqlite")]
mod rusqlite;
#[cfg(feature = "turso")]
mod turso;

use core::marker::PhantomData;

use crate::error::DrizzleError;

// =============================================================================
// Select Target Markers
// =============================================================================

/// Marker: `SELECT *` — R inferred from the table, grows with joins.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectStar;

/// Marker: explicit columns — R inferred from column value types, stable across joins.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectCols<Cols>(PhantomData<Cols>);

/// Marker: raw SQL or untyped expression — R must be user-specified.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectExpr;

// =============================================================================
// FromDrizzleRow — offset-based row extraction
// =============================================================================

/// Extracts a Rust value from a database row at a given column offset.
///
/// Unlike `TryFrom<Row>`, supports offset-based reading so joined results
/// can split a single row across multiple model types.
///
/// Tuple impls compose: `(A, B)` reads A at `offset`, then B at
/// `offset + A::COLUMN_COUNT`.
#[diagnostic::on_unimplemented(
    message = "cannot deserialize `{Self}` from a database row",
    label = "this type does not implement FromDrizzleRow",
    note = "derive #[SQLiteFromRow] or #[PostgresFromRow], or use .all_as::<T>()"
)]
pub trait FromDrizzleRow<Row: ?Sized>: Sized {
    /// Number of columns this type reads from the row.
    const COLUMN_COUNT: usize;

    /// Read this type from `row` starting at column `offset`.
    fn from_row_at(row: &Row, offset: usize) -> Result<Self, DrizzleError>;

    /// Read from offset 0.
    fn from_row(row: &Row) -> Result<Self, DrizzleError> {
        Self::from_row_at(row, 0)
    }
}

// -- Tuple impls: generic over Row, composing inner impls --

macro_rules! impl_from_drizzle_row_tuple {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<__Row: ?Sized, $($T: FromDrizzleRow<__Row>),+> FromDrizzleRow<__Row> for ($($T,)+) {
            const COLUMN_COUNT: usize = 0 $(+ <$T as FromDrizzleRow<__Row>>::COLUMN_COUNT)+;

            #[allow(non_snake_case)]
            fn from_row_at(
                row: &__Row,
                offset: usize,
            ) -> Result<Self, DrizzleError> {
                let mut __off = offset;
                $(
                    let $T = <$T as FromDrizzleRow<__Row>>::from_row_at(row, __off)?;
                    __off += <$T as FromDrizzleRow<__Row>>::COLUMN_COUNT;
                )+
                Ok(($($T,)+))
            }
        }
    };
}

with_col_sizes_8!(impl_from_drizzle_row_tuple);

// =============================================================================
// SQLTypeToRust — SQL type marker × dialect → canonical Rust type
// =============================================================================

/// Maps a SQL type marker to its canonical Rust type for a given dialect.
///
/// Parameterized by `D` (a dialect marker such as
/// [`SQLiteDialect`](crate::dialect::SQLiteDialect) or
/// [`PostgresDialect`](crate::dialect::PostgresDialect)) so that
/// type mappings can differ per database.
///
/// ## Universal types
///
/// `Int`, `Text`, `Bool`, etc. map to the same Rust type on every dialect
/// via blanket `impl<D> SQLTypeToRust<D>`.
///
/// ## Dialect-specific types
///
/// `Date`, `Time`, `Timestamp`, `TimestampTz`, `Uuid`, `Json`, `Jsonb`
/// have per-dialect implementations:
///
/// - **SQLite**: Always falls back to `String` without the feature flag,
///   because SQLite stores these as TEXT.
/// - **PostgreSQL**: Requires the corresponding feature flag (`chrono`,
///   `uuid`, `serde`).  Without the flag there is **no impl**, producing
///   a compile error that guides the user.
#[diagnostic::on_unimplemented(
    message = "SQL type `{Self}` has no default Rust mapping for dialect `{D}`",
    label = "use .all_as::<T>() to specify the Rust type explicitly",
    note = "enable `chrono` for Date/Time/Timestamp/TimestampTz, `uuid` for Uuid, or `serde` for Json/Jsonb"
)]
pub trait SQLTypeToRust<D> {
    type RustType;
}

// -- Universal mappings (same on every dialect) --------------------------------

macro_rules! sql_rust_mapping {
    ($($sql:ident => $rust:ty),+ $(,)?) => {
        $(
            impl<D> SQLTypeToRust<D> for crate::types::$sql {
                type RustType = $rust;
            }
        )+
    };
}

sql_rust_mapping! {
    SmallInt => i16,
    Int      => i32,
    BigInt   => i64,
    Float    => f32,
    Double   => f64,
    Text     => crate::prelude::String,
    VarChar  => crate::prelude::String,
    Bool     => bool,
    Bytes    => crate::prelude::Vec<u8>,
    Any      => crate::prelude::String,
}

// -- SQLite dialect mappings ---------------------------------------------------

#[allow(unused_imports)]
use crate::dialect::{PostgresDialect, SQLiteDialect};

// JSON: String without serde, serde_json::Value with serde
#[cfg(feature = "serde")]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Json {
    type RustType = serde_json::Value;
}
#[cfg(not(feature = "serde"))]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Json {
    type RustType = crate::prelude::String;
}
#[cfg(feature = "serde")]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Jsonb {
    type RustType = serde_json::Value;
}
#[cfg(not(feature = "serde"))]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Jsonb {
    type RustType = crate::prelude::String;
}

// Temporal: chrono types when enabled, String otherwise (SQLite stores as TEXT)
#[cfg(feature = "chrono")]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Date {
    type RustType = chrono::NaiveDate;
}
#[cfg(not(feature = "chrono"))]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Date {
    type RustType = crate::prelude::String;
}
#[cfg(feature = "chrono")]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Time {
    type RustType = chrono::NaiveTime;
}
#[cfg(not(feature = "chrono"))]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Time {
    type RustType = crate::prelude::String;
}
#[cfg(feature = "chrono")]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Timestamp {
    type RustType = chrono::NaiveDateTime;
}
#[cfg(not(feature = "chrono"))]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Timestamp {
    type RustType = crate::prelude::String;
}
#[cfg(feature = "chrono")]
impl SQLTypeToRust<SQLiteDialect> for crate::types::TimestampTz {
    type RustType = chrono::DateTime<chrono::Utc>;
}
#[cfg(not(feature = "chrono"))]
impl SQLTypeToRust<SQLiteDialect> for crate::types::TimestampTz {
    type RustType = crate::prelude::String;
}

// UUID: uuid::Uuid when enabled, String otherwise (SQLite stores as TEXT)
#[cfg(feature = "uuid")]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Uuid {
    type RustType = uuid::Uuid;
}
#[cfg(not(feature = "uuid"))]
impl SQLTypeToRust<SQLiteDialect> for crate::types::Uuid {
    type RustType = crate::prelude::String;
}

// -- PostgreSQL dialect mappings -----------------------------------------------
// No String fallbacks — missing feature → compile error.

#[cfg(feature = "serde")]
impl SQLTypeToRust<PostgresDialect> for crate::types::Json {
    type RustType = serde_json::Value;
}
#[cfg(feature = "serde")]
impl SQLTypeToRust<PostgresDialect> for crate::types::Jsonb {
    type RustType = serde_json::Value;
}

#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for crate::types::Date {
    type RustType = chrono::NaiveDate;
}
#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for crate::types::Time {
    type RustType = chrono::NaiveTime;
}
#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for crate::types::Timestamp {
    type RustType = chrono::NaiveDateTime;
}
#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for crate::types::TimestampTz {
    type RustType = chrono::DateTime<chrono::Utc>;
}

#[cfg(feature = "uuid")]
impl SQLTypeToRust<PostgresDialect> for crate::types::Uuid {
    type RustType = uuid::Uuid;
}

// =============================================================================
// WrapNullable — Option<T> wrapping based on nullability
// =============================================================================

/// Wraps a Rust type in `Option<T>` when nullable.
pub trait WrapNullable<T> {
    type Output;
}

impl<T> WrapNullable<T> for crate::expr::NonNull {
    type Output = T;
}

impl<T> WrapNullable<T> for crate::expr::Null {
    type Output = Option<T>;
}

// =============================================================================
// ExprValueType — "what Rust type does this expression produce?"
// =============================================================================

/// Resolves the Rust value type for a column or typed expression.
///
/// Implemented for:
/// - Column ZSTs (proc macro generates alongside `ColumnValueType`)
/// - `SQLExpr<T, N, A>` where `T: SQLTypeToRust<D>` and `N: WrapNullable`
///
/// For `SQL<'a, V>` (raw SQL), `ValueType = ()` — the user must specify
/// the concrete row type via turbofish (`.all::<T>()`) or `.all_as::<T>()`.
#[diagnostic::on_unimplemented(
    message = "cannot infer Rust type for expression `{Self}`",
    label = "use .all_as::<T>() to specify the Rust type",
    note = "raw SQL and JSON expressions require explicit type annotation"
)]
pub trait ExprValueType {
    type ValueType;
}

impl<'a, V: crate::SQLParam, T, N, A> ExprValueType for crate::expr::SQLExpr<'a, V, T, N, A>
where
    T: crate::types::DataType + SQLTypeToRust<V::DialectMarker>,
    N: crate::expr::Nullability + WrapNullable<<T as SQLTypeToRust<V::DialectMarker>>::RustType>,
    A: crate::expr::AggregateKind,
{
    type ValueType = <N as WrapNullable<<T as SQLTypeToRust<V::DialectMarker>>::RustType>>::Output;
}

/// Raw SQL fallback — value type is `()`, user must specify the concrete type.
impl<'a, V: crate::SQLParam> ExprValueType for crate::sql::SQL<'a, V> {
    type ValueType = ();
}

// =============================================================================
// HasSelectModel — table → Select model (lifetime-free)
// =============================================================================

/// Associates a table with its Select model type and column count.
///
/// Generated by `#[SQLiteTable]` / `#[PostgresTable]` alongside `SQLTable`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a drizzle table",
    label = "ensure this type was derived with #[SQLiteTable] or #[PostgresTable]"
)]
pub trait HasSelectModel {
    type SelectModel;
    const COLUMN_COUNT: usize;
}

// =============================================================================
// ResolveRow — Marker + Table → default row type R
// =============================================================================

/// Given a select marker and a table, determines the default row type R.
/// Evaluated at `.from(table)` time.
#[diagnostic::on_unimplemented(
    message = "cannot resolve return type for this query",
    label = "the selected columns and table do not produce a known row type"
)]
pub trait ResolveRow<Table> {
    type Row;
}

impl<T: HasSelectModel> ResolveRow<T> for SelectStar {
    type Row = T::SelectModel;
}

impl<T> ResolveRow<T> for SelectExpr {
    type Row = ();
}

// -- SelectCols: column value types → row tuple --

macro_rules! impl_resolve_row_cols {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<__Table, $($T: ExprValueType),+> ResolveRow<__Table> for SelectCols<($($T,)+)> {
            type Row = ($(<$T as ExprValueType>::ValueType,)+);
        }
    };
}

with_col_sizes_8!(impl_resolve_row_cols);

// =============================================================================
// AfterJoin — how joins transform the row type
// =============================================================================

/// Determines the new row type after a JOIN.
pub trait AfterJoin<CurrentRow, JoinedTable> {
    type NewRow;
}

/// `SELECT *` + JOIN → `(CurrentRow, JoinedTable::SelectModel)`.
impl<R, T: HasSelectModel> AfterJoin<R, T> for SelectStar {
    type NewRow = (R, T::SelectModel);
}

/// Explicit columns + JOIN → R unchanged.
impl<Cols, R, T> AfterJoin<R, T> for SelectCols<Cols> {
    type NewRow = R;
}

/// Raw/untyped + JOIN → R unchanged.
impl<R, T> AfterJoin<R, T> for SelectExpr {
    type NewRow = R;
}

// =============================================================================
// Fallback impls for () marker (backward compatibility during transition)
// =============================================================================

/// When Marker = `()` (default before `IntoSelectTarget` is wired), R defaults to `()`.
impl<T> ResolveRow<T> for () {
    type Row = ();
}

/// When Marker = `()`, joins leave R unchanged.
impl<R, T> AfterJoin<R, T> for () {
    type NewRow = R;
}

// =============================================================================
// IntoSelectTarget — select arguments → Marker type
// =============================================================================

/// Determines the select marker from what was passed to `.select()`.
///
/// The marker controls how row types are inferred:
/// - `SelectStar` — infer R from the table's Select model
/// - `SelectCols<C>` — infer R from the column value types
/// - `SelectExpr` — R must be specified by the user
///
/// Implemented automatically for:
/// - `()` → `SelectStar`
/// - `SQL<'a, V>` → `SelectExpr`
/// - `SQLExpr<'a, V, T, N, A>` → `SelectCols<(Self,)>`
/// - Tuples `(A, B, ...)` → `SelectCols<(A, B, ...)>`
/// - Column ZSTs (proc macro generated)
/// - Table structs (proc macro generated) → `SelectStar`
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a select target",
    label = "this type does not implement IntoSelectTarget",
    note = "implement IntoSelectTarget or use a column, table, or typed expression"
)]
pub trait IntoSelectTarget {
    type Marker;
}

/// `select(())` → `SelectStar` — infer row type from the table.
impl IntoSelectTarget for () {
    type Marker = SelectStar;
}

/// `select(sql!(...))` → `SelectExpr` — user must specify row type.
impl<'a, V: crate::SQLParam> IntoSelectTarget for crate::sql::SQL<'a, V> {
    type Marker = SelectExpr;
}

/// `select(typed_expr)` → `SelectCols<(Expr,)>` — single typed expression.
impl<'a, V: crate::SQLParam, T, N, A> IntoSelectTarget for crate::expr::SQLExpr<'a, V, T, N, A>
where
    T: crate::types::DataType,
    N: crate::expr::Nullability,
    A: crate::expr::AggregateKind,
{
    type Marker = SelectCols<(Self,)>;
}

/// Tuples of select targets → `SelectCols<(A, B, ...)>`.
macro_rules! impl_into_select_target_tuple {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<$($T),+> IntoSelectTarget for ($($T,)+) {
            type Marker = SelectCols<($($T,)+)>;
        }
    };
}

with_col_sizes_8!(impl_into_select_target_tuple);
