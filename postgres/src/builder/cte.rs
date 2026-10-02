//! Common table expression (`WITH`) types for `PostgreSQL` queries.
//!
//! Build a CTE with a SELECT builder's `.into_cte::<Tag>()`, then pass it to
//! [`QueryBuilder::with`](crate::builder::QueryBuilder::with).

use crate::values::PostgresValue;

/// Something that can be listed in a `PostgreSQL` `WITH` clause.
///
/// Implemented for every core [`CTEDefinition`](drizzle_core::cte::CTEDefinition)
/// over [`PostgresValue`].
pub trait CTEDefinition<'a>: drizzle_core::cte::CTEDefinition<'a, PostgresValue<'a>> {}

impl<'a, T> CTEDefinition<'a> for T where T: drizzle_core::cte::CTEDefinition<'a, PostgresValue<'a>> {}

/// A named `PostgreSQL` CTE that can be read like a table.
///
/// See [`drizzle_core::cte::CTEView`].
pub type CTEView<'a, Table, Query> =
    drizzle_core::cte::CTEView<'a, PostgresValue<'a>, Table, Query>;
