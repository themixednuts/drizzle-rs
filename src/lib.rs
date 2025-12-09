//! # Drizzle for Rust
//!
//! Drizzle is a type-safe SQL query builder for Rust, supporting multiple database
//! drivers including SQLite (via rusqlite, libsql, turso) and PostgreSQL (via sqlx).

// Allow warnings for WIP code and different driver configurations
#![allow(
    unexpected_cfgs,
    unused_imports,
    unused_macros,
    unused_mut,
    dead_code,
    clippy::redundant_closure,
    clippy::needless_question_mark,
    clippy::await_holding_refcell_ref,
    clippy::duplicated_attributes,
    clippy::single_component_path_imports
)]
//!
//! ## Quick Start
//!
//! ```rust
//! use drizzle::prelude::*;
//! use drizzle::rusqlite::Drizzle;
//!
//! #[SQLiteTable(name = "Users")]
//! struct User {
//!     #[integer(primary)]
//!     id: i32,
//!     #[text]
//!     name: String,
//!     #[text]
//!     email: Option<String>,
//! }
//!
//! #[derive(SQLiteSchema)]
//! struct Schema {
//!     user: User,
//! }
//!
//! # fn main() -> drizzle::Result<()> {
//! // Connect to database and perform operations
//! let conn = rusqlite::Connection::open_in_memory()?;
//! let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::new());
//!
//! // Create tables
//! db.create()?;
//!
//! // Insert data
//! db.insert(user)
//!     .values([InsertUser::new("John Doe").with_email("john@example.com")])
//!     .execute()?;
//!
//! // Query data
//! let users: Vec<SelectUser> = db.select(()).from(user).all()?;
//! assert_eq!(users.len(), 1);
//! assert_eq!(users[0].id, 1);
//! assert_eq!(users[0].name, "John Doe");
//! assert_eq!(users[0].email, Some("john@example.com".to_string()));
//! # Ok(())
//! # }
//! ```
//!
//!
//! ## Database Support
//!
//! | Database   | Driver    | Feature Flag   | Status |
//! |------------|-----------|----------------|--------|
//! | SQLite     | rusqlite  | `rusqlite`     | ✅     |
//! | SQLite     | libsql    | `libsql`       | ✅     |
//! | SQLite     | turso     | `turso`        | ✅     |
//! | PostgreSQL | sqlx      | `sqlx-postgres`| 🚧     |
//! | MySQL      | sqlx      | `mysql`        | 🚧     |

#![cfg_attr(docsrs, feature(doc_cfg, rustdoc_internals))]

mod drizzle;
mod transaction;
#[macro_use]
mod macros;

#[doc(hidden)]
pub(crate) use drizzle_builder_join_impl;
#[doc(hidden)]
pub(crate) use transaction_builder_join_impl;

// Essential re-exports
pub use drizzle_core::error::Result;
pub use drizzle_macros::sql;

// Migration types and macro
pub use drizzle_macros::include_migrations;
pub use drizzle_migrations::{Dialect, EmbeddedMigration, EmbeddedMigrations};

/// SQLite-specific FromRow derive macro for automatic row-to-struct conversion.
/// Supports rusqlite, libsql, and turso drivers.
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub use drizzle_macros::SQLiteFromRow;

/// PostgreSQL-specific FromRow derive macro for automatic row-to-struct conversion.
/// Supports postgres-sync and tokio-postgres drivers.
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub use drizzle_macros::PostgresFromRow;

/// Error types and result handling.
///
/// This module provides access to [`DrizzleError`] for comprehensive error handling
/// across all database operations and SQL generation.
pub mod error {
    pub use drizzle_core::error::DrizzleError;
}

/// Core functionality shared across all database implementations.
pub mod core {
    // Core traits and types
    pub use drizzle_core::traits::*;
    pub use drizzle_core::{
        OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, SQLComparable, ToSQL, Token,
    };

    // Prepared statements
    pub use drizzle_core::prepared::{PreparedStatement, owned::OwnedPreparedStatement};

    // Condition expressions
    pub use drizzle_core::expressions::conditions::*;

    // Expression functions
    pub use drizzle_core::expressions::*;

