//! Core `QueryBuilder` — pure SQL generation layer, no connection.

use core::marker::PhantomData;

use crate::SQLParam;
use crate::prelude::*;
use crate::relation::{CardWrap, RelationDef};

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
/// `()` when empty, `(RelationHandle<V, R, N, C>, Rest)` when populated.
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
pub struct QueryBuilder<V: SQLParam, T, Rels = (), Cols = AllColumns, Cl = Clauses> {
    #[doc(hidden)]
    pub where_clause: String,
    #[doc(hidden)]
    pub where_params: Vec<V>,
    #[doc(hidden)]
    pub order_by_clause: String,
    #[doc(hidden)]
    pub limit: Option<u32>,
    #[doc(hidden)]
    pub offset: Option<u32>,
    #[doc(hidden)]
    pub relations: Rels,
    #[doc(hidden)]
    pub cols: Cols,
    _marker: PhantomData<(T, Cl)>,
}

impl<V: SQLParam, T> QueryBuilder<V, T> {
    /// Creates a new empty `QueryBuilder` for the given table type.
    pub fn new() -> Self {
        Self {
            where_clause: String::new(),
            where_params: Vec::new(),
            order_by_clause: String::new(),
            limit: None,
            offset: None,
            relations: (),
            cols: AllColumns,
            _marker: PhantomData,
        }
    }
}

impl<V: SQLParam, T> Default for QueryBuilder<V, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: SQLParam, T, Rels, Cols, Cl> QueryBuilder<V, T, Rels, Cols, Cl> {
    /// Includes a relation in the query results.
    #[allow(clippy::type_complexity)]
    pub fn with<R, N, C, RCl>(
        self,
        handle: RelationHandle<V, R, N, C, RCl>,
    ) -> QueryBuilder<V, T, (RelationHandle<V, R, N, C, RCl>, Rels), Cols, Cl>
    where
        R: RelationDef<Source = T> + 'static,
    {
        QueryBuilder {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: self.offset,
            relations: (handle, self.relations),
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// WHERE is only available when no WHERE clause has been set yet.
impl<V: SQLParam, T, Rels, Cols, Ord, Lim>
    QueryBuilder<V, T, Rels, Cols, Clauses<NoWhere, Ord, Lim>>
{
    /// Sets the WHERE clause for the query.
    ///
    /// Can only be called once. To combine multiple conditions, use boolean
    /// operators: `and(cond_a, cond_b)` or `or(cond_a, cond_b)`.
    pub fn r#where<'a, E>(
        self,
        condition: E,
    ) -> QueryBuilder<V, T, Rels, Cols, Clauses<HasWhere, Ord, Lim>>
    where
        E: crate::expr::Expr<'a, V>,
        E::SQLType: crate::types::BooleanLike,
        V: 'a,
    {
        let sql = condition.to_sql();
        let (text, params) = sql.build();
        QueryBuilder {
            where_clause: text,
            where_params: params.into_iter().cloned().collect(),
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: self.offset,
            relations: self.relations,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// ORDER BY is only available when no ORDER BY clause has been set yet.
impl<V: SQLParam, T, Rels, Cols, W, Lim>
    QueryBuilder<V, T, Rels, Cols, Clauses<W, NoOrderBy, Lim>>
{
    /// Adds a typed ORDER BY clause.
    ///
    /// Can only be called once. ORDER BY expressions are column references
    /// (e.g., `asc(col)`, `desc(col)`), which never produce bind parameters.
    pub fn order_by<'a, E>(
        self,
        expr: E,
    ) -> QueryBuilder<V, T, Rels, Cols, Clauses<W, HasOrderBy, Lim>>
    where
        E: crate::traits::ToSQL<'a, V>,
        V: 'a,
    {
        let sql = expr.to_sql();
        let (text, _) = sql.build();
        QueryBuilder {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: text,
            limit: self.limit,
            offset: self.offset,
            relations: self.relations,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// LIMIT is only available when no LIMIT has been set yet.
impl<V: SQLParam, T, Rels, Cols, W, Ord> QueryBuilder<V, T, Rels, Cols, Clauses<W, Ord, NoLimit>> {
    /// Sets a LIMIT on the query.
    ///
    /// Can only be called once. Enables calling `.offset()`.
    pub fn limit(self, n: u32) -> QueryBuilder<V, T, Rels, Cols, Clauses<W, Ord, HasLimit>> {
        QueryBuilder {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: Some(n),
            offset: self.offset,
            relations: self.relations,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// OFFSET requires LIMIT to have been set first.
impl<V: SQLParam, T, Rels, Cols, W, Ord> QueryBuilder<V, T, Rels, Cols, Clauses<W, Ord, HasLimit>> {
    /// Sets an OFFSET on the query. Requires `.limit()` to have been called first.
    pub fn offset(self, n: u32) -> QueryBuilder<V, T, Rels, Cols, Clauses<W, Ord, HasOffset>> {
        QueryBuilder {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: Some(n),
            relations: self.relations,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// Methods only available when all columns are selected (prevents double-calling).
impl<V: SQLParam, T: QueryTable, Rels, Cl> QueryBuilder<V, T, Rels, AllColumns, Cl> {
    /// Selects only the specified columns (include list).
    ///
    /// The result model becomes `T::PartialSelect` with only selected columns populated.
    pub fn columns<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> QueryBuilder<V, T, Rels, PartialColumns, Cl> {
        QueryBuilder {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
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
    ) -> QueryBuilder<V, T, Rels, PartialColumns, Cl> {
        let omitted = selector.into_column_names();
        let columns = T::COLUMN_NAMES
            .iter()
            .copied()
            .filter(|c| !omitted.contains(c))
            .collect();
        QueryBuilder {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: self.offset,
            relations: self.relations,
            cols: PartialColumns { columns },
            _marker: PhantomData,
        }
    }
}

// =============================================================================
// BuildStore
// =============================================================================

/// Maps a builder's type-level relation list to a `RelEntry` storage chain.
pub trait BuildStore {
    /// The concrete storage type for this relation configuration.
    type Store;
}

impl BuildStore for () {
    type Store = ();
}

impl<V: SQLParam, R, Nested, Rest, Cols, Cl> BuildStore
    for (RelationHandle<V, R, Nested, Cols, Cl>, Rest)
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
}
