use drizzle_core::SQLColumn;

use crate::values::SQLiteValue;

pub trait SQLiteColumn<'a>: SQLColumn<'a, SQLiteValue<'a>> {
    const AUTOINCREMENT: bool = false;
}
