mod column;
mod table;
mod value;

pub use column::*;
use drizzle_core::{SQL, ToSQL};
pub use table::*;
pub use value::*;

use crate::SQLiteValue;

pub type SQLiteSQL<'a> = SQL<'a, SQLiteValue<'a>>;

pub trait ToSQLiteSQL<'a>: ToSQL<'a, SQLiteValue<'a>> {
    // fn to_sqlite_sql(&self) -> SQLiteSQL<'a> {
    //     self.to_sql()
    // }
}

impl<'a, T: ToSQL<'a, SQLiteValue<'a>>> ToSQLiteSQL<'a> for T {}
