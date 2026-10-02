//! The typed `SQLite` query builder.
//!
//! [`QueryBuilder`] builds SELECT, INSERT, UPDATE and DELETE statements (with
//! optional common table expressions) and renders them to [`SQL`]. The
//! submodules hold each statement's builder states. The `drizzle` crate's
//! drivers wrap this builder to run the queries.

use drizzle_core::Token;
pub use drizzle_core::builder::{BuilderInit, ExecutableState};
pub use drizzle_core::{
    OrderBy, SQL, ToSQL,
    traits::{SQLSchema, SQLTable},
};

// Local imports
use crate::{common::SQLiteSchemaType, traits::SQLiteTable, values::SQLiteValue};
use core::{fmt::Debug, marker::PhantomData};

// Import modules - these provide specific builder types
pub mod cte;
pub mod delete;
pub mod insert;
pub mod prepared;
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
pub use select::{
    SelectFromSet, SelectGroupSet, SelectInitial, SelectJoinSet, SelectLimitSet, SelectOffsetSet,
    SelectOrderSet, SelectSetOpSet, SelectWhereSet,
};
pub use update::{UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet};

/// Builder state after [`QueryBuilder::with`]: the next call starts the
/// statement that uses the common table expressions.
#[derive(Debug, Clone)]
pub struct CTEInit;

impl ExecutableState for CTEInit {}

/// Type-safe SQL query builder for `SQLite`.
///
/// Start with [`QueryBuilder::new`], then call [`select`](Self::select),
/// [`insert`](Self::insert), [`update`](Self::update),
/// [`delete`](Self::delete) or [`with`](Self::with). Each method returns a
/// builder in a new state, and each state only offers the clauses that may
/// come next, so an out-of-order query does not compile. Call
/// [`ToSQL::to_sql`] to get the SQL text and its bound parameters.
///
/// [`SelectBuilder`](select::SelectBuilder),
/// [`InsertBuilder`](insert::InsertBuilder),
/// [`UpdateBuilder`](update::UpdateBuilder) and
/// [`DeleteBuilder`](delete::DeleteBuilder) are aliases of this type and
/// document the clause order of each statement.
///
/// # Type parameters
///
/// - `Schema`: the schema the query runs against.
/// - `State`: which clauses have been added so far (for example
///   [`SelectWhereSet`]).
/// - `Table`: the table the last FROM or JOIN added, or the target table of
///   an INSERT, UPDATE or DELETE.
/// - `Marker`: the selected columns and the tables in scope; used to check
///   column references and to infer the row type.
/// - `Row`: the Rust type of one result row.
/// - `Grouped`: the GROUP BY columns, used to check which columns may be
///   selected outside an aggregate.
///
/// # Examples
///
/// ```
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
/// use drizzle::sqlite::prelude::*;
/// use drizzle::sqlite::builder::QueryBuilder;
///
/// #[SQLiteTable(name = "users")]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
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
/// let query = builder.select(user.name).from(user);
/// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
/// ```
///
/// SELECT:
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
/// # use drizzle::core::expr::gt;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder.select((user.id, user.name)).from(user).r#where(gt(user.id, 10));
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"SELECT "users"."id", "users"."name" FROM "users" WHERE "users"."id" > ?"#
/// );
/// ```
///
/// INSERT:
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
///     .insert(user)
///     .values([InsertUser::new("Alice")]);
/// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" ("name") VALUES (?)"#);
/// ```
///
/// UPDATE:
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
/// # use drizzle::core::expr::eq;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .update(user)
///     .set(UpdateUser::default().with_name("Bob"))
///     .r#where(eq(user.id, 1));
/// assert_eq!(query.to_sql().sql(), r#"UPDATE "users" SET "name" = ? WHERE "users"."id" = ?"#);
/// ```
///
/// DELETE:
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
/// # use drizzle::core::expr::lt;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .delete(user)
///     .r#where(lt(user.id, 10));
/// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "users" WHERE "users"."id" < ?"#);
/// ```
///
/// Common table expressions (WITH). [`into_cte`](Self::into_cte) turns a
/// SELECT into a CTE whose columns you can reference like a table's:
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
/// # struct ActiveUsersTag;
/// # impl drizzle::core::Tag for ActiveUsersTag {
/// #     const NAME: &'static str = "active_users";
/// # }
/// let active_users = builder
///     .select((user.id, user.name))
///     .from(user)
///     .into_cte::<ActiveUsersTag>();
///
/// // The CTE derefs to an aliased `User` table, so its columns are typed.
/// let query = builder
///     .with(&active_users)
///     .select(active_users.name)
///     .from(&active_users);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"WITH "active_users" AS (SELECT "users"."id", "users"."name" FROM "users") SELECT "active_users"."name" FROM "active_users""#
/// );
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
    pub sql: SQL<'a, SQLiteValue<'a>>,
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

