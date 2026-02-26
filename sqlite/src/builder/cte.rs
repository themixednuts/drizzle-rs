//! Common Table Expression (CTE / `WITH`) support for SQLite.

use crate::values::SQLiteValue;

drizzle_core::impl_cte_types!(value_type: SQLiteValue<'a>);
