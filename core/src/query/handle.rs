//! `RelationHandle` — builder for configuring a single relation's loading.

use core::marker::PhantomData;

use crate::SQLParam;
use crate::relation::RelationDef;

use super::builder::{
    AllColumns, Clauses, HasLimit, HasOffset, HasOrderBy, HasWhere, IntoColumnSelection, NoLimit,
    NoOrderBy, NoWhere, PartialColumns, QueryTable,
};

/// A builder for configuring how a single relation is loaded.
///
/// Created by the table ZST accessor methods (e.g., `user.posts()`).
/// Supports WHERE, ORDER BY, LIMIT, OFFSET, and nested `.with()`.
///
/// The `Nested` type parameter is the actual storage for nested relation
/// handles — `()` when empty, `(RelationHandle<V, NR, NN, NC>, Rest)` when
/// populated. This means the full relation tree is preserved in the type
/// system, not erased to a runtime Vec.
///
/// The `Cols` type parameter controls column selection for the target table —
/// `AllColumns` (default) selects all columns, `PartialColumns` selects a subset.
///
/// The `Cl` type parameter is a [`Clauses`] composite tracking which query
/// clauses have been set (WHERE, ORDER BY, LIMIT/OFFSET). Each clause can
/// only be set once — the typestate prevents double-calling at compile time.
pub struct RelationHandle<V: SQLParam, R: RelationDef, Nested = (), Cols = AllColumns, Cl = Clauses>
{
    pub(crate) where_clause: String,
    pub(crate) where_params: Vec<V>,
    pub(crate) order_by_clause: String,
    pub(crate) limit: Option<u32>,
    pub(crate) offset: Option<u32>,
    pub(crate) nested: Nested,
    pub(crate) cols: Cols,
    pub(crate) _marker: PhantomData<(R, Cl)>,
}

impl<V: SQLParam, R: RelationDef> RelationHandle<V, R> {
    /// Creates a new unconfigured `RelationHandle`.
    pub fn new() -> Self {
        Self {
            where_clause: String::new(),
            where_params: Vec::new(),
            order_by_clause: String::new(),
            limit: None,
            offset: None,
            nested: (),
            cols: AllColumns,
            _marker: PhantomData,
        }
    }
}

