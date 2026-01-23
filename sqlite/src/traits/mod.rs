//! SQLite-specific traits for tables, columns, and values

mod column;
mod table;
mod value;

pub use column::*;
pub use table::*;
pub use value::*;

use crate::values::SQLiteValue;

/// Type alias for SQL fragments with SQLite values.
pub type SQLiteSQL<'a> = drizzle_core::SQL<'a, SQLiteValue<'a>>;

// Use `drizzle_core::ToSQL<'a, SQLiteValue<'a>>` directly for bounds.
