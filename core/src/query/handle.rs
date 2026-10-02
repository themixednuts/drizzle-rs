//! [`RelationHandle`]: settings for loading one relation.

use core::marker::PhantomData;

use crate::relation::RelationDef;
use crate::{PaginationArg, SQL, SQLParam};

use super::builder::{
    AllColumns, Clauses, HasLimit, HasOffset, HasOrderBy, HasWhere, IntoColumnSelection, NoLimit,
    NoOrderBy, NoWhere, PartialColumns, QueryTable,
};

/// Settings for loading one relation: filters, order, limits, columns, and
/// nested relations.
///
/// Created by the relation methods the table macros generate (for example
/// `users.posts()`) and passed to `.with(...)`.
///
/// - `Nested` holds nested relation handles: `()` when empty,
///   `(RelationHandle<...>, Rest)` otherwise, so the whole tree is part of
///   the type.
/// - `Cols` is [`AllColumns`] (default) or [`PartialColumns`].
/// - `Cl` is a [`Clauses`] value recording which of WHERE, ORDER BY,
///   LIMIT and OFFSET are set. Each can be set once.
///
/// WHERE and ORDER BY may only read columns of the relation's target table.
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
    /// Creates a handle with no settings.
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
    /// Also loads the relation `handle` of the target table, nested inside
    /// each loaded row.
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
    /// Sets the WHERE clause for the loaded rows.
    ///
    /// Can only be called once; combine conditions with `and(...)` or
    /// `or(...)`. The condition must be boolean and may only read columns of
    /// the target table.
    pub fn r#where<E, ScopeProof>(
        self,
        condition: E,
    ) -> RelationHandle<'a, V, R, Nested, Cols, Clauses<HasWhere, Ord, Lim>>
    where
        E: crate::expr::Expr<'a, V>,
        E::Sources: crate::scope::SourcesIn<crate::Cons<R::Target, crate::Nil>, ScopeProof>,
        E::SQLType: crate::types::BooleanLike,
        V: 'a,
    {
        RelationHandle {
            where_sql: condition.into_expr_sql(),
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
    /// Sets the ORDER BY clause for the loaded rows, such as `asc(col)`.
    ///
    /// Can only be called once. The expressions may only read columns of the
    /// target table.
    pub fn order_by<E, ScopeProof>(
        self,
        expr: E,
    ) -> RelationHandle<'a, V, R, Nested, Cols, Clauses<W, HasOrderBy, Lim>>
    where
        E: crate::traits::ToSQL<'a, V> + crate::expr::ExprSources,
        E::Sources: crate::scope::SourcesIn<crate::Cons<R::Target, crate::Nil>, ScopeProof>,
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
    /// Sets the LIMIT for the loaded rows. Can only be called once, and must
    /// come before `.offset(...)`.
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

    /// Same as `.limit(1)`: loads at most one row.
    ///
    /// The field keeps its type, so a many-relation is still a `Vec` (with at
    /// most one element).
    pub fn first(self) -> RelationHandle<'a, V, R, Nested, Cols, Clauses<W, Ord, HasLimit>> {
        self.limit(1u32)
    }
}

/// OFFSET requires LIMIT to have been set first.
impl<'a, V: SQLParam, R: RelationDef, Nested, Cols, W, Ord>
    RelationHandle<'a, V, R, Nested, Cols, Clauses<W, Ord, HasLimit>>
{
    /// Sets the OFFSET for the loaded rows. Only available after
    /// `.limit(...)`.
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

// Column selection can only be chosen once.
impl<'a, V: SQLParam, R: RelationDef, Nested, Cl> RelationHandle<'a, V, R, Nested, AllColumns, Cl>
where
    R::Target: QueryTable,
{
    /// Loads only the given columns of the target table.
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

    /// Loads every column of the target table except the given ones.
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