impl<'a, Schema, State, Table, Marker, Row, Grouped> ToSQL<'a, SQLiteValue<'a>>
    for QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>
{
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        self.sql.clone()
    }
}

impl<'a, Schema, State, Table, Marker, Row, Grouped>
    QueryBuilder<'a, Schema, State, Table, Marker, Row, Grouped>
where
    State: ExecutableState,
{
    /// Prepends a [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment (`/*...*/`) to the query.
    ///
    /// `/*` and `*/` inside `text` are broken up (`/ *`, `* /`) so the text
    /// cannot end the comment early. An empty `text` leaves the query
    /// unchanged.
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
    /// let query = builder.select(user.id).from(user).comment("list users");
    /// assert_eq!(query.to_sql().sql(), r#"/*list users*/ SELECT "users"."id" FROM "users""#);
    /// ```
    #[must_use]
    pub fn comment(mut self, text: impl AsRef<str>) -> Self {
        let fragment = drizzle_core::sql::comment::<SQLiteValue<'a>>(text);
        if fragment.chunks.is_empty() {
            return self;
        }
        let existing = core::mem::replace(&mut self.sql, fragment);
        self.sql.append_mut(existing);
        self
    }

    /// Prepends a tag-style [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment to the query.
    ///
    /// Each `(key, value)` pair is URL-encoded and written as `key='value'`.
    /// Pairs are sorted and joined with `,`. Pairs with an empty value are
    /// skipped; if none remain, the query is unchanged.
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
    ///     .select(user.id)
    ///     .from(user)
    ///     .comment_tags([("route", "/users"), ("action", "list")]);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"/*action='list',route='%2Fusers'*/ SELECT "users"."id" FROM "users""#
    /// );
    /// ```
    #[must_use]
    pub fn comment_tags<I, K, V>(mut self, pairs: I) -> Self
    where
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        let fragment = drizzle_core::sql::comment_tags::<SQLiteValue<'a>, _, _, _>(pairs);
        if fragment.chunks.is_empty() {
            return self;
        }
        let existing = core::mem::replace(&mut self.sql, fragment);
        self.sql.append_mut(existing);
        self
    }
}

impl<'a> QueryBuilder<'a> {
    /// Creates a query builder for the schema `S`.
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
    /// use drizzle::sqlite::prelude::*;
    /// use drizzle::sqlite::builder::QueryBuilder;
    ///
    /// #[SQLiteTable(name = "users")]
    /// struct User {
    ///     #[column(primary)]
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// #[derive(SQLiteSchema)]
    /// struct MySchema {
    ///     user: User,
    /// }
    ///
    /// let builder = QueryBuilder::new::<MySchema>();
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
    /// Starts a SELECT with the given columns.
    ///
    /// Pass one column or expression, a tuple of them, or `()` to select
    /// every column of the FROM table (and of joined tables). Call
    /// [`from`](select::SelectBuilder::from) next.
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
    /// // Select a single column
    /// let query = builder.select(user.name).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
    ///
    /// // Select multiple columns
    /// let query = builder.select((user.id, user.name)).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."id", "users"."name" FROM "users""#);
    ///
    /// // Select every column
    /// let query = builder.select(()).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."id", "users"."name" FROM "users""#);
    /// ```
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
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

