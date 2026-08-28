//! Common table expression support for MySQL.

use crate::values::MySQLValue;

/// A value that can provide a MySQL CTE definition.
pub trait CTEDefinition<'a>: drizzle_core::cte::CTEDefinition<'a, MySQLValue<'a>> {}

impl<'a, T> CTEDefinition<'a> for T where T: drizzle_core::cte::CTEDefinition<'a, MySQLValue<'a>> {}

/// A typed MySQL CTE view.
pub type CTEView<'a, Table, Query> = drizzle_core::cte::CTEView<'a, MySQLValue<'a>, Table, Query>;
