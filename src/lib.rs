//! # Drizzle for Rust
//!
//! A type-safe SQL query builder for Rust, supporting SQLite and PostgreSQL.
//!
//! ## Quick Start
//!
//! ```rust
//! use drizzle::sqlite::prelude::*;
//! use drizzle::sqlite::rusqlite::Drizzle;
//!
//! #[SQLiteTable(name = "Users")]
//! struct User {
//!     #[column(primary)]
//!     id: i32,
//!     name: String,
//!     email: Option<String>,
//! }
//!
//! #[derive(SQLiteSchema)]
//! struct Schema {
//!     user: User,
//! }
//!
//! # fn main() -> drizzle::Result<()> {
//! let conn = rusqlite::Connection::open_in_memory()?;
//! let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::new());
//!
//! db.create()?;
//! db.insert(user)
//!     .values([InsertUser::new("John Doe").with_email("john@example.com")])
//!     .execute()?;
//!
//! let users: Vec<SelectUser> = db.select(()).from(user).all()?;
//! # Ok(())
//! # }
//! ```
//!
//! ## Database Support
//!
//! | Database   | Driver         | Feature Flag     | Status |
//! |------------|----------------|------------------|--------|
//! | SQLite     | rusqlite       | `rusqlite`       | ✅     |
//! | SQLite     | libsql         | `libsql`         | ✅     |
//! | SQLite     | turso          | `turso`          | ✅     |
//! | PostgreSQL | postgres       | `postgres-sync`  | ✅     |
//! | PostgreSQL | tokio-postgres | `tokio-postgres` | ✅     |
//!
//! For schema declarations, import the database prelude:
//! - `drizzle::sqlite::prelude::*`
//! - `drizzle::postgres::prelude::*`
//!
//! For expressions and conditions, import from `drizzle::core::expr`.

#![cfg_attr(docsrs, feature(doc_cfg, rustdoc_internals))]
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

#[macro_use]
mod builder;

#[macro_use]
mod transaction;

#[macro_use]
mod macros;

#[doc(hidden)]
pub(crate) use drizzle_builder_join_impl;
#[doc(hidden)]
pub(crate) use transaction_builder_join_impl;

/// Result type for drizzle operations.
pub use drizzle_core::error::Result;

/// SQL template macro.
pub use drizzle_macros::sql;

/// Database dialect enum.
pub use drizzle_types::Dialect;

/// Error types.
pub mod error {
    pub use drizzle_core::error::DrizzleError;
}

/// Schema and DDL types used by migrations.
pub use drizzle_types as ddl;

/// Migration helpers and schema snapshots.
pub use drizzle_migrations as migrations;

/// Core traits, SQL types, and expressions shared across drivers.
pub mod core {
    /// SQL building blocks.
    pub use drizzle_core::{OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, Token};

    /// Conversion trait for SQL generation.
    pub use drizzle_core::ToSQL;

    /// Comparison trait for SQL expressions.
    pub use drizzle_core::SQLComparable;

    /// Core traits (SQLTable, SQLColumn, SQLSchema, SQLModel, etc.).
    pub use drizzle_core::traits::*;

    /// Prepared statement types.
    pub use drizzle_core::prepared::{OwnedPreparedStatement, PreparedStatement};

    /// SQL type markers used by expressions.
    pub use drizzle_core::types;

    /// Type-safe expressions and helpers.
    pub use drizzle_core::expr;

    #[doc(hidden)]
    pub use drizzle_core::impl_try_from_int;

    #[doc(hidden)]
    pub use drizzle_core::schema::SQLEnumInfo;
}

