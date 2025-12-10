//! SQLite implementation for Drizzle
//!
//! This crate provides SQLite-specific functionality for Drizzle.

//------------------------------------------------------------------------------
// Module Declarations
//------------------------------------------------------------------------------

pub mod attrs;
pub mod builder;
pub mod columns;
pub mod common;
pub mod conditions;
pub mod expression;
pub mod helpers;
pub mod pragma;
pub mod traits;
pub mod values;

//------------------------------------------------------------------------------
// Prelude
//------------------------------------------------------------------------------

/// A prelude module that re-exports commonly used types and traits
pub mod prelude {
    pub use crate::SQLiteTransactionType;
    pub use crate::pragma::Pragma;
    pub use crate::traits::SQLiteColumn;
    pub use crate::traits::{DrizzleRow, FromSQLiteValue};
    pub use crate::values::SQLiteValue;

    // Column attribute markers for IDE documentation
    // These are used in #[column(...)] attributes
    pub use crate::attrs::{
        AUTOINCREMENT, ColumnMarker, DEFAULT, DEFAULT_FN, ENUM, JSON, JSONB, ON_DELETE, ON_UPDATE,
        PRIMARY, PRIMARY_KEY, REFERENCES, UNIQUE,
    };

    // Referential action markers for on_delete/on_update
    pub use crate::attrs::{
        CASCADE, NO_ACTION, RESTRICT, ReferentialAction, SET_DEFAULT, SET_NULL,
    };

    // Table attribute markers for IDE documentation
    // These are used in #[SQLiteTable(...)] attributes
    pub use crate::attrs::{STRICT, TableMarker, WITHOUT_ROWID};

    // Shared markers (used by both column and table attributes)
    pub use crate::attrs::{NAME, NameMarker};

    // Re-export rusqlite trait implementations when the feature is enabled
    #[cfg(feature = "rusqlite")]
    pub use ::rusqlite::types::ToSql;
}

pub use self::values::{OwnedSQLiteValue, SQLiteInsertValue, SQLiteValue};

// Re-export ParamBind for use in macros
pub use drizzle_core::ParamBind;
use drizzle_core::SQL;

#[cfg(not(any(feature = "libsql", feature = "rusqlite", feature = "turso")))]
use std::marker::PhantomData;

/// Reference to different SQLite driver connection types
#[derive(Debug)]
pub enum ConnectionRef<'a> {
    #[cfg(feature = "libsql")]
    LibSql(&'a libsql::Connection),
    #[cfg(feature = "rusqlite")]
    Rusqlite(&'a rusqlite::Connection),
    #[cfg(feature = "turso")]
    Turso(&'a turso::Connection),
    #[cfg(not(any(feature = "libsql", feature = "rusqlite", feature = "turso")))]
    _Phantom(PhantomData<&'a ()>),
}

// Implement Into trait for each connection type
#[cfg(feature = "libsql")]
impl<'a> From<&'a libsql::Connection> for ConnectionRef<'a> {
    fn from(conn: &'a libsql::Connection) -> Self {
        ConnectionRef::LibSql(conn)
    }
}

#[cfg(feature = "rusqlite")]
impl<'a> From<&'a rusqlite::Connection> for ConnectionRef<'a> {
    fn from(conn: &'a rusqlite::Connection) -> Self {
        ConnectionRef::Rusqlite(conn)
    }
}

#[cfg(feature = "turso")]
impl<'a> From<&'a turso::Connection> for ConnectionRef<'a> {
    fn from(conn: &'a turso::Connection) -> Self {
        ConnectionRef::Turso(conn)
    }
}

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

// Convert to turso::TransactionBehavior
#[cfg(feature = "turso")]
impl From<SQLiteTransactionType> for turso::transaction::TransactionBehavior {
    fn from(tx_type: SQLiteTransactionType) -> Self {
        match tx_type {
            SQLiteTransactionType::Deferred => turso::transaction::TransactionBehavior::Deferred,
            SQLiteTransactionType::Immediate => turso::transaction::TransactionBehavior::Immediate,
            SQLiteTransactionType::Exclusive => turso::transaction::TransactionBehavior::Exclusive,
        }
    }
}

// Convert from turso::TransactionBehavior
#[cfg(feature = "turso")]
impl From<turso::transaction::TransactionBehavior> for SQLiteTransactionType {
    fn from(behavior: turso::transaction::TransactionBehavior) -> Self {
        match behavior {
            turso::transaction::TransactionBehavior::Deferred => SQLiteTransactionType::Deferred,
            turso::transaction::TransactionBehavior::Immediate => SQLiteTransactionType::Immediate,
            turso::transaction::TransactionBehavior::Exclusive => SQLiteTransactionType::Exclusive,
            _ => SQLiteTransactionType::Deferred,
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
/// use drizzle_sqlite::params;
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
        $crate::ParamBind::new(stringify!($key), $crate::SQLiteValue::from($value))
    };
    // Positional parameter
    ($value:expr) => {
        $crate::ParamBind::new("", $crate::SQLiteValue::from($value))
    };
}

pub type SQLiteSQL<'a> = SQL<'a, SQLiteValue<'a>>;