impl<V: SQLParam, R: RelationDef> Default for RelationHandle<V, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<V: SQLParam, R: RelationDef, Nested, Cols, Cl> RelationHandle<V, R, Nested, Cols, Cl> {
    /// Nests a relation on the target table.
    #[allow(clippy::type_complexity)]
    pub fn with<NR, NN, NC, NCl>(
        self,
        handle: RelationHandle<V, NR, NN, NC, NCl>,
    ) -> RelationHandle<V, R, (RelationHandle<V, NR, NN, NC, NCl>, Nested), Cols, Cl>
    where
        NR: RelationDef<Source = R::Target> + 'static,
    {
        RelationHandle {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: self.offset,
            nested: (handle, self.nested),
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// WHERE is only available when no WHERE clause has been set yet.
impl<V: SQLParam, R: RelationDef, Nested, Cols, Ord, Lim>
    RelationHandle<V, R, Nested, Cols, Clauses<NoWhere, Ord, Lim>>
{
    /// Sets the WHERE clause for the relation subquery.
    ///
    /// Can only be called once. To combine multiple conditions, use boolean
    /// operators: `and(cond_a, cond_b)` or `or(cond_a, cond_b)`.
    pub fn r#where<'a, E>(
        self,
        condition: E,
    ) -> RelationHandle<V, R, Nested, Cols, Clauses<HasWhere, Ord, Lim>>
    where
        E: crate::expr::Expr<'a, V>,
        E::SQLType: crate::types::BooleanLike,
        V: 'a,
    {
        let sql = condition.to_sql();
        let (text, params) = sql.build();
        RelationHandle {
            where_clause: text,
            where_params: params.into_iter().cloned().collect(),
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: self.offset,
            nested: self.nested,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// ORDER BY is only available when no ORDER BY clause has been set yet.
impl<V: SQLParam, R: RelationDef, Nested, Cols, W, Lim>
    RelationHandle<V, R, Nested, Cols, Clauses<W, NoOrderBy, Lim>>
{
    /// Adds a typed ORDER BY clause to the relation subquery.
    ///
    /// Can only be called once. ORDER BY expressions are column references
    /// (e.g., `asc(col)`, `desc(col)`), which never produce bind parameters.
    pub fn order_by<'a, E>(
        self,
        expr: E,
    ) -> RelationHandle<V, R, Nested, Cols, Clauses<W, HasOrderBy, Lim>>
    where
        E: crate::traits::ToSQL<'a, V>,
        V: 'a,
    {
        let sql = expr.to_sql();
        let (text, _) = sql.build();
        RelationHandle {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: text,
            limit: self.limit,
            offset: self.offset,
            nested: self.nested,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// LIMIT is only available when no LIMIT has been set yet.
impl<V: SQLParam, R: RelationDef, Nested, Cols, W, Ord>
    RelationHandle<V, R, Nested, Cols, Clauses<W, Ord, NoLimit>>
{
    /// Sets a LIMIT on the relation subquery.
    ///
    /// Can only be called once. Enables calling `.offset()`.
    pub fn limit(self, n: u32) -> RelationHandle<V, R, Nested, Cols, Clauses<W, Ord, HasLimit>> {
        RelationHandle {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: Some(n),
            offset: self.offset,
            nested: self.nested,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// OFFSET requires LIMIT to have been set first.
impl<V: SQLParam, R: RelationDef, Nested, Cols, W, Ord>
    RelationHandle<V, R, Nested, Cols, Clauses<W, Ord, HasLimit>>
{
    /// Sets an OFFSET on the relation subquery. Requires `.limit()` first.
    pub fn offset(self, n: u32) -> RelationHandle<V, R, Nested, Cols, Clauses<W, Ord, HasOffset>> {
        RelationHandle {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: Some(n),
            nested: self.nested,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// Methods only available when all columns are selected (prevents double-calling).
impl<V: SQLParam, R: RelationDef, Nested, Cl> RelationHandle<V, R, Nested, AllColumns, Cl>
where
    R::Target: QueryTable,
{
    /// Selects only the specified columns on this relation (include list).
    pub fn columns<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> RelationHandle<V, R, Nested, PartialColumns, Cl> {
        RelationHandle {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: self.offset,
            nested: self.nested,
            cols: PartialColumns {
                columns: selector.into_column_names(),
            },
            _marker: PhantomData,
        }
    }

    /// Excludes the specified columns on this relation (exclude list).
    pub fn omit<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> RelationHandle<V, R, Nested, PartialColumns, Cl> {
        let omitted = selector.into_column_names();
        let columns = <R::Target as QueryTable>::COLUMN_NAMES
            .iter()
            .copied()
            .filter(|c| !omitted.contains(c))
            .collect();
        RelationHandle {
            where_clause: self.where_clause,
            where_params: self.where_params,
            order_by_clause: self.order_by_clause,
            limit: self.limit,
            offset: self.offset,
            nested: self.nested,
            cols: PartialColumns { columns },
            _marker: PhantomData,
        }
    }
}

// SAFETY: R is a ZST marker (PhantomData only). String/Vec<V>/Option<u32> are
// Send+Sync when V is. Nested consists of more RelationHandles (recursively safe).
// Cols is AllColumns (ZST) or PartialColumns (Vec<&'static str> — Send+Sync).
// Cl is Clauses<W, Ord, Lim> — ZST markers, always Send+Sync.
unsafe impl<V: SQLParam, R: RelationDef, Nested: Send, Cols: Send, Cl: Send> Send
    for RelationHandle<V, R, Nested, Cols, Cl>
{
}
unsafe impl<V: SQLParam, R: RelationDef, Nested: Sync, Cols: Sync, Cl: Sync> Sync
    for RelationHandle<V, R, Nested, Cols, Cl>
{
}