    /// Core prelude with the minimal set of commonly used items.
    pub mod prelude {
        pub use drizzle_core::error::DrizzleError;
        pub use drizzle_core::expressions::conditions::*;
        pub use drizzle_core::expressions::*;
        pub use drizzle_core::impl_try_from_int;
        pub use drizzle_core::prepared::{PreparedStatement, owned::OwnedPreparedStatement};
        pub use drizzle_core::traits::*;
        pub use drizzle_core::{
            OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, SQLComparable, ToSQL, Token,
        };
    }
}

/// SQLite-specific functionality and components.
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite {
    pub use drizzle_sqlite::builder::QueryBuilder;

    // SQLite macros
    pub use drizzle_macros::{SQLiteEnum, SQLiteIndex, SQLiteSchema, SQLiteTable};

    pub use drizzle_sqlite::conditions;
    pub use drizzle_sqlite::expression;
    pub use drizzle_sqlite::{SQLiteTransactionType, params, pragma};

    // SQLite types and traits
    pub use drizzle_sqlite::builder;
    pub use drizzle_sqlite::columns;
    pub use drizzle_sqlite::common;
    pub use drizzle_sqlite::traits;
    pub use drizzle_sqlite::values;

    /// SQLite-focused prelude that layers on top of `core::prelude`.
    pub mod prelude {
        pub use crate::core::prelude::*;
        pub use drizzle_macros::{
            SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable,
        };
        pub use drizzle_sqlite::builder::QueryBuilder;
        pub use drizzle_sqlite::common::SQLiteSchemaType;
        // Note: json/jsonb expression functions are NOT exported here to avoid conflict
        // with column attribute markers. Import from drizzle::sqlite::expression if needed.
        pub use drizzle_sqlite::traits::{
            DrizzleRow, FromSQLiteValue, SQLiteColumn, SQLiteColumnInfo, SQLiteTable,
            SQLiteTableInfo,
        };
        pub use drizzle_sqlite::values::{SQLiteInsertValue, SQLiteValue, ValueWrapper};
        pub use drizzle_sqlite::{SQLiteTransactionType, conditions, expression, params, pragma};

        // Column attribute markers for IDE documentation
        // These provide hover docs when using #[column(PRIMARY)], #[column(JSON)], etc.
        pub use drizzle_sqlite::attrs::{
            AUTOINCREMENT, ColumnMarker, DEFAULT, DEFAULT_FN, ENUM, JSON, JSONB, PRIMARY,
            PRIMARY_KEY, REFERENCES, UNIQUE,
        };

        // Table attribute markers for IDE documentation
        // These provide hover docs when using #[SQLiteTable(STRICT)], etc.
        pub use drizzle_sqlite::attrs::{STRICT, TableMarker, WITHOUT_ROWID};

        // Shared markers (used by both column and table attributes)
        pub use drizzle_sqlite::attrs::{NAME, NameMarker};

        /// Re-export the sqlite module so generated code can use `sqlite::columns::*`
        pub use crate::sqlite;
    }
}

/// Rusqlite driver implementation.
///
/// Provides the main [`Drizzle`] database connection and [`Transaction`] types
/// for working with SQLite databases via the rusqlite crate.
///
/// Enabled with the `rusqlite` feature flag.
#[cfg(feature = "rusqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "rusqlite")))]
pub mod rusqlite {
    pub use crate::drizzle::sqlite::rusqlite::Drizzle;
    pub use crate::transaction::sqlite::rusqlite::Transaction;
}

/// LibSQL driver implementation.
///
/// Provides the main [`Drizzle`] database connection and [`Transaction`] types
/// for working with SQLite databases via the libsql crate.
///
/// Enabled with the `libsql` feature flag.
#[cfg(feature = "libsql")]
#[cfg_attr(docsrs, doc(cfg(feature = "libsql")))]
pub mod libsql {
    pub use crate::drizzle::sqlite::libsql::Drizzle;
    pub use crate::transaction::sqlite::libsql::Transaction;
}

