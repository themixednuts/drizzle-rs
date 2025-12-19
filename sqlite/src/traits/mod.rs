//! SQLite-specific traits for tables, columns, and values

mod column;
mod table;
mod value;

pub use column::*;
pub use table::*;
pub use value::*;

use drizzle_core::traits::ToSQL;

use crate::values::SQLiteValue;

/// Type alias for SQL fragments with SQLite values.
pub type SQLiteSQL<'a> = drizzle_core::SQL<'a, SQLiteValue<'a>>;

/// Trait alias for types that can be converted to SQLite SQL.
///
/// This is a convenience trait that combines `ToSQL` with `SQLiteValue`.
pub trait ToSQLiteSQL<'a>: ToSQL<'a, SQLiteValue<'a>> {}

impl<'a, T: ToSQL<'a, SQLiteValue<'a>>> ToSQLiteSQL<'a> for T {}
