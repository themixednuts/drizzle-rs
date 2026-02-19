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
use crate::{Cons, Nil};

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

/// Marker: explicit row model target chosen by user.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectAs<R>(PhantomData<R>);

/// Marker wrapper that carries in-scope tables.
#[derive(Debug, Clone, Copy, Default)]
pub struct Scoped<Marker, Scope>(PhantomData<(Marker, Scope)>);

/// Declares the set of tables a custom row model requires.
pub trait SelectRequiredTables {
    type RequiredTables;
}

/// Type-level witness that a table exists at the head of a scope list.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScopeHere;

/// Type-level witness that a table exists deeper in a scope list.
#[derive(Debug, Clone, Copy, Default)]
pub struct ScopeThere<Prev>(PhantomData<Prev>);

/// Type-level table membership in a scope list.
pub trait ScopeContains<Table, Witness> {}

impl<Head, Tail> ScopeContains<Head, ScopeHere> for Cons<Head, Tail> {}

impl<Head, Tail, Table, Witness> ScopeContains<Table, ScopeThere<Witness>> for Cons<Head, Tail> where
    Tail: ScopeContains<Table, Witness>
{
}

/// Required-table list satisfaction.
pub trait ScopeSatisfies<Required, Proof> {}

impl<Scope> ScopeSatisfies<Nil, ()> for Scope {}

impl<Scope, Head, Tail, HeadProof, TailProof>
    ScopeSatisfies<Cons<Head, Tail>, (HeadProof, TailProof)> for Scope
where
    Scope: ScopeContains<Head, HeadProof>,
    Scope: ScopeSatisfies<Tail, TailProof>,
{
}

/// Marker-level required-table extraction.
pub trait MarkerRequiredTables {
    type RequiredTables;
}

impl MarkerRequiredTables for SelectStar {
    type RequiredTables = Nil;
}

impl<Cols> MarkerRequiredTables for SelectCols<Cols> {
    type RequiredTables = Nil;
}

impl MarkerRequiredTables for SelectExpr {
    type RequiredTables = Nil;
}

impl<R> MarkerRequiredTables for SelectAs<R>
where
    R: SelectRequiredTables,
{
    type RequiredTables = R::RequiredTables;
}

/// Marker validation for a specific scope-satisfaction proof.
#[diagnostic::on_unimplemented(
    message = "selected row requires tables not present in the current query scope",
    label = "add .join(...) entries for every table referenced by this selector",
    note = "for aliased selectors, use the same alias type in #[from(...)] and .from(...)"
)]
///
/// ```
/// use drizzle_core::{Cons, Nil, Scoped, SelectAs, SelectRequiredTables};
/// use drizzle_core::row::{MarkerScopeValidFor, ScopeHere};
///
/// struct Users;
/// struct Model;
///
/// impl SelectRequiredTables for Model {
///     type RequiredTables = Cons<Users, Nil>;
/// }
///
/// type Good = Scoped<SelectAs<Model>, Cons<Users, Nil>>;
///
/// fn needs_valid<M: MarkerScopeValidFor<(ScopeHere, ())>>() {}
///
/// fn main() {
///     needs_valid::<Good>();
/// }
/// ```
///
/// ```compile_fail
/// use drizzle_core::{Cons, Nil, Scoped, SelectAs, SelectRequiredTables};
/// use drizzle_core::row::{MarkerScopeValidFor, ScopeHere};
///
/// struct Users;
/// struct Posts;
/// struct Model;
///
/// impl SelectRequiredTables for Model {
///     type RequiredTables = Cons<Users, Nil>;
/// }
///
/// type Bad = Scoped<SelectAs<Model>, Cons<Posts, Nil>>;
///
/// fn needs_valid<M: MarkerScopeValidFor<(ScopeHere, ())>>() {}
///
/// fn main() {
///     needs_valid::<Bad>();
/// }
/// ```
pub trait MarkerScopeValidFor<Proof> {}

impl<M, Scope, Proof> MarkerScopeValidFor<Proof> for Scoped<M, Scope>
where
    M: MarkerRequiredTables,
    Scope: ScopeSatisfies<<M as MarkerRequiredTables>::RequiredTables, Proof>,
{
}

// =============================================================================
// Marker column-count validation for strict decode paths
// =============================================================================

/// Type-level column-list representation for a row decode target.
///
/// Each consumed column is represented by a `Cons<T, ...>` node where `T`
/// is the decoded Rust type for that column.
pub trait RowColumnList<Row: ?Sized> {
    type Columns: crate::TypeSet;
}