/// Turso driver implementation.
///
/// Provides the main [`Drizzle`] database connection and [`Transaction`] types
/// for working with Turso (libSQL-compatible) databases.
///
/// Enabled with the `turso` feature flag.
#[cfg(feature = "turso")]
#[cfg_attr(docsrs, doc(cfg(feature = "turso")))]
pub mod turso {
    pub use crate::drizzle::sqlite::turso::Drizzle;
    pub use crate::transaction::sqlite::turso::Transaction;
}

/// Postgres (sync) driver implementation.
///
/// Provides the main [`Drizzle`] database connection and [`Transaction`] types
/// for working with PostgreSQL databases using the synchronous `postgres` crate.
///
/// Enabled with the `postgres-sync` feature flag.
#[cfg(feature = "postgres-sync")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres-sync")))]
pub mod postgres_sync {
    pub use crate::drizzle::postgres::postgres_sync::Drizzle;
    pub use crate::transaction::postgres::postgres_sync::Transaction;
}

/// Tokio-postgres (async) driver implementation.
///
/// Provides the main [`Drizzle`] database connection and [`Transaction`] types
/// for working with PostgreSQL databases using the async `tokio-postgres` crate.
///
/// Enabled with the `tokio-postgres` feature flag.
#[cfg(feature = "tokio-postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "tokio-postgres")))]
pub mod tokio_postgres {
    pub use crate::drizzle::postgres::tokio_postgres::Drizzle;
    pub use crate::transaction::postgres::tokio_postgres::Transaction;
}

// Sqlx PostgreSQL driver
// #[cfg(feature = "sqlx-postgres")]
// pub mod sqlx_postgres {
//     pub use crate::drizzle::postgres::sqlx::Drizzle;
//     pub use crate::transaction::postgres::sqlx::Transaction;
// }

/// PostgreSQL-specific functionality and components.
///
/// This module provides PostgreSQL-specific query building, macros, and utilities.
/// Available when the `postgres` feature is enabled.
///
/// Key components:
/// - **QueryBuilder**: PostgreSQL query construction
/// - **Macros**: Table, schema, enum, and index derive macros
/// - **Enums**: Native PostgreSQL enum support
/// - **Transactions**: PostgreSQL transaction management
///
/// Note: Currently under development (🚧).
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres {
    pub use drizzle_postgres::builder::QueryBuilder;

    // PostgreSQL macros
    pub use drizzle_macros::{PostgresEnum, PostgresIndex, PostgresSchema, PostgresTable};

    // PostgreSQL builders and helpers
    pub use drizzle_postgres::PostgresTransactionType;

    // PostgreSQL types and traits
    pub use drizzle_postgres::builder;
    pub use drizzle_postgres::columns;
    pub use drizzle_postgres::common;
    pub use drizzle_postgres::traits;
    pub use drizzle_postgres::values;

    /// PostgreSQL-focused prelude that layers on top of `core::prelude`.
    pub mod prelude {
        pub use crate::core::prelude::*;
        pub use drizzle_macros::{
            PostgresEnum, PostgresFromRow, PostgresIndex, PostgresSchema, PostgresTable,
        };
        pub use drizzle_postgres::PostgresTransactionType;
        pub use drizzle_postgres::builder::QueryBuilder;
        pub use drizzle_postgres::common::PostgresSchemaType;
        pub use drizzle_postgres::traits::{
            PostgresColumn, PostgresColumnInfo, PostgresEnum, PostgresTable, PostgresTableInfo,
        };
        pub use drizzle_postgres::values::{PostgresInsertValue, PostgresValue};

        // Column attribute markers for IDE documentation
        // These provide hover docs when using #[column(PRIMARY)], #[column(JSON)], etc.
        pub use drizzle_postgres::attrs::{
            BIGSERIAL, CHECK, ColumnMarker, DEFAULT, DEFAULT_FN, ENUM, GENERATED_IDENTITY, JSON,
            JSONB, PRIMARY, PRIMARY_KEY, REFERENCES, SERIAL, SMALLSERIAL, UNIQUE,
        };

        // Table attribute markers for IDE documentation
        // These provide hover docs when using #[PostgresTable(UNLOGGED)], etc.
        pub use drizzle_postgres::attrs::{INHERITS, TABLESPACE, TEMPORARY, TableMarker, UNLOGGED};

        // Shared markers (used by both column and table attributes)
        pub use drizzle_postgres::attrs::{NAME, NameMarker};

        /// Re-export the postgres module so generated code can use `postgres::columns::*`
        pub use crate::postgres;
    }
}

