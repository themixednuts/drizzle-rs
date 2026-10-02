use drizzle_core::SQLTable;

use crate::common::SQLiteSchemaType;
use crate::values::SQLiteValue;

/// A `SQLite` table. Implemented by `#[SQLiteTable]`.
pub trait SQLiteTable<'a>: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>> {
    /// `true` if the table was declared `WITHOUT ROWID`.
    const WITHOUT_ROWID: bool;
    /// `true` if the table was declared `STRICT`.
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
