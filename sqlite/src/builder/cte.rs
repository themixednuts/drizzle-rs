//! Common table expression support for SQLite.

use crate::values::SQLiteValue;

/// A value that can provide a SQLite CTE definition.
pub trait CTEDefinition<'a>: drizzle_core::cte::CTEDefinition<'a, SQLiteValue<'a>> {}

impl<'a, T> CTEDefinition<'a> for T where T: drizzle_core::cte::CTEDefinition<'a, SQLiteValue<'a>> {}

/// A typed SQLite CTE view.
pub type CTEView<'a, Table, Query> = drizzle_core::cte::CTEView<'a, SQLiteValue<'a>, Table, Query>;
