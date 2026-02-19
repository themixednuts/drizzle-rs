use crate::common::PostgresSchemaType;
use crate::helpers;
use crate::traits::PostgresTable;
use crate::values::PostgresValue;
use core::fmt::Debug;
use core::marker::PhantomData;
use drizzle_core::ToSQL;
use drizzle_core::traits::SQLTable;
use paste::paste;

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of SelectBuilder.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectInitial;

/// Marker for the state after FROM clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectFromSet;

/// Marker for the state after JOIN clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectJoinSet;

/// Marker for the state after WHERE clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectWhereSet;

/// Marker for the state after GROUP BY clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectGroupSet;

/// Marker for the state after ORDER BY clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectOrderSet;

/// Marker for the state after LIMIT clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectLimitSet;

/// Marker for the state after OFFSET clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectOffsetSet;

/// Marker for the state after set operations (UNION/INTERSECT/EXCEPT)
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectSetOpSet;

/// Marker for the state after FOR UPDATE/SHARE clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectForSet;

#[doc(hidden)]
macro_rules! join_impl {
    () => {
        join_impl!(natural, Join::new().natural());
        join_impl!(natural_left, Join::new().natural().left());
        join_impl!(left, Join::new().left());
        join_impl!(left_outer, Join::new().left().outer());
        join_impl!(natural_left_outer, Join::new().natural().left().outer());
        join_impl!(natural_right, Join::new().natural().right());
        join_impl!(right, Join::new().right());
        join_impl!(right_outer, Join::new().right().outer());
        join_impl!(natural_right_outer, Join::new().natural().right().outer());
        join_impl!(natural_full, Join::new().natural().full());
        join_impl!(full, Join::new().full());
        join_impl!(full_outer, Join::new().full().outer());
        join_impl!(natural_full_outer, Join::new().natural().full().outer());
        join_impl!(inner, Join::new().inner());
        join_impl!(cross, Join::new().cross());

        // USING variants only for non-natural, non-cross joins
        join_using_impl!(left);
        join_using_impl!(left_outer);
        join_using_impl!(right);
        join_using_impl!(right_outer);
        join_using_impl!(full);
        join_using_impl!(full_outer);
        join_using_impl!(inner);
        join_using_impl!(); // Plain JOIN
    };
    ($type:ident, $join_expr:expr) => {
        paste! {
            /// JOIN with ON clause
            pub fn [<$type _join>]<J: crate::helpers::JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> SelectBuilder<'a, S, SelectJoinSet, J::JoinedTable, <M as drizzle_core::ScopePush<J::JoinedTable>>::Out, <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow>
            where
                M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
            {
                use drizzle_core::Join;
                SelectBuilder {
                    sql: self.sql.append(arg.into_join_sql($join_expr)),
                    schema: PhantomData,
                    state: PhantomData,
                    table: PhantomData,
                    marker: PhantomData,
                    row: PhantomData,
                }
            }
        }
    };
}

macro_rules! join_using_impl {
    () => {
        /// JOIN with USING clause (PostgreSQL-specific)
        pub fn join_using<U: PostgresTable<'a>>(
            self,
            table: U,
            columns: impl ToSQL<'a, PostgresValue<'a>>,
        ) -> SelectBuilder<
            'a,
            S,
            SelectJoinSet,
            U,
            <M as drizzle_core::ScopePush<U>>::Out,
            <M as drizzle_core::AfterJoin<R, U>>::NewRow,
        >
        where
            M: drizzle_core::AfterJoin<R, U> + drizzle_core::ScopePush<U>,
        {
            SelectBuilder {
                sql: self.sql.append(helpers::join_using(table, columns)),
                schema: PhantomData,
                state: PhantomData,
                table: PhantomData,
                marker: PhantomData,
                row: PhantomData,
            }
        }
    };
    ($type:ident) => {
        paste! {
            /// JOIN with USING clause (PostgreSQL-specific)
            pub fn [<$type _join_using>]<U: PostgresTable<'a>>(
                self,
                table: U,
                columns: impl ToSQL<'a, PostgresValue<'a>>,
            ) -> SelectBuilder<
                'a,
                S,
                SelectJoinSet,
                U,
                <M as drizzle_core::ScopePush<U>>::Out,
                <M as drizzle_core::AfterJoin<R, U>>::NewRow,
            >
            where
                M: drizzle_core::AfterJoin<R, U> + drizzle_core::ScopePush<U>,
            {
                SelectBuilder {
                    sql: self.sql.append(helpers::[<$type _join_using>](table, columns)),
                    schema: PhantomData,
                    state: PhantomData,
                    table: PhantomData,
                    marker: PhantomData,
                    row: PhantomData,
                }
            }
        }
    };
}

// Mark states that can execute queries as implementing the ExecutableState trait
impl ExecutableState for SelectFromSet {}
impl ExecutableState for SelectWhereSet {}
impl ExecutableState for SelectLimitSet {}
impl ExecutableState for SelectOffsetSet {}
impl ExecutableState for SelectOrderSet {}
impl ExecutableState for SelectGroupSet {}
impl ExecutableState for SelectJoinSet {}
impl ExecutableState for SelectSetOpSet {}
impl ExecutableState for SelectForSet {}