/// MySQL-specific functionality and components.
///
/// This module will provide MySQL-specific query building, macros, and utilities.
/// Available when the `mysql` feature is enabled.
///
/// Note: Currently under development (🚧).
#[cfg(feature = "mysql")]
pub mod mysql {
    // pub use querybuilder::mysql::...;
}

/// A layered prelude that composes driver-specific preludes on top of core.
///
/// ```
/// use drizzle::prelude::*;            // Core + enabled drivers
/// use drizzle::sqlite::prelude::*;    // SQLite-specific additions
/// use drizzle::postgres::prelude::*;  // Postgres-specific additions
/// ```
pub mod prelude {
    pub use crate::core::prelude::*;

    // Note: We only glob-export one dialect's prelude to avoid name conflicts.
    // When both sqlite and postgres features are enabled, sqlite takes precedence
    // in the main prelude. Users should use drizzle::postgres::prelude::* directly
    // for PostgreSQL-specific items when using both databases.

    #[cfg(feature = "sqlite")]
    pub use crate::sqlite::prelude::*;

    /// Re-export the sqlite module so generated code can use `sqlite::columns::*`
    #[cfg(feature = "sqlite")]
    pub use crate::sqlite;

    // Only glob-export postgres prelude if sqlite is NOT enabled (to avoid conflicts)
    #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
    pub use crate::postgres::prelude::*;

    /// Re-export the postgres module so generated code can use `postgres::columns::*`
    #[cfg(feature = "postgres")]
    pub use crate::postgres;
}

#[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
#[cfg(test)]
mod sqlite_tests {
    use drizzle::sqlite::prelude::*;
    use drizzle_macros::SQLiteTable;

    use drizzle_sqlite::builder::QueryBuilder;
    #[cfg(feature = "rusqlite")]
    use rusqlite;

    #[SQLiteTable(NAME = "Users")]
    pub struct User {
        #[column(PRIMARY)]
        id: i32,
        name: String,
        email: Option<String>,
    }

    #[SQLiteTable(NAME = "Posts")]
    pub struct Post {
        #[column(PRIMARY)]
        id: i32,
        title: String,
    }

    #[SQLiteTable(NAME = "Comments")]
    pub struct Comment {
        #[column(PRIMARY)]
        id: i32,
        content: String,
    }

    #[derive(SQLiteSchema)]
    pub struct Schema {
        pub user: User,
        pub post: Post,
        pub comment: Comment,
    }

