//! A type-safe SQL query builder and ORM for Rust, inspired by
//! [Drizzle ORM](https://orm.drizzle.team).
//!
//! You describe each table as a Rust struct. Drizzle generates typed columns
//! and row models from it, builds SQL from typed expressions, and runs that SQL
//! on a database connection you create and own. Many query mistakes become
//! compile errors: comparing a number column with text, reading a table the
//! query never joined, or decoding a column that can be `NULL` into a
//! non-`Option` field.
//!
//! SQLite, PostgreSQL, and MySQL are supported. The examples below use SQLite
//! through [`rusqlite`](https://docs.rs/rusqlite); the other drivers expose the
//! same query API (async drivers add `.await`).
//!
//! # Getting started
//!
//! ## 1. Add the dependency
//!
//! Turn on the feature for the database client you already use:
//!
//! ```toml
//! [dependencies]
//! drizzle = { version = "0.2", features = ["rusqlite"] }
//! rusqlite = { version = "0.39", features = ["bundled"] }
//! ```
//!
//! ## 2. Define tables and a schema
//!
//! `#[SQLiteTable]` turns a struct into a table. Each field is a column; an
//! `Option<T>` field is a nullable column. `#[column(references = ...)]`
//! declares a foreign key, which joins and relational queries use.
//! `#[derive(SQLiteSchema)]` groups the tables your database holds.
//!
//! ```
//! # #[cfg(feature = "rusqlite")]
//! # fn main() {
//! use drizzle::sqlite::prelude::*;
//!
//! #[SQLiteTable]
//! pub struct Users {
//!     #[column(primary, autoincrement)]
//!     pub id: i64,
//!     pub name: String,
//!     pub email: Option<String>, // nullable column
//!     pub age: i64,
//! }
//!
//! #[SQLiteTable]
//! pub struct Posts {
//!     #[column(primary, autoincrement)]
//!     pub id: i64,
//!     pub title: String,
//!     pub content: Option<String>,
//!     #[column(references = Users::id)] // foreign key to users.id
//!     pub author_id: i64,
//! }
//!
//! #[derive(SQLiteSchema)]
//! pub struct Schema {
//!     pub users: Users,
//!     pub posts: Posts,
//! }
//! # }
//! # #[cfg(not(feature = "rusqlite"))]
//! # fn main() {}
//! ```
//!
//! Each table also gets generated row models, named after the struct:
//!
//! | Model          | Use                                                                     |
//! |----------------|-------------------------------------------------------------------------|
//! | `SelectUsers`  | A full row read by a query.                                             |
//! | `InsertUsers`  | A row to insert. `new(..)` takes the required fields; `with_*` adds optional ones. |
//! | `UpdateUsers`  | The columns to change. Start from `default()` and call `with_*`.        |
//!
//! ## 3. Connect
//!
//! `Drizzle::new` wraps your connection and returns it together with the
//! schema value, whose fields are the table handles you query with.
//! `db.create()` runs `CREATE TABLE` for every table in the schema, which is
//! handy for tests; use migrations (the `drizzle` CLI and
//! [`include_migrations!`]) for real databases.
//!
//! ```
//! # #[cfg(feature = "rusqlite")]
//! # fn main() -> drizzle::Result<()> {
//! # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
//! # use app::Schema;
//! use drizzle::sqlite::rusqlite::Drizzle;
//!
//! let conn = rusqlite::Connection::open_in_memory()?;
//! let (db, Schema { users, posts, .. }) = Drizzle::new(conn);
//! db.create()?;
//! # let _ = (users, posts);
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "rusqlite"))]
//! # fn main() {}
//! ```
//!
//! When no pattern names the schema type, put it on the call:
//! `Drizzle::<Schema>::new(conn)`. Use `let (db, ()) = Drizzle::new(conn)` for a
//! connection with no schema.
//!
//! # Queries
//!
//! Start a query from `db`, chain clauses, and finish with a terminal method
//! that runs it:
//!
//! | Method      | Runs the query and returns                                      |
//! |-------------|-----------------------------------------------------------------|
//! | `.all()`    | every row, decoded into `Vec<R>`                                |
//! | `.get()`    | the first row, or an error when there is none                   |
//! | `.rows()`   | a cursor over the rows, decoded into the query's own row type   |
//! | `.execute()`| the number of rows changed (for `INSERT`/`UPDATE`/`DELETE`)     |
//!
//! Comparison and boolean helpers such as `eq`, `gt`, `and`, and `count` live
//! in [`core::expr`]. A tuple of conditions means `AND`. The ordering helpers
//! [`asc`](core::asc) and [`desc`](core::desc) live in [`core`].
//!
//! ```
//! # #[cfg(feature = "rusqlite")]
//! # fn main() -> drizzle::Result<()> {
//! # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
//! # use app::*;
//! use drizzle::core::desc;
//! use drizzle::core::expr::{eq, gt};
//! # let (db, Schema { users, .. }) = app::database()?;
//!
//! // INSERT INTO "users" ("name", "email", "age") VALUES (?, ?, ?)
//! db.insert(users)
//!     .value(InsertUsers::new("Dana", 41).with_email("dana@example.com"))
//!     .execute()?;
//!
//! // SELECT "users"."id", "users"."name", ... FROM "users" WHERE "users"."age" > ?
//! let adults: Vec<SelectUsers> = db.select(()).from(users).r#where(gt(users.age, 18)).all()?;
//!
//! // Select specific columns into a tuple.
//! let names: Vec<(i64, String)> = db
//!     .select((users.id, users.name))
//!     .from(users)
//!     .order_by(desc(users.age))
//!     .limit(10)
//!     .all()?;
//!
//! // One row. `.get()` fails when nothing matches.
//! let dana: SelectUsers = db.select(()).from(users).r#where(eq(users.name, "Dana")).get()?;
//!
//! // UPDATE "users" SET "age" = ? WHERE "users"."id" = ?
//! db.update(users)
//!     .set(UpdateUsers::default().with_age(42))
//!     .r#where(eq(users.id, dana.id))
//!     .execute()?;
//!
//! // DELETE FROM "users" WHERE "users"."id" = ?
//! let deleted = db.delete(users).r#where(eq(users.id, dana.id)).execute()?;
//! assert_eq!(deleted, 1);
//! # let _ = (adults, names);
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "rusqlite"))]
//! # fn main() {}
//! ```
//!
//! Every builder implements [`ToSQL`](core::ToSQL), so you can look at the SQL
//! without running it:
//!
//! ```
//! # #[cfg(feature = "rusqlite")]
//! # fn main() -> drizzle::Result<()> {
//! # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
//! # use app::*;
//! use drizzle::core::ToSQL;
//! use drizzle::core::expr::eq;
//! # let (db, Schema { users, .. }) = app::database()?;
//!
//! let query = db.select(users.name).from(users).r#where(eq(users.id, 1));
//! assert_eq!(
//!     query.to_sql().sql(),
//!     r#"SELECT "users"."name" FROM "users" WHERE "users"."id" = ?"#,
//! );
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "rusqlite"))]
//! # fn main() {}
//! ```
//!
//! # Joins
//!
//! Pass a table to `join`/`left_join`/... to join on its foreign key, or a
//! `(table, condition)` pair to give the `ON` condition yourself. After a
//! `LEFT JOIN`, the joined table's columns can be `NULL`, so they decode as
//! `Option<T>`:
//!
//! ```
//! # #[cfg(feature = "rusqlite")]
//! # fn main() -> drizzle::Result<()> {
//! # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
//! # use app::*;
//! use drizzle::core::expr::eq;
//! # let (db, Schema { users, posts, .. }) = app::database()?;
//!
//! // ... FROM "users" INNER JOIN "posts" ON "users"."id" = "posts"."author_id"
//! let written: Vec<(String, String)> = db
//!     .select((users.name, posts.title))
//!     .from(users)
//!     .inner_join((posts, eq(users.id, posts.author_id)))
//!     .all()?;
//!
//! // ... FROM "users" LEFT JOIN "posts" ON "posts"."author_id" = "users"."id"
//! let all_users: Vec<(String, Option<String>)> = db
//!     .select((users.name, posts.title))
//!     .from(users)
//!     .left_join(posts)
//!     .all()?;
//! # let _ = (written, all_users);
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "rusqlite"))]
//! # fn main() {}
//! ```
//!
//! To map joined columns into a named struct, derive
//! [`SQLiteFromRow`](sqlite::SQLiteFromRow).
//!
//! # Transactions
//!
//! `transaction` runs a closure inside `BEGIN ... COMMIT`. Return `Ok` to
//! commit and `Err` to roll back; a panic also rolls back. Inside the closure,
//! `tx` has the same query methods as `db`, and `tx.savepoint(..)` nests a
//! savepoint that can roll back on its own.
//!
//! ```
//! # #[cfg(feature = "rusqlite")]
//! # fn main() -> drizzle::Result<()> {
//! # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
//! # use app::*;
//! use drizzle::sqlite::TransactionConfig;
//! # let (mut db, Schema { users, .. }) = app::database()?;
//!
//! let total = db.transaction(TransactionConfig::Deferred, |tx| {
//!     tx.insert(users).value(InsertUsers::new("Eve", 35)).execute()?;
//!     let rows: Vec<SelectUsers> = tx.select(()).from(users).all()?;
//!     Ok(rows.len())
//! })?;
//! assert_eq!(total, 4);
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "rusqlite"))]
//! # fn main() {}
//! ```
//!
//! # Compile-time checks
//!
//! The type system checks a query before it can run:
//!
//! - **Types.** Comparisons and functions accept only compatible SQL types, so
//!   `eq(users.age, "ten")` does not compile.
//! - **Scope.** Every column a query reads must come from a table in its
//!   `FROM` or `JOIN` list. This is checked when you call `.all()`, `.get()`,
//!   or `.rows()`.
//! - **NULL.** A column that can be `NULL` (an `Option<T>` field, or any column
//!   of a table brought in by `LEFT`, `RIGHT`, or `FULL JOIN`) must be decoded
//!   into `Option<T>`.
//! - **Grouping.** With `GROUP BY`, each column in a selected tuple must be
//!   grouped (or belong to a table grouped by its primary key) or sit inside
//!   an aggregate such as `count`.
//!
//! Reading `posts` without joining it is rejected:
//!
//! ```compile_fail,E0277
//! # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
//! # use app::*;
//! use drizzle::core::expr::eq;
//! # fn main() -> drizzle::Result<()> {
//! # let (db, Schema { users, posts, .. }) = app::database()?;
//!
//! // error: `Posts` is not in this query's FROM/JOIN scope
//! let names: Vec<String> = db
//!     .select(users.name)
//!     .from(users)
//!     .r#where(eq(posts.title, "Hello"))
//!     .all()?;
//! # Ok(())
//! # }
//! ```
//!
//! So is decoding a `LEFT JOIN` column as if it were never `NULL`:
//!
//! ```compile_fail,E0277
//! # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
//! # use app::*;
//! # fn main() -> drizzle::Result<()> {
//! # let (db, Schema { users, posts, .. }) = app::database()?;
//! // error: `posts.title` must decode as `Option<String>` after a LEFT JOIN
//! let rows: Vec<(String, String)> = db
//!     .select((users.name, posts.title))
//!     .from(users)
//!     .left_join(posts)
//!     .all()?;
//! # Ok(())
//! # }
//! ```
//!
//! Database errors, such as a constraint violation or a row that does not
//! match on `.get()`, are still runtime errors. They come back as
//! [`DrizzleError`](error::DrizzleError) through [`Result`].
//!
//! # Drivers and features
//!
//! | Feature                           | Driver module                                          |
//! |-----------------------------------|--------------------------------------------------------|
//! | `rusqlite`                        | `sqlite::rusqlite` (blocking)                          |
//! | `libsql`, `turso`                 | `sqlite::libsql`, `sqlite::turso` (async)              |
//! | `d1`, `durable`                   | `sqlite::d1`, `sqlite::durable` (Cloudflare, `wasm32` only) |
//! | `postgres-sync`                   | `postgres::sync` (blocking)                            |
//! | `tokio-postgres`, `hyperdrive`    | `postgres::tokio` (async; `hyperdrive` is `wasm32` only) |
//! | `aws-data-api`                    | `postgres::aws` (Aurora Data API over HTTP)            |
//! | `mysql-sync`, `mysql-async`       | `mysql::mysql_sync`, `mysql::mysql_async`              |
//!
//! Other features: `query` adds relational queries (`db.query(table).with(..)`),
//! `serde` adds JSON columns, and `uuid`, `chrono`, `time`, `jiff`, and
//! `rust-decimal` add column types from those crates.
//!
//! # Modules
//!
//! - [`sqlite`], [`postgres`], [`mysql`]: table macros, a `prelude` for schema
//!   files, dialect types, and one module per driver.
//! - [`core`]: the dialect-independent traits, expressions ([`core::expr`]),
//!   and SQL building blocks.
//! - [`migrations`]: embedded migrations and schema snapshots.
//! - [`error`]: [`DrizzleError`](error::DrizzleError).
//!
//! The project README covers migrations, relational queries, prepared
//! statements, and the CLI in more depth.
#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg, rustdoc_internals))]
#![allow(
    unexpected_cfgs,
    unused_imports,
    unused_macros,
    unused_mut,
    dead_code,
    clippy::redundant_closure,
    clippy::needless_question_mark,
    clippy::duplicated_attributes,
    clippy::single_component_path_imports
)]

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[macro_use]
mod builder;

