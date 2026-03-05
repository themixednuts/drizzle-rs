use drizzle_core::SQLTable;

use crate::common::SQLiteSchemaType;
use crate::values::SQLiteValue;

pub trait SQLiteTable<'a>: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>> {
    const WITHOUT_ROWID: bool;
    const STRICT: bool;
}

impl<'a, T> SQLiteTable<'a> for &T
where
    T: SQLiteTable<'a>,
    for<'x> &'x T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
{
    const WITHOUT_ROWID: bool = T::WITHOUT_ROWID;
    const STRICT: bool = T::STRICT;
}
