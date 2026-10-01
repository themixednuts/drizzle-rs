use drizzle_core::Token;
// Re-export common enums and traits from core
pub use drizzle_core::builder::{BuilderInit, ExecutableState};
pub use drizzle_core::{
    OrderBy, SQL, ToSQL,
    traits::{SQLSchema, SQLTable},
};

// Local imports
use crate::{common::PostgresSchemaType, traits::PostgresTable, values::PostgresValue};
use core::{fmt::Debug, marker::PhantomData};

// Import modules - these provide specific builder types
pub mod cte;
pub mod delete;
pub mod insert;
pub mod prepared;
pub mod refresh;
pub mod select;
pub mod update;

// Re-export CTE types
pub use cte::{CTEDefinition, CTEView};

// Export state markers for easier use
pub use delete::{DeleteInitial, DeleteReturningSet, DeleteWhereSet};
pub use insert::{
    InsertColumnsSet, InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet,
    InsertValuesSet, OnConflictBuilder,
};
pub use refresh::{
    RefreshConcurrently, RefreshInitial, RefreshMaterializedView, RefreshWithNoData,
    refresh_materialized_view,
};
pub use select::{
    SelectForSet, SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet,
    SelectOffsetSet, SelectOrderSet, SelectSetOpSet, SelectWhereSet,
};
pub use update::{
    UpdateFromSet, UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
};

// Re-export SQLViewInfo for convenience when using refresh_materialized_view
pub use drizzle_core::traits::SQLViewInfo;

/// Builder state after `.with(cte)`: the next call starts the main statement.
#[derive(Debug, Clone)]
pub struct CTEInit;

impl ExecutableState for CTEInit {}

/// Builds `PostgreSQL` statements as SQL plus bound parameters, without a connection.
///
/// Create one with [`QueryBuilder::new`], then start a statement with
/// [`select`](QueryBuilder::select), [`insert`](QueryBuilder::insert),
/// [`update`](QueryBuilder::update), [`delete`](QueryBuilder::delete) or
/// [`with`](QueryBuilder::with). Each call returns a builder whose type
/// tracks the clauses added so far, so out-of-order clauses (such as
/// `.where()` before `.from()`) do not compile. Call `.to_sql()` to get the
/// statement. The driver crates wrap this builder to run the query.
///
/// `Schema` is the schema type from `#[derive(PostgresSchema)]`. The other
/// type parameters are builder state and are inferred.
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
///     .select((user.id, user.name))
///     .from(user)
///     .r#where(eq(user.name, "Alice"));
///
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"SELECT "users"."id", "users"."name" FROM "users" WHERE "users"."name" = $1"#
/// );
/// # }
/// ```
#[derive(Debug, Clone, Default)]
pub struct QueryBuilder<
    'a,
    Schema = (),
    State = (),
    Table = (),
    Marker = (),
    Row = (),
    Grouped = (),
> {
    /// The SQL built so far.
    pub sql: SQL<'a, PostgresValue<'a>>,
    schema: PhantomData<Schema>,
    state: PhantomData<State>,
    table: PhantomData<Table>,
    marker: PhantomData<Marker>,
    row: PhantomData<Row>,
    grouped: PhantomData<Grouped>,
}

//------------------------------------------------------------------------------
// QueryBuilder Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, State, Table, Marker, Row, Grouped> ToSQL<'a, PostgresValue<'a>>
    for QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>
{
    fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
        self.sql.clone()
    }
}

impl<'a, Schema, State, Table, Marker, Row, Grouped>
    QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>
