use crate::helpers::{self, JoinArg};
use crate::values::SQLiteValue;
use core::marker::PhantomData;
use drizzle_core::{SQLTable, ToSQL};
use paste::paste;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{
    AsCteState, SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet,
    SelectOffsetSet, SelectOrderSet, SelectSetOpSet, SelectWhereSet,
};

#[doc(hidden)]
pub trait SelectWhereAllowed: drizzle_core::WhereAllowed {}

impl SelectWhereAllowed for SelectFromSet {}
impl SelectWhereAllowed for SelectJoinSet {}

//------------------------------------------------------------------------------
// Join macro (generates all join variants)
//------------------------------------------------------------------------------

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
            ) -> SelectBuilder<'a, S, SelectJoinSet, J::JoinedTable, <M as drizzle_core::ScopePush<J::JoinedTable>>::Out, <M as $join_trait<R, J::JoinedTable>>::NewRow, G>
            where
                M: $join_trait<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
            {
                use drizzle_core::Join;
                SelectBuilder {
                    sql: self.sql.append(arg.into_join_sql($join_expr)),
                    schema: PhantomData,
                    state: PhantomData,
                    table: PhantomData,
                    marker: PhantomData,
                    row: PhantomData,
                    grouped: PhantomData,
                }
            }
        }
    };
}

//------------------------------------------------------------------------------
// SelectBuilder Definition
//------------------------------------------------------------------------------

/// Builds a SELECT query specifically for `SQLite`.
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
/// #             #[cfg(feature = "rusqlite")]
/// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #             #[cfg(feature = "libsql")]
/// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #             #[cfg(feature = "turso")]
/// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
/// #             #[cfg(feature = "rusqlite")]
/// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #             #[cfg(feature = "libsql")]
/// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #             #[cfg(feature = "turso")]
/// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
/// #             #[cfg(feature = "rusqlite")]
/// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #             #[cfg(feature = "libsql")]
/// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #             #[cfg(feature = "turso")]
/// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
pub type SelectBuilder<'a, Schema, State, Table = (), Marker = (), Row = (), Grouped = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>;

//------------------------------------------------------------------------------
// Initial State: .from()
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
    /// #             #[cfg(feature = "rusqlite")]
    /// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #             #[cfg(feature = "libsql")]
    /// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #             #[cfg(feature = "turso")]
    /// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
        let sql = self.sql.append(helpers::from(query));
        SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Capability-gated methods (generic over State)
//------------------------------------------------------------------------------