#[macro_use]
mod macros;

#[macro_use]
mod transaction;

#[doc(hidden)]
pub(crate) use drizzle_builder_join_impl;
#[doc(hidden)]
pub(crate) use drizzle_pg_builder_join_impl;
#[doc(hidden)]
pub(crate) use drizzle_pg_builder_join_using_impl;

/// Result type for drizzle operations.
#[doc(inline)]
pub use drizzle_core::error::Result;

#[cfg(feature = "std")]
#[doc(inline)]
pub use drizzle_macros::include_migrations;
/// SQL template macro.
#[doc(inline)]
pub use drizzle_macros::sql;

// `#[drizzle::test]` is internal to drizzle's own test suite: its expansion
// calls `crate::common::helpers` in `tests/`, so it cannot work in other
// crates. It stays public only because that suite is a separate crate.
#[doc(hidden)]
pub use drizzle_macros::test;

/// Database dialect enum.
#[doc(inline)]
pub use drizzle_types::Dialect;

/// Identifier casing used by codegen (`camelCase` / `snake_case`).
///
/// Re-exported from [`drizzle_types::Casing`] alongside [`Dialect`] so the
/// two configuration enums are reachable through the same canonical path.
#[doc(inline)]
pub use drizzle_types::Casing;

/// Re-export const_format for proc macro generated compile-time SQL.
#[doc(hidden)]
pub use const_format;

