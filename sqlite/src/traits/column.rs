use drizzle_core::SQLColumn;

use crate::values::SQLiteValue;

pub trait SQLiteColumn<'a>: SQLColumn<'a, SQLiteValue<'a>> {
    const AUTOINCREMENT: bool = false;
}

impl<'a, 'r, T> SQLiteColumn<'a> for &'r T
where
    T: SQLiteColumn<'a>,
    &'r T: SQLColumn<'a, SQLiteValue<'a>>,
{
    const AUTOINCREMENT: bool = T::AUTOINCREMENT;
}
