use crate::helpers::{self, JoinArg};
use crate::values::SQLiteValue;
use core::fmt::Debug;
use core::marker::PhantomData;
use drizzle_core::{SQL, SQLTable, ToSQL};
use paste::paste;

// Import the ExecutableState trait
use super::ExecutableState;

#[inline]
fn append_sql<'a>(
    mut base: SQL<'a, SQLiteValue<'a>>,
    fragment: SQL<'a, SQLiteValue<'a>>,
) -> SQL<'a, SQLiteValue<'a>> {
    base.append_mut(fragment);
    base
}

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

#[doc(hidden)]
macro_rules! join_impl {
    () => {
        join_impl!(natural, Join::new().natural(), drizzle_core::AfterJoin);
        join_impl!(natural_left, Join::new().natural().left(), drizzle_core::AfterLeftJoin);
        join_impl!(left, Join::new().left(), drizzle_core::AfterLeftJoin);
        join_impl!(left_outer, Join::new().left().outer(), drizzle_core::AfterLeftJoin);
        join_impl!(natural_left_outer, Join::new().natural().left().outer(), drizzle_core::AfterLeftJoin);
        join_impl!(natural_right, Join::new().natural().right(), drizzle_core::AfterRightJoin);
        join_impl!(right, Join::new().right(), drizzle_core::AfterRightJoin);
        join_impl!(right_outer, Join::new().right().outer(), drizzle_core::AfterRightJoin);
        join_impl!(natural_right_outer, Join::new().natural().right().outer(), drizzle_core::AfterRightJoin);
        join_impl!(natural_full, Join::new().natural().full(), drizzle_core::AfterFullJoin);
        join_impl!(full, Join::new().full(), drizzle_core::AfterFullJoin);
        join_impl!(full_outer, Join::new().full().outer(), drizzle_core::AfterFullJoin);
        join_impl!(natural_full_outer, Join::new().natural().full().outer(), drizzle_core::AfterFullJoin);
        join_impl!(inner, Join::new().inner(), drizzle_core::AfterJoin);
        join_impl!(cross, Join::new().cross(), drizzle_core::AfterJoin);
    };
    ($type:ident, $join_expr:expr, $join_trait:path) => {
        paste! {
            #[allow(clippy::type_complexity)]
            pub fn [<$type _join>]<J: JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> SelectBuilder<'a, S, SelectJoinSet, J::JoinedTable, <M as drizzle_core::ScopePush<J::JoinedTable>>::Out, <M as $join_trait<R, J::JoinedTable>>::NewRow>
            where
                M: $join_trait<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
            {
                use drizzle_core::Join;
                SelectBuilder {
                    sql: append_sql(self.sql, arg.into_join_sql($join_expr)),
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

/// Builds a SELECT query specifically for SQLite.
///
/// `SelectBuilder` provides a type-safe, fluent API for constructing SELECT statements
/// with compile-time verification of query structure and table relationships.
///
/// ## Type Parameters
///
/// - `Schema`: The database schema type, ensuring only valid tables can be referenced
/// - `State`: The current builder state, enforcing proper query construction order
/// - `Table`: The primary table being queried (when applicable)
///
/// ## Query Building Flow
///
/// 1. Start with `QueryBuilder::select()` to specify columns
/// 2. Add `from()` to specify the source table
/// 3. Optionally add joins, conditions, grouping, ordering, and limits
///
/// ## Basic Usage
///
/// ```rust
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod sqlite {
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// use drizzle::sqlite::prelude::*;
/// use drizzle::sqlite::builder::QueryBuilder;
///
/// #[SQLiteTable(name = "users")]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
///     email: Option<String>,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     user: User,
/// }
///
/// let builder = QueryBuilder::new::<Schema>();
/// let Schema { user } = Schema::new();
///
/// // Basic SELECT
/// let query = builder.select(user.name).from(user);
/// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
///
/// // SELECT with WHERE clause
/// use drizzle::core::expr::gt;
/// let query = builder
///     .select((user.id, user.name))
///     .from(user)
///     .r#where(gt(user.id, 10));
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"SELECT "users"."id", "users"."name" FROM "users" WHERE "users"."id" > ?"#
/// );
/// ```
///
/// ## Advanced Queries
///
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod sqlite {
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::core::expr::eq;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[SQLiteTable(name = "posts")] struct Post { #[column(primary)] id: i32, user_id: i32, title: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User, post: Post }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user, post } = Schema::new();
/// let query = builder
///     .select((user.name, post.title))
///     .from(user)
///     .join((post, eq(user.id, post.user_id)));
/// ```
///
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod sqlite {
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .select(user.name)
///     .from(user)
///     .order_by(asc(user.name))
///     .limit(10);
/// ```
pub type SelectBuilder<'a, Schema, State, Table = (), Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S, M> SelectBuilder<'a, S, SelectInitial, (), M> {
    /// Specifies the table or subquery to select FROM.
    ///
    /// This method transitions the builder from the initial state to the FROM state,
    /// enabling subsequent WHERE, JOIN, ORDER BY, and other clauses.
    ///
    /// The row type `R` is resolved from the select marker `M` and the table `T`
    /// via the `ResolveRow` trait.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod sqlite {
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Select from a table
    /// let query = builder.select(user.name).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
    /// ```
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
        T: ToSQL<'a, SQLiteValue<'a>>,
        M: drizzle_core::ResolveRow<T>,
    {
        let sql = append_sql(self.sql, helpers::from(query));
        SelectBuilder {
            sql,
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
    /// Adds an INNER JOIN clause to the query.
    ///
    /// Joins another table to the current query using the specified condition.
    /// The joined table must be part of the schema and the condition should
    /// relate columns from both tables.
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod sqlite {
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::eq;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[SQLiteTable(name = "posts")] struct Post { #[column(primary)] id: i32, user_id: i32, title: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User, post: Post }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = builder
    ///     .select((user.name, post.title))
    ///     .from(user)
    ///     .join((post, eq(user.id, post.user_id)));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name", "posts"."title" FROM "users" JOIN "posts" ON "users"."id" = "posts"."user_id""#
    /// );
    /// ```
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn join<J: JoinArg<'a, T>>(
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
        SelectBuilder {
            sql: append_sql(self.sql, arg.into_join_sql(drizzle_core::Join::new())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    join_impl!();

    /// Adds a WHERE clause to filter query results.
    ///
    /// This method applies conditions to filter the rows returned by the query.
    /// You can use various condition functions from `drizzle::core::expr::conditions`.
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod sqlite {
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::{gt, and, eq};
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, age: Option<i32> }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Single condition
    /// let query = builder
    ///     .select(user.name)
    ///     .from(user)
    ///     .r#where(gt(user.id, 10));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name" FROM "users" WHERE "users"."id" > ?"#
    /// );
    ///
    /// // Multiple conditions
    /// let query = builder
    ///     .select(user.name)
    ///     .from(user)
    ///     .r#where(and([gt(user.id, 10), eq(user.name, "Alice")]));
    /// ```
    #[inline]
    pub fn r#where<E>(self, condition: E) -> SelectBuilder<'a, S, SelectWhereSet, T, M, R>
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::r#where(condition)),
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
        expressions: impl IntoIterator<Item = impl ToSQL<'a, SQLiteValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R> {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::group_by(expressions)),
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
            sql: append_sql(self.sql, helpers::limit(limit)),
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
            sql: append_sql(self.sql, helpers::offset(offset)),
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
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::order_by(expressions)),
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
    pub fn r#where<E>(self, condition: E) -> SelectBuilder<'a, S, SelectWhereSet, T, M, R>
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder {
            sql: append_sql(self.sql, crate::helpers::r#where(condition)),
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
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::order_by(expressions)),
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
    pub fn join<J: JoinArg<'a, T>>(
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
        SelectBuilder {
            sql: append_sql(self.sql, arg.into_join_sql(drizzle_core::Join::new())),
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
        expressions: impl IntoIterator<Item = impl ToSQL<'a, SQLiteValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R> {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::group_by(expressions)),
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
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::order_by(expressions)),
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
            sql: append_sql(self.sql, helpers::limit(limit)),
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
    pub fn having<E>(self, condition: E) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R>
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::having(condition)),
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
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::order_by(expressions)),
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
        let sql = helpers::limit(limit);
        SelectBuilder {
            sql: append_sql(self.sql, sql),
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
            sql: append_sql(self.sql, helpers::offset(offset)),
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
    T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a typed CTE using alias tag name.
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::Aliased<Tag>,
        Self,
    > {
        let name = Tag::NAME;
        super::CTEView::new(
            <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::alias::<Tag>(),
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
    pub fn union<OtherState, OtherTable>(
        self,
        other: SelectBuilder<'a, S, OtherState, OtherTable, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R>
    where
        OtherState: ExecutableState,
    {
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
    pub fn union_all<OtherState, OtherTable>(
        self,
        other: SelectBuilder<'a, S, OtherState, OtherTable, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R>
    where
        OtherState: ExecutableState,
    {
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
    pub fn intersect<OtherState, OtherTable>(
        self,
        other: SelectBuilder<'a, S, OtherState, OtherTable, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R>
    where
        OtherState: ExecutableState,
    {
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
    pub fn intersect_all<OtherState, OtherTable>(
        self,
        other: SelectBuilder<'a, S, OtherState, OtherTable, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R>
    where
        OtherState: ExecutableState,
    {
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
    pub fn except<OtherState, OtherTable>(
        self,
        other: SelectBuilder<'a, S, OtherState, OtherTable, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R>
    where
        OtherState: ExecutableState,
    {
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
    pub fn except_all<OtherState, OtherTable>(
        self,
        other: SelectBuilder<'a, S, OtherState, OtherTable, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R>
    where
        OtherState: ExecutableState,
    {
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

impl<'a, S, State, T, M, R> drizzle_core::expr::Expr<'a, SQLiteValue<'a>>
    for SelectBuilder<'a, S, State, T, M, R>
where
    State: ExecutableState,
    M: drizzle_core::expr::SubqueryType<'a, SQLiteValue<'a>>,
{
    type SQLType = <M as drizzle_core::expr::SubqueryType<'a, SQLiteValue<'a>>>::SQLType;
    type Nullable = drizzle_core::expr::Null;
    type Aggregate = drizzle_core::expr::Scalar;
}

impl<'a, S, T, M, R> SelectBuilder<'a, S, SelectSetOpSet, T, M, R> {
    /// Sorts the results of a set operation.
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R>
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: append_sql(self.sql, helpers::order_by(expressions)),
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
            sql: append_sql(self.sql, helpers::limit(limit)),
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
            sql: append_sql(self.sql, helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}
