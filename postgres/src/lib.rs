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
pub mod columns;
pub mod common;
pub mod helpers;
pub mod traits;
pub mod values;

/// Prelude for PostgreSQL - import commonly used traits and attribute markers.
pub mod prelude {
    pub use super::PostgresTransactionType;
    pub use super::traits::{DrizzleRow, FromPostgresValue, PostgresColumn, PostgresEnum};

    // Column attribute markers for IDE documentation
    // These are used in #[column(...)] attributes
    pub use super::attrs::{
        BIGSERIAL, CHECK, ColumnMarker, DEFAULT, DEFAULT_FN, ENUM, GENERATED_IDENTITY, JSON, JSONB,
        PRIMARY, PRIMARY_KEY, REFERENCES, SERIAL, SMALLSERIAL, UNIQUE,
    };

    // Table attribute markers for IDE documentation
    // These are used in #[PostgresTable(...)] attributes
    pub use super::attrs::{INHERITS, TABLESPACE, TEMPORARY, TableMarker, UNLOGGED};

    // Shared markers (used by both column and table attributes)
    pub use super::attrs::{NAME, NameMarker};
}

// Re-export key types for easier access
pub use builder::{BuilderInit, BuilderState, CTEInit, ExecutableState, QueryBuilder};
pub use common::{Join, JoinType, Number, PostgresSchemaType};
pub use traits::{DrizzleRow, FromPostgresValue, PostgresColumn, PostgresColumnInfo, PostgresEnum};
pub use values::{PostgresInsertValue, PostgresValue, ValueWrapper};

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