where
    State: ExecutableState,
{
    /// Prepends a [sqlcommenter](https://google.github.io/sqlcommenter/) comment
    /// to the statement.
    ///
    /// The text is wrapped in `/* ... */`. Any `/*` or `*/` in the input is
    /// neutralised so it cannot end the comment early. Empty text adds nothing.
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
    /// let query = db.select(user.id).from(user).comment("list-users");
    /// assert_eq!(query.to_sql().sql(), r#"/*list-users*/ SELECT "users"."id" FROM "users""#);
    /// # }
    /// ```
    #[must_use]
    pub fn comment(mut self, text: impl AsRef<str>) -> Self {
        let fragment = drizzle_core::sql::comment::<PostgresValue<'a>>(text);
        if fragment.chunks.is_empty() {
            return self;
        }
        let existing = core::mem::replace(&mut self.sql, fragment);
        self.sql.append_mut(existing);
        self
    }

    /// Prepends a key-value [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment to the statement.
    ///
    /// Each pair renders as `key='value'`, URL-encoded. Pairs are sorted by
    /// key, joined with `,` and wrapped in `/* ... */`. Pairs with an empty
    /// value are skipped; if none remain, nothing is added.
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
    ///     .select(user.id)
    ///     .from(user)
    ///     .comment_tags([("route", "/users"), ("action", "list")]);
    /// let sql = query.to_sql().sql();
    /// assert!(sql.starts_with("/*action='list',route='%2Fusers'*/"));
    /// # }
    /// ```
    #[must_use]
    pub fn comment_tags<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let fragment = drizzle_core::sql::comment_tags::<PostgresValue<'a>, _, _, _>(pairs);
        if fragment.chunks.is_empty() {
            return self;
        }
        let existing = core::mem::replace(&mut self.sql, fragment);
        self.sql.append_mut(existing);
        self
    }
}