/// SQLite types, macros, and query builder.
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite {
    pub use drizzle_macros::{
        SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable, SQLiteView,
    };
    pub use drizzle_sqlite::QueryBuilder;
    pub use drizzle_sqlite::params;
    pub use drizzle_sqlite::{
        DrizzleRow, FromSQLiteValue, SQLiteColumn, SQLiteColumnInfo, SQLiteSchemaType, SQLiteTable,
        SQLiteTableInfo, SQLiteTransactionType, SQLiteValue,
    };
    pub use drizzle_sqlite::{attrs, builder, common, conditions, expression, traits, values};

    #[cfg(feature = "rusqlite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rusqlite")))]
    pub mod rusqlite {
        pub use crate::builder::sqlite::rusqlite::{Drizzle, DrizzleBuilder};
        pub use crate::transaction::sqlite::rusqlite::Transaction;
    }

    #[cfg(feature = "libsql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "libsql")))]
    pub mod libsql {
        pub use crate::builder::sqlite::libsql::{Drizzle, DrizzleBuilder};
        pub use crate::transaction::sqlite::libsql::Transaction;
    }

    #[cfg(feature = "turso")]
    #[cfg_attr(docsrs, doc(cfg(feature = "turso")))]
    pub mod turso {
        pub use crate::builder::sqlite::turso::{Drizzle, DrizzleBuilder};
        pub use crate::transaction::sqlite::turso::Transaction;
    }

    /// SQLite prelude for schema declarations.
    pub mod prelude {
        pub use super::{
            SQLiteColumn, SQLiteColumnInfo, SQLiteSchemaType, SQLiteTableInfo, SQLiteValue,
        };
        pub use crate::core::{OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, Token};
        pub use crate::core::{SQLComparable, ToSQL};
        pub use drizzle_core::prepared::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::traits::*;
        pub use drizzle_macros::{
            SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable, SQLiteView,
        };
        pub use drizzle_sqlite::attrs::*;
        pub use drizzle_sqlite::params;
        pub use drizzle_sqlite::{SQLiteInsertValue, SQLiteTable};
    }
}

/// PostgreSQL types, macros, and query builder.
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres {
    pub use drizzle_macros::{
        PostgresEnum, PostgresFromRow, PostgresIndex, PostgresSchema, PostgresTable, PostgresView,
    };
    pub use drizzle_postgres::QueryBuilder;
    pub use drizzle_postgres::params;
    pub use drizzle_postgres::{
        DrizzleRow, FromPostgresValue, PostgresColumn, PostgresColumnInfo, PostgresEnum,
        PostgresSchemaType, PostgresTable, PostgresTableInfo, PostgresTransactionType,
        PostgresValue,
    };

    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    pub use drizzle_postgres::Row;
    #[cfg(feature = "tokio-postgres")]
    pub use drizzle_postgres::Row;

    pub use drizzle_postgres::{attrs, builder, common, traits, values};

    #[cfg(feature = "postgres-sync")]
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres-sync")))]
    pub mod sync {
        pub use crate::builder::postgres::postgres_sync::{Drizzle, DrizzleBuilder};
        pub use crate::transaction::postgres::postgres_sync::Transaction;
    }

    #[cfg(feature = "tokio-postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio-postgres")))]
    pub mod tokio {
        pub use crate::builder::postgres::tokio_postgres::{Drizzle, DrizzleBuilder};
        pub use crate::transaction::postgres::tokio_postgres::Transaction;
    }

    /// PostgreSQL prelude for schema declarations.
    pub mod prelude {
        pub use super::{
            PostgresColumn, PostgresColumnInfo, PostgresSchemaType, PostgresTableInfo,
            PostgresValue,
        };
        pub use crate::core::{OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, Token};
        pub use crate::core::{SQLComparable, ToSQL};
        pub use drizzle_core::prepared::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::traits::*;
        pub use drizzle_macros::{
            PostgresEnum, PostgresFromRow, PostgresIndex, PostgresSchema, PostgresTable,
            PostgresView,
        };
        pub use drizzle_postgres::attrs::*;
        pub use drizzle_postgres::{PostgresInsertValue, PostgresTable};
    }
}

/// MySQL types and macros (WIP).
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql {}
