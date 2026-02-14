//! Core traits for SQL generation.

use crate::prelude::*;
use core::any::Any;

mod column;
mod constraint;
mod foreign_key;
mod index;
mod param;
mod primary_key;
mod table;
mod to_sql;
mod tuple;
mod type_set;
mod view;

pub use column::*;
pub use constraint::*;
pub use foreign_key::*;
pub use index::*;
pub use param::*;
pub use primary_key::*;
pub use table::*;
pub use to_sql::*;
pub use type_set::*;
pub use view::*;

use crate::sql::SQL;

/// Trait for schema elements (tables, columns, etc.).
///
/// The `'a` lifetime ties any borrowed parameter values to generated SQL.
pub trait SQLSchema<'a, T, V: SQLParam + 'a>: ToSQL<'a, V> {
    const NAME: &'static str;
    const TYPE: T;
    /// Static SQL string for schema creation (e.g., CREATE TABLE ...)
    const SQL: &'static str;

    /// Generate SQL for this schema element.
    /// Default implementation wraps the static SQL string.
    fn sql(&self) -> SQL<'a, V> {
        SQL::raw(Self::SQL)
    }
}

/// Maps a schema item type to its table contribution.
///
/// - Table items map to `Cons<Table, Nil>`
/// - Non-table items (indexes/views/enums) map to `Nil`
pub trait SchemaItemTables {
    type Tables: TypeSet;
}

/// Marker trait for schema types (used for type-level discrimination).
pub trait SQLSchemaType: core::fmt::Debug + Any + Send + Sync {}

/// Trait for schema implementations that can generate CREATE statements.
pub trait SQLSchemaImpl: Any + Send + Sync {
    fn tables(&self) -> &'static [&'static dyn crate::traits::table::SQLTableInfo];
    fn create_statements(&self) -> crate::error::Result<Box<dyn Iterator<Item = String> + '_>>;
}