/// Error types.
pub mod error {
    #[doc(inline)]
    pub use drizzle_core::error::DrizzleError;
}

/// DDL types and schema definitions.
pub mod ddl {
    #[doc(inline)]
    pub use drizzle_types::mysql;
    #[doc(inline)]
    pub use drizzle_types::postgres;
    #[doc(inline)]
    pub use drizzle_types::sqlite;
}

/// Migration helpers and schema snapshots.
#[cfg(feature = "std")]
pub mod migrations {
    #[doc(inline)]
    pub use drizzle_migrations::*;
}

/// Core traits, SQL types, and expressions shared across drivers.
pub mod core {
    /// SQL building blocks.
    #[doc(inline)]
    pub use drizzle_core::{
        ColumnDialect, ColumnFlags, ColumnRef, ConstraintRef, Derived, DerivedField, ForeignKeyRef,
        OrderBy, Param, ParamBind, ParamSet, Placeholder, PrimaryKeyRef, SQL, SQLChunk,
        TableDialect, TableRef, TableSqlRef, Token, TypedPlaceholder, asc, desc,
    };

    /// Conversion trait for SQL generation.
    #[doc(inline)]
    pub use drizzle_core::ToSQL;

    /// Core traits (`SQLTable`, `SQLColumn`, `SQLSchema`, `SQLModel`, etc.).
    #[doc(inline)]
    pub use drizzle_core::traits::*;

