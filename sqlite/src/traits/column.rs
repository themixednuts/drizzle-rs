use drizzle_core::SQLColumn;

use crate::values::SQLiteValue;

/// A column of a `SQLite` table. Implemented by `#[SQLiteTable]`.
pub trait SQLiteColumn<'a>: SQLColumn<'a, SQLiteValue<'a>> {
    /// `true` if the column was declared `AUTOINCREMENT`.
    const AUTOINCREMENT: bool = false;
}

impl<'a, T> SQLiteColumn<'a> for &T
where
    T: SQLiteColumn<'a>,
    for<'r> &'r T: SQLColumn<'a, SQLiteValue<'a>>,
{
    const AUTOINCREMENT: bool = T::AUTOINCREMENT;
}
