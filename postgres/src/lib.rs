//! PostgreSQL support for drizzle-rs
//!
//! This crate provides PostgreSQL-specific types, query builders, and utilities
//! for the drizzle-rs ORM. It supports runtime-agnostic database connections
//! through optional sqlx integration.

#![allow(unexpected_cfgs)]

// Re-export common types and traits
pub use drizzle_core::{OrderBy, SQL, ToSQL};

// Re-export PostgreSQL-specific modules
pub mod attrs;
pub mod builder;
pub mod common;
pub mod helpers;
pub mod traits;
pub mod values;

/// Prelude for PostgreSQL - import commonly used traits and attribute markers.
pub mod prelude {
    pub use super::PostgresTransactionType;
    pub use super::traits::{DrizzleRow, FromPostgresValue, PostgresColumn, PostgresEnum};

    // Value types for model operations (needed for JSON serialization in macros)
    pub use super::values::{PostgresInsertValue, PostgresValue, ValueWrapper};

    // Core types for building SQL
    pub use drizzle_core::SQL;

    // Re-export core traits and types needed by macro-generated code
    pub use super::common::PostgresSchemaType;
    pub use super::traits::{PostgresColumnInfo, PostgresTable, PostgresTableInfo};
    pub use drizzle_core::error::DrizzleError;
    pub use drizzle_core::{
        SQLColumn, SQLColumnInfo, SQLIndexInfo, SQLModel, SQLParam, SQLPartial, SQLSchema,
        SQLSchemaImpl, SQLTable, SQLTableInfo, ToSQL, Token,
    };

    // Builder for query construction
    pub use super::builder::QueryBuilder;

    // Re-export modules directly so macro-generated code can use `traits::*`, etc.
    pub use crate::attrs::*;
    pub use crate::common;
    pub use crate::traits;
    pub use crate::values;

    // Shared markers (used by both column and table attributes)
    pub use super::attrs::{NAME, NameMarker};
}

// Re-export key types for easier access
pub use builder::{BuilderInit, BuilderState, CTEInit, ExecutableState, QueryBuilder};
pub use common::{Join, JoinType, Number, PostgresSchemaType};
pub use traits::{DrizzleRow, FromPostgresValue, PostgresColumn, PostgresColumnInfo, PostgresEnum};
pub use values::{PostgresInsertValue, PostgresValue, ValueWrapper};

// Re-export Row type from the active postgres driver.
// Note: postgres::Row is a re-export of tokio_postgres::Row, so they're the same type.
// We prefer tokio_postgres when available since it's the underlying implementation.
#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
pub use postgres::Row;
#[cfg(feature = "tokio-postgres")]
pub use tokio_postgres::Row;

// Transaction types - PostgreSQL specific isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PostgresTransactionType {
    /// READ UNCOMMITTED isolation level
    ReadUncommitted,
    /// READ COMMITTED isolation level (PostgreSQL default)
    #[default]
    ReadCommitted,
    /// REPEATABLE READ isolation level
    RepeatableRead,
    /// SERIALIZABLE isolation level
    Serializable,
}

impl std::fmt::Display for PostgresTransactionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let level = match self {
            PostgresTransactionType::ReadUncommitted => "READ UNCOMMITTED",
            PostgresTransactionType::ReadCommitted => "READ COMMITTED",
            PostgresTransactionType::RepeatableRead => "REPEATABLE READ",
            PostgresTransactionType::Serializable => "SERIALIZABLE",
        };
        write!(f, "{}", level)
    }
}

pub type PostgresSQL<'a> = SQL<'a, PostgresValue<'a>>;

pub trait ToPostgresSQL<'a>: ToSQL<'a, PostgresValue<'a>> {
    fn to_pg_sql(&self) -> PostgresSQL<'a> {
        self.to_sql()
    }
}
impl<'a, T: ToSQL<'a, PostgresValue<'a>>> ToPostgresSQL<'a> for T {}

// Re-export ParamBind for use in macros
pub use drizzle_core::ParamBind;

/// Creates an array of SQL parameters for binding values to placeholders.
///
/// # Syntax
/// - `{ name: value }` - Named parameter (creates :name placeholder)
///
/// # Examples
///
/// ```
/// use drizzle_postgres::params;
///
/// let params = params![{ name: "alice" }, { active: true }];
/// ```
#[macro_export]
macro_rules! params {
    // Multiple parameters - creates a fixed-size array of ParamBind structs
    [$($param:tt),+ $(,)?] => {
        [
            $(
                $crate::params_internal!($param)
            ),+
        ]
    };
}

/// Internal helper macro for params! - converts individual items to ParamBind structs
#[macro_export]
macro_rules! params_internal {
    // Named parameter
    ({ $key:ident: $value:expr }) => {
        $crate::ParamBind::new(stringify!($key), $crate::PostgresValue::from($value))
    };
    // Positional parameter
    ($value:expr) => {
        $crate::ParamBind::new("", $crate::PostgresValue::from($value))
    };
}
