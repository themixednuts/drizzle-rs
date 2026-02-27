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
/// The driver layer wraps this with a connection reference for execution.
pub struct QueryBuilder<V: SQLParam, T, Rels = (), Cols = AllColumns> {
    pub where_clause: String,
    pub where_params: Vec<V>,
    pub order_by_clause: String,
    pub limit: Option<u32>,
    pub offset: Option<u32>,
    pub relations: Rels,
    pub cols: Cols,
    _marker: PhantomData<T>,
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

impl<V: SQLParam, T, Rels, Cols> QueryBuilder<V, T, Rels, Cols> {
    /// Adds a relation to be loaded.
    #[allow(clippy::type_complexity)]
    pub fn with<R, N, C>(
        self,
        handle: RelationHandle<V, R, N, C>,
    ) -> QueryBuilder<V, T, (RelationHandle<V, R, N, C>, Rels), Cols>
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

    /// Adds a typed WHERE clause.
    pub fn r#where<'a, E>(mut self, condition: E) -> Self
    where
        E: crate::expr::Expr<'a, V>,
        E::SQLType: crate::types::BooleanLike,
        V: 'a,
    {
        let sql = condition.to_sql();
        let (text, params) = sql.build();
        self.where_clause = text;
        self.where_params = params.into_iter().cloned().collect();
        self
    }

    /// Adds a typed ORDER BY clause.
    pub fn order_by<'a, E>(mut self, expr: E) -> Self
    where
        E: crate::traits::ToSQL<'a, V>,
        V: 'a,
    {
        let sql = expr.to_sql();
        let (text, _) = sql.build();
        self.order_by_clause = text;
        self
    }

    /// Sets LIMIT.
    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }

    /// Sets OFFSET.
    pub fn offset(mut self, n: u32) -> Self {
        self.offset = Some(n);
        self
    }
}

/// Methods only available when all columns are selected (prevents double-calling).
impl<V: SQLParam, T: QueryTable, Rels> QueryBuilder<V, T, Rels, AllColumns> {
    /// Selects only the specified columns (whitelist).
    ///
    /// The result model becomes `T::PartialSelect` with only selected columns populated.
    pub fn columns<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> QueryBuilder<V, T, Rels, PartialColumns> {
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

    /// Excludes the specified columns (blacklist).
    ///
    /// The result model becomes `T::PartialSelect` with all columns except the omitted ones.
    pub fn omit<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> QueryBuilder<V, T, Rels, PartialColumns> {
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

impl<V: SQLParam, R, Nested, Rest, Cols> BuildStore for (RelationHandle<V, R, Nested, Cols>, Rest)
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