/// Type-level column-list representation for selected column tuples.
pub trait SelectedColumnList {
    type Columns: crate::TypeSet;
}

trait SameType<T> {}
impl<T> SameType<T> for T {}

trait ColumnTypeCompatible<Row: ?Sized, Expected, Actual> {}

impl<Row: ?Sized, T> ColumnTypeCompatible<Row, T, T> for () {}

trait TypeListCompatible<Row: ?Sized, ActualList> {}

impl<Row: ?Sized> TypeListCompatible<Row, crate::Nil> for crate::Nil {}

impl<Row: ?Sized, EH, ET, AH, AT> TypeListCompatible<Row, crate::Cons<AH, AT>>
    for crate::Cons<EH, ET>
where
    (): ColumnTypeCompatible<Row, EH, AH>,
    ET: TypeListCompatible<Row, AT>,
{
}

trait SqliteDecodeRow {}

#[cfg(feature = "rusqlite")]
impl<'r> SqliteDecodeRow for ::rusqlite::Row<'r> {}

#[cfg(feature = "libsql")]
impl SqliteDecodeRow for ::libsql::Row {}

#[cfg(feature = "turso")]
impl SqliteDecodeRow for ::turso::Row {}

macro_rules! impl_sqlite_integer_decode_compat {
    ($expected:ty => $($actual:ty),+ $(,)?) => {
        $(
            impl<Row> ColumnTypeCompatible<Row, $expected, $actual> for ()
            where
                Row: SqliteDecodeRow,
            {
            }
        )+
    };
}

impl_sqlite_integer_decode_compat!(
    i64 => i8, i16, i32, isize, u8, u16, u32, u64, usize, bool
);

impl_sqlite_integer_decode_compat!(
    Option<i64> =>
        Option<i8>,
        Option<i16>,
        Option<i32>,
        Option<isize>,
        Option<u8>,
        Option<u16>,
        Option<u32>,
        Option<u64>,
        Option<usize>,
        Option<bool>
);

macro_rules! impl_row_column_list_one {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl<Row: ?Sized> RowColumnList<Row> for $ty {
                type Columns = crate::Cons<$ty, crate::Nil>;
            }
        )+
    };
}

impl_row_column_list_one!(
    i8,
    i16,
    i32,
    i64,
    isize,
    u8,
    u16,
    u32,
    u64,
    usize,
    f32,
    f64,
    bool,
    crate::prelude::String,
    crate::prelude::Vec<u8>
);

impl<Row: ?Sized> RowColumnList<Row> for () {
    type Columns = crate::Cons<(), crate::Nil>;
}

