use drizzle_core::schema::SQLEnumInfo;
use drizzle_core::traits::SQLViewInfo;
use drizzle_core::{SQLIndexInfo, SQLSchemaType};

use crate::traits::PostgresTableInfo;

/// The type of database object
#[derive(Debug, Clone)]
pub enum PostgresSchemaType {
    /// A regular table
    Table(&'static dyn PostgresTableInfo),
    /// A view
    View(&'static dyn SQLViewInfo),
    /// An index
    Index(&'static dyn SQLIndexInfo),
    /// A trigger
    Trigger,
    /// A database enum type (PostgreSQL)
    Enum(&'static dyn SQLEnumInfo),
}

impl SQLSchemaType for PostgresSchemaType {}

//------------------------------------------------------------------------------
// Number Type
//------------------------------------------------------------------------------

/// Numeric type that can be either an integer or a floating point value
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Number {
    /// Integer value
    Integer(i64),
    /// Floating point value
    Real(f64),
}

impl Default for Number {
    fn default() -> Self {
        Self::Integer(Default::default())
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Real(value)
    }
}

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

impl core::fmt::Display for PostgresTransactionType {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let level = match self {
            PostgresTransactionType::ReadUncommitted => "READ UNCOMMITTED",
            PostgresTransactionType::ReadCommitted => "READ COMMITTED",
            PostgresTransactionType::RepeatableRead => "REPEATABLE READ",
            PostgresTransactionType::Serializable => "SERIALIZABLE",
        };
        write!(f, "{}", level)
    }
}

// Note: Generic From implementation is removed to avoid conflicts.
// The table macro will generate specific implementations using PostgresEnumVisitor.

// Re-export Join from core
pub use drizzle_core::{Join, JoinType};
