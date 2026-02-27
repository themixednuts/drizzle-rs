//! `RelationHandle` — builder for configuring a single relation's loading.

use core::marker::PhantomData;

use crate::SQLParam;
use crate::relation::RelationDef;

use super::builder::{AllColumns, IntoColumnSelection, PartialColumns, QueryTable};

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
pub struct RelationHandle<V: SQLParam, R: RelationDef, Nested = (), Cols = AllColumns> {
    pub(crate) where_clause: String,
    pub(crate) where_params: Vec<V>,
    pub(crate) order_by_clause: String,
    pub(crate) limit: Option<u32>,
    pub(crate) offset: Option<u32>,
    pub(crate) nested: Nested,
    pub(crate) cols: Cols,
    pub(crate) _marker: PhantomData<R>,
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

impl<V: SQLParam, R: RelationDef, Nested, Cols> RelationHandle<V, R, Nested, Cols> {
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

    /// Sets a LIMIT on the relation subquery.
    pub fn limit(mut self, n: u32) -> Self {
        self.limit = Some(n);
        self
    }

    /// Sets an OFFSET on the relation subquery.
    pub fn offset(mut self, n: u32) -> Self {
        self.offset = Some(n);
        self
    }

    /// Nests a relation on the target table.
    #[allow(clippy::type_complexity)]
    pub fn with<NR, NN, NC>(
        self,
        handle: RelationHandle<V, NR, NN, NC>,
    ) -> RelationHandle<V, R, (RelationHandle<V, NR, NN, NC>, Nested), Cols>
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

/// Methods only available when all columns are selected (prevents double-calling).
impl<V: SQLParam, R: RelationDef, Nested> RelationHandle<V, R, Nested, AllColumns>
where
    R::Target: QueryTable,
{
    /// Selects only the specified columns on this relation (whitelist).
    pub fn columns<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> RelationHandle<V, R, Nested, PartialColumns> {
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

    /// Excludes the specified columns on this relation (blacklist).
    pub fn omit<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> RelationHandle<V, R, Nested, PartialColumns> {
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
unsafe impl<V: SQLParam, R: RelationDef, Nested: Send, Cols: Send> Send
    for RelationHandle<V, R, Nested, Cols>
{
}
unsafe impl<V: SQLParam, R: RelationDef, Nested: Sync, Cols: Sync> Sync
    for RelationHandle<V, R, Nested, Cols>
{
}