#[cfg(feature = "uuid")]
impl<Row: ?Sized> RowColumnList<Row> for uuid::Uuid {
    type Columns = crate::Cons<uuid::Uuid, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::NaiveDate {
    type Columns = crate::Cons<chrono::NaiveDate, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::NaiveTime {
    type Columns = crate::Cons<chrono::NaiveTime, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::NaiveDateTime {
    type Columns = crate::Cons<chrono::NaiveDateTime, crate::Nil>;
}

#[cfg(feature = "chrono")]
impl<Row: ?Sized> RowColumnList<Row> for chrono::DateTime<chrono::Utc> {
    type Columns = crate::Cons<chrono::DateTime<chrono::Utc>, crate::Nil>;
}

#[cfg(feature = "serde")]
impl<Row: ?Sized> RowColumnList<Row> for serde_json::Value {
    type Columns = crate::Cons<serde_json::Value, crate::Nil>;
}

#[cfg(feature = "arrayvec")]
impl<Row: ?Sized, const N: usize> RowColumnList<Row> for arrayvec::ArrayString<N> {
    type Columns = crate::Cons<arrayvec::ArrayString<N>, crate::Nil>;
}

#[cfg(feature = "arrayvec")]
impl<Row: ?Sized, T, const N: usize> RowColumnList<Row> for arrayvec::ArrayVec<T, N> {
    type Columns = crate::Cons<arrayvec::ArrayVec<T, N>, crate::Nil>;
}

impl<Row: ?Sized> RowColumnList<Row> for compact_str::CompactString {
    type Columns = crate::Cons<compact_str::CompactString, crate::Nil>;
}

#[cfg(feature = "bytes")]
impl<Row: ?Sized> RowColumnList<Row> for bytes::Bytes {
    type Columns = crate::Cons<bytes::Bytes, crate::Nil>;
}

#[cfg(feature = "bytes")]
impl<Row: ?Sized> RowColumnList<Row> for bytes::BytesMut {
    type Columns = crate::Cons<bytes::BytesMut, crate::Nil>;
}

impl<Row: ?Sized, A: smallvec::Array> RowColumnList<Row> for smallvec::SmallVec<A> {
    type Columns = crate::Cons<smallvec::SmallVec<A>, crate::Nil>;
}

impl<Row: ?Sized, T> RowColumnList<Row> for Option<T> {
    type Columns = crate::Cons<Option<T>, crate::Nil>;
}

impl<Row: ?Sized, A> RowColumnList<Row> for (A,)
where
    A: RowColumnList<Row>,
{
    type Columns = <A as RowColumnList<Row>>::Columns;
}

impl<Row: ?Sized, A, B> RowColumnList<Row> for (A, B)
where
    A: RowColumnList<Row>,
    B: RowColumnList<Row>,
    <A as RowColumnList<Row>>::Columns: crate::Concat<<B as RowColumnList<Row>>::Columns>,
{
    type Columns = <<A as RowColumnList<Row>>::Columns as crate::Concat<
        <B as RowColumnList<Row>>::Columns,
    >>::Output;
}

impl<Row: ?Sized, A, B, C> RowColumnList<Row> for (A, B, C)
where
    A: RowColumnList<Row>,
    B: RowColumnList<Row>,
    C: RowColumnList<Row>,
    <A as RowColumnList<Row>>::Columns: crate::Concat<<B as RowColumnList<Row>>::Columns>,
    <<A as RowColumnList<Row>>::Columns as crate::Concat<<B as RowColumnList<Row>>::Columns>>::Output:
        crate::Concat<<C as RowColumnList<Row>>::Columns>,
{
    type Columns = <<<A as RowColumnList<Row>>::Columns as crate::Concat<
        <B as RowColumnList<Row>>::Columns,
    >>::Output as crate::Concat<<C as RowColumnList<Row>>::Columns>>::Output;
}

impl<Row: ?Sized, A, B, C, D> RowColumnList<Row> for (A, B, C, D)
where
    A: RowColumnList<Row>,
    B: RowColumnList<Row>,
    C: RowColumnList<Row>,
    D: RowColumnList<Row>,
    (A, B, C): RowColumnList<Row>,
    <(A, B, C) as RowColumnList<Row>>::Columns: crate::Concat<<D as RowColumnList<Row>>::Columns>,
{
    type Columns = <<(A, B, C) as RowColumnList<Row>>::Columns as crate::Concat<
        <D as RowColumnList<Row>>::Columns,
    >>::Output;
}

impl<Row: ?Sized, A, B, C, D, E> RowColumnList<Row> for (A, B, C, D, E)
where
    E: RowColumnList<Row>,
    (A, B, C, D): RowColumnList<Row>,
    <(A, B, C, D) as RowColumnList<Row>>::Columns:
        crate::Concat<<E as RowColumnList<Row>>::Columns>,
{
    type Columns = <<(A, B, C, D) as RowColumnList<Row>>::Columns as crate::Concat<
        <E as RowColumnList<Row>>::Columns,
    >>::Output;
}

impl<Row: ?Sized, A, B, C, D, E, F> RowColumnList<Row> for (A, B, C, D, E, F)
where
    F: RowColumnList<Row>,
    (A, B, C, D, E): RowColumnList<Row>,
    <(A, B, C, D, E) as RowColumnList<Row>>::Columns:
        crate::Concat<<F as RowColumnList<Row>>::Columns>,
{
    type Columns = <<(A, B, C, D, E) as RowColumnList<Row>>::Columns as crate::Concat<
        <F as RowColumnList<Row>>::Columns,
    >>::Output;
}

impl<Row: ?Sized, A, B, C, D, E, F, G> RowColumnList<Row> for (A, B, C, D, E, F, G)
where
    G: RowColumnList<Row>,
    (A, B, C, D, E, F): RowColumnList<Row>,
    <(A, B, C, D, E, F) as RowColumnList<Row>>::Columns:
        crate::Concat<<G as RowColumnList<Row>>::Columns>,
{
    type Columns = <<(A, B, C, D, E, F) as RowColumnList<Row>>::Columns as crate::Concat<
        <G as RowColumnList<Row>>::Columns,
    >>::Output;
}

impl<Row: ?Sized, A, B, C, D, E, F, G, H> RowColumnList<Row> for (A, B, C, D, E, F, G, H)
where
    H: RowColumnList<Row>,
    (A, B, C, D, E, F, G): RowColumnList<Row>,
    <(A, B, C, D, E, F, G) as RowColumnList<Row>>::Columns:
        crate::Concat<<H as RowColumnList<Row>>::Columns>,
{
    type Columns = <<(A, B, C, D, E, F, G) as RowColumnList<Row>>::Columns as crate::Concat<
        <H as RowColumnList<Row>>::Columns,
    >>::Output;
}

/// Marker-level column-count compatibility check used by strict `.all()` / `.get()`.
///
/// Currently enforced for `SelectCols<_>` where selected shape is explicit.
#[diagnostic::on_unimplemented(
    message = "selected shape does not match decode target `{Actual}`",
    label = "this decode target is not type-compatible with .select(...) output",
    note = "use .all_as::<T>() for explicit remapping when selecting custom expressions"
)]
pub trait MarkerColumnCountValid<Row: ?Sized, Inferred, Actual> {}

/// Marker-level guard for strict decode entry points.
///
/// Raw `SelectExpr` (`select(sql!(...))`) is intentionally excluded so strict
/// decode requires either typed expressions (`raw_non_null`, `sql!(.., Type)`) or
/// explicit remapping via `.all_as::<T>()`.
#[diagnostic::on_unimplemented(
    message = "raw select expressions require explicit typing in strict decode",
    label = "`select(sql!(...)).all()/get()` is not allowed in strict mode",
    note = "use typed wrappers like `raw_non_null`/`raw_nullable` or call `.all_as::<T>()` / `.get_as::<T>()`"
)]
pub trait StrictDecodeMarker {}