    /// Relation metadata types and traits.
    #[doc(inline)]
    pub use drizzle_core::relation::{Joinable, Relation, SchemaHasTable};

    /// Full relation module exports.
    pub mod relation {
        #[doc(inline)]
        pub use drizzle_core::relation::*;
    }

    /// Prepared statement types.
    #[doc(inline)]
    pub use drizzle_core::prepared::{OwnedPreparedStatement, PreparedStatement};

    /// SQL type markers used by expressions.
    #[doc(inline)]
    pub use drizzle_core::types;

    /// Type-safe expressions and helpers.
    #[doc(inline)]
    pub use drizzle_core::expr;

    #[doc(hidden)]
    pub use drizzle_core::impl_try_from_int;

    #[doc(hidden)]
    pub use drizzle_core::schema::SQLEnumInfo;

    #[doc(hidden)]
    pub use drizzle_core::{
        InsertColumn, InsertSelectAllColumns, InsertSelectTable, MaybeNull, ProjectionIn,
    };

    /// Type-level query scope: which sources a query reads, and how outer
    /// joins make them nullable.
    #[doc(inline)]
    pub use drizzle_core::scope;

    #[doc(inline)]
    pub use drizzle_core::{
        AliasKey, ScopeContains, ScopeEntry, SelectTableFields, Src, TableFields,
    };

    /// Bind parameter type mapping trait.
    #[doc(inline)]
    pub use drizzle_core::ValueTypeForDialect;

    /// Dialect markers (`SQLiteDialect`, `PostgresDialect`, etc.).
    pub mod dialect {
        #[doc(inline)]
        pub use drizzle_core::dialect::*;
    }

    /// Query API types (relational queries with nested loading).
    #[cfg(feature = "query")]
    pub mod query {
        #[doc(inline)]
        pub use drizzle_core::query::*;
    }

