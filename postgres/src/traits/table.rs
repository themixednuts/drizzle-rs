use drizzle_core::SQLTable;

use crate::common::PostgresSchemaType;
use crate::values::PostgresValue;

pub trait PostgresTable<'a>: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> {}

impl<'a, 'r, T> PostgresTable<'a> for &'r T
where
    T: PostgresTable<'a>,
    &'r T: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>>,
{
}