impl StrictDecodeMarker for SelectStar {}
impl<Cols> StrictDecodeMarker for SelectCols<Cols> {}
impl<R> StrictDecodeMarker for SelectAs<R> {}
impl<M, Scope> StrictDecodeMarker for Scoped<M, Scope> where M: StrictDecodeMarker {}

impl<Row: ?Sized, Inferred, Actual> MarkerColumnCountValid<Row, Inferred, Actual> for SelectStar {}

impl<Row: ?Sized, Cols, Inferred, Actual> MarkerColumnCountValid<Row, Inferred, Actual>
    for SelectCols<Cols>
where
    Cols: SelectedColumnList,
    Actual: RowColumnList<Row>,
    <Cols as SelectedColumnList>::Columns:
        TypeListCompatible<Row, <Actual as RowColumnList<Row>>::Columns>,
{
}

impl<Row: ?Sized, Inferred, Actual> MarkerColumnCountValid<Row, Inferred, Actual> for SelectExpr where
    Inferred: SameType<Actual>
{
}

impl<Row: ?Sized, R, Inferred, Actual> MarkerColumnCountValid<Row, Inferred, Actual>
    for SelectAs<R>
{
}

impl<M, Scope, Row: ?Sized, Inferred, Actual> MarkerColumnCountValid<Row, Inferred, Actual>
    for Scoped<M, Scope>
where
    M: MarkerColumnCountValid<Row, Inferred, Actual>,
{
}

/// Pushes a joined table into the marker scope.
pub trait ScopePush<Joined> {
    type Out;
}

impl<M, Scope, Joined> ScopePush<Joined> for Scoped<M, Scope> {
    type Out = Scoped<M, Cons<Joined, Scope>>;
}

/// Marker-directed row decoding for `.all()`/`.get()`.
pub trait DecodeSelectedRef<RowRef, R> {
    fn decode(row: RowRef) -> Result<R, DrizzleError>;
}

impl<RowRef, R> DecodeSelectedRef<RowRef, R> for SelectAs<R>
where
    R: TryFrom<RowRef>,
    <R as TryFrom<RowRef>>::Error: Into<DrizzleError>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        R::try_from(row).map_err(Into::into)
    }
}

impl<RowRef, R, M, Scope> DecodeSelectedRef<RowRef, R> for Scoped<M, Scope>
where
    M: DecodeSelectedRef<RowRef, R>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        M::decode(row)
    }
}

impl<RowRef, Row: ?Sized, R> DecodeSelectedRef<RowRef, R> for SelectStar
where
    RowRef: core::ops::Deref<Target = Row>,
    R: FromDrizzleRow<Row>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        R::from_row(&*row)
    }
}

impl<RowRef, Row: ?Sized, Cols, R> DecodeSelectedRef<RowRef, R> for SelectCols<Cols>
where
    RowRef: core::ops::Deref<Target = Row>,
    R: FromDrizzleRow<Row>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        R::from_row(&*row)
    }
}