    /// Drizzle-owned wrapper for JSON column values.
    ///
    /// Not part of the dialect preludes, so it never collides with the
    /// `Json` SQL type marker in `drizzle::postgres::types` or
    /// `drizzle::mysql::types`; import it as `drizzle::core::Json`.
    #[cfg(feature = "serde")]
    #[doc(inline)]
    pub use drizzle_core::Json;

    /// The JSON column wrapper and the field conversions generated models use.
    #[cfg(feature = "serde")]
    pub mod json {
        #[doc(inline)]
        pub use drizzle_core::json::*;
    }

    /// Re-export serde for proc macro generated code.
    #[cfg(any(feature = "serde", feature = "query"))]
    #[doc(hidden)]
    pub use drizzle_core::serde;
    #[cfg(any(feature = "serde", feature = "query"))]
    #[doc(hidden)]
    pub use drizzle_core::serde_json;

    /// Row inference types and traits.
    #[doc(inline)]
    pub use drizzle_core::row::{
        DecodeSelectedRef, ExprValueType, FromDrizzleRow, GroupByIdentity, HasSelectModel,
        IntoGroupBy, IntoSelectTarget, JoinedStarRow, LeftLateralSelection, MarkerColumnCountValid,
        MarkerScopeValidFor, NullProbeRow, OuterJoined, PkGroup, ResolveRow, RowColumnList,
        SQLTypeToRust, Scoped, SelectAs, SelectAsFrom, SelectCols, SelectExpr, SelectStar,
        WrapNullable,
    };
    #[doc(inline)]
    pub use drizzle_core::scope::{
        FullJoin, HasScope, InnerJoin, JoinStep, Lateral, LeftJoin, RightJoin,
    };
}

/// `SQLite` types, macros, and query builder.
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite {
    #[doc(inline)]
    pub use drizzle_macros::{
        SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable, SQLiteView,
    };
    #[doc(inline)]
    pub use drizzle_sqlite::{
        SQLiteTransactionType, TransactionConfig, attrs, builder, common, connection, expr,
        helpers, pragma, traits, types, values,
    };

    #[cfg(feature = "rusqlite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rusqlite")))]
    pub mod rusqlite {
        #[doc(inline)]
        pub use crate::builder::sqlite::rusqlite::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::sqlite::rusqlite::Transaction;
        #[doc(hidden)]
        pub use ::rusqlite::{Error, Result, Row, types};
    }

    #[cfg(feature = "libsql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "libsql")))]
    pub mod libsql {
        #[doc(inline)]
        pub use crate::builder::sqlite::libsql::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::sqlite::libsql::Transaction;
        #[doc(hidden)]
        pub use ::libsql::{Row, Value};
    }

    #[cfg(feature = "turso")]
    #[cfg_attr(docsrs, doc(cfg(feature = "turso")))]
    pub mod turso {
        #[doc(inline)]
        pub use crate::builder::sqlite::turso::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::sqlite::turso::Transaction;
        #[doc(hidden)]
        pub use ::turso::{Error, IntoValue, Result, Row, Value};
    }

    /// Cloudflare D1 driver (async, WASM-only).
    #[cfg(all(feature = "d1", target_arch = "wasm32"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "d1", target_arch = "wasm32"))))]
    pub mod d1 {
        #[doc(inline)]
        pub use crate::builder::sqlite::d1::{Drizzle, DrizzleBuilder};
    }

    /// Cloudflare Durable Objects SQL storage driver (sync, WASM-only).
    #[cfg(all(feature = "durable", target_arch = "wasm32"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "durable", target_arch = "wasm32"))))]
    pub mod durable {
        #[doc(inline)]
        pub use crate::builder::sqlite::durable::{Drizzle, DrizzleBuilder, DurableStorage};
        #[doc(inline)]
        pub use crate::transaction::sqlite::durable::Transaction;
    }

    /// `SQLite` prelude for schema declarations.
    pub mod prelude {
        // Core types and traits
        pub use crate::core::ToSQL;
        pub use crate::core::{Joinable, Relation, SchemaHasTable};
        pub use crate::core::{
            OrderBy, Param, ParamBind, ParamSet, Placeholder, SQL, SQLChunk, Token,
            TypedPlaceholder, asc, desc,
        };
        pub use crate::core::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::tag;
        pub use drizzle_core::traits::*;
        // SQLite macros
        pub use drizzle_macros::{
            SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable, SQLiteView,
        };
        // SQLite types
        pub use drizzle_sqlite::TransactionConfig;
        pub use drizzle_sqlite::attrs::*;
        pub use drizzle_sqlite::common::SQLiteSchemaType;
        pub use drizzle_sqlite::traits::{DrizzleSQLiteColumn, SQLiteColumn, SQLiteTable};
        pub use drizzle_sqlite::values::{
            OwnedSQLiteValue, SQLiteInsertValue, SQLiteUpdateValue, SQLiteValue, SQLiteValueRef,
        };
    }
}

