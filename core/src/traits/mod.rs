//! Core traits for SQL generation.

use crate::prelude::*;
use core::any::Any;

mod column;
mod index;
mod param;
mod table;
mod to_sql;
mod tuple;
mod view;

pub use column::*;
pub use index::*;
pub use param::*;
pub use table::*;
pub use to_sql::*;
pub use view::*;

use crate::sql::SQL;

/// Trait for schema elements (tables, columns, etc.).
///
/// The `'a` lifetime ties any borrowed parameter values to generated SQL.
pub trait SQLSchema<'a, T, V: SQLParam + 'a>: ToSQL<'a, V> {
    const NAME: &'a str;
    const TYPE: T;
    /// Static SQL string for schema creation (e.g., CREATE TABLE ...)
    const SQL: &'static str;

    /// Generate SQL for this schema element.
    /// Default implementation wraps the static SQL string.
    fn sql(&self) -> SQL<'a, V> {
        SQL::raw(Self::SQL)
    }
}

/// Marker trait for schema types (used for type-level discrimination).
pub trait SQLSchemaType: core::fmt::Debug + Any + Send + Sync {}

/// Trait for schema implementations that can generate CREATE statements.
pub trait SQLSchemaImpl: Any + Send + Sync {
    fn create_statements(&self) -> Vec<String>;
}