impl<'a> QueryBuilder<'a> {
    /// Creates a query builder for schema `S`.
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
    /// # let Schema { user, post } = Schema::new();
    /// let db = QueryBuilder::new::<Schema>();
    /// let query = db.select(()).from(user);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."id", "users"."name", "users"."email" FROM "users""#
    /// );
    /// # }
    /// ```
    #[must_use]
    pub const fn new<S>() -> QueryBuilder<'a, S, BuilderInit> {
        QueryBuilder {
            sql: SQL::empty(),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

impl<'a, Schema> QueryBuilder<'a, Schema, BuilderInit> {
    /// Starts a `SELECT` of the given columns.
    ///
    /// Pass one column or expression, a tuple of them, or `()` for every
    /// column of the `FROM` table. Continue with `.from(table)`.
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
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let sql = crate::helpers::select(columns);
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts a `SELECT DISTINCT`, which drops duplicate rows.
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
    /// let query = db.select_distinct(user.name).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT DISTINCT "users"."name" FROM "users""#);
    /// # }
    /// ```
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let sql = crate::helpers::select_distinct(columns);
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts a `SELECT DISTINCT ON (...)`, which keeps one row per distinct
    /// value of `on`.
    ///
    /// The first row of each group wins, so add an `order_by` that starts
    /// with the `on` columns to choose which row is kept.
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
    ///     .select_distinct_on(user.email, (user.id, user.email))
    ///     .from(user)
    ///     .order_by(user.email);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT DISTINCT ON ("users"."email") "users"."id", "users"."email" FROM "users" ORDER BY "users"."email""#
    /// );
    /// # }
    /// ```
    pub fn select_distinct_on<On, Columns>(
        &self,
        on: On,
        columns: Columns,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), Columns::Marker>
    where
        On: ToSQL<'a, PostgresValue<'a>>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let sql = crate::helpers::select_distinct_on(on, columns);
        select::SelectBuilder {
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

impl<'a, Schema> QueryBuilder<'a, Schema, CTEInit> {
    /// Starts the main `SELECT` after `WITH ...`. See [`QueryBuilder::with`].
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let sql = self.sql.clone().append(crate::helpers::select(columns));
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts the main `SELECT DISTINCT` after `WITH ...`.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let sql = self
            .sql
            .clone()
            .append(crate::helpers::select_distinct(columns));
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts the main `SELECT DISTINCT ON (...)` after `WITH ...`.
    pub fn select_distinct_on<On, Columns>(
        &self,
        on: On,
        columns: Columns,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), Columns::Marker>
    where
        On: ToSQL<'a, PostgresValue<'a>>,
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let sql = self
            .sql
            .clone()
            .append(crate::helpers::select_distinct_on(on, columns));
        select::SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts the main `INSERT` after `WITH ...`.
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = self
            .sql
            .clone()
            .append(crate::helpers::insert::<Table>(&table));

        insert::InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts the main `UPDATE` after `WITH ...`.
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::update::<
            Table,
            PostgresSchemaType,
            PostgresValue<'a>,
        >(&table));

        update::UpdateBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts the main `DELETE` after `WITH ...`.
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::delete::<
            Table,
            PostgresSchemaType,
            PostgresValue<'a>,
        >(&table));

        delete::DeleteBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds another common table expression to the `WITH` list.
    #[must_use]
    pub fn with<C>(&self, cte: &C) -> Self
    where
        C: CTEDefinition<'a>,
    {
        let sql = self
            .sql
            .clone()
            .push(Token::COMMA)
            .append(cte.cte_definition());
        QueryBuilder {
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

impl<'a, Schema> QueryBuilder<'a, Schema, BuilderInit> {
    /// Starts an `INSERT INTO table`. Continue with `.values(...)` or `.select(...)`.
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
    /// let query = db.insert(user).values([InsertUser::new("Alice")]);
    /// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" ("name") VALUES ($1)"#);
    /// # }
    /// ```
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = crate::helpers::insert::<Table>(&table);

        insert::InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts an `UPDATE table`. Continue with `.set(...)`.
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
    ///     .update(user)
    ///     .set(UpdateUser::default().with_name("Bob"))
    ///     .r#where(eq(user.id, 1));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"UPDATE "users" SET "name" = $1 WHERE "users"."id" = $2"#
    /// );
    /// # }
    /// ```
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = crate::helpers::update::<Table, PostgresSchemaType, PostgresValue<'a>>(&table);

        update::UpdateBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts a `DELETE FROM table`.
    ///
    /// Without `.where(...)` the statement deletes every row.
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
    /// use drizzle::core::expr::gt;
    ///
    /// let query = db.delete(user).r#where(gt(user.id, 10));
    /// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "users" WHERE "users"."id" > $1"#);
    /// # }
    /// ```
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: PostgresTable<'a>,
    {
        let sql = crate::helpers::delete::<Table, PostgresSchemaType, PostgresValue<'a>>(&table);

        delete::DeleteBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Starts a `WITH` clause from a common table expression (CTE).
    ///
    /// Build the CTE with `SelectBuilder::into_cte::<Tag>()`, then reference
    /// it in the main statement through its `.table` field. Chain more
    /// `.with(...)` calls to add more CTEs.
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
    /// use drizzle::core::expr::gt;
    ///
    /// struct Recent;
    /// impl drizzle::core::Tag for Recent {
    ///     const NAME: &'static str = "recent";
    /// }
    ///
    /// let recent = db
    ///     .select((user.id, user.name))
    ///     .from(user)
    ///     .r#where(gt(user.id, 100))
    ///     .into_cte::<Recent>();
    /// let r = recent.table;
    ///
    /// let query = db.with(&recent).select(r.name).from(&recent);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"WITH "recent" AS (SELECT "users"."id", "users"."name" FROM "users" WHERE "users"."id" > $1) SELECT "recent"."name" FROM "recent""#
    /// );
    /// # }
    /// ```
    pub fn with<C>(&self, cte: &C) -> QueryBuilder<'a, Schema, CTEInit>
    where
        C: CTEDefinition<'a>,
    {
        let sql = SQL::from(Token::WITH).append(cte.cte_definition());
        QueryBuilder {
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

// Marker trait to indicate a query builder state is executable
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_builder_new() {
        let qb = QueryBuilder::new::<()>();
        let sql = qb.to_sql();
        assert_eq!(sql.sql(), "");
        assert_eq!(sql.params().count(), 0);
    }

    #[test]
    fn test_builder_init_type() {
        let _state = BuilderInit;
    }
}
