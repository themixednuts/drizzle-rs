//! Common table expression support for MySQL.

use crate::values::MySQLValue;

drizzle_core::impl_cte_types!(value_type: MySQLValue<'a>);