/// `PostgreSQL` types, macros, and query builder.
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres {
    #[doc(inline)]
    pub use drizzle_macros::{
        PostgresEnum, PostgresFromRow, PostgresIndex, PostgresPolicy, PostgresSchema,
        PostgresTable, PostgresView,
    };
    #[doc(hidden)]
    pub use drizzle_postgres::driver_types;
    #[doc(inline)]
    pub use drizzle_postgres::{
        AccessMode, IsolationLevel, TransactionConfig, attrs, builder, common, expr, helpers,
        traits, transaction, types, values,
    };

    #[cfg(all(
        feature = "postgres-sync",
        not(any(feature = "tokio-postgres", feature = "hyperdrive"))
    ))]
    #[doc(inline)]
    pub use drizzle_postgres::Row;
    #[cfg(any(feature = "tokio-postgres", feature = "hyperdrive"))]
    #[doc(inline)]
    pub use drizzle_postgres::Row;

    /// AWS Aurora Data API row + helpers (re-exported from drizzle-postgres).
    ///
    /// Required by the `#[PostgresTable]` macro when the `aws-data-api` feature
    /// is enabled — macro-generated code refers to
    /// `drizzle::postgres::aws_data_api::Row` and
    /// `drizzle::postgres::aws_data_api::is_null_at`.
    #[cfg(feature = "aws-data-api")]
    #[cfg_attr(docsrs, doc(cfg(feature = "aws-data-api")))]
    #[doc(inline)]
    pub use drizzle_postgres::aws_data_api;

    #[cfg(feature = "postgres-sync")]
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres-sync")))]
    pub mod sync {
        #[doc(inline)]
        pub use crate::builder::postgres::postgres_sync::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::postgres::postgres_sync::Transaction;
    }

    /// Async `PostgreSQL` driver over [`tokio_postgres`].
    ///
    /// Enabled by either `tokio-postgres` (native) or `hyperdrive`
    /// (Cloudflare Workers) — both compile the same driver; only the way the
    /// connection is dialed differs. On wasm32 with the `hyperdrive` feature,
    /// `drizzle::postgres::hyperdrive` supplies the dial.
    #[cfg(any(feature = "tokio-postgres", feature = "hyperdrive"))]
    #[cfg_attr(
        docsrs,
        doc(cfg(any(feature = "tokio-postgres", feature = "hyperdrive")))
    )]
    pub mod tokio {
        #[doc(inline)]
        pub use crate::builder::postgres::tokio_postgres::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::postgres::tokio_postgres::Transaction;
    }

    /// Cloudflare Hyperdrive connector (WASM-only).
    ///
    /// Dials the [`tokio`](self::tokio) driver through a Workers `Hyperdrive`
    /// binding instead of a TCP socket. The resulting `Drizzle` is the exact
    /// type the native driver produces.
    #[cfg(all(feature = "hyperdrive", target_arch = "wasm32"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "hyperdrive", target_arch = "wasm32"))))]
    pub mod hyperdrive {
        #[doc(inline)]
        pub use crate::builder::postgres::hyperdrive::{connect, connect_raw};
    }

    /// AWS Aurora Serverless Data API driver (HTTP-based, async).
    #[cfg(feature = "aws-data-api")]
    #[cfg_attr(docsrs, doc(cfg(feature = "aws-data-api")))]
    pub mod aws {
        #[doc(inline)]
        pub use crate::builder::postgres::aws_data_api::{Drizzle, DrizzleBuilder, Rows};
        #[doc(inline)]
        pub use crate::transaction::postgres::aws_data_api::{Transaction, TransactionBuilder};
    }

    /// `PostgreSQL` prelude for schema declarations.
    pub mod prelude {
        // Core types and traits
        pub use crate::core::ToSQL;
        pub use crate::core::{Joinable, Relation, SchemaHasTable};
        pub use crate::core::{
            OrderBy, Param, ParamBind, ParamSet, Placeholder, SQL, SQLChunk, Token,
            TypedPlaceholder, asc, desc,
        };
        pub use crate::core::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::tag;
        pub use drizzle_core::traits::*;
        // PostgreSQL macros
        pub use drizzle_macros::{
            PostgresEnum, PostgresFromRow, PostgresIndex, PostgresPolicy, PostgresSchema,
            PostgresTable, PostgresView,
        };
        // PostgreSQL types
        pub use drizzle_postgres::attrs::*;
        pub use drizzle_postgres::common::PostgresSchemaType;
        pub use drizzle_postgres::traits::{DrizzlePostgresColumn, PostgresColumn, PostgresTable};
        pub use drizzle_postgres::values::{
            OwnedPostgresValue, PostgresInsertValue, PostgresUpdateValue, PostgresValue,
        };
        pub use drizzle_postgres::{AccessMode, IsolationLevel, TransactionConfig};
    }
}

