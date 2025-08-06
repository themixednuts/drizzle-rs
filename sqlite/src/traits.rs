use std::any::Any;

use crate::SQLiteValue;
use drizzle_core::SQLColumn;

pub trait SQLiteColumn<'a>: SQLColumn<'a, SQLiteValue<'a>> {
    const AUTOINCREMENT: bool = false;
}

pub trait SQLiteColumnInfo: Any {
    fn is_autoincrement(&self) -> bool;
}
