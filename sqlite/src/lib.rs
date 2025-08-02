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
    pub use crate::common::Number;
    pub use crate::values::SQLiteValue;

    // Re-export rusqlite trait implementations when the feature is enabled
    #[cfg(feature = "rusqlite")]
    pub use ::rusqlite::types::ToSql;
}

// Re-export types from common and values
pub use self::values::SQLiteValue;

// Import core types
use drizzle_core::{SQL, SQLColumn, SQLSchema, ToSQL, traits::SQLTable};

/// SQLite transaction types
#[derive(Debug, Clone, Copy)]
pub enum SQLiteTransactionType {
    /// A deferred transaction is the default - it does not acquire locks until needed
    Deferred,
    /// An immediate transaction acquires a RESERVED lock immediately
    Immediate,
    /// An exclusive transaction acquires an EXCLUSIVE lock immediately
    Exclusive,
}

pub trait SQLiteColumn<'a>: SQLColumn<'a, SQLiteValue<'a>> {
    const AUTOINCREMENT: bool = false;
}