impl<RowRef, Row: ?Sized, R> DecodeSelectedRef<RowRef, R> for SelectExpr
where
    RowRef: core::ops::Deref<Target = Row>,
    R: FromDrizzleRow<Row>,
{
    fn decode(row: RowRef) -> Result<R, DrizzleError> {
        R::from_row(&*row)
    }
}

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

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_from_drizzle_row_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_from_drizzle_row_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_from_drizzle_row_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_from_drizzle_row_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_from_drizzle_row_tuple);

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

impl<D, T> SQLTypeToRust<D> for crate::types::Array<T>
where
    T: crate::types::DataType + SQLTypeToRust<D>,
{
    type RustType = crate::prelude::Vec<<T as SQLTypeToRust<D>>::RustType>;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Integer {
    type RustType = i64;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Real {
    type RustType = f64;
}

impl SQLTypeToRust<SQLiteDialect> for drizzle_types::sqlite::types::Blob {
    type RustType = crate::prelude::Vec<u8>;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Int2 {
    type RustType = i16;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Int4 {
    type RustType = i32;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Int8 {
    type RustType = i64;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Float4 {
    type RustType = f32;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Float8 {
    type RustType = f64;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Varchar {
    type RustType = crate::prelude::String;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Bytea {
    type RustType = crate::prelude::Vec<u8>;
}

impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Boolean {
    type RustType = bool;
}

#[cfg(feature = "chrono")]
impl SQLTypeToRust<PostgresDialect> for drizzle_types::postgres::types::Timestamptz {
    type RustType = chrono::DateTime<chrono::Utc>;
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

impl<R, T> ResolveRow<T> for SelectAs<R>
where
    R: SelectAsFrom<T>,
{
    type Row = R;
}

impl<M, Scope, T> ResolveRow<T> for Scoped<M, Scope>
where
    M: ResolveRow<T>,
{
    type Row = M::Row;
}

/// Compile-time constraint for `.select(MyRow::Select).from(table)` base table matching.
///
/// `#[from(Table)]` on `*FromRow` structs emits `impl SelectAsFrom<Table> for MyRow`.
/// Structs without `#[from(...)]` may opt into any table.
#[diagnostic::on_unimplemented(
    message = "row selector `{Self}` cannot be used with table `{Table}`",
    label = "the #[from(...)] table does not match .from(...)",
    note = "set #[from(TheTable)] to the same table passed to .from(...)"
)]
pub trait SelectAsFrom<Table> {}

// -- SelectCols: column value types → row tuple --

macro_rules! impl_resolve_row_cols {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<__Table, $($T: ExprValueType),+> ResolveRow<__Table> for SelectCols<($($T,)+)> {
            type Row = ($(<$T as ExprValueType>::ValueType,)+);
        }
    };
}

with_col_sizes_8!(impl_resolve_row_cols);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_resolve_row_cols);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_resolve_row_cols);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_resolve_row_cols);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_resolve_row_cols);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_resolve_row_cols);

macro_rules! selected_columns_cons {
    () => {
        crate::Nil
    };
    ($head:ident $(, $tail:ident)*) => {
        crate::Cons<<$head as ExprValueType>::ValueType, selected_columns_cons!($($tail),*)>
    };
}

macro_rules! impl_selected_column_list_tuple {
    ($($T:ident),+; $($idx:tt),+) => {
        impl<$($T: ExprValueType),+> SelectedColumnList for ($($T,)+) {
            type Columns = selected_columns_cons!($($T),+);
        }
    };
}

with_col_sizes_8!(impl_selected_column_list_tuple);

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_selected_column_list_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_selected_column_list_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_selected_column_list_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_selected_column_list_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_selected_column_list_tuple);

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

/// Explicit model + JOIN → R unchanged.
impl<Row, R, T> AfterJoin<R, T> for SelectAs<Row> {
    type NewRow = R;
}

impl<M, Scope, R, T> AfterJoin<R, T> for Scoped<M, Scope>
where
    M: AfterJoin<R, T>,
{
    type NewRow = M::NewRow;
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

#[cfg(any(
    feature = "col16",
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_16!(impl_into_select_target_tuple);

#[cfg(any(
    feature = "col32",
    feature = "col64",
    feature = "col128",
    feature = "col200"
))]
with_col_sizes_32!(impl_into_select_target_tuple);

#[cfg(any(feature = "col64", feature = "col128", feature = "col200"))]
with_col_sizes_64!(impl_into_select_target_tuple);

#[cfg(any(feature = "col128", feature = "col200"))]
with_col_sizes_128!(impl_into_select_target_tuple);

#[cfg(feature = "col200")]
with_col_sizes_200!(impl_into_select_target_tuple);
