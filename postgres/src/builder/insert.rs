use crate::traits::PostgresTable;
use crate::values::PostgresValue;
use core::marker::PhantomData;
use drizzle_core::builder::{
    OnConflictBuilder as CoreOnConflictBuilder, OnConflictOutput, PostgresConflictTarget,
};
use drizzle_core::{
    ConflictTarget, InsertSelectCompatible, InsertSelectTable, InsertTargetColumns,
    NamedConstraint, PartialInsertSelectCompatible, SQL, ToSQL, Token,
};

use super::select::{CompletedSelect, IntoSelectQuery};

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{
    InsertColumnsSet, InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet,
    InsertValuesSet,
};

//------------------------------------------------------------------------------
// OnConflictBuilder
//------------------------------------------------------------------------------

/// The `ON CONFLICT` target of an `INSERT`, waiting for its action.
///
/// Created by [`InsertBuilder::on_conflict()`] or
/// [`InsertBuilder::on_conflict_on_constraint()`].
/// Call [`do_nothing()`](Self::do_nothing) or [`do_update()`](Self::do_update)
/// to complete the clause.
pub type OnConflictBuilder<'a, S, T> = CoreOnConflictBuilder<
    'a,
    PostgresValue<'a>,
    S,
    T,
    PostgresConflictTarget<'a, PostgresValue<'a>>,
    PostgresOnConflictOutput,
>;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PostgresOnConflictOutput;