#[doc(hidden)]
pub trait AsCteState {}

impl AsCteState for SelectFromSet {}
impl AsCteState for SelectJoinSet {}
impl AsCteState for SelectWhereSet {}
impl AsCteState for SelectGroupSet {}
impl AsCteState for SelectOrderSet {}
impl AsCteState for SelectLimitSet {}
impl AsCteState for SelectOffsetSet {}

//------------------------------------------------------------------------------
// SelectBuilder Definition
//------------------------------------------------------------------------------

/// Builds a SELECT query specifically for PostgreSQL
pub type SelectBuilder<'a, Schema, State, Table = (), Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S, M> SelectBuilder<'a, S, SelectInitial, (), M> {
    /// Specifies the table to select FROM and transitions state.
    ///
    /// The row type `R` is resolved from the select marker `M` and the table `T`
    /// via the `ResolveRow` trait.
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn from<T>(
        self,
        query: T,
    ) -> SelectBuilder<
        'a,
        S,
        SelectFromSet,
        T,
        drizzle_core::Scoped<M, drizzle_core::Cons<T, drizzle_core::Nil>>,
        <M as drizzle_core::ResolveRow<T>>::Row,
    >
    where
        T: ToSQL<'a, PostgresValue<'a>>,
        M: drizzle_core::ResolveRow<T>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::from(query)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-FROM State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectFromSet, T, M, R> {
    /// Adds a JOIN clause to the query
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn join<J: crate::helpers::JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        J::JoinedTable,
        <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
        <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
    >
    where
        M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
    {
        use drizzle_core::Join;
        SelectBuilder {
            sql: self.sql.append(arg.into_join_sql(Join::new())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    join_impl!();

    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectWhereSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::r#where(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds a GROUP BY clause to the query
    pub fn group_by(
        self,
        expressions: impl IntoIterator<Item = impl ToSQL<'a, PostgresValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Limits the number of rows returned
    #[inline]
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Sets the offset for the query results
    #[inline]
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Sorts the query results
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-JOIN State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectJoinSet, T, M, R> {
    /// Adds a WHERE condition after a JOIN
    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectWhereSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(crate::helpers::r#where(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
    /// Sorts the query results
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
    /// Adds a JOIN clause to the query
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn join<J: crate::helpers::JoinArg<'a, T>>(
        self,
        arg: J,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        J::JoinedTable,
        <M as drizzle_core::ScopePush<J::JoinedTable>>::Out,
        <M as drizzle_core::AfterJoin<R, J::JoinedTable>>::NewRow,
    >
    where
        M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
    {
        use drizzle_core::Join;
        SelectBuilder {
            sql: self.sql.append(arg.into_join_sql(Join::new())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
    join_impl!();
}

//------------------------------------------------------------------------------
// Post-WHERE State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectWhereSet, T, M, R> {
    /// Adds a GROUP BY clause after a WHERE
    pub fn group_by(
        self,
        expressions: impl IntoIterator<Item = impl ToSQL<'a, PostgresValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds an ORDER BY clause after a WHERE
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds a LIMIT clause after a WHERE
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-GROUP BY State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectGroupSet, T, M, R> {
    /// Adds a HAVING clause after GROUP BY
    pub fn having(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::having(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds an ORDER BY clause after GROUP BY
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-ORDER BY State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectOrderSet, T, M, R> {
    /// Adds a LIMIT clause after ORDER BY
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-LIMIT State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectLimitSet, T, M, R> {
    /// Adds an OFFSET clause after LIMIT
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

impl<'a, S, State, T, M, R> SelectBuilder<'a, S, State, T, M, R>
where
    State: AsCteState,
    T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
    <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased:
        drizzle_core::TaggableAlias,
{
    /// Converts this SELECT query into a typed CTE using alias tag name.
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag>(
        self,
    ) -> super::CTEView<
        'a,
        <<T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased as drizzle_core::TaggableAlias>::Tagged<Tag>,
        Self,
    >{
        let name = Tag::NAME;
        super::CTEView::new(
            <<T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased as drizzle_core::TaggableAlias>::tag::<Tag>(
                <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::alias_named(name),
            ),
            name,
            self,
        )
    }
}

//------------------------------------------------------------------------------
// Set operation support (UNION / INTERSECT / EXCEPT)
//------------------------------------------------------------------------------

impl<'a, S, State, T, M, R> SelectBuilder<'a, S, State, T, M, R>
where
    State: ExecutableState,
{
    /// Combines this query with another using UNION.
    pub fn union(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R> {
        SelectBuilder {
            sql: helpers::union(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Combines this query with another using UNION ALL.
    pub fn union_all(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R> {
        SelectBuilder {
            sql: helpers::union_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Combines this query with another using INTERSECT.
    pub fn intersect(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R> {
        SelectBuilder {
            sql: helpers::intersect(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Combines this query with another using INTERSECT ALL.
    pub fn intersect_all(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R> {
        SelectBuilder {
            sql: helpers::intersect_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Combines this query with another using EXCEPT.
    pub fn except(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R> {
        SelectBuilder {
            sql: helpers::except(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Combines this query with another using EXCEPT ALL.
    pub fn except_all(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R> {
        SelectBuilder {
            sql: helpers::except_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectSetOpSet, T, M, R> {
    /// Sorts the results of a set operation.
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Limits the results of a set operation.
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Offsets the results of a set operation.
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// FOR UPDATE/SHARE Row Locking (PostgreSQL-specific)
//------------------------------------------------------------------------------

/// Trait for states that can have FOR UPDATE/SHARE clauses applied.
/// This is a subset of executable states - specifically those after filtering/ordering.
pub trait ForLockableState {}

impl ForLockableState for SelectFromSet {}
impl ForLockableState for SelectWhereSet {}
impl ForLockableState for SelectOrderSet {}
impl ForLockableState for SelectLimitSet {}
impl ForLockableState for SelectOffsetSet {}
impl ForLockableState for SelectJoinSet {}
impl ForLockableState for SelectGroupSet {}

impl<'a, S, State, T, M, R> SelectBuilder<'a, S, State, T, M, R>
where
    State: ForLockableState,
{
    /// Adds FOR UPDATE clause to lock selected rows for update.
    ///
    /// This prevents other transactions from modifying or locking the selected rows
    /// until the current transaction ends.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.select(()).from(users).where(eq(users.id, 1)).for_update()
    /// // SELECT ... FROM "users" WHERE "id" = $1 FOR UPDATE
    /// ```
    pub fn for_update(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_update()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds FOR SHARE clause to lock selected rows for shared access.
    ///
    /// This allows other transactions to read the rows but prevents them from
    /// modifying or exclusively locking them.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.select(()).from(users).where(eq(users.id, 1)).for_share()
    /// // SELECT ... FROM "users" WHERE "id" = $1 FOR SHARE
    /// ```
    pub fn for_share(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_share()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds FOR NO KEY UPDATE clause.
    ///
    /// Similar to FOR UPDATE but weaker - allows SELECT FOR KEY SHARE on the same rows.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.select(()).from(users).where(eq(users.id, 1)).for_no_key_update()
    /// // SELECT ... FROM "users" WHERE "id" = $1 FOR NO KEY UPDATE
    /// ```
    pub fn for_no_key_update(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_no_key_update()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds FOR KEY SHARE clause.
    ///
    /// The weakest lock - only blocks SELECT FOR UPDATE.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.select(()).from(users).where(eq(users.id, 1)).for_key_share()
    /// // SELECT ... FROM "users" WHERE "id" = $1 FOR KEY SHARE
    /// ```
    pub fn for_key_share(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_key_share()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds FOR UPDATE OF table clause to lock only rows from a specific table.
    ///
    /// Useful in joins to specify which table's rows should be locked.
    /// Note: Uses unqualified table name per PostgreSQL requirements.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.select(())
    ///     .from(users)
    ///     .join((orders, eq(users.id, orders.user_id)))
    ///     .for_update_of(users)
    /// // SELECT ... FOR UPDATE OF "users"
    /// ```
    pub fn for_update_of<U: PostgresTable<'a>>(
        self,
        table: U,
    ) -> SelectBuilder<'a, S, SelectForSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_update_of(table.name())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds FOR SHARE OF table clause to lock only rows from a specific table.
    ///
    /// Useful in joins to specify which table's rows should be locked.
    /// Note: Uses unqualified table name per PostgreSQL requirements.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.select(())
    ///     .from(users)
    ///     .join((orders, eq(users.id, orders.user_id)))
    ///     .for_share_of(users)
    /// // SELECT ... FOR SHARE OF "users"
    /// ```
    pub fn for_share_of<U: PostgresTable<'a>>(
        self,
        table: U,
    ) -> SelectBuilder<'a, S, SelectForSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_share_of(table.name())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-FOR State Implementation (NOWAIT / SKIP LOCKED)
//------------------------------------------------------------------------------

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectForSet, T, M, R> {
    /// Adds NOWAIT option to fail immediately if rows are locked.
    ///
    /// Instead of waiting for locked rows to become available, the query
    /// will fail with an error if any selected rows are currently locked.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.select(()).from(users).for_update().nowait()
    /// // SELECT ... FOR UPDATE NOWAIT
    /// ```
    pub fn nowait(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::nowait()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds SKIP LOCKED option to skip over locked rows.
    ///
    /// Instead of waiting for locked rows, the query will skip them and
    /// only return/lock rows that are currently available.
    ///
    /// # Example
    ///
    /// ```ignore
    /// db.select(()).from(jobs).where(eq(jobs.status, "pending")).for_update().skip_locked()
    /// // SELECT ... FOR UPDATE SKIP LOCKED
    /// ```
    pub fn skip_locked(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R> {
        SelectBuilder {
            sql: self.sql.append(helpers::skip_locked()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{SQL, ToSQL};

    #[test]
    fn test_select_builder_creation() {
        let builder = SelectBuilder::<(), SelectInitial> {
            sql: SQL::raw("SELECT *"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "SELECT *");
    }
}
