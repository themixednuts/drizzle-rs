//! Core `QueryBuilder` — pure SQL generation layer, no connection.

use core::marker::PhantomData;

use crate::prelude::*;
use crate::relation::{CardWrap, RelationDef};
use crate::{PaginationArg, SQL, SQLParam};

use super::handle::RelationHandle;
use super::row::QueryRow;
use super::store::RelEntry;

// =============================================================================
// Column selection types
// =============================================================================

// =============================================================================
// Clause typestates
// =============================================================================

/// Marker: no WHERE clause has been set (default).
pub struct NoWhere;

/// Marker: a WHERE clause has been set.
pub struct HasWhere;

/// Marker: no ORDER BY clause has been set (default).
pub struct NoOrderBy;

/// Marker: an ORDER BY clause has been set.
pub struct HasOrderBy;

/// Marker: no LIMIT has been set (default).
pub struct NoLimit;

/// Marker: a LIMIT has been set (allows calling `.offset()`).
pub struct HasLimit;

/// Marker: both LIMIT and OFFSET have been set.
pub struct HasOffset;

/// Composite typestate tracking which query clauses have been set.
///
/// Each type parameter is a zero-sized marker:
/// - `W`: [`NoWhere`] (default) or [`HasWhere`]
/// - `Ord`: [`NoOrderBy`] (default) or [`HasOrderBy`]
/// - `Lim`: [`NoLimit`] (default), [`HasLimit`], or [`HasOffset`]
pub struct Clauses<W = NoWhere, Ord = NoOrderBy, Lim = NoLimit> {
    _marker: PhantomData<(W, Ord, Lim)>,
}

/// Marker: select all columns (default). The result model is `T::Select`.
pub struct AllColumns;

/// Partial column selection. The result model is `T::PartialSelect`.
pub struct PartialColumns {
    /// The subset of column names to include in the query.
    pub columns: Vec<&'static str>,
}

/// Maps a column selection mode to the appropriate select model type.
pub trait ResolveSelect<T: QueryTable> {
    /// The model type produced by this column selection.
    type Model;
}

impl<T: QueryTable> ResolveSelect<T> for AllColumns {
    type Model = T::Select;
}

impl<T: QueryTable> ResolveSelect<T> for PartialColumns {
    type Model = T::PartialSelect;
}

/// Converts a column selector into a list of column names.
pub trait IntoColumnSelection {
    /// Consumes self and returns the selected column names.
    fn into_column_names(self) -> Vec<&'static str>;
}

// =============================================================================
// QueryBuilder
// =============================================================================

/// Core query builder. Holds relation handles and query config.
///
/// The `Rels` type parameter is the actual storage for relation handles —
/// `()` when empty, `(RelationHandle<'a, V, R, N, C>, Rest)` when populated.
/// This preserves the full relation tree in the type system.
///
/// The `Cols` type parameter controls column selection — `AllColumns` (default)
/// selects all columns, `PartialColumns` selects a subset.
///
/// The `Cl` type parameter is a [`Clauses`] composite tracking which query
/// clauses have been set (WHERE, ORDER BY, LIMIT/OFFSET). Each clause can
/// only be set once — the typestate prevents double-calling at compile time.
///
/// The driver layer wraps this with a connection reference for execution.
pub struct QueryBuilder<'a, V: SQLParam, T, Rels = (), Cols = AllColumns, Cl = Clauses> {
    #[doc(hidden)]
    pub where_sql: SQL<'a, V>,
    #[doc(hidden)]
    pub order_by_sql: SQL<'a, V>,
    #[doc(hidden)]
    pub limit: Option<SQL<'a, V>>,
    #[doc(hidden)]
    pub offset: Option<SQL<'a, V>>,
    #[doc(hidden)]
    pub relations: Rels,
    #[doc(hidden)]
    pub cols: Cols,
    _marker: PhantomData<(T, Cl)>,
}

impl<'a, V: SQLParam, T> QueryBuilder<'a, V, T> {
    /// Creates a new empty `QueryBuilder` for the given table type.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            where_sql: SQL::empty(),
            order_by_sql: SQL::empty(),
            limit: None,
            offset: None,
            relations: (),
            cols: AllColumns,
            _marker: PhantomData,
        }
    }
}

