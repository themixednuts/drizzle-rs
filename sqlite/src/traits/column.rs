use drizzle_core::SQLColumn;

use crate::values::SQLiteValue;

pub trait SQLiteColumn<'a>: SQLColumn<'a, SQLiteValue<'a>> {
    const AUTOINCREMENT: bool = false;
}

impl<'a, T> SQLiteColumn<'a> for &T
where
    T: SQLiteColumn<'a>,
    for<'r> &'r T: SQLColumn<'a, SQLiteValue<'a>>,
{
    const AUTOINCREMENT: bool = T::AUTOINCREMENT;
}
