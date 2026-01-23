//! PostgreSQL support for drizzle-rs
//!
//! This crate provides PostgreSQL-specific types, query builders, and utilities.

#![allow(unexpected_cfgs)]

// Public modules
pub mod attrs;
pub mod builder;
pub mod common;
pub mod expressions;
pub mod helpers;
pub mod traits;
pub mod values;

// Re-export key types at crate root
pub use builder::{BuilderInit, CTEInit, ExecutableState, QueryBuilder};
pub use common::{Join, JoinType, Number, PostgresSchemaType};
pub use traits::{
    DrizzleRow, FromPostgresValue, PostgresColumn, PostgresColumnInfo, PostgresEnum, PostgresTable,
    PostgresTableInfo,
};
pub use values::{PostgresInsertValue, PostgresValue, ValueWrapper};

// Re-export Row type from the active postgres driver
#[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
pub use postgres::Row;
#[cfg(feature = "tokio-postgres")]
pub use tokio_postgres::Row;

// Re-export ParamBind for use in params! macro
pub use drizzle_core::ParamBind;

// Type alias for convenience
pub type PostgresSQL<'a> = drizzle_core::SQL<'a, PostgresValue<'a>>;

/// PostgreSQL transaction isolation levels
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
    [$($param:tt),+ $(,)?] => {
        [
            $(
                $crate::params_internal!($param)
            ),+
        ]
    };
}

/// Internal helper macro for params!
#[macro_export]
macro_rules! params_internal {
    ({ $key:ident: $value:expr }) => {
        $crate::ParamBind::named(stringify!($key), $crate::PostgresValue::from($value))
    };
    ($value:expr) => {
        $crate::ParamBind::positional($crate::PostgresValue::from($value))
    };
}
