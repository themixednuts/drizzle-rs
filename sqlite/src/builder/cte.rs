//! Common table expression support for SQLite.

use crate::values::SQLiteValue;

/// A common table expression that can be passed to
/// [`QueryBuilder::with`](super::QueryBuilder::with).
///
/// Implemented for every [`CTEView`], which
/// [`into_cte`](super::QueryBuilder::into_cte) returns.
pub trait CTEDefinition<'a>: drizzle_core::cte::CTEDefinition<'a, SQLiteValue<'a>> {}

impl<'a, T> CTEDefinition<'a> for T where T: drizzle_core::cte::CTEDefinition<'a, SQLiteValue<'a>> {}

/// A named SELECT used as a common table expression.
///
/// Created by [`into_cte`](super::QueryBuilder::into_cte). It derefs to an
/// aliased copy of the source table, so `cte.column` gives typed column
/// references that render as `"cte_name"."column"`.
pub type CTEView<'a, Table, Query> = drizzle_core::cte::CTEView<'a, SQLiteValue<'a>, Table, Query>;
