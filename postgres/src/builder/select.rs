use crate::common::PostgresSchemaType;
use crate::helpers;
use crate::traits::PostgresTable;
use crate::values::PostgresValue;
use drizzle_core::traits::SQLTable;
use drizzle_core::ToSQL;
use paste::paste;
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of SelectBuilder.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectInitial;

impl SelectInitial {
    /// Creates a new SelectInitial marker
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

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

// Const constructors for all marker types
impl SelectFromSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectJoinSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectWhereSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectGroupSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectOrderSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectLimitSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectOffsetSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectSetOpSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectForSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

#[doc(hidden)]
macro_rules! join_impl {
    () => {
        join_impl!(natural);
        join_impl!(natural_left);
        join_impl!(left);
        join_impl!(left_outer);
        join_impl!(natural_left_outer);
        join_impl!(natural_right);
        join_impl!(right);
        join_impl!(right_outer);
        join_impl!(natural_right_outer);
        join_impl!(natural_full);
        join_impl!(full);
        join_impl!(full_outer);
        join_impl!(natural_full_outer);
        join_impl!(inner);
        join_impl!(cross);

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
    ($type:ident) => {
        paste! {
            /// JOIN with ON clause
            pub fn [<$type _join>]<U:  PostgresTable<'a>>(
                self,
                table: U,
                condition: impl ToSQL<'a, PostgresValue<'a>>,
            ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
                SelectBuilder {
                    sql: self.sql.append(helpers::[<$type _join>](table, condition)),
                    schema: PhantomData,
                    state: PhantomData,
                    table: PhantomData,
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
        ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
            SelectBuilder {
                sql: self.sql.append(helpers::join_using(table, columns)),
                schema: PhantomData,
                state: PhantomData,
                table: PhantomData,
            }
        }
    };
    ($type:ident) => {
        paste! {
            /// JOIN with USING clause (PostgreSQL-specific)
            pub fn [<$type _join_using>]<U:  PostgresTable<'a>>(
                self,
                table: U,
                columns: impl ToSQL<'a, PostgresValue<'a>>,
            ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
                SelectBuilder {
                    sql: self.sql.append(helpers::[<$type _join_using>](table, columns)),
                    schema: PhantomData,
                    state: PhantomData,
                    table: PhantomData,
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
pub type SelectBuilder<'a, Schema, State, Table = ()> =
    super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S> SelectBuilder<'a, S, SelectInitial> {
    /// Specifies the table to select FROM and transitions state
    #[inline]
    pub fn from<T>(self, query: T) -> SelectBuilder<'a, S, SelectFromSet, T>
    where
        T: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::from(query)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-FROM State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectFromSet, T> {
    /// Adds a JOIN clause to the query
    #[inline]
    pub fn join<U: PostgresTable<'a>>(
        self,
        table: U,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::join(table, condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    join_impl!();

    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectWhereSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::r#where(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a GROUP BY clause to the query
    pub fn group_by(
        self,
        expressions: impl IntoIterator<Item = impl ToSQL<'a, PostgresValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Limits the number of rows returned
    #[inline]
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Sets the offset for the query results
    #[inline]
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Sorts the query results
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-JOIN State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectJoinSet, T> {
    /// Adds a WHERE condition after a JOIN
    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectWhereSet, T> {
        SelectBuilder {
            sql: self.sql.append(crate::helpers::r#where(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
    /// Sorts the query results
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
    /// Adds a JOIN clause to the query
    #[inline]
    pub fn join<U: PostgresTable<'a>>(
        self,
        table: U,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::join(table, condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
    join_impl!();
}

//------------------------------------------------------------------------------
// Post-WHERE State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectWhereSet, T> {
    /// Adds a GROUP BY clause after a WHERE
    pub fn group_by(
        self,
        expressions: impl IntoIterator<Item = impl ToSQL<'a, PostgresValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds an ORDER BY clause after a WHERE
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a LIMIT clause after a WHERE
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-GROUP BY State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectGroupSet, T> {
    /// Adds a HAVING clause after GROUP BY
    pub fn having(
        self,
        condition: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::having(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds an ORDER BY clause after GROUP BY
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-ORDER BY State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectOrderSet, T> {
    /// Adds a LIMIT clause after ORDER BY
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-LIMIT State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectLimitSet, T> {
    /// Adds an OFFSET clause after LIMIT
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, S, State, T> SelectBuilder<'a, S, State, T>
where
    State: AsCteState,
    T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn into_cte(
        self,
        name: &'static str,
    ) -> super::CTEView<'a, <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased, Self>
    {
        super::CTEView::new(
            <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::alias(name),
            name,
            self,
        )
    }
}

//------------------------------------------------------------------------------
// Set operation support (UNION / INTERSECT / EXCEPT)
//------------------------------------------------------------------------------

impl<'a, S, State, T> SelectBuilder<'a, S, State, T>
where
    State: ExecutableState,
{
    /// Combines this query with another using UNION.
    pub fn union(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::union(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using UNION ALL.
    pub fn union_all(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::union_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using INTERSECT.
    pub fn intersect(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::intersect(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using INTERSECT ALL.
    pub fn intersect_all(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::intersect_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using EXCEPT.
    pub fn except(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::except(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using EXCEPT ALL.
    pub fn except_all(
        self,
        other: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::except_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, S, T> SelectBuilder<'a, S, SelectSetOpSet, T> {
    /// Sorts the results of a set operation.
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Limits the results of a set operation.
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Offsets the results of a set operation.
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
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

impl<'a, S, State, T> SelectBuilder<'a, S, State, T>
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
    pub fn for_update(self) -> SelectBuilder<'a, S, SelectForSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_update()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
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
    pub fn for_share(self) -> SelectBuilder<'a, S, SelectForSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_share()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
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
    pub fn for_no_key_update(self) -> SelectBuilder<'a, S, SelectForSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_no_key_update()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
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
    pub fn for_key_share(self) -> SelectBuilder<'a, S, SelectForSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_key_share()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
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
    ///     .join(orders, eq(users.id, orders.user_id))
    ///     .for_update_of(users)
    /// // SELECT ... FOR UPDATE OF "users"
    /// ```
    pub fn for_update_of<U: PostgresTable<'a>>(
        self,
        table: U,
    ) -> SelectBuilder<'a, S, SelectForSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_update_of(table.name())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
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
    ///     .join(orders, eq(users.id, orders.user_id))
    ///     .for_share_of(users)
    /// // SELECT ... FOR SHARE OF "users"
    /// ```
    pub fn for_share_of<U: PostgresTable<'a>>(
        self,
        table: U,
    ) -> SelectBuilder<'a, S, SelectForSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_share_of(table.name())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-FOR State Implementation (NOWAIT / SKIP LOCKED)
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectForSet, T> {
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
    pub fn nowait(self) -> SelectBuilder<'a, S, SelectForSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::nowait()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
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
    pub fn skip_locked(self) -> SelectBuilder<'a, S, SelectForSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::skip_locked()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{ToSQL, SQL};

    #[test]
    fn test_select_builder_creation() {
        let builder = SelectBuilder::<(), SelectInitial> {
            sql: SQL::raw("SELECT *"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "SELECT *");
    }
}
