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
    message = "builder state `{Self}` does not allow `{C}`",
    label = "not available at this point of the query",
    note = "SELECT clauses go in order: FROM, JOIN, WHERE, GROUP BY, HAVING, ORDER BY, LIMIT, OFFSET",
    note = "only a SELECT can be a set operand, a subquery, a derived table, or an INSERT source"
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

/// Clause marker for `OFFSET` without a preceding `LIMIT`.
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
            #[doc = concat!("Adds a `", stringify!($type), "` join (`NATURAL`).")]
            ///
            /// A natural join matches the columns both sides share by name,
            /// so it takes a table or derived table and no ON condition.
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
            #[doc = concat!("Adds a `", stringify!($type), "` join.")]
            ///
            /// Pass `(table, condition)` for an explicit ON condition, or a
            /// bare table to join on its foreign key to the previous table.
            /// See [`join`](Self::join) for an example.
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

/// A SELECT query being built for `SQLite`.
///
/// This is [`QueryBuilder`](super::QueryBuilder) in one of the `Select*`
/// states. Start it with [`QueryBuilder::select`](super::QueryBuilder::select)
/// or [`select_distinct`](super::QueryBuilder::select_distinct), then call
/// [`from`](Self::from).
///
/// # Clause order
///
/// Clauses must be added in SQL order. Each method is only available in the
/// states listed here:
///
/// | After | You can call |
/// |---|---|
/// | `select` | `from` |
/// | `from` | joins, `where`, `group_by`, `order_by`, `limit`, `offset`, set operations |
/// | a join | more joins, `where`, `group_by`, `order_by`, `limit`, set operations |
/// | `where` | `group_by`, `order_by`, `limit`, set operations |
/// | `group_by` | `having`, `order_by`, `limit`, set operations |
/// | `having` | `having`, `order_by`, `limit`, set operations |
/// | `order_by` | `limit`, set operations |
/// | `limit` | `offset`, set operations |
/// | `offset` | set operations |
/// | a set operation | more set operations, `order_by`, `limit`, `offset` |
///
/// `offset` without `limit` is only offered right after `from` or a set
/// operation; elsewhere, add a `limit` first. Every state after `from` can
/// be executed, used as a subquery, or named as a derived table with
/// [`alias`](Self::alias). Every state except a compound query can become a
/// CTE with [`into_cte`](Self::into_cte).
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
/// Joins:
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
///
/// Ordering and pagination:
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
///     .limit(10)
///     .offset(20);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"SELECT "users"."name" FROM "users" ORDER BY "users"."name" ASC LIMIT 10 OFFSET 20"#
/// );
/// ```
///
/// # Compile-time checks
///
/// A clause added out of order does not compile:
///
/// ```rust,compile_fail
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod sqlite {
/// #         pub use drizzle_sqlite::*;
/// #         #[cfg(feature = "rusqlite")]
/// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #         #[cfg(feature = "libsql")]
/// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #         #[cfg(feature = "turso")]
/// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::core::expr::gt;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// // WHERE cannot follow LIMIT.
/// let query = builder.select(user.name).from(user).limit(10).r#where(gt(user.id, 1));
/// ```
///
/// `having` needs a `group_by` first, and a WHERE or HAVING condition must
/// be a boolean expression. Column references are scope-checked: a query
/// that names a table missing from its FROM and JOIN clauses is rejected when
/// it is executed (`.all()`, `.get()`, ...), used as a derived table, or used
/// as an INSERT source.
pub type SelectBuilder<'a, Schema, State, Table = (), Marker = (), Row = (), Grouped = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>;

//------------------------------------------------------------------------------
// Initial State: .from()
//------------------------------------------------------------------------------

impl<'a, S, M> SelectBuilder<'a, S, SelectInitial, (), M> {
    /// Sets the FROM source: a table, a CTE, or a derived table made with
    /// [`alias`](Self::alias).
    ///
    /// The result row type is inferred from the selected columns and this
    /// source. With `select(())`, the row is the table's generated select
    /// model.
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
    /// Adds a `JOIN` (an inner join).
    ///
    /// Pass `(table, condition)` for an explicit ON condition, or a bare table
    /// to join on its foreign key to the previous table. A derived table
    /// (see [`alias`](Self::alias)) also works in the tuple form.
    ///
    /// The other join methods (`left_join`, `right_join`, `full_join`,
    /// `inner_join`, the `_outer` and `natural_` variants, and
    /// [`cross_join`](Self::cross_join)) take the same arguments. With
    /// `select(())`, a LEFT, RIGHT or FULL join makes the side that may be
    /// missing an `Option` in the row type.
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

    /// Adds a `CROSS JOIN`, which pairs every row with every row of `arg`.
    ///
    /// A bare table renders `CROSS JOIN`. For backwards compatibility,
    /// `(table, condition)` renders the equivalent `INNER JOIN ... ON ...`.
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
    /// Adds a WHERE clause.
    ///
    /// The condition must be a boolean expression. Combine conditions with
    /// `and` and `or` from `drizzle_core::expr`.
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
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name" FROM "users" WHERE ("users"."id" > ? AND "users"."name" = ?)"#
    /// );
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
    /// Adds a GROUP BY clause. Pass one expression or a tuple.
    ///
    /// Every selected column that is not inside an aggregate must appear in
    /// the GROUP BY list; this is checked when the query is executed. One
    /// exception: grouping by a table's single-column primary key determines
    /// the whole row, so any column of that table may be selected. Prefer
    /// `.group_by(table.pk)` over listing every selected column; it also
    /// lets `SQLite` read groups in key order instead of sorting them in a
    /// temporary B-tree.
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
    /// #         pub use drizzle_sqlite::*;
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::{count, gt};
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "posts")] struct Post { #[column(primary)] id: i32, user_id: i32, title: String }
    /// # #[derive(SQLiteSchema)] struct Schema { post: Post }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { post } = Schema::new();
    /// let query = builder
    ///     .select((post.user_id, count(post.id)))
    ///     .from(post)
    ///     .group_by(post.user_id)
    ///     .having(gt(count(post.id), 5));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "posts"."user_id", COUNT ("posts"."id") FROM "posts" GROUP BY "posts"."user_id" HAVING COUNT ("posts"."id")> ?"#
    /// );
    /// ```
    #[allow(clippy::type_complexity)]
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
    /// Adds a HAVING clause, which filters groups.
    ///
    /// Only available after [`group_by`](Self::group_by). The condition must
    /// be a boolean expression and may use aggregates. See `group_by` for an
    /// example.
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
    /// Adds an ORDER BY clause.
    ///
    /// Pass one ordering term or a tuple. Wrap a column in `asc` or `desc`
    /// to set the direction.
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
    /// #         pub use drizzle_sqlite::*;
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
    ///     .order_by((desc(user.name), asc(user.id)));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name" FROM "users" ORDER BY "users"."name" DESC, "users"."id" ASC"#
    /// );
    /// ```
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
    /// output columns.
    ///
    /// Column references are written without their table name, because the
    /// combined rows no longer belong to one table (turso rejects the
    /// qualified form). See [`union`](Self::union) for an example.
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
    /// Adds a LIMIT clause.
    ///
    /// Pass a non-negative integer, which is written into the SQL, or an
    /// integer placeholder, which is bound when the query runs. See
    /// [`SelectBuilder`] for an example.
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
    /// Skips the first `offset` rows without limiting the row count.
    ///
    /// `SQLite` only accepts `OFFSET` after a `LIMIT`, so this renders
    /// `LIMIT -1 OFFSET n`; a negative limit means no limit. Only available
    /// right after `from` or a set operation; elsewhere call
    /// [`limit`](Self::limit) first.
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
    /// #         pub use drizzle_sqlite::*;
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
    /// let query = builder.select(user.name).from(user).offset(5);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users" LIMIT -1 OFFSET 5"#);
    /// ```
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
    /// Adds an OFFSET clause after LIMIT.
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
    M: drizzle_core::DerivedSelection<'a, SQLiteValue<'a>, crate::common::SQLiteSchemaType, T>,
{
    /// Names this query so it can be used as a derived table in `from` or a
    /// join.
    ///
    /// `name` is a value of a [`Tag`](drizzle_core::Tag) type; its `NAME`
    /// becomes the SQL alias. The result exposes the selected columns, so
    /// the outer query can reference them with typed accessors.
    ///
    /// # Panics
    ///
    /// Panics when the projection contains duplicate output names. Name a
    /// computed expression with [`drizzle_core::expr::AliasExt::named`] to
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
    /// Turns this SELECT into a common table expression named `Tag::NAME`.
    ///
    /// The result derefs to an aliased copy of the FROM table, so you can
    /// select its columns with the usual field access. Pass it to
    /// [`QueryBuilder::with`](super::QueryBuilder::with) and then use it in
    /// `from`. Not available on a compound query (after a set operation).
    /// See [`QueryBuilder::with`](super::QueryBuilder::with) for an example.
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    /// Combines this query with `other` using UNION, which drops duplicate
    /// rows.
    ///
    /// Both queries must select the same row type. After a set operation you
    /// can chain more set operations, then `order_by`, `limit` and `offset`
    /// for the combined result.
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
    /// #         pub use drizzle_sqlite::*;
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::{eq, gt};
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder
    ///     .select(user.name)
    ///     .from(user)
    ///     .r#where(eq(user.id, 1))
    ///     .union(builder.select(user.name).from(user).r#where(gt(user.id, 100)))
    ///     .order_by(asc(user.name));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name" FROM "users" WHERE "users"."id" = ? UNION SELECT "users"."name" FROM "users" WHERE "users"."id" > ? ORDER BY "name" ASC"#
    /// );
    /// ```
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

    /// Combines this query with `other` using UNION ALL, which keeps
    /// duplicate rows. See [`union`](Self::union).
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

    /// Keeps only rows that `other` also returns (INTERSECT). See
    /// [`union`](Self::union).
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

    /// Keeps only rows that `other` does not return (EXCEPT). See
    /// [`union`](Self::union).
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
    M: drizzle_core::expr::SubqueryType<'a, SQLiteValue<'a>> + drizzle_core::SelectSources,
{
    type SQLType = <M as drizzle_core::expr::SubqueryType<'a, SQLiteValue<'a>>>::SQLType;
    type Nullable = drizzle_core::expr::Null;
    type Aggregate = drizzle_core::expr::Scalar;
}

impl<S, State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>, T, M, R, G>
    drizzle_core::expr::SelectQuery for SelectBuilder<'_, S, State, T, M, R, G>
{
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

/// A query that can be the right-hand side of a set operation.
///
/// Implemented for completed [`SelectBuilder`]s and for the driver
/// builders in the `drizzle` crate that wrap one.
pub trait IntoSelect<'a, S, M, R> {
    /// Builder state of the converted query.
    type State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>;
    /// FROM table of the converted query.
    type Table;
    /// Returns the underlying [`SelectBuilder`].
    fn into_select(self) -> SelectBuilder<'a, S, Self::State, Self::Table, M, R>;
}

impl<'a, S, State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>, T, M, R, G>
    IntoSelect<'a, S, M, R> for SelectBuilder<'a, S, State, T, M, R, G>
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
    pub trait Sealed {}
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
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
{
}

impl<'a, S, State, T, M, R, G> CompletedSelect<'a, S, R> for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
{
    type Marker = M;
    type Grouped = G;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, SQLiteValue<'a>> {
        self.sql
    }
}

impl<'a, S, State, T, M, R, G> IntoSelectQuery<'a, S, R> for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
{
    type Marker = M;
    type Grouped = G;
    type Select = Self;

    fn into_select_query(self) -> Self::Select {
        self
    }
}
