use crate::helpers::{self, JoinArg};
use crate::values::SQLiteValue;
use core::marker::PhantomData;
use drizzle_core::{SQLTable, ToSQL};
use paste::paste;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{
    SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
    SelectOrderSet, SelectSetOpSet, SelectWhereSet,
};

/// Clause gate for SELECT methods whose names collide with INSERT/UPDATE/DELETE
/// builder methods on the shared `QueryBuilder` type.
///
/// Coherence can only rule out overlapping inherent impls through a trait
/// local to this crate, so these clauses use this trait instead of
/// [`drizzle_core::ClauseAllowed`].
#[doc(hidden)]
#[diagnostic::on_unimplemented(
    message = "`{C}` cannot be added in builder state `{Self}`",
    label = "this clause is not available at this point of the query",
    note = "SELECT clauses go in order: FROM, JOIN, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET"
)]
pub trait SelectClause<C> {}

impl SelectClause<drizzle_core::clause::Where> for SelectFromSet {}
impl SelectClause<drizzle_core::clause::Where> for SelectJoinSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectFromSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectJoinSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectWhereSet {}
impl SelectClause<drizzle_core::clause::OrderBy> for SelectGroupSet {}
// `SelectSetOpSet` takes no plain ORDER BY: a compound query orders by its
// output columns, which the dedicated `order_by` on that state renders.

/// `SQLite` clause marker: `OFFSET` without a preceding `LIMIT`.
#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct StandaloneOffset;

impl drizzle_core::ClauseAllowed<StandaloneOffset> for SelectFromSet {}
impl drizzle_core::ClauseAllowed<StandaloneOffset> for SelectSetOpSet {}

//------------------------------------------------------------------------------
// Join macro (generates all join variants)
//------------------------------------------------------------------------------

