use drizzle_core::SQLTable;

use crate::common::SQLiteSchemaType;
use crate::values::SQLiteValue;

pub trait SQLiteTable<'a>: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>> {
    const WITHOUT_ROWID: bool;
    const STRICT: bool;
}

impl<'a, 'r, T> SQLiteTable<'a> for &'r T
where
    T: SQLiteTable<'a>,
    &'r T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
{
    const WITHOUT_ROWID: bool = T::WITHOUT_ROWID;
    const STRICT: bool = T::STRICT;
}
