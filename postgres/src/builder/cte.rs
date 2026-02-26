//! Common Table Expression (CTE / `WITH`) support for PostgreSQL.

use crate::values::PostgresValue;

drizzle_core::impl_cte_types!(value_type: PostgresValue<'a>);
