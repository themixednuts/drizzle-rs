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

// Driver builder modules (Drizzle connection wrappers)
#[macro_use]
mod builder;

// Transaction support modules
#[macro_use]
mod transaction;

#[macro_use]
mod macros;

// Re-export macros from modules
#[doc(hidden)]
pub(crate) use drizzle_builder_join_impl;
#[doc(hidden)]
pub(crate) use transaction_builder_join_impl;

// =============================================================================
// Root-level exports
// =============================================================================

/// Result type for drizzle operations
pub use drizzle_core::error::Result;

/// SQL template macro
pub use drizzle_macros::sql;

/// Database dialect enum
pub use drizzle_types::Dialect;

/// Error types
pub mod error {
    pub use drizzle_core::error::DrizzleError;
}

/// DDL schema types for migrations and table definitions.
///
/// This module provides types for database schema definitions used by migrations:
/// - `sqlite::ddl` - SQLite DDL types (TableDef, ColumnDef, ForeignKeyDef, etc.)
/// - `postgres::ddl` - PostgreSQL DDL types
///
/// **Note:** For SQL expression type markers (`Int`, `Text`, `Bool`), see [`core::types`].
pub use drizzle_types as ddl;

/// Migration utilities for schema management.
///
/// Provides types for generating and comparing database schemas:
/// - `Schema` - Schema representation trait
/// - `Snapshot` - Point-in-time schema snapshot
/// - `sqlite::SQLiteSnapshot` - SQLite-specific snapshot
/// - `postgres::PostgresSnapshot` - PostgreSQL-specific snapshot
pub use drizzle_migrations as migrations;

// =============================================================================
// Core module - shared functionality
// =============================================================================

/// Core types and traits shared across all database implementations.
///
/// # Module Structure
///
/// - **Types**: `SQL`, `SQLChunk`, `Token`, `Param`, `ParamBind`, `OrderBy`, etc.
/// - **Traits**: `ToSQL`, `SQLComparable`, `SQLTable`, `SQLColumn`, etc.
/// - **Expressions**: Type-safe expressions including comparisons (`eq`, `neq`, `gt`),
///   logical operators (`and`, `or`), and aggregates (`count`, `sum`, `alias`).
///
/// # Import Patterns
///
/// ```rust,ignore
/// // Import expressions for WHERE clauses and SELECT expressions
/// use drizzle::core::expr::{eq, gt, and, or, alias, count, sum};
/// ```
pub mod core {
    // ==========================================================================
    // Types - fundamental SQL building blocks
    // ==========================================================================

    /// Core SQL types for building queries
    pub use drizzle_core::{OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, Token};

    // ==========================================================================
    // Traits - interfaces for SQL generation
    // ==========================================================================

    /// Conversion trait for SQL generation
    pub use drizzle_core::ToSQL;

    /// Comparison trait for SQL expressions
    pub use drizzle_core::SQLComparable;

    /// All core traits (SQLTable, SQLColumn, SQLSchema, SQLModel, etc.)
    pub use drizzle_core::traits::*;

    // ==========================================================================
    // Prepared statements
    // ==========================================================================

    /// Prepared statement support
    pub use drizzle_core::prepared::{OwnedPreparedStatement, PreparedStatement};

    // ==========================================================================
    // Type-safe expressions
    // ==========================================================================

    /// SQL type markers for compile-time type checking.
    ///
    /// Includes `Int`, `BigInt`, `Text`, `Bool`, `Float`, `Double`, etc.
    /// Used by the `Expr` trait to track expression types.
    pub use drizzle_core::types;

    /// SQL expressions and conditions (type-safe).
    ///
    /// Includes aggregate functions (`count`, `sum`, `avg`, `min`, `max`), comparisons
    /// (`eq`, `neq`, `gt`, `gte`, `lt`, `lte`), logical operators (`and`, `or`, `not`),
    /// and more (`like`, `in_array`, `is_null`, `between`, `exists`, etc.)
    ///
    /// ```rust,ignore
    /// use drizzle::core::expr::{eq, gt, and, count, alias};
    ///
    /// db.select(alias(count(user.id), "total"))
    ///   .from(user)
    ///   .r#where(and([eq(user.name, "Alice"), gt(user.age, 21)]))
    /// ```
    pub use drizzle_core::expr;

    // ==========================================================================
    // Hidden re-exports for macro-generated code
    // ==========================================================================

    #[doc(hidden)]
    pub use drizzle_core::impl_try_from_int;

    #[doc(hidden)]
    pub use drizzle_core::schema::SQLEnumInfo;
}

// =============================================================================
// SQLite module
// =============================================================================

/// SQLite-specific types, macros, and query builder.
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite {
    // Macros
    pub use drizzle_macros::{SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable};
    pub use drizzle_sqlite::params;

    // Query builder
    pub use drizzle_sqlite::QueryBuilder;

    // Types and traits
    pub use drizzle_sqlite::{
        DrizzleRow, FromSQLiteValue, SQLiteColumn, SQLiteColumnInfo, SQLiteSchemaType, SQLiteTable,
        SQLiteTableInfo, SQLiteTransactionType, SQLiteValue,
    };

    // Sub-modules for advanced use
    pub use drizzle_sqlite::{attrs, builder, common, conditions, expression, traits, values};

    // Driver modules
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

    /// SQLite prelude - import this for schema declarations.
    ///
    /// Since proc macros qualify paths, you mainly need macros and attribute markers.
    pub mod prelude {
        // Macros for schema declarations
        pub use drizzle_macros::{
            SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable,
        };

        // Parameter macro for prepared statements
        pub use drizzle_sqlite::params;

        // Core types and traits (but not expressions/conditions)
        pub use crate::core::{OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, Token};
        pub use crate::core::{SQLComparable, ToSQL};
        pub use drizzle_core::prepared::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::traits::*;

        // SQLite-specific types and traits
        pub use super::{
            SQLiteColumn, SQLiteColumnInfo, SQLiteSchemaType, SQLiteTableInfo, SQLiteValue,
        };
        pub use drizzle_sqlite::{SQLiteInsertValue, SQLiteTable};

        // Attribute markers (NAME, PRIMARY, etc.)
        pub use drizzle_sqlite::attrs::*;
    }
}

