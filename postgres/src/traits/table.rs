use drizzle_core::SQLTable;

use crate::common::PostgresSchemaType;
use crate::values::PostgresValue;

pub trait PostgresTable<'a>: SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> {}
