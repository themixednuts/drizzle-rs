//! SQLite implementation for Drizzle
//!
//! This crate provides SQLite-specific types, query builders, and utilities.

// Public modules
pub mod attrs;
pub mod builder;
pub mod common;
pub mod conditions;
pub mod connection;
pub mod expression;
pub mod helpers;
pub mod pragma;
pub mod traits;
pub mod values;

// Re-export key types at crate root
pub use builder::QueryBuilder;
pub use common::SQLiteSchemaType;
pub use connection::{ConnectionRef, SQLiteTransactionType};
pub use traits::{
    DrizzleRow, FromSQLiteValue, SQLiteColumn, SQLiteColumnInfo, SQLiteTable, SQLiteTableInfo,
};
pub use values::{OwnedSQLiteValue, SQLiteInsertValue, SQLiteValue, ValueWrapper};

// Re-export ParamBind for use in params! macro
pub use drizzle_core::ParamBind;

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
        $crate::ParamBind::named(stringify!($key), $crate::SQLiteValue::from($value))
    };
    // Positional parameter
    ($value:expr) => {
        $crate::ParamBind::positional($crate::SQLiteValue::from($value))
    };
}

// Re-export type alias from traits
pub use traits::SQLiteSQL;
