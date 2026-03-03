use drizzle_core::SQLTable;

use crate::common::SQLiteSchemaType;
use crate::values::SQLiteValue;

pub trait SQLiteTable<'a>: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>> {
    const WITHOUT_ROWID: bool;
    const STRICT: bool;
}