    /// Starts a SELECT DISTINCT, which drops duplicate rows from the result.
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
    /// let query = builder.select_distinct(user.name).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT DISTINCT "users"."name" FROM "users""#);
    /// ```
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
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
}

impl<'a, Schema> QueryBuilder<'a, Schema, CTEInit> {
    /// Starts a SELECT after the WITH clause.
    ///
    /// See [`QueryBuilder::with`] for an example.
    pub fn select<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
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

    /// Starts a SELECT DISTINCT after the WITH clause.
    pub fn select_distinct<T>(
        &self,
        columns: T,
    ) -> select::SelectBuilder<'a, Schema, select::SelectInitial, (), T::Marker>
    where
        T: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
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

    /// Starts an INSERT after the WITH clause.
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::insert::<
            Table,
            SQLiteSchemaType,
            SQLiteValue<'a>,
        >(&table));

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

    /// Starts an UPDATE after the WITH clause.
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::update::<
            Table,
            SQLiteSchemaType,
            SQLiteValue<'a>,
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

    /// Starts a DELETE after the WITH clause.
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = self.sql.clone().append(crate::helpers::delete::<
            Table,
            SQLiteSchemaType,
            SQLiteValue<'a>,
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

    /// Adds another common table expression to the WITH clause.
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
    /// Starts an INSERT into `table`.
    ///
    /// Call [`values`](insert::InsertBuilder::values),
    /// [`value`](insert::InsertBuilder::value),
    /// [`select`](insert::InsertBuilder::select) or
    /// [`columns`](insert::InsertBuilder::columns) next.
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
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice")]);
    /// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" ("name") VALUES (?)"#);
    /// ```
    pub fn insert<Table>(
        &self,
        table: Table,
    ) -> insert::InsertBuilder<'a, Schema, insert::InsertInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = crate::helpers::insert::<Table, SQLiteSchemaType, SQLiteValue<'a>>(&table);

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

    /// Starts an UPDATE of `table`. Call [`set`](update::UpdateBuilder::set) next.
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
    /// # use drizzle::core::expr::eq;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder
    ///     .update(user)
    ///     .set(UpdateUser::default().with_name("Bob"))
    ///     .r#where(eq(user.id, 1));
    /// assert_eq!(query.to_sql().sql(), r#"UPDATE "users" SET "name" = ? WHERE "users"."id" = ?"#);
    /// ```
    pub fn update<Table>(
        &self,
        table: Table,
    ) -> update::UpdateBuilder<'a, Schema, update::UpdateInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = crate::helpers::update::<Table, SQLiteSchemaType, SQLiteValue<'a>>(&table);

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

    /// Starts a DELETE from `table`.
    ///
    /// Without `where` the statement
    /// deletes every row.
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
    /// # use drizzle::core::expr::lt;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder
    ///     .delete(user)
    ///     .r#where(lt(user.id, 10));
    /// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "users" WHERE "users"."id" < ?"#);
    /// ```
    pub fn delete<Table>(
        &self,
        table: Table,
    ) -> delete::DeleteBuilder<'a, Schema, delete::DeleteInitial, Table>
    where
        Table: SQLiteTable<'a>,
    {
        let sql = crate::helpers::delete::<Table, SQLiteSchemaType, SQLiteValue<'a>>(&table);

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

    /// Starts a WITH clause with one common table expression.
    ///
    /// Build the CTE with [`into_cte`](select::SelectBuilder::into_cte). Add
    /// more CTEs with another `.with(..)`, then start the main statement.
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
    /// # use drizzle::core::expr::gt;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// struct Recent;
    /// impl drizzle::core::Tag for Recent {
    ///     const NAME: &'static str = "recent";
    /// }
    ///
    /// let recent = builder
    ///     .select((user.id, user.name))
    ///     .from(user)
    ///     .r#where(gt(user.id, 100))
    ///     .into_cte::<Recent>();
    ///
    /// let query = builder.with(&recent).select(recent.name).from(&recent);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"WITH "recent" AS (SELECT "users"."id", "users"."name" FROM "users" WHERE "users"."id" > ?) SELECT "recent"."name" FROM "recent""#
    /// );
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