// JOIN (available from SelectFromSet and SelectJoinSet)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::JoinAllowed,
{
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
    /// #             #[cfg(feature = "rusqlite")]
    /// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #             #[cfg(feature = "libsql")]
    /// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #             #[cfg(feature = "turso")]
    /// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
        G,
    >
    where
        M: drizzle_core::AfterJoin<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
    {
        SelectBuilder {
            sql: self
                .sql
                .append(arg.into_join_sql(drizzle_core::Join::new())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    join_impl!();
}

// WHERE (available from SelectFromSet and SelectJoinSet)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectWhereAllowed,
{
    /// Adds a WHERE clause to filter query results.
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
    /// #             #[cfg(feature = "rusqlite")]
    /// #             pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #             #[cfg(feature = "libsql")]
    /// #             pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #             #[cfg(feature = "turso")]
    /// #             pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
    ///     .r#where(and(gt(user.id, 10), eq(user.name, "Alice")));
    /// ```
    #[inline]
    pub fn r#where<E>(self, condition: E) -> SelectBuilder<'a, S, SelectWhereSet, T, M, R, G>
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::r#where(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

// GROUP BY (available from SelectFromSet, SelectJoinSet, SelectWhereSet)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::GroupByAllowed,
{
    /// Adds a GROUP BY clause to the query.
    pub fn group_by<Gr>(
        self,
        columns: Gr,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R, Gr::Columns>
    where
        Gr: drizzle_core::IntoGroupBy<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by_expr(columns)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

// HAVING (available only from SelectGroupSet)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::HavingAllowed,
{
    /// Adds a HAVING clause after GROUP BY.
    pub fn having<E>(self, condition: E) -> SelectBuilder<'a, S, SelectGroupSet, T, M, R, G>
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::having(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

// ORDER BY (available from many states)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::OrderByAllowed,
{
    /// Sorts the query results.
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R, G>
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

// LIMIT (available from many states)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::LimitAllowed,
{
    /// Limits the number of rows returned.
    ///
    /// # Panics
    ///
    /// Panics when a signed numeric argument is negative or a numeric value
    /// does not fit in `usize`.
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn limit<P>(self, limit: P) -> SelectBuilder<'a, S, SelectLimitSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

// OFFSET (available from SelectFromSet and SelectLimitSet)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::OffsetAllowed,
{
    /// Sets the offset for the query results.
    ///
    /// # Panics
    ///
    /// Panics when a signed numeric argument is negative or a numeric value
    /// does not fit in `usize`.
    #[inline]
    #[must_use]
    #[track_caller]
    pub fn offset<P>(self, offset: P) -> SelectBuilder<'a, S, SelectOffsetSet, T, M, R, G>
    where
        P: drizzle_core::PaginationArg<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// CTE support
//------------------------------------------------------------------------------

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: AsCteState,
    T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a typed CTE using alias tag name.
    #[inline]
    #[must_use]
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

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ExecutableState,
{
    /// Combines this query with another using UNION.
    pub fn union(
        self,
        other: impl IntoSelect<'a, S, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
        SelectBuilder {
            sql: helpers::union(self.sql, other.into_select()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Combines this query with another using UNION ALL.
    pub fn union_all(
        self,
        other: impl IntoSelect<'a, S, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
        SelectBuilder {
            sql: helpers::union_all(self.sql, other.into_select()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Combines this query with another using INTERSECT.
    pub fn intersect(
        self,
        other: impl IntoSelect<'a, S, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
        SelectBuilder {
            sql: helpers::intersect(self.sql, other.into_select()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Combines this query with another using INTERSECT ALL.
    pub fn intersect_all(
        self,
        other: impl IntoSelect<'a, S, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
        SelectBuilder {
            sql: helpers::intersect_all(self.sql, other.into_select()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Combines this query with another using EXCEPT.
    pub fn except(
        self,
        other: impl IntoSelect<'a, S, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
        SelectBuilder {
            sql: helpers::except(self.sql, other.into_select()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Combines this query with another using EXCEPT ALL.
    pub fn except_all(
        self,
        other: impl IntoSelect<'a, S, M, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
        SelectBuilder {
            sql: helpers::except_all(self.sql, other.into_select()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Expr impl for subquery usage
//------------------------------------------------------------------------------

impl<'a, S, State, T, M, R, G> drizzle_core::expr::Expr<'a, SQLiteValue<'a>>
    for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ExecutableState,
    M: drizzle_core::expr::SubqueryType<'a, SQLiteValue<'a>>,
{
    type SQLType = <M as drizzle_core::expr::SubqueryType<'a, SQLiteValue<'a>>>::SQLType;
    type Nullable = drizzle_core::expr::Null;
    type Aggregate = drizzle_core::expr::Scalar;
}

//------------------------------------------------------------------------------
// IntoSelect conversion trait
//------------------------------------------------------------------------------

/// Conversion trait for types that can become a `SelectBuilder`.
/// Used by set operations to accept both raw `SelectBuilder` and `DrizzleBuilder`.
pub trait IntoSelect<'a, S, M, R> {
    type State: drizzle_core::ExecutableState;
    type Table;
    fn into_select(self) -> SelectBuilder<'a, S, Self::State, Self::Table, M, R>;
}

impl<'a, S, State: drizzle_core::ExecutableState, T, M, R, G> IntoSelect<'a, S, M, R>
    for SelectBuilder<'a, S, State, T, M, R, G>
{
    type State = State;
    type Table = T;
    fn into_select(self) -> SelectBuilder<'a, S, State, T, M, R> {
        SelectBuilder {
            sql: self.sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}
