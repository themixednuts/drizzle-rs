use drizzle_core::SQLTable;

use crate::common::PostgresSchemaType;
use crate::values::PostgresValue;

/// A `PostgreSQL` table. Implemented by `#[PostgresTable]`.
pub trait PostgresTable<'a>: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> {}

impl<'a, T> PostgresTable<'a> for &T
where
    T: PostgresTable<'a>,
    for<'r> &'r T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
}
