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
use drizzle_core::{SQL, ToSQL, traits::SQLTable};

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

// Define a simple Schema marker type to use with IsInSchema
#[derive(Clone, Debug)]
pub struct Schema;

/// A column in a SQLite table
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SQLiteColumn<'a, T, Tbl>
where
    T: TryInto<SQLiteValue<'a>> + Clone + std::fmt::Debug,
    Tbl: SQLTable<'a>,
{
    pub(crate) name: &'a str,
    pub(crate) sql: &'a str,
    pub(crate) _table: std::marker::PhantomData<Tbl>,
    pub(crate) _type: std::marker::PhantomData<T>,
}

impl<'a, T, Tbl> SQLiteColumn<'a, T, Tbl>
where
    T: TryInto<SQLiteValue<'a>> + Clone + std::fmt::Debug,
    Tbl: SQLTable<'a>,
{
    /// Create a new SQLite column definition.
    pub const fn new(name: &'a str, sql: &'a str) -> Self {
        Self {
            name,
            sql,
            _table: std::marker::PhantomData,
            _type: std::marker::PhantomData,
        }
    }
}

impl<'a, T, Tbl> ToSQL<'a, SQLiteValue<'a>> for SQLiteColumn<'a, T, Tbl>
where
    T: TryInto<SQLiteValue<'a>> + Clone + std::fmt::Debug,
    Tbl: SQLTable<'a>,
{
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        SQL::raw(self.name)
    }
}

// Add this implementation to track the column's value type
impl<'a, T, Tbl> SQLiteColumn<'a, T, Tbl>
where
    T: TryInto<SQLiteValue<'a>> + Clone + std::fmt::Debug,
    Tbl: SQLTable<'a>,
{
    /// Gets the type of this column, useful for type checking in expressions
    pub fn column_type(&self) -> std::marker::PhantomData<T> {
        self._type
    }
}
