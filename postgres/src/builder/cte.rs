//! Common table expression support for PostgreSQL.

use crate::values::PostgresValue;

/// A value that can provide a PostgreSQL CTE definition.
pub trait CTEDefinition<'a>: drizzle_core::cte::CTEDefinition<'a, PostgresValue<'a>> {}

impl<'a, T> CTEDefinition<'a> for T where T: drizzle_core::cte::CTEDefinition<'a, PostgresValue<'a>> {}

/// A typed PostgreSQL CTE view.
pub type CTEView<'a, Table, Query> =
    drizzle_core::cte::CTEView<'a, PostgresValue<'a>, Table, Query>;