#[doc(hidden)]
macro_rules! join_impl {
    () => {
        join_impl!(@natural natural, Join::new().natural(), drizzle_core::InnerJoin);
        join_impl!(@natural natural_left, Join::new().natural().left(), drizzle_core::LeftJoin);
        join_impl!(left, Join::new().left(), drizzle_core::LeftJoin);
        join_impl!(left_outer, Join::new().left().outer(), drizzle_core::LeftJoin);
        join_impl!(@natural natural_left_outer, Join::new().natural().left().outer(), drizzle_core::LeftJoin);
        join_impl!(@natural natural_right, Join::new().natural().right(), drizzle_core::RightJoin);
        join_impl!(right, Join::new().right(), drizzle_core::RightJoin);
        join_impl!(right_outer, Join::new().right().outer(), drizzle_core::RightJoin);
        join_impl!(@natural natural_right_outer, Join::new().natural().right().outer(), drizzle_core::RightJoin);
        join_impl!(@natural natural_full, Join::new().natural().full(), drizzle_core::FullJoin);
        join_impl!(full, Join::new().full(), drizzle_core::FullJoin);
        join_impl!(full_outer, Join::new().full().outer(), drizzle_core::FullJoin);
        join_impl!(@natural natural_full_outer, Join::new().natural().full().outer(), drizzle_core::FullJoin);
        join_impl!(inner, Join::new().inner(), drizzle_core::InnerJoin);
    };
    (@natural $type:ident, $join_expr:expr, $kind:ty) => {
        paste! {
            /// Adds a NATURAL join. The database matches the columns both
            /// sides share by name, so it takes a source and no ON condition.
            #[allow(clippy::type_complexity)]
            pub fn [<$type _join>]<J: helpers::JoinSource<'a>>(
                self,
                source: J,
            ) -> SelectBuilder<'a, S, SelectJoinSet, J::JoinedTable, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind>>::Marker, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind>>::Row, G>
            where
                M: drizzle_core::JoinStep<R, J::JoinedTable, $kind>,
            {
                use drizzle_core::{Join, ToSQL};
                SelectBuilder {
                    sql: self
                        .sql
                        .append($join_expr.to_sql())
                        .append(drizzle_core::SQL::raw(" "))
                        .append(source.into_join_source_sql()),
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
    ($type:ident, $join_expr:expr, $kind:ty) => {
        paste! {
            #[allow(clippy::type_complexity)]
            pub fn [<$type _join>]<J: JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> SelectBuilder<'a, S, SelectJoinSet, J::JoinedTable, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>>::Marker, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>>::Row, G>
            where
                M: drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>,
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
        drizzle_core::FromMarker<M, T>,
        <M as drizzle_core::ResolveRow<T>>::Row,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::ScopeEntry,
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Join>,
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
        <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>>::Marker,
        <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>>::Row,
        G,
    >
    where
        M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>,
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

    /// Adds a cross join.
    ///
    /// A bare source renders `CROSS JOIN`. For backwards compatibility,
    /// `(source, predicate)` renders the equivalent `INNER JOIN ... ON ...`.
    #[allow(clippy::type_complexity)]
    pub fn cross_join<Arg: helpers::CrossJoinArg<'a, T>>(
        self,
        arg: Arg,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Arg::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::InnerJoin,
            Arg::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Arg::JoinedTable,
            drizzle_core::InnerJoin,
            Arg::OnSources,
        >>::Row,
        G,
    >
    where
        M: drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>,
    {
        SelectBuilder {
            sql: self.sql.append(arg.into_cross_join_sql()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

// WHERE (available from SelectFromSet and SelectJoinSet)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: SelectClause<drizzle_core::clause::Where>,
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
    #[allow(clippy::type_complexity)]
    pub fn r#where<E>(
        self,
        condition: E,
    ) -> SelectBuilder<
        'a,
        S,
        SelectWhereSet,
        T,
        <M as drizzle_core::HasScope>::With<E::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::GroupBy>,
{
    /// Adds a GROUP BY clause to the query.
    ///
    /// Non-aggregate columns in SELECT must appear in the GROUP BY list, with
    /// one exception: grouping by a table's single-column primary key
    /// functionally determines the whole row (SQL:1999), so any scalar column
    /// of that table may be selected. Prefer `.group_by(table.pk)` over
    /// listing every selected column — it also lets `SQLite` stream groups in
    /// key order instead of sorting through a temp B-tree.
    pub fn group_by<Gr>(
        self,
        columns: Gr,
    ) -> SelectBuilder<
        'a,
        S,
        SelectGroupSet,
        T,
        <M as drizzle_core::HasScope>::With<Gr::Sources>,
        R,
        Gr::Columns,
    >
    where
        M: drizzle_core::HasScope,
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Having>,
{
    /// Adds a HAVING clause after GROUP BY.
    #[allow(clippy::type_complexity)]
    pub fn having<E>(
        self,
        condition: E,
    ) -> SelectBuilder<
        'a,
        S,
        SelectGroupSet,
        T,
        <M as drizzle_core::HasScope>::With<E::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
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
    State: SelectClause<drizzle_core::clause::OrderBy>,
{
    /// Sorts the query results.
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<
        'a,
        S,
        SelectOrderSet,
        T,
        <M as drizzle_core::HasScope>::With<TOrderBy::Sources>,
        R,
        G,
    >
    where
        M: drizzle_core::HasScope,
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>> + drizzle_core::expr::ExprSources,
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

// ORDER BY on a compound query: the combined rows carry no table scope, so the
// ordering terms are rendered as output column names.
impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectSetOpSet, T, M, R, G> {
    /// Sorts a compound (`UNION` / `INTERSECT` / `EXCEPT`) result by its
    /// output columns. Column references are rendered unqualified, which is
    /// the only spelling PostgreSQL and turso accept here.
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R, G>
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self
                .sql
                .append(drizzle_core::helpers::set_order_by(expressions)),
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Limit>,
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

// OFFSET without LIMIT (available from SelectFromSet and SelectSetOpSet)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<StandaloneOffset>,
{
    /// Sets the offset for the query results.
    ///
    /// `SQLite` only accepts `OFFSET` after a `LIMIT`, so this renders
    /// `LIMIT -1 OFFSET n`; a negative limit means no limit.
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
            sql: self.sql.append(helpers::standalone_offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

// OFFSET after LIMIT
impl<'a, S, T, M, R, G> SelectBuilder<'a, S, SelectLimitSet, T, M, R, G> {
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
    State: drizzle_core::ExecutableState,
    M: drizzle_core::DerivedSelection<'a, SQLiteValue<'a>, crate::common::SQLiteSchemaType, T>,
{
    /// Names this completed query so it can be used as a derived source.
    ///
    /// # Panics
    ///
    /// Panics when the projection contains duplicate output names. Name a
    /// computed expression with [`drizzle_core::expr::NamedExt::named`] to
    /// make each output unique.
    #[inline]
    #[must_use]
    pub fn alias<Name, AggProof>(
        self,
        _name: Name,
    ) -> drizzle_core::Derived<
        'a,
        SQLiteValue<'a>,
        Name,
        <M as drizzle_core::DerivedSelection<
            'a,
            SQLiteValue<'a>,
            crate::common::SQLiteSchemaType,
            T,
        >>::Projection,
        Self,
    >
    where
        Name: drizzle_core::Tag,
        <M as drizzle_core::DerivedSelection<
            'a,
            SQLiteValue<'a>,
            crate::common::SQLiteSchemaType,
            T,
        >>::Projection: drizzle_core::DerivedProjection<Name>,
        M: drizzle_core::row::MarkerAggValidFor<G, AggProof>,
    {
        // SAFETY: The executable-state, aggregate, and projection bounds
        // above prove that this query matches the derived projection; its
        // scope travels in `Self`'s sources and is checked where it is used.
        unsafe { drizzle_core::Derived::new_unchecked(self) }
    }
}

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple>,
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
    #[allow(clippy::type_complexity)]
    pub fn union<M2>(
        self,
        other: impl IntoSelect<'a, S, M2, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, <M as drizzle_core::SetOperand<M2>>::Combined, R, G>
    where
        M: drizzle_core::SetOperand<M2>,
    {
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
    #[allow(clippy::type_complexity)]
    pub fn union_all<M2>(
        self,
        other: impl IntoSelect<'a, S, M2, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, <M as drizzle_core::SetOperand<M2>>::Combined, R, G>
    where
        M: drizzle_core::SetOperand<M2>,
    {
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
    #[allow(clippy::type_complexity)]
    pub fn intersect<M2>(
        self,
        other: impl IntoSelect<'a, S, M2, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, <M as drizzle_core::SetOperand<M2>>::Combined, R, G>
    where
        M: drizzle_core::SetOperand<M2>,
    {
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

    /// Combines this query with another using EXCEPT.
    #[allow(clippy::type_complexity)]
    pub fn except<M2>(
        self,
        other: impl IntoSelect<'a, S, M2, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, <M as drizzle_core::SetOperand<M2>>::Combined, R, G>
    where
        M: drizzle_core::SetOperand<M2>,
    {
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
}

//------------------------------------------------------------------------------
// Expr impl for subquery usage
//------------------------------------------------------------------------------

impl<'a, S, State, T, M, R, G> drizzle_core::expr::Expr<'a, SQLiteValue<'a>>
    for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ExecutableState,
    M: drizzle_core::expr::SubqueryType<'a, SQLiteValue<'a>> + drizzle_core::SelectSources,
{
    type SQLType = <M as drizzle_core::expr::SubqueryType<'a, SQLiteValue<'a>>>::SQLType;
    type Nullable = drizzle_core::expr::Null;
    type Aggregate = drizzle_core::expr::Scalar;
}

impl<S, State, T, M, R, G> drizzle_core::expr::ExprSources
    for SelectBuilder<'_, S, State, T, M, R, G>
where
    M: drizzle_core::SelectSources,
{
    type Sources = M::Sources;
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

mod insert_select_private {
    use super::{
        SelectFromSet, SelectGroupSet, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
        SelectOrderSet, SelectSetOpSet, SelectWhereSet,
    };

    pub trait Sealed {}
    pub trait Completed: drizzle_core::ExecutableState {}

    impl Completed for SelectFromSet {}
    impl Completed for SelectJoinSet {}
    impl Completed for SelectWhereSet {}
    impl Completed for SelectGroupSet {}
    impl Completed for SelectOrderSet {}
    impl Completed for SelectLimitSet {}
    impl Completed for SelectOffsetSet {}
    impl Completed for SelectSetOpSet {}
}

/// A completed SELECT that can supply rows to an INSERT.
#[doc(hidden)]
pub trait CompletedSelect<'a, S, R>: insert_select_private::Sealed {
    type Marker;
    type Grouped;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, SQLiteValue<'a>>;
}

/// Converts a completed SELECT or attached SELECT wrapper into its checked source.
#[doc(hidden)]
pub trait IntoSelectQuery<'a, S, R> {
    type Marker;
    type Grouped;
    type Select: CompletedSelect<'a, S, R, Marker = Self::Marker, Grouped = Self::Grouped>;

    fn into_select_query(self) -> Self::Select;
}

impl<'a, S, State, T, M, R, G> insert_select_private::Sealed
    for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: insert_select_private::Completed,
{
}

impl<'a, S, State, T, M, R, G> CompletedSelect<'a, S, R> for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: insert_select_private::Completed,
{
    type Marker = M;
    type Grouped = G;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, SQLiteValue<'a>> {
        self.sql
    }
}

impl<'a, S, State, T, M, R, G> IntoSelectQuery<'a, S, R> for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: insert_select_private::Completed,
{
    type Marker = M;
    type Grouped = G;
    type Select = Self;

    fn into_select_query(self) -> Self::Select {
        self
    }
}
