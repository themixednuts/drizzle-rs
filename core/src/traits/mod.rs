//! Core traits for SQL generation.

use crate::prelude::*;
use core::any::Any;

#[macro_use]
mod tuple;

mod column;
mod constraint;
mod foreign_key;
mod index;
mod param;
mod primary_key;
mod table;
mod to_sql;
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
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a SQL schema element",
    label = "this type was not derived with a drizzle schema macro"
)]
pub trait SQLSchema<'a, T, V: SQLParam + 'a>: ToSQL<'a, V> {
    const NAME: &'static str;
    const TYPE: T;
    /// Static SQL string for schema creation (e.g., CREATE TABLE ...)
    const SQL: &'static str;

    /// Generate DDL SQL for this schema element (e.g., CREATE TABLE ...).
    /// Default implementation wraps the static SQL string.
    fn ddl(&self) -> SQL<'a, V> {
        SQL::raw(Self::SQL)
    }
}

/// Maps a schema item type to its table contribution.
///
/// - Table items map to `Cons<Table, Nil>`
/// - Non-table items (indexes/views/enums) map to `Nil`
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a recognized schema item",
    label = "schema items must be tables, views, indexes, or enums derived with drizzle macros"
)]
pub trait SchemaItemTables {
    type Tables: TypeSet;
}

/// Marker trait for schema types (used for type-level discrimination).
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a SQL schema type marker",
    label = "expected a dialect marker like SQLiteSchemaType or PostgresSchemaType"
)]
pub trait SQLSchemaType: core::fmt::Debug + Any + Send + Sync {}

/// Trait for schema implementations that can generate CREATE statements.
pub trait SQLSchemaImpl: Any + Send + Sync {
    fn tables(&self) -> &'static [&'static dyn crate::traits::table::SQLTableInfo];
    fn create_statements(&self) -> crate::error::Result<impl Iterator<Item = String>>;
}
