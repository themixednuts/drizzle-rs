//! PostgreSQL support for drizzle-rs
//!
//! This crate provides PostgreSQL-specific types, query builders, and utilities
//! for the drizzle-rs ORM. It supports runtime-agnostic database connections
//! through optional sqlx integration.

// Re-export common types and traits
pub use drizzle_core::{OrderBy, SQL, ToSQL};

// Re-export PostgreSQL-specific modules
pub mod builder;
pub mod common;
pub mod helpers;
pub mod traits;
pub mod values;

// Re-export key types for easier access
pub use builder::{BuilderInit, BuilderState, CTEInit, ExecutableState, QueryBuilder};
pub use common::{PostgresSchemaType, Number, Join, JoinType};
pub use traits::{PostgresColumn, PostgresColumnInfo, PostgresEnum};
pub use values::{InsertValue, PostgresValue, ValueWrapper};

// Transaction types - PostgreSQL specific isolation levels
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PostgresTransactionType {
    /// READ UNCOMMITTED isolation level
    ReadUncommitted,
    /// READ COMMITTED isolation level (PostgreSQL default)
    ReadCommitted,
    /// REPEATABLE READ isolation level
    RepeatableRead,
    /// SERIALIZABLE isolation level
    Serializable,
}

impl Default for PostgresTransactionType {
    fn default() -> Self {
        Self::ReadCommitted
    }
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