/// `MySQL` dialect, query builders, values, and adapters.
///
/// ```
/// use drizzle::ddl::mysql::{MySQLType, MySQLTypeCategory};
/// use drizzle::mysql::{
///     MySQLDialect, MySQLMutationResult, MySQLValue, OwnedMySQLValue, TransactionConfig,
/// };
/// ```
///
/// First-class compatibility targets Oracle MySQL 8.0.31 and newer. MariaDB
/// and SingleStore are not part of this compatibility contract. The concrete
/// adapters inherit TLS configuration from the upstream `mysql` or
/// `mysql_async` resource supplied by the application.
///
/// Both adapters set each connection's session time zone to UTC and remove
/// `NO_UNSIGNED_SUBTRACTION` and `REAL_AS_FLOAT` before typed queries. This makes
/// temporal decoding and numeric types agree with the static Rust types.
///
/// MySQL upserts use `InsertBuilder::on_duplicate_key_update`. SQL `RETURNING`,
/// full joins, partial-index predicates, and PostgreSQL `UPDATE ... FROM` are
/// intentionally absent. Offset-only queries render MySQL's maximum-limit
/// sentinel, and typed string concatenation renders `CONCAT(...)`, never `||`.
///
/// ```compile_fail
/// use drizzle::mysql::builder::UpdateFromSet;
/// ```
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql {
    #[doc(inline)]
    pub use drizzle_macros::{
        MySQLEnum, MySQLFromRow, MySQLIndex, MySQLSchema, MySQLTable, MySQLView,
    };
    #[doc(inline)]
    pub use drizzle_mysql::values::{MySQLValue, OwnedMySQLValue};
    #[doc(inline)]
    pub use drizzle_mysql::{
        AccessMode, IndexKeyPart, IndexOrder, IsolationLevel, MySQLDialect, MySQLMutationResult,
        ParamBind, TransactionConfig, ViewAlgorithm, ViewCheckOption, ViewSqlSecurity, attrs,
        builder, common, driver, helpers, index, traits, transaction, types, values,
    };

    /// Blocking adapter backed by the `mysql` crate.
    ///
    /// It owns the exact `mysql::Conn` or checked-out `mysql::PooledConn` passed
    /// to `Drizzle::new`; construction does not perform I/O. See
    /// [`Drizzle`](mysql_sync::Drizzle) for connection and transaction examples.
    #[cfg(feature = "mysql-sync")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mysql-sync")))]
    pub mod mysql_sync {
        #[doc(inline)]
        pub use crate::builder::mysql::mysql_sync::{Drizzle, DrizzleBuilder, Rows, prepared};
        #[doc(inline)]
        pub use crate::transaction::mysql::mysql_sync::Transaction;
    }

    /// Async adapter backed by the `mysql_async` crate.
    ///
    /// It accepts an owned `mysql_async::Conn` or lazy `mysql_async::Pool`.
    /// Pool-backed operations check out a connection per operation; call
    /// [`Drizzle::disconnect`](mysql_async::Drizzle::disconnect) before the
    /// owning Tokio runtime stops.
    #[cfg(feature = "mysql-async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "mysql-async")))]
    pub mod mysql_async {
        #[doc(inline)]
        pub use crate::builder::mysql::mysql_async::{Drizzle, DrizzleBuilder, Rows, prepared};
        #[doc(inline)]
        pub use crate::transaction::mysql::mysql_async::Transaction;
    }

    /// `MySQL` prelude for schema declarations.
    pub mod prelude {
        // Core types and traits
        pub use crate::core::ToSQL;
        pub use crate::core::{Joinable, Relation, SchemaHasTable};
        pub use crate::core::{
            OrderBy, Param, ParamBind, ParamSet, Placeholder, SQL, SQLChunk, Token,
            TypedPlaceholder,
        };
        pub use crate::core::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::tag;
        pub use drizzle_core::traits::*;
        pub use drizzle_mysql::helpers::{MySQLIndexHintExt, asc, desc, output_alias};
        // MySQL macros
        pub use drizzle_macros::{
            MySQLEnum, MySQLFromRow, MySQLIndex, MySQLSchema, MySQLTable, MySQLView,
        };
        // MySQL types
        pub use drizzle_mysql::MySQLMutationResult;
        pub use drizzle_mysql::attrs::*;
        pub use drizzle_mysql::common::MySQLSchemaType;
        pub use drizzle_mysql::common::MySQLViewInfo;
        pub use drizzle_mysql::traits::{
            DrizzleMySQLColumn, MySQLColumn, MySQLEnum, MySQLIndexColumn, MySQLTable,
        };
        pub use drizzle_mysql::values::{
            MySQLInsertValue, MySQLUpdateValue, MySQLValue, OwnedMySQLValue,
        };
        pub use drizzle_mysql::{AccessMode, IsolationLevel, TransactionConfig};
        pub use drizzle_mysql::{
            IndexKeyPart, IndexOrder, MySQLIndexAlgorithm, MySQLIndexLock, MySQLIndexMethod,
        };
        pub use drizzle_mysql::{ViewAlgorithm, ViewCheckOption, ViewSqlSecurity};
    }
}

