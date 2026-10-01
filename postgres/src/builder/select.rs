//! `SELECT` builder states and clause methods.
//!
//! [`SelectBuilder`] is the builder returned by `QueryBuilder::select`. Its
//! state parameter only allows clauses in SQL order: `FROM`, joins, `WHERE`,
//! `GROUP BY`, `HAVING`, `ORDER BY`, `LIMIT`, `OFFSET`, then row locks.

use crate::common::PostgresSchemaType;
use crate::helpers;
use crate::traits::PostgresTable;
use crate::values::PostgresValue;
use core::marker::PhantomData;
use drizzle_core::ToSQL;
use drizzle_core::traits::SQLTable;
use paste::paste;

// Import the ExecutableState trait
use super::ExecutableState;

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

/// Builder state after a row-locking clause (`FOR UPDATE`, `FOR SHARE`, ...).
///
/// Only `.nowait()` and `.skip_locked()` can follow.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectForSet;

//------------------------------------------------------------------------------
// Join macros (generates all join variants)
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
        // USING variants only for non-natural, non-cross joins
        join_using_impl!(left, drizzle_core::LeftJoin);
        join_using_impl!(left_outer, drizzle_core::LeftJoin);
        join_using_impl!(right, drizzle_core::RightJoin);
        join_using_impl!(right_outer, drizzle_core::RightJoin);
        join_using_impl!(full, drizzle_core::FullJoin);
        join_using_impl!(full_outer, drizzle_core::FullJoin);
        join_using_impl!(inner, drizzle_core::InnerJoin);
        join_using_impl!(); // Plain JOIN
    };
    (@natural $type:ident, $join_expr:expr, $kind:ty) => {
        paste! {
            /// Adds a `NATURAL` join of the kind named by the method.
            ///
            /// The database joins on every column name both sides share, so
            /// this takes only a table or other source, with no `ON` condition.
            #[allow(clippy::type_complexity)]
            pub fn [<$type _join>]<J: crate::helpers::JoinSource<'a>>(
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
            /// Adds a join of the kind named by the method, with an `ON` condition.
            ///
            /// Pass `(source, condition)`. For `LEFT`, `RIGHT` and `FULL`
            /// joins, the outer-joined side's columns become nullable in the
            /// result row type.
            pub fn [<$type _join>]<J: crate::helpers::JoinArg<'a, T>>(
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

macro_rules! join_using_impl {
    () => {
        /// Adds `JOIN table USING (columns)`, joining on equal values of
        /// same-named columns.
        ///
        /// `columns` is rendered as given, and `PostgreSQL` requires bare
        /// names here: pass `SQL::ident("id")`, not a table column, which
        /// renders qualified.
        pub fn join_using<U: PostgresTable<'a>>(
            self,
            table: U,
            columns: impl ToSQL<'a, PostgresValue<'a>>,
        ) -> SelectBuilder<
            'a,
            S,
            SelectJoinSet,
            U,
            <M as drizzle_core::JoinStep<R, U, drizzle_core::InnerJoin>>::Marker,
            <M as drizzle_core::JoinStep<R, U, drizzle_core::InnerJoin>>::Row,
            G,
        >
        where
            M: drizzle_core::JoinStep<R, U, drizzle_core::InnerJoin>,
        {
            SelectBuilder {
                sql: self.sql.append(helpers::join_using(table, columns)),
                schema: PhantomData,
                state: PhantomData,
                table: PhantomData,
                marker: PhantomData,
                row: PhantomData,
                grouped: PhantomData,
            }
        }
    };
    ($type:ident, $kind:ty) => {
        paste! {
            /// Adds a join of the kind named by the method, with
            /// `USING (columns)`: joins on equal values of same-named columns.
            ///
            /// Pass bare column names, such as `SQL::ident("id")`; see
            /// [`join_using`](Self::join_using).
            pub fn [<$type _join_using>]<U: PostgresTable<'a>>(
                self,
                table: U,
                columns: impl ToSQL<'a, PostgresValue<'a>>,
            ) -> SelectBuilder<
                'a,
                S,
                SelectJoinSet,
                U,
                <M as drizzle_core::JoinStep<R, U, $kind>>::Marker,
                <M as drizzle_core::JoinStep<R, U, $kind>>::Row,
                G,
            >
            where
                M: drizzle_core::JoinStep<R, U, $kind>,
            {
                SelectBuilder {
                    sql: self.sql.append(helpers::[<$type _join_using>](table, columns)),
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
// Capability trait impls for each state
//------------------------------------------------------------------------------

impl ExecutableState for SelectForSet {}
// A locking SELECT still feeds a derived table or INSERT ... SELECT, but
// cannot be a set operand.
impl drizzle_core::ClauseAllowed<drizzle_core::clause::Source> for SelectForSet {}

//------------------------------------------------------------------------------
// SelectBuilder Definition
//------------------------------------------------------------------------------

/// A `PostgreSQL` `SELECT` being built: a [`QueryBuilder`](super::QueryBuilder)
/// in one of the `Select*` states.
///
/// `State` limits which clause can come next. `Table`, `Marker`, `Row` and
/// `Grouped` track the sources in scope, the selected columns, the row type
/// and the `GROUP BY` columns, so the compiler can reject columns that are
/// not in scope or not grouped.
///
/// # Examples
///
/// ```rust
/// # extern crate self as drizzle;
/// # mod _drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
/// #     pub mod postgres {
/// #         pub mod values { pub use drizzle_postgres::values::*; }
/// #         pub mod traits { pub use drizzle_postgres::traits::*; }
/// #         pub mod common { pub use drizzle_postgres::common::*; }
/// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
/// #         pub mod builder { pub use drizzle_postgres::builder::*; }
/// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
/// #         pub mod expr { pub use drizzle_postgres::expr::*; }
/// #         pub mod types { pub use drizzle_postgres::types::*; }
/// #         #[cfg(feature = "aws-data-api")]
/// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
/// #         pub struct Row;
/// #         impl Row {
/// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
/// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
/// #         }
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
/// #             pub use drizzle_postgres::attrs::*;
/// #             pub use drizzle_postgres::common::PostgresSchemaType;
/// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
/// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # pub use _drizzle::*;
/// # pub use const_format;
/// # fn main() {
/// # use drizzle::postgres::prelude::*;
/// # use drizzle::postgres::builder::QueryBuilder;
/// # #[PostgresTable(name = "users")]
/// # struct User {
/// #     #[column(serial, primary)]
/// #     id: i32,
/// #     name: String,
/// #     email: Option<String>,
/// # }
/// # #[PostgresTable(name = "posts")]
/// # struct Post {
/// #     #[column(serial, primary)]
/// #     id: i32,
/// #     #[column(references = User::id)]
/// #     author_id: i32,
/// #     title: String,
/// # }
/// # #[derive(PostgresSchema)]
/// # struct Schema {
/// #     user: User,
/// #     post: Post,
/// # }
/// # let db = QueryBuilder::new::<Schema>();
/// # let Schema { user, post } = Schema::new();
/// use drizzle::core::desc;
/// use drizzle::core::expr::eq;
///
/// let query = db
///     .select((user.id, user.name))
///     .from(user)
///     .r#where(eq(user.name, "Alice"))
///     .order_by(desc(user.id))
///     .limit(10)
///     .offset(20);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"SELECT "users"."id", "users"."name" FROM "users" WHERE "users"."name" = $1 ORDER BY "users"."id" DESC LIMIT $2 OFFSET $3"#
/// );
/// # }
/// ```
pub type SelectBuilder<'a, Schema, State, Table = (), Marker = (), Row = (), Grouped = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>;

//------------------------------------------------------------------------------
// Initial State: .from()
//------------------------------------------------------------------------------

impl<'a, S, M> SelectBuilder<'a, S, SelectInitial, (), M> {
    /// Sets the `FROM` source: a table, a view, an aliased table, a derived
    /// table (`.alias(...)`), or a CTE.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = db.select(()).from(user);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users""#
    /// );
    /// # }
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
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::ScopeEntry,
        M: drizzle_core::ResolveRow<T>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::from(query)),
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
    /// Adds an inner `JOIN ... ON ...`.
    ///
    /// Pass `(source, condition)`. Other join kinds have their own methods:
    /// `left_join`, `right_join`, `full_join`, `inner_join`, their `_outer`
    /// forms, `natural_*` joins, `*_join_using`, and the `*_lateral` joins.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// use drizzle::core::expr::eq;
    ///
    /// let query = db
    ///     .select((user.name, post.title))
    ///     .from(user)
    ///     .join((post, eq(post.author_id, user.id)));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name", "posts"."title" FROM "users" JOIN "posts" ON "posts"."author_id" = "users"."id""#
    /// );
    /// # }
    /// ```
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
        <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>>::Marker,
        <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>>::Row,
        G,
    >
    where
        M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>,
{
        use drizzle_core::Join;
        SelectBuilder {
            sql: self.sql.append(arg.into_join_sql(Join::new())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    join_impl!();

    /// Adds a `CROSS JOIN`: every row of the left side paired with every row
    /// of `source`.
    ///
    /// For backwards compatibility, `(source, condition)` is also accepted and
    /// renders the equivalent `INNER JOIN ... ON ...`.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = db.select((user.name, post.title)).from(user).cross_join(post);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name", "posts"."title" FROM "users" CROSS JOIN "posts""#
    /// );
    /// # }
    /// ```
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn cross_join<Arg: crate::helpers::CrossJoinArg<'a, T>>(
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

    /// Adds `INNER JOIN LATERAL (subquery) AS alias ON condition`.
    ///
    /// A lateral subquery can refer to columns of the sources joined before
    /// it. Pass `(derived_table, condition)`, where the derived table comes
    /// from `.alias(tag)` on a `SELECT`.
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn inner_join_lateral<J>(
        self,
        arg: J,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        J::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            J::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
            J::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            J::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
            J::OnSources,
        >>::Row,
        G,
    >
    where
        J: drizzle_core::LateralArg<'a, PostgresValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                J::JoinedTable,
                drizzle_core::Lateral<drizzle_core::InnerJoin>,
                J::OnSources,
            >,
    {
        use drizzle_core::Join;
        SelectBuilder {
            sql: self.sql.append(arg.into_lateral_sql(Join::new().inner())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `LEFT JOIN LATERAL (subquery) AS alias ON condition`.
    ///
    /// Like [`inner_join_lateral`](Self::inner_join_lateral), but keeps
    /// left rows with no match; the subquery's columns become nullable.
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn left_join_lateral<J, SelectionProof>(
        self,
        arg: J,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        J::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            J::JoinedTable,
            drizzle_core::Lateral<drizzle_core::LeftJoin>,
            J::OnSources,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            J::JoinedTable,
            drizzle_core::Lateral<drizzle_core::LeftJoin>,
            J::OnSources,
        >>::Row,
        G,
    >
    where
        J: drizzle_core::LateralArg<'a, PostgresValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                J::JoinedTable,
                drizzle_core::Lateral<drizzle_core::LeftJoin>,
                J::OnSources,
            > + drizzle_core::LeftLateralSelection<SelectionProof>,
    {
        use drizzle_core::Join;
        SelectBuilder {
            sql: self.sql.append(arg.into_lateral_sql(Join::new().left())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `CROSS JOIN LATERAL (subquery) AS alias`, with no `ON` condition.
    ///
    /// The subquery runs once per left row and can refer to its columns.
    #[inline]
    #[allow(clippy::type_complexity)]
    pub fn cross_join_lateral<Source>(
        self,
        source: Source,
    ) -> SelectBuilder<
        'a,
        S,
        SelectJoinSet,
        Source::JoinedTable,
        <M as drizzle_core::JoinStep<
            R,
            Source::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
        >>::Marker,
        <M as drizzle_core::JoinStep<
            R,
            Source::JoinedTable,
            drizzle_core::Lateral<drizzle_core::InnerJoin>,
        >>::Row,
        G,
    >
    where
        Source: drizzle_core::LateralSource<'a, PostgresValue<'a>>,
        M: drizzle_core::JoinStep<
                R,
                Source::JoinedTable,
                drizzle_core::Lateral<drizzle_core::InnerJoin>,
            >,
    {
        SelectBuilder {
            sql: self.sql.append(source.into_cross_lateral_sql()),
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
    /// Adds a `WHERE` condition.
    ///
    /// The condition must be boolean, such as `eq(...)`, `and(...)` or a
    /// `boolean` column. Every column it uses must come from a source in
    /// `FROM` or a join; this is checked when the query is run.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// use drizzle::core::expr::{and, eq, gt};
    ///
    /// let query = db
    ///     .select(user.id)
    ///     .from(user)
    ///     .r#where(and(eq(user.name, "Alice"), gt(user.id, 10)));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."id" FROM "users" WHERE ("users"."name" = $1 AND "users"."id" > $2)"#
    /// );
    /// # }
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
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
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
    /// Adds a `GROUP BY` list: one expression or a tuple of them.
    ///
    /// Non-aggregate columns in the `SELECT` list must appear in the
    /// `GROUP BY` list, with one exception: grouping by a table's
    /// single-column primary key determines the whole row, so any column of
    /// that table may be selected without being listed (`PostgreSQL` allows
    /// this too).
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// use drizzle::core::expr::{count, gt};
    ///
    /// let query = db
    ///     .select((post.author_id, count(post.id)))
    ///     .from(post)
    ///     .group_by(post.author_id)
    ///     .having(gt(count(post.id), 5));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "posts"."author_id", COUNT ("posts"."id") FROM "posts" GROUP BY "posts"."author_id" HAVING COUNT ("posts"."id")> $1"#
    /// );
    /// # }
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
        Gr: drizzle_core::IntoGroupBy<'a, PostgresValue<'a>>,
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
    /// Adds a `HAVING` condition, which filters groups after `GROUP BY`.
    ///
    /// See [`group_by`](Self::group_by) for an example.
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
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
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
    /// Adds `ORDER BY`.
    ///
    /// Pass a column (ascending by default), `asc(col)` / `desc(col)`, or an
    /// array or tuple of them.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// use drizzle::core::{asc, desc};
    ///
    /// let query = db
    ///     .select(user.name)
    ///     .from(user)
    ///     .order_by([asc(user.name), desc(user.id)]);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name" FROM "users" ORDER BY "users"."name" ASC, "users"."id" DESC"#
    /// );
    /// # }
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
        TOrderBy: ToSQL<'a, PostgresValue<'a>> + drizzle_core::expr::ExprSources,
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
    /// Adds `ORDER BY` to a compound (`UNION` / `INTERSECT` / `EXCEPT`) query.
    ///
    /// The terms sort the combined output, so column references render
    /// unqualified (`"name"`, not `"users"."name"`), as `PostgreSQL` requires.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = db
    ///     .select(user.name)
    ///     .from(user)
    ///     .union_all(db.select(post.title).from(post))
    ///     .order_by(user.name);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name" FROM "users" UNION ALL SELECT "posts"."title" FROM "posts" ORDER BY "name""#
    /// );
    /// # }
    /// ```
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T, M, R, G>
    where
        TOrderBy: ToSQL<'a, PostgresValue<'a>>,
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
    /// Adds `LIMIT n`, returning at most `n` rows.
    ///
    /// `n` is a non-negative integer (bound as a parameter) or a placeholder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = db.select(user.id).from(user).limit(5);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."id" FROM "users" LIMIT $1"#);
    /// # }
    /// ```
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
        P: drizzle_core::PaginationArg<'a, PostgresValue<'a>>,
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

// OFFSET (available from SelectFromSet, SelectLimitSet, SelectSetOpSet)
impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Offset>,
{
    /// Adds `OFFSET n`, skipping the first `n` rows.
    ///
    /// `n` is a non-negative integer (bound as a parameter) or a placeholder.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = db.select(user.id).from(user).limit(10).offset(20);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."id" FROM "users" LIMIT $1 OFFSET $2"#
    /// );
    /// # }
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
        P: drizzle_core::PaginationArg<'a, PostgresValue<'a>>,
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
{
    /// Turns this `SELECT` into a derived table, `(SELECT ...) AS tag`.
    ///
    /// The result can be used in `.from(...)` or a lateral join; read its
    /// columns with `.fields()`.
    ///
    /// # Panics
    ///
    /// Panics when the projection contains duplicate output names. Name a
    /// computed expression with [`drizzle_core::expr::AliasExt::named`] to
    /// make each output unique.
    #[inline]
    #[must_use]
    pub fn alias<Tag, AggProof>(
        self,
        _tag: Tag,
    ) -> drizzle_core::Derived<
        'a,
        PostgresValue<'a>,
        Tag,
        <M as drizzle_core::DerivedSelection<
            'a,
            PostgresValue<'a>,
            PostgresSchemaType,
            T,
        >>::Projection,
        Self,
    >
    where
        Tag: drizzle_core::Tag,
        M: drizzle_core::DerivedSelection<'a, PostgresValue<'a>, PostgresSchemaType, T>
            + drizzle_core::row::MarkerAggValidFor<G, AggProof>,
        <M as drizzle_core::DerivedSelection<
            'a,
            PostgresValue<'a>,
            PostgresSchemaType,
            T,
        >>::Projection: drizzle_core::DerivedProjection<Tag>,
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
    T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
    /// Turns this `SELECT` into a common table expression named `Tag::NAME`.
    ///
    /// The result has the same columns as the `FROM` table. Pass it to
    /// `QueryBuilder::with`, and read its columns through its `.table` field;
    /// see [`QueryBuilder::with`](super::QueryBuilder::with) for an example.
    #[inline]
    #[must_use]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::Aliased<Tag>,
        Self,
    > {
        let name = Tag::NAME;
        super::CTEView::new(
            <T as SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>>::alias::<Tag>(),
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
    /// Combines this query with `other` using `UNION`, which drops duplicate rows.
    ///
    /// Both queries must select the same number of columns with compatible
    /// types. The `union_all`, `intersect`, `intersect_all`, `except` and
    /// `except_all` methods work the same way.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = db
    ///     .select(user.name)
    ///     .from(user)
    ///     .union(db.select(post.title).from(post));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name" FROM "users" UNION SELECT "posts"."title" FROM "posts""#
    /// );
    /// # }
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

    /// Combines this query with `other` using `UNION ALL`, which keeps duplicates.
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

    /// Keeps the rows present in both queries (`INTERSECT`), without duplicates.
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

    /// Keeps the rows present in both queries (`INTERSECT ALL`), with duplicates.
    #[allow(clippy::type_complexity)]
    pub fn intersect_all<M2>(
        self,
        other: impl IntoSelect<'a, S, M2, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, <M as drizzle_core::SetOperand<M2>>::Combined, R, G>
    where
        M: drizzle_core::SetOperand<M2>,
    {
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

    /// Keeps the rows of this query that are not in `other` (`EXCEPT`), without duplicates.
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

    /// Keeps the rows of this query that are not in `other` (`EXCEPT ALL`), with duplicates.
    #[allow(clippy::type_complexity)]
    pub fn except_all<M2>(
        self,
        other: impl IntoSelect<'a, S, M2, R>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T, <M as drizzle_core::SetOperand<M2>>::Combined, R, G>
    where
        M: drizzle_core::SetOperand<M2>,
    {
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

impl<'a, S, State, T, M, R, G> drizzle_core::expr::Expr<'a, PostgresValue<'a>>
    for SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
    M: drizzle_core::expr::SubqueryType<'a, PostgresValue<'a>> + drizzle_core::SelectSources,
{
    type SQLType = <M as drizzle_core::expr::SubqueryType<'a, PostgresValue<'a>>>::SQLType;
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

/// A query that can be the right-hand side of `UNION`, `INTERSECT` or `EXCEPT`.
///
/// Implemented by [`SelectBuilder`] and by the driver crates' query wrappers.
pub trait IntoSelect<'a, S, M, R> {
    /// The builder state of the converted query.
    type State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>;
    /// The `FROM` table of the converted query.
    type Table;
    /// Returns the query as a [`SelectBuilder`].
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

/// A complete `SELECT` that can supply rows to `INSERT ... SELECT`.
#[doc(hidden)]
pub trait CompletedSelect<'a, S, R>: insert_select_private::Sealed {
    type Marker;
    type Grouped;

    fn into_select_sql(self) -> drizzle_core::SQL<'a, PostgresValue<'a>>;
}

/// Converts a complete `SELECT`, or a driver wrapper around one, into a
/// [`CompletedSelect`].
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

    fn into_select_sql(self) -> drizzle_core::SQL<'a, PostgresValue<'a>> {
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

//------------------------------------------------------------------------------
// FOR UPDATE/SHARE Row Locking (PostgreSQL-specific)
//------------------------------------------------------------------------------

impl<'a, S, State, T, M, R, G> SelectBuilder<'a, S, State, T, M, R, G>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple>,
{
    /// Adds `FOR UPDATE`, locking the selected rows against updates and
    /// deletes by other transactions until this one ends.
    ///
    /// Follow with [`nowait`](Self::nowait) or [`skip_locked`](Self::skip_locked)
    /// to change what happens when a row is already locked.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = db.select(user.id).from(user).for_update().skip_locked();
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."id" FROM "users" FOR UPDATE SKIP LOCKED"#
    /// );
    /// # }
    /// ```
    #[must_use]
    pub fn for_update(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R, G> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_update()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `FOR SHARE`: other transactions can still read and share-lock the
    /// rows, but cannot update or delete them.
    #[must_use]
    pub fn for_share(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R, G> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_share()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `FOR NO KEY UPDATE`: like `FOR UPDATE`, but does not block
    /// `FOR KEY SHARE` locks (for example, foreign-key checks).
    #[must_use]
    pub fn for_no_key_update(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R, G> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_no_key_update()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `FOR KEY SHARE`: blocks deletes and key changes, but allows other
    /// updates.
    #[must_use]
    pub fn for_key_share(self) -> SelectBuilder<'a, S, SelectForSet, T, M, R, G> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_key_share()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `FOR UPDATE OF table`, locking rows of that table only.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         #[cfg(feature = "aws-data-api")]
    /// #         pub mod aws_data_api { pub use drizzle_postgres::aws_data_api::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// # fn main() {
    /// # use drizzle::postgres::prelude::*;
    /// # use drizzle::postgres::builder::QueryBuilder;
    /// # #[PostgresTable(name = "users")]
    /// # struct User {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     name: String,
    /// #     email: Option<String>,
    /// # }
    /// # #[PostgresTable(name = "posts")]
    /// # struct Post {
    /// #     #[column(serial, primary)]
    /// #     id: i32,
    /// #     #[column(references = User::id)]
    /// #     author_id: i32,
    /// #     title: String,
    /// # }
    /// # #[derive(PostgresSchema)]
    /// # struct Schema {
    /// #     user: User,
    /// #     post: Post,
    /// # }
    /// # let db = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// use drizzle::core::expr::eq;
    ///
    /// let query = db
    ///     .select((user.id, post.id))
    ///     .from(user)
    ///     .join((post, eq(post.author_id, user.id)))
    ///     .for_update_of(user);
    /// assert!(query.to_sql().sql().ends_with(r#"FOR UPDATE OF "users""#));
    /// # }
    /// ```
    pub fn for_update_of<U: PostgresTable<'a>>(
        self,
        table: U,
    ) -> SelectBuilder<'a, S, SelectForSet, T, M, R, G> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_update_of(table.name())),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `FOR SHARE OF table`, locking rows of that table only.
    pub fn for_share_of<U: PostgresTable<'a>>(
        self,
        table: U,
    ) -> SelectBuilder<'a, S, SelectForSet, T, M, R, G> {
        SelectBuilder {
            sql: self.sql.append(helpers::for_share_of(table.name())),
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
// Post-FOR State Implementation (NOWAIT / SKIP LOCKED)
//------------------------------------------------------------------------------

impl<S, T, M, R, G> SelectBuilder<'_, S, SelectForSet, T, M, R, G> {
    /// Adds `NOWAIT`: the query fails at once instead of waiting when a row
    /// is already locked.
    #[must_use]
    pub fn nowait(self) -> Self {
        SelectBuilder {
            sql: self.sql.append(helpers::nowait()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `SKIP LOCKED`: rows already locked by another transaction are
    /// left out of the result instead of waited for.
    #[must_use]
    pub fn skip_locked(self) -> Self {
        SelectBuilder {
            sql: self.sql.append(helpers::skip_locked()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
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
            grouped: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "SELECT *");
    }
}