impl<'a, S, T> OnConflictOutput<'a, PostgresValue<'a>, S, T> for PostgresOnConflictOutput {
    type OnConflictSet = InsertBuilder<'a, S, InsertOnConflictSet, T>;
    type DoUpdateSet = InsertBuilder<'a, S, InsertDoUpdateSet, T>;

    fn on_conflict(sql: SQL<'a, PostgresValue<'a>>) -> Self::OnConflictSet {
        InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    fn do_update(sql: SQL<'a, PostgresValue<'a>>) -> Self::DoUpdateSet {
        InsertBuilder {
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
// InsertBuilder Definition
//------------------------------------------------------------------------------

/// A `PostgreSQL` `INSERT` being built: a [`QueryBuilder`](super::QueryBuilder)
/// in one of the `Insert*` states.
///
/// Rows come from `.values(...)` (the table's generated `Insert*` model) or
/// from `.select(...)`. Then add `ON CONFLICT` handling and `RETURNING` as
/// needed. `State` allows only these steps, in this order.
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
///     .insert(user)
///     .values([InsertUser::new("Alice"), InsertUser::new("Bob")])
///     .on_conflict_do_nothing()
///     .returning(user.id);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"INSERT INTO "users" ("name") VALUES ($1), ($2) ON CONFLICT DO NOTHING RETURNING "users"."id""#
/// );
/// # }
/// ```
pub type InsertBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as drizzle_core::IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as drizzle_core::IntoSelectTarget>::Marker as drizzle_core::ResolveRow<Table>>::Row;

type ReturningBuilder<'a, S, T, Columns> = InsertBuilder<
    'a,
    S,
    InsertReturningSet,
    T,
    ReturningMarker<T, Columns>,
    ReturningRow<T, Columns>,
>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> InsertBuilder<'a, Schema, InsertInitial, Table>
where
    Table: PostgresTable<'a>,
{
    /// Inserts one row. Shorthand for `.values([row])`.
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
    /// let query = db.insert(user).value(InsertUser::new("Alice"));
    /// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" ("name") VALUES ($1)"#);
    /// # }
    /// ```
    #[inline]
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table> {
        self.values([value])
    }

    /// Inserts the given rows, built with the table's `Insert*` model.
    ///
    /// All rows must set the same columns, so they have the same model type.
    /// Columns not set use their database default.
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
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice"), InsertUser::new("Bob")]);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name") VALUES ($1), ($2)"#
    /// );
    /// # }
    /// ```
    #[inline]
    pub fn values<I, T>(self, values: I) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        I: IntoIterator<Item = Table::Insert<T>>,
    {
        let sql = crate::helpers::values::<'a, Table, T>(values);
        InsertBuilder {
            sql: self.sql.append(sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Sets the target columns, in order, for `INSERT ... SELECT`.
    ///
    /// The list must include every required column (non-null without a
    /// default). Continue with [`select`](Self::select), whose columns must
    /// match these in number, order and type.
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
    ///     .insert(post)
    ///     .columns((post.author_id, post.title))
    ///     .select(db.select((user.id, user.name)).from(user));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "posts" ("author_id", "title") SELECT "users"."id", "users"."name" FROM "users""#
    /// );
    /// # }
    /// ```
    #[inline]
    pub fn columns<Columns>(
        self,
        columns: Columns,
    ) -> InsertBuilder<'a, Schema, InsertColumnsSet<Columns::Columns>, Table>
    where
        Columns: InsertTargetColumns<'a, PostgresValue<'a>, Table>,
    {
        InsertBuilder {
            sql: self.sql.append(columns.into_target_columns_sql()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Inserts the rows of a `SELECT` into every insertable column of the table.
    ///
    /// The query's columns must match the table's insertable columns in
    /// number, order, type and nullability; this is checked at compile time.
    /// To fill only some columns, call [`columns`](Self::columns) first.
    #[inline]
    pub fn select<Q, R, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Table: InsertSelectTable,
        Q: IntoSelectQuery<'a, Schema, R>,
        Q::Marker: InsertSelectCompatible<'a, PostgresValue<'a>, Table, R>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        let select = query.into_select_query().into_select_sql();
        InsertBuilder {
            sql: self
                .sql
                .append(Table::insert_columns_sql::<PostgresValue<'a>>())
                .append(select),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Inserts the rows of a raw `SELECT`, with no target column list.
    ///
    /// Nothing about the query is checked: not its column count, types,
    /// nullability, sources or aggregates. Prefer [`select`](Self::select).
    #[inline]
    pub fn select_raw<Q>(self, query: Q) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Q: ToSQL<'a, PostgresValue<'a>>,
    {
        InsertBuilder {
            sql: self.sql.append(query.into_sql()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

impl<'a, Schema, Table, Targets> InsertBuilder<'a, Schema, InsertColumnsSet<Targets>, Table>
where
    Table: PostgresTable<'a> + InsertSelectTable,
{
    /// Inserts the rows of a `SELECT` into the columns chosen with `.columns(...)`.
    ///
    /// The query's columns must match the target columns in number, order,
    /// type and nullability; this is checked at compile time.
    #[inline]
    pub fn select<Q, R, RequiredProof, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: IntoSelectQuery<'a, Schema, R>,
        Q::Marker: PartialInsertSelectCompatible<'a, PostgresValue<'a>, Targets>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        let select = query.into_select_query().into_select_sql();
        InsertBuilder {
            sql: self.sql.append(select),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Inserts the rows of a raw `SELECT` into the chosen target columns.
    ///
    /// Only the target list is checked (it must include every required
    /// column); the query itself is not. Prefer [`select`](Self::select).
    #[inline]
    pub fn select_raw<Q, RequiredProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: ToSQL<'a, PostgresValue<'a>>,
    {
        InsertBuilder {
            sql: self.sql.append(query.into_sql()),
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
// Post-VALUES Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertValuesSet, T> {
    /// Starts `ON CONFLICT (columns)`, handling rows that would violate a
    /// unique constraint on the target.
    ///
    /// The target is a primary-key column, a unique column, or a unique
    /// index (anything implementing `ConflictTarget<T>`, which the macros
    /// generate). Finish with `.do_nothing()` or `.do_update(update_model)`.
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
    /// fn main() {
    /// use drizzle::postgres::prelude::*;
    /// use drizzle::postgres::builder::QueryBuilder;
    ///
    /// #[PostgresTable(name = "users")]
    /// struct User {
    ///     #[column(serial, primary)]
    ///     id: i32,
    ///     name: String,
    ///     #[column(unique)]
    ///     email: Option<String>,
    /// }
    ///
    /// #[PostgresIndex(unique)]
    /// struct UserEmailIdx(User::email);
    ///
    /// #[derive(PostgresSchema)]
    /// struct Schema {
    ///     user: User,
    ///     user_email_idx: UserEmailIdx,
    /// }
    ///
    /// let builder = QueryBuilder::new::<Schema>();
    /// let schema = Schema::new();
    /// let user = schema.user;
    ///
    /// // Target a specific column
    /// builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict(user.id).do_nothing();
    ///
    /// // DO UPDATE with new values
    /// let query = builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict(user.email).do_update(UpdateUser::default().with_name("updated"));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name") VALUES ($1) ON CONFLICT ("email") DO UPDATE SET "name" = $2"#
    /// );
    ///
    /// // Target a unique index
    /// builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict(schema.user_email_idx).do_nothing();
    /// }
    /// ```
    pub fn on_conflict<C: ConflictTarget<T>>(self, target: C) -> OnConflictBuilder<'a, S, T> {
        let columns = target.conflict_columns();
        let target_where = target.conflict_where_clause().map(SQL::raw);
        let target_sql = SQL::join(columns.iter().map(|c| SQL::ident(*c)), Token::COMMA);
        OnConflictBuilder::new(self.sql, PostgresConflictTarget::columns(target_sql))
            .with_target_where_sql(target_where)
    }

    /// Starts `ON CONFLICT ON CONSTRAINT name`, naming the unique constraint
    /// to handle.
    ///
    /// The target is a unique column or a named unique constraint (anything
    /// implementing `NamedConstraint<T>`, which the macros generate). A
    /// standalone unique index is not a constraint, so `PostgreSQL` rejects it
    /// here; use [`on_conflict`](Self::on_conflict) for indexes. Finish with
    /// `.do_nothing()` or `.do_update(update_model)`.
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
    /// fn main() {
    /// use drizzle::postgres::prelude::*;
    /// use drizzle::postgres::builder::QueryBuilder;
    ///
    /// #[PostgresTable(name = "users")]
    /// struct User {
    ///     #[column(serial, primary)]
    ///     id: i32,
    ///     name: String,
    ///     #[column(unique)]
    ///     email: Option<String>,
    /// }
    ///
    /// #[derive(PostgresSchema)]
    /// struct Schema {
    ///     user: User,
    /// }
    ///
    /// let builder = QueryBuilder::new::<Schema>();
    /// let schema = Schema::new();
    ///
    /// let user = schema.user;
    /// builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict_on_constraint(user.email).do_nothing();
    /// }
    /// ```
    pub fn on_conflict_on_constraint<C: NamedConstraint<T>>(
        self,
        target: C,
    ) -> OnConflictBuilder<'a, S, T> {
        OnConflictBuilder::new(
            self.sql,
            PostgresConflictTarget::constraint(target.constraint_name()),
        )
    }

    /// Adds `ON CONFLICT DO NOTHING` with no target: rows that violate any
    /// unique or exclusion constraint are skipped.
    #[must_use]
    pub fn on_conflict_do_nothing(self) -> InsertBuilder<'a, S, InsertOnConflictSet, T> {
        let conflict_sql = SQL::from_iter([Token::ON, Token::CONFLICT, Token::DO, Token::NOTHING]);
        InsertBuilder {
            sql: self.sql.append(conflict_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `RETURNING columns`, so the statement returns the inserted rows.
    ///
    /// Pass a column, a tuple of columns, or `()` for all columns. Only
    /// columns of the target table are allowed.
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
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice")])
    ///     .returning((user.id, user.name));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name") VALUES ($1) RETURNING "users"."id", "users"."name""#
    /// );
    /// # }
    /// ```
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
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
// Post-ON CONFLICT Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertOnConflictSet, T> {
    /// Adds `RETURNING columns` after the `ON CONFLICT` clause.
    ///
    /// Rows skipped by `DO NOTHING` are not returned.
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
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
// Post-DO UPDATE SET Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertDoUpdateSet, T> {
    /// Adds a `WHERE` condition to `DO UPDATE`: conflicting rows are updated
    /// only when it holds.
    ///
    /// Renders `ON CONFLICT (...) DO UPDATE SET ... WHERE condition`. The
    /// condition may only use columns of the target table.
    pub fn r#where<E, ScopeProof>(
        self,
        condition: E,
    ) -> InsertBuilder<'a, S, InsertOnConflictSet, T>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let sql = self
            .sql
            .push(Token::WHERE)
            .append(condition.into_expr_sql());
        InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds `RETURNING columns` after `DO UPDATE SET`.
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
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
    fn test_insert_builder_creation() {
        let builder = InsertBuilder::<(), InsertInitial, ()> {
            sql: SQL::raw("INSERT INTO test"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "INSERT INTO test");
    }
}
