//! SQLite implementation for Drizzle
//!
//! This crate provides SQLite-specific functionality for Drizzle.

//------------------------------------------------------------------------------
// Module Declarations
//------------------------------------------------------------------------------

pub mod builder;
pub mod common;
pub mod conditions;
pub mod helpers;
pub mod traits;
pub mod values;

//------------------------------------------------------------------------------
// Prelude
//------------------------------------------------------------------------------

/// A prelude module that re-exports commonly used types and traits
pub mod prelude {
    pub use crate::SQLiteTransactionType;
    pub use crate::traits::SQLiteColumn;
    pub use crate::values::SQLiteValue;

    // Re-export rusqlite trait implementations when the feature is enabled
    #[cfg(feature = "rusqlite")]
    pub use ::rusqlite::types::ToSql;
}

pub use self::values::{InsertValue, OwnedSQLiteValue, SQLiteValue};

/// SQLite transaction types
#[derive(Default, Debug, Clone, Copy)]
pub enum SQLiteTransactionType {
    #[default]
    /// A deferred transaction is the default - it does not acquire locks until needed
    Deferred,
    /// An immediate transaction acquires a RESERVED lock immediately
    Immediate,
    /// An exclusive transaction acquires an EXCLUSIVE lock immediately
    Exclusive,
}

#[cfg(feature = "rusqlite")]
impl From<SQLiteTransactionType> for ::rusqlite::TransactionBehavior {
    fn from(tx_type: SQLiteTransactionType) -> Self {
        match tx_type {
            SQLiteTransactionType::Deferred => ::rusqlite::TransactionBehavior::Deferred,
            SQLiteTransactionType::Immediate => ::rusqlite::TransactionBehavior::Immediate,
            SQLiteTransactionType::Exclusive => ::rusqlite::TransactionBehavior::Exclusive,
        }
    }
}

#[cfg(feature = "rusqlite")]
impl From<::rusqlite::TransactionBehavior> for SQLiteTransactionType {
    fn from(behavior: ::rusqlite::TransactionBehavior) -> Self {
        match behavior {
            ::rusqlite::TransactionBehavior::Deferred => SQLiteTransactionType::Deferred,
            ::rusqlite::TransactionBehavior::Immediate => SQLiteTransactionType::Immediate,
            ::rusqlite::TransactionBehavior::Exclusive => SQLiteTransactionType::Exclusive,
            _ => SQLiteTransactionType::Deferred, // Default for any future variants
        }
    }
}

// Convert to libsql::TransactionBehavior
#[cfg(feature = "libsql")]
impl From<SQLiteTransactionType> for libsql::TransactionBehavior {
    fn from(tx_type: SQLiteTransactionType) -> Self {
        match tx_type {
            SQLiteTransactionType::Deferred => libsql::TransactionBehavior::Deferred,
            SQLiteTransactionType::Immediate => libsql::TransactionBehavior::Immediate,
            SQLiteTransactionType::Exclusive => libsql::TransactionBehavior::Exclusive,
        }
    }
}

// Convert from libsql::TransactionBehavior
#[cfg(feature = "libsql")]
impl From<libsql::TransactionBehavior> for SQLiteTransactionType {
    fn from(behavior: libsql::TransactionBehavior) -> Self {
        match behavior {
            libsql::TransactionBehavior::Deferred => SQLiteTransactionType::Deferred,
            libsql::TransactionBehavior::Immediate => SQLiteTransactionType::Immediate,
            libsql::TransactionBehavior::Exclusive => SQLiteTransactionType::Exclusive,
            libsql::TransactionBehavior::ReadOnly => SQLiteTransactionType::Deferred, // Map ReadOnly to Deferred as closest equivalent
        }
    }
}

/// Creates an array of SQL parameters for binding values to placeholders.
///
/// # Syntax
/// - `{ name: value }` - Colon parameter (creates :name placeholder)
///
/// # Examples
///
/// ```
/// use drizzle_rs::prelude::*;
///
/// let params = params![{ name: "alice" }, { active: true }];
/// ```
#[macro_export]
macro_rules! params {
    // Multiple parameters - creates a fixed-size array of Param structs
    [$($param:tt),+ $(,)?] => {
        [
            $(
                $crate::params_internal!($param)
            ),+
        ]
    };
}

/// Internal helper macro for params! - converts individual items to Param structs
#[macro_export]
macro_rules! params_internal {
    // Colon-style named parameter
    ({ $key:ident: $value:expr }) => {
        ::drizzle_rs::core::ParamBind::new(stringify!($key), $crate::SQLiteValue::from($value))
    };
    // Positional parameter
    ($value:expr) => {
        ::drizzle_rs::core::ParamBind::new($crate::SQLiteValue::from($value))
    };
}
