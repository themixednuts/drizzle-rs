//! `RelationHandle` — builder for configuring a single relation's loading.

use core::marker::PhantomData;

use crate::relation::RelationDef;
use crate::{PaginationArg, SQL, SQLParam};

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
/// handles — `()` when empty, `(RelationHandle<'a, V, NR, NN, NC>, Rest)` when
/// populated. This means the full relation tree is preserved in the type
/// system, not erased to a runtime Vec.
///
/// The `Cols` type parameter controls column selection for the target table —
/// `AllColumns` (default) selects all columns, `PartialColumns` selects a subset.
///
/// The `Cl` type parameter is a [`Clauses`] composite tracking which query
/// clauses have been set (WHERE, ORDER BY, LIMIT/OFFSET). Each clause can
/// only be set once — the typestate prevents double-calling at compile time.
pub struct RelationHandle<
    'a,
    V: SQLParam,
    R: RelationDef,
    Nested = (),
    Cols = AllColumns,
    Cl = Clauses,
> {
    pub(crate) where_sql: SQL<'a, V>,
    pub(crate) order_by_sql: SQL<'a, V>,
    pub(crate) limit: Option<SQL<'a, V>>,
    pub(crate) offset: Option<SQL<'a, V>>,
    pub(crate) nested: Nested,
    pub(crate) cols: Cols,
    pub(crate) _marker: PhantomData<(R, Cl)>,
}

impl<'a, V: SQLParam, R: RelationDef> RelationHandle<'a, V, R> {
    /// Creates a new unconfigured `RelationHandle`.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            where_sql: SQL::empty(),
            order_by_sql: SQL::empty(),
            limit: None,
            offset: None,
            nested: (),
            cols: AllColumns,
            _marker: PhantomData,
        }
    }
}

impl<'a, V: SQLParam, R: RelationDef> Default for RelationHandle<'a, V, R> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a, V: SQLParam, R: RelationDef, Nested, Cols, Cl> RelationHandle<'a, V, R, Nested, Cols, Cl> {
    /// Nests a relation on the target table.
    #[allow(clippy::type_complexity)]
    pub fn with<NR, NN, NC, NCl>(
        self,
        handle: RelationHandle<'a, V, NR, NN, NC, NCl>,
    ) -> RelationHandle<'a, V, R, (RelationHandle<'a, V, NR, NN, NC, NCl>, Nested), Cols, Cl>
    where
        NR: RelationDef<Source = R::Target> + 'static,
    {
        RelationHandle {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: self.offset,
            nested: (handle, self.nested),
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// WHERE is only available when no WHERE clause has been set yet.
impl<'a, V: SQLParam, R: RelationDef, Nested, Cols, Ord, Lim>
    RelationHandle<'a, V, R, Nested, Cols, Clauses<NoWhere, Ord, Lim>>
{
    /// Sets the WHERE clause for the relation subquery.
    ///
    /// Can only be called once. To combine multiple conditions, use boolean
    /// operators: `and(cond_a, cond_b)` or `or(cond_a, cond_b)`.
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> RelationHandle<'a, V, R, Nested, Cols, Clauses<HasWhere, Ord, Lim>>
    where
        E: crate::expr::Expr<'a, V>,
        E::SQLType: crate::types::BooleanLike,
        V: 'a,
    {
        RelationHandle {
            where_sql: condition.to_sql(),
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: self.offset,
            nested: self.nested,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// ORDER BY is only available when no ORDER BY clause has been set yet.
impl<'a, V: SQLParam, R: RelationDef, Nested, Cols, W, Lim>
    RelationHandle<'a, V, R, Nested, Cols, Clauses<W, NoOrderBy, Lim>>
{
    /// Adds a typed ORDER BY clause to the relation subquery.
    ///
    /// Can only be called once. ORDER BY expressions are column references
    /// (e.g., `asc(col)`, `desc(col)`), which never produce bind parameters.
    pub fn order_by<E>(
        self,
        expr: E,
    ) -> RelationHandle<'a, V, R, Nested, Cols, Clauses<W, HasOrderBy, Lim>>
    where
        E: crate::traits::ToSQL<'a, V>,
        V: 'a,
    {
        RelationHandle {
            where_sql: self.where_sql,
            order_by_sql: expr.to_sql(),
            limit: self.limit,
            offset: self.offset,
            nested: self.nested,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// LIMIT is only available when no LIMIT has been set yet.
impl<'a, V: SQLParam, R: RelationDef, Nested, Cols, W, Ord>
    RelationHandle<'a, V, R, Nested, Cols, Clauses<W, Ord, NoLimit>>
{
    /// Sets a LIMIT on the relation subquery.
    ///
    /// Can only be called once. Enables calling `.offset()`.
    pub fn limit<P>(self, n: P) -> RelationHandle<'a, V, R, Nested, Cols, Clauses<W, Ord, HasLimit>>
    where
        P: PaginationArg<'a, V>,
        V: 'a,
    {
        RelationHandle {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: Some(n.into_pagination_sql()),
            offset: self.offset,
            nested: self.nested,
            cols: self.cols,
            _marker: PhantomData,
        }
    }

    /// Sugar for `.limit(1)`. Limits the relation subquery to at most one row.
    ///
    /// The result is still `Vec<T>` — call `Vec::first()` on the result to
    /// get `Option<&T>`.
    pub fn first(self) -> RelationHandle<'a, V, R, Nested, Cols, Clauses<W, Ord, HasLimit>> {
        self.limit(1u32)
    }
}

/// OFFSET requires LIMIT to have been set first.
impl<'a, V: SQLParam, R: RelationDef, Nested, Cols, W, Ord>
    RelationHandle<'a, V, R, Nested, Cols, Clauses<W, Ord, HasLimit>>
{
    /// Sets an OFFSET on the relation subquery. Requires `.limit()` first.
    pub fn offset<P>(
        self,
        n: P,
    ) -> RelationHandle<'a, V, R, Nested, Cols, Clauses<W, Ord, HasOffset>>
    where
        P: PaginationArg<'a, V>,
        V: 'a,
    {
        RelationHandle {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: Some(n.into_pagination_sql()),
            nested: self.nested,
            cols: self.cols,
            _marker: PhantomData,
        }
    }
}

/// Methods only available when all columns are selected (prevents double-calling).
impl<'a, V: SQLParam, R: RelationDef, Nested, Cl> RelationHandle<'a, V, R, Nested, AllColumns, Cl>
where
    R::Target: QueryTable,
{
    /// Selects only the specified columns on this relation (include list).
    pub fn columns<S: IntoColumnSelection>(
        self,
        selector: S,
    ) -> RelationHandle<'a, V, R, Nested, PartialColumns, Cl> {
        RelationHandle {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
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
    ) -> RelationHandle<'a, V, R, Nested, PartialColumns, Cl> {
        let omitted = selector.into_column_names();
        let columns = <R::Target as QueryTable>::COLUMN_NAMES
            .iter()
            .copied()
            .filter(|c| !omitted.contains(c))
            .collect();
        RelationHandle {
            where_sql: self.where_sql,
            order_by_sql: self.order_by_sql,
            limit: self.limit,
            offset: self.offset,
            nested: self.nested,
            cols: PartialColumns { columns },
            _marker: PhantomData,
        }
    }
}