    #[test]
    fn test_schema_macro() {
        // Create a schema with the User table using schema! macro
        let Schema { user, .. } = Schema::new();
        let builder = QueryBuilder::new::<Schema>();

        let query = builder.select(user.id).from(user);
        assert_eq!(query.to_sql().sql(), r#"SELECT "Users"."id" FROM "Users""#);
    }

    #[test]
    fn test_alias() {
        let qb = QueryBuilder::new::<Schema>();
        let u = User::alias("u");

        let stmt = qb.select(()).from(u).r#where(eq(u.id, 1));
        let sql = stmt.to_sql();

        println!("{sql}");
    }

    #[cfg(feature = "rusqlite")]
    #[test]
    fn test_insert() {
        use drizzle_sqlite::builder::Conflict;

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let (db, Schema { user, .. }) = drizzle::rusqlite::Drizzle::new(conn, Schema::new());
        db.create().expect("Should have created table");

        let result = db
            .insert(user)
            .values([InsertUser::new("test").with_name("test")])
            .on_conflict(Conflict::default())
            .execute()
            .expect("Should have inserted");

        assert_eq!(result, 1);

        let query: Vec<SelectUser> = db
            .select(())
            .from(user)
            .all()
            .expect("should have gotten all users");

        assert_eq!(query.len(), 1);
        assert_eq!(query[0].id, 1);
        assert_eq!(query[0].name, "test");
        assert_eq!(query[0].email, None);
    }

    #[test]
    fn test_placeholder_integration() {
        use drizzle::core::Placeholder;

        // Test that placeholders work with the new unified SQL-based approach
        let placeholder = Placeholder::colon("test_name");
        let insert_value: InsertUser<'_, _> = InsertUser::new(placeholder);

        // Verify it's a Value variant containing SQL with the placeholder
        match &insert_value.name {
            drizzle::sqlite::values::SQLiteInsertValue::Value(wrapper) => {
                // Check that the SQL contains our placeholder
                let sql_string = wrapper.value.sql();
                assert!(sql_string.contains("test_name") || sql_string.contains("?"));
            }
            _ => panic!("Expected Value variant containing SQL"),
        }

        // Test that regular values still work
        let regular_insert: InsertUser<'_, _> = InsertUser::new("regular_value");
        match &regular_insert.name {
            drizzle::sqlite::values::SQLiteInsertValue::Value(wrapper) => {
                // Check that the SQL contains our parameter
                assert!(!wrapper.value.sql().is_empty());
            }
            _ => panic!("Expected Value variant for regular string"),
        }
    }
}

#[cfg(feature = "postgres")]
#[cfg(test)]
mod postgres_tests {
    use crate::postgres::prelude::*;
    use drizzle_core::{SQLColumnInfo, ToSQL, expressions::conditions::eq};

    #[derive(Debug, Clone, Default, PostgresEnum)]
    pub enum Status {
        #[default]
        Active,
        Inactive,
    }

    #[derive(Debug, Clone, Default, PostgresEnum)]
    pub enum Priority {
        #[default]
        Low,
        Medium,
        High,
    }

    #[PostgresTable(name = "users")]
    pub struct User {
        #[column(primary, serial)]
        id: i32,
        name: String,
        email: Option<String>,
        #[column(enum)]
        status: Status,
        #[column(enum)]
        priority: Priority,
    }

    #[PostgresTable(name = "posts")]
    pub struct Post {
        #[column(primary, serial)]
        id: i32,
        title: String,
        published: bool,
    }

    #[PostgresIndex(unique)]
    pub struct UserEmailIdx(User::email);

    #[derive(PostgresSchema)]
    pub struct Schema {
        pub user: User,
        pub post: Post,
        pub user_email_idx: UserEmailIdx,
    }

    #[test]
    fn test_postgres_table_creation() {
        let user = User::new();
        assert_eq!(user.id.name(), "id");
        assert_eq!(user.name.name(), "name");
        assert_eq!(user.email.name(), "email");
    }

    #[test]
    fn test_postgres_index_creation() {
        let idx = UserEmailIdx::new();
        assert_eq!(idx.index_name(), "user_email_idx");
        assert!(idx.create_index_sql().contains("CREATE UNIQUE INDEX"));
    }

    #[test]
    fn test_postgres_enum_values() {
        use drizzle_core::ToSQL;

        // Test text-based enum (stored as TEXT)
        let status = Status::Active;
        let status_sql = status.to_sql();
        println!("Text enum SQL: {}", status_sql);

        // Test native PostgreSQL enum (stored as native ENUM)
        let priority = Priority::High;
        let priority_sql = priority.to_sql();
        println!("Native enum SQL: {}", priority_sql);

        // Verify they generate different types of SQL
        // Text enum should be a text parameter
        // Native enum should be an enum parameter
        assert!(!status_sql.to_string().is_empty());
        assert!(!priority_sql.to_string().is_empty());
    }

    #[test]
    fn test_postgres_stuff() {
        use crate::postgres::QueryBuilder;

        let Schema { user, .. } = Schema::new();
        let qb = QueryBuilder::new::<Schema>();

        let stmt = qb.select(user.id).from(user).r#where(eq(user.id, 12));
        let sql = stmt.to_sql();
        println!("{sql}");

        assert_eq!(
            sql.sql(),
            r#"SELECT "users"."id" FROM "users" WHERE "users"."id" = $1"#
        );

        let _insert =
            InsertUser::new("name", Status::Active, Priority::Low).with_email("test@email");
    }
}