impl<'a, V: SQLParam, T> Default for QueryBuilder<'a, V, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, V: SQLParam, T, Rels, Cols, Cl> QueryBuilder<'a, V, T, Rels, Cols, Cl> {
    /// Includes a relation in the query results.
    #[allow(clippy::type_complexity)]
    pub fn with<R, N, C, RCl>(
        self,
        handle: RelationHandle<'a, V, R, N, C, RCl>,
    ) -> QueryBuilder<'a, V, T, (RelationHandle<'a, V, R, N, C, RCl>, Rels), Cols, Cl>
    where
        R: RelationDef<Source = T> + 'static,
    {
        QueryBuilder {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: self.offset,
            relations: (handle, self.relations),
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// WHERE is only available when no WHERE clause has been set yet.
impl<'a, V: SQLParam, T, Rels, Cols, Ord, Lim>
    QueryBuilder<'a, V, T, Rels, Cols, Clauses<NoWhere, Ord, Lim>>
{
    /// Sets the WHERE clause for the query.
    ///
    /// Can only be called once. To combine multiple conditions, use boolean
    /// operators: `and(cond_a, cond_b)` or `or(cond_a, cond_b)`.
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> QueryBuilder<'a, V, T, Rels, Cols, Clauses<HasWhere, Ord, Lim>>
    where
        E: crate::expr::Expr<'a, V>,
        E::SQLType: crate::types::BooleanLike,
        V: 'a,
    {
        QueryBuilder {
            where_sql: condition.to_sql(),
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: self.offset,
            relations: self.relations,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// ORDER BY is only available when no ORDER BY clause has been set yet.
impl<'a, V: SQLParam, T, Rels, Cols, W, Lim>
    QueryBuilder<'a, V, T, Rels, Cols, Clauses<W, NoOrderBy, Lim>>
{
    /// Adds a typed ORDER BY clause.
    ///
    /// Can only be called once. ORDER BY expressions are column references
    /// (e.g., `asc(col)`, `desc(col)`), which never produce bind parameters.
    pub fn order_by<E>(
        self,
        expr: E,
    ) -> QueryBuilder<'a, V, T, Rels, Cols, Clauses<W, HasOrderBy, Lim>>
    where
        E: crate::traits::ToSQL<'a, V>,
        V: 'a,
    {
        QueryBuilder {
            where_sql: self.where_sql,
            order_by_sql: expr.to_sql(),
            limit: self.limit,
            offset: self.offset,
            relations: self.relations,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// LIMIT is only available when no LIMIT has been set yet.
impl<'a, V: SQLParam, T, Rels, Cols, W, Ord>
    QueryBuilder<'a, V, T, Rels, Cols, Clauses<W, Ord, NoLimit>>
{
    /// Sets a LIMIT on the query.
    ///
    /// Can only be called once. Enables calling `.offset()`.
    pub fn limit<P>(self, n: P) -> QueryBuilder<'a, V, T, Rels, Cols, Clauses<W, Ord, HasLimit>>
    where
        P: PaginationArg<'a, V>,
        V: 'a,
    {
        QueryBuilder {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: Some(n.into_pagination_sql()),
            offset: self.offset,
            relations: self.relations,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// OFFSET requires LIMIT to have been set first.
impl<'a, V: SQLParam, T, Rels, Cols, W, Ord>
    QueryBuilder<'a, V, T, Rels, Cols, Clauses<W, Ord, HasLimit>>
{
    /// Sets an OFFSET on the query. Requires `.limit()` to have been called first.
    pub fn offset<P>(self, n: P) -> QueryBuilder<'a, V, T, Rels, Cols, Clauses<W, Ord, HasOffset>>
    where
        P: PaginationArg<'a, V>,
        V: 'a,
    {
        QueryBuilder {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: Some(n.into_pagination_sql()),
            relations: self.relations,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// Methods only available when all columns are selected (prevents double-calling).
impl<'a, V: SQLParam, T: QueryTable, Rels, Cl> QueryBuilder<'a, V, T, Rels, AllColumns, Cl> {
    /// Selects only the specified columns (include list).
    ///
    /// The result model becomes `T::PartialSelect` with only selected columns populated.
    pub fn columns<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> QueryBuilder<'a, V, T, Rels, PartialColumns, Cl> {
        QueryBuilder {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: self.offset,
            relations: self.relations,
            cols: PartialColumns {
                columns: selector.into_column_names(),
            },
            _marker: PhantomData,
        }
    }

    /// Excludes the specified columns (exclude list).
    ///
    /// The result model becomes `T::PartialSelect` with all columns except the omitted ones.
    pub fn omit<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> QueryBuilder<'a, V, T, Rels, PartialColumns, Cl> {
        let omitted = selector.into_column_names();
        let columns = T::COLUMN_NAMES
            .iter()
            .copied()
            .filter(|c| !omitted.contains(c))
            .collect();
        QueryBuilder {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: self.offset,
            relations: self.relations,
            cols: PartialColumns { columns },
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// BuildStore / BuildRow
// =============================================================================

/// Maps a builder's relation list to the JSON-decode store type.
pub trait BuildStore {
    /// Storage type for the configured relations.
    type Store;
}

impl BuildStore for () {
    type Store = ();
}

impl<'a, V: SQLParam, R, Nested, Rest, Cols, Cl> BuildStore
    for (RelationHandle<'a, V, R, Nested, Cols, Cl>, Rest)
where
    R: RelationDef,
    R::Target: QueryTable,
    Cols: ResolveSelect<R::Target>,
    Nested: BuildStore,
    Rest: BuildStore,
{
    type Store = RelEntry<
        R,
        <R::Card as CardWrap>::Wrap<
            QueryRow<<Cols as ResolveSelect<R::Target>>::Model, <Nested as BuildStore>::Store>,
        >,
        <Rest as BuildStore>::Store,
    >;
}

/// Assembles a decoded relation store into the public query row type.
///
/// With no relations, `Row` is the base select / partial-select model. Each
/// `.with(...)` wraps that row in a generated `*With*` struct that exposes the
/// relation as a named field.
pub trait BuildRow<Base>: BuildStore {
    /// Row type returned by `find_many` / `find_first`.
    type Row;

    /// Builds the public row from a base model and decoded relation store.
    fn assemble(base: Base, store: Self::Store) -> Self::Row;
}

impl<Base> BuildRow<Base> for () {
    type Row = Base;

    fn assemble(base: Base, (): ()) -> Self::Row {
        base
    }
}

impl<'a, V: SQLParam, R, Nested, Rest, Cols, Cl, Base> BuildRow<Base>
    for (RelationHandle<'a, V, R, Nested, Cols, Cl>, Rest)
where
    R: RelationDef + crate::relation::AssembleRel,
    R::Target: QueryTable,
    Cols: ResolveSelect<R::Target>,
    Nested: BuildRow<<Cols as ResolveSelect<R::Target>>::Model>,
    Rest: BuildRow<Base>,
    Self: BuildStore<
        Store = RelEntry<
            R,
            <R::Card as CardWrap>::Wrap<
                QueryRow<<Cols as ResolveSelect<R::Target>>::Model, Nested::Store>,
            >,
            Rest::Store,
        >,
    >,
{
    type Row = <R as crate::relation::AssembleRel>::Row<Rest::Row, Nested::Row>;

    fn assemble(base: Base, store: Self::Store) -> Self::Row {
        let (data, rest) = store.into_parts();
        let inner = Rest::assemble(base, rest);
        let children = R::Card::map_wrap(data, |child_row| {
            let (child_base, child_store) = child_row.into_parts();
            Nested::assemble(child_base, child_store)
        });
        R::assemble_row(inner, children)
    }
}

// =============================================================================
// QueryTable
// =============================================================================

/// Dialect-agnostic table metadata for the query API.
///
/// Provides the table name, column names, and select model types
/// without requiring dialect-specific type parameters.
pub trait QueryTable {
    /// The select model type for this table (all columns).
    type Select;
    /// The partial select model type (all fields `Option<T>`).
    type PartialSelect;
    /// The SQL table name.
    const TABLE_NAME: &'static str;
    /// All column names in SELECT order.
    const COLUMN_NAMES: &'static [&'static str];
    /// Column names that store BLOB data (e.g., UUID as bytes in `SQLite`).
    ///
    /// These columns are wrapped with `hex()` inside `json_object()` calls
    /// because `SQLite`'s JSON functions cannot serialize BLOB values directly.
    /// The query JSON decoder then parses the hex string back.
    const BLOB_COLUMNS: &'static [&'static str] = &[];
}