// =============================================================================
// Compile-fail tests (verified during `cargo test --doc`)
// =============================================================================

/// Type safety: abs() rejects non-numeric columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::abs;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = abs(user.name);
/// }
/// ```
///
/// Type safety: avg() rejects non-numeric columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::avg;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = avg(user.name);
/// }
/// ```
///
/// Type safety: sum() rejects non-numeric columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::sum;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = sum(user.name);
/// }
/// ```
///
/// Type safety: Blob is not compatible with Integer.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::eq;
///
/// #[SQLiteTable]
/// struct Config {
///     #[column(primary)]
///     id: i32,
///     data: Vec<u8>,
/// }
///
/// fn main() {
///     let config = Config::default();
///     let _ = eq(config.data, 42);
/// }
/// ```
///
/// Type safety: Int is not compatible with Text.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::eq;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = eq(user.id, "hello");
/// }
/// ```
///
/// Type safety: coalesce() rejects incompatible types.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::coalesce;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = coalesce(user.id, "default");
/// }
/// ```
///
/// Type safety: concat() rejects non-textual columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::concat;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = concat(user.id, user.name);
/// }
/// ```
///
/// Type safety: date() rejects non-temporal columns (Blob).
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::date;
///
/// #[SQLiteTable]
/// struct Data {
///     #[column(primary)]
///     id: i32,
///     content: Vec<u8>,
/// }
///
/// fn main() {
///     let data = Data::default();
///     let _ = date(data.content);
/// }
/// ```
///
/// Type safety: like() rejects non-textual columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::like;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = like(user.id, "%test%");
/// }
/// ```
///
/// Type safety: FK column type must match referenced column type.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Parent {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[SQLiteTable]
/// struct Child {
///     #[column(primary)]
///     id: i32,
///     #[column(references = Parent::id)]
///     parent_ref: String,
/// }
///
/// fn main() {}
/// ```
///
/// Type safety: FK target table must be in the schema.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Parent {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[SQLiteTable]
/// struct Child {
///     #[column(primary)]
///     id: i32,
///     #[column(references = Parent::id)]
///     parent_id: Option<i32>,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct BadSchema {
///     child: Child,
/// }
///
/// fn main() {}
/// ```
///
/// Type safety: HasConstraint requires actual FK on the table.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Simple {
///     #[column(primary)]
///     id: i32,
///     value: String,
/// }
///
/// fn requires_fk_constraint<T: HasConstraint<ForeignKeyK>>() {}
///
/// fn main() {
///     requires_fk_constraint::<Simple>();
/// }
/// ```
///
/// Type safety: Relation requires a FK between the tables.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Parent {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[SQLiteTable]
/// struct Simple {
///     #[column(primary)]
///     id: i32,
///     value: String,
/// }
///
/// fn requires_relation<T: Relation<Parent>>() {}
///
/// fn main() {
///     requires_relation::<Simple>();
/// }
/// ```
///
/// Type safety: Joinable requires a FK relationship.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Parent {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[SQLiteTable]
/// struct Unrelated {
///     #[column(primary)]
///     id: i32,
///     value: String,
/// }
///
/// fn requires_joinable<A: Joinable<B>, B>() {}
///
/// fn main() {
///     requires_joinable::<Unrelated, Parent>();
/// }
/// ```
#[cfg(doctest)]
struct _CompileFailTests;

/// Compiles and runs the README's Rust examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../README.md")]
struct _ReadmeDoctests;