// =============================================================================
// PostgreSQL module
// =============================================================================

/// PostgreSQL-specific types, macros, and query builder.
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres {
    // Macros
    pub use drizzle_macros::{
        PostgresEnum, PostgresFromRow, PostgresIndex, PostgresSchema, PostgresTable,
    };
    pub use drizzle_postgres::params;

    // Query builder
    pub use drizzle_postgres::QueryBuilder;

    // Types and traits
    pub use drizzle_postgres::{
        DrizzleRow, FromPostgresValue, PostgresColumn, PostgresColumnInfo, PostgresEnum,
        PostgresSchemaType, PostgresTable, PostgresTableInfo, PostgresTransactionType,
        PostgresValue,
    };

    // Re-export Row type (conditionally available based on driver feature)
    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    pub use drizzle_postgres::Row;
    #[cfg(feature = "tokio-postgres")]
    pub use drizzle_postgres::Row;

    // Sub-modules for advanced use
    pub use drizzle_postgres::{attrs, builder, common, traits, values};

    // Driver modules
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

    /// PostgreSQL prelude - import this for schema declarations.
    ///
    /// Since proc macros qualify paths, you mainly need macros and attribute markers.
    pub mod prelude {
        // Macros for schema declarations
        pub use drizzle_macros::{
            PostgresEnum, PostgresFromRow, PostgresIndex, PostgresSchema, PostgresTable,
        };

        // Core types and traits (but not expressions/conditions)
        pub use crate::core::{OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, Token};
        pub use crate::core::{SQLComparable, ToSQL};
        pub use drizzle_core::prepared::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::traits::*;

        // PostgreSQL-specific types and traits
        pub use super::{
            PostgresColumn, PostgresColumnInfo, PostgresSchemaType, PostgresTableInfo,
            PostgresValue,
        };
        pub use drizzle_postgres::{PostgresInsertValue, PostgresTable};

        // Attribute markers (NAME, PRIMARY, etc.)
        pub use drizzle_postgres::attrs::*;
    }
}

// =============================================================================
// MySQL module (WIP)
// =============================================================================

/// MySQL-specific types, macros, and query builder.
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql {}

// =============================================================================
// Note: No global prelude - use database-specific preludes
// =============================================================================
//
// For database-specific functionality, use:
// - `drizzle::sqlite::prelude::*` for SQLite
// - `drizzle::postgres::prelude::*` for PostgreSQL
//
// For expressions and conditions, import explicitly:
// - `use drizzle::core::expr::{eq, gt, and, or, alias, count, sum};`

// =============================================================================
// Tests
// =============================================================================

// TODO: Tests disabled - Schema derive macros need updating to use Cow<'static, str> API
// #[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
// #[cfg(test)]
// mod sqlite_tests {
//     use crate::sqlite::QueryBuilder;
//     use crate::sqlite::prelude::*;
//
//     #[SQLiteTable(NAME = "Users")]
//     pub struct User {
//         #[column(PRIMARY)]
//         id: i32,
//         name: String,
//         email: Option<String>,
//     }
//
//     #[SQLiteTable(NAME = "Posts")]
//     pub struct Post {
//         #[column(PRIMARY)]
//         id: i32,
//         title: String,
//     }
//
//     #[derive(SQLiteSchema)]
//     pub struct Schema {
//         pub user: User,
//         pub post: Post,
//     }
//
//     #[test]
//     fn test_schema_macro() {
//         let Schema { user, .. } = Schema::new();
//         let builder = QueryBuilder::new::<Schema>();
//         let query = builder.select(user.id).from(user);
//         assert_eq!(query.to_sql().sql(), r#"SELECT "Users"."id" FROM "Users""#);
//     }
// }
//
// #[cfg(feature = "postgres")]
// #[cfg(test)]
// mod postgres_tests {
//     use crate::core::expr::eq;
//     use crate::postgres::QueryBuilder;
//     use crate::postgres::prelude::*;
//
//     #[derive(Debug, Clone, Default, PostgresEnum)]
//     pub enum Status {
//         #[default]
//         Active,
//         Inactive,
//     }
//
//     #[PostgresTable(name = "users")]
//     pub struct User {
//         #[column(primary, serial)]
//         id: i32,
//         name: String,
//         email: Option<String>,
//         #[column(enum)]
//         status: Status,
//     }
//
//     #[derive(PostgresSchema)]
//     pub struct Schema {
//         pub user: User,
//     }
//
//     #[test]
//     fn test_postgres_query() {
//         let Schema { user, .. } = Schema::new();
//         let qb = QueryBuilder::new::<Schema>();
//         let stmt = qb.select(user.id).from(user).r#where(eq(user.id, 12));
//         let sql = stmt.to_sql();
//         assert_eq!(
//             sql.sql(),
//             r#"SELECT "users"."id" FROM "users" WHERE "users"."id" = $1"#
//         );
//     }
// }
