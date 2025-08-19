use std::any::Any;

use crate::SQLiteValue;
use drizzle_core::{SQLColumn, SQLColumnInfo};

pub trait SQLiteColumn<'a>: SQLColumn<'a, SQLiteValue<'a>> {
    const AUTOINCREMENT: bool = false;
}

pub trait SQLiteColumnInfo: SQLColumnInfo + Any {
    fn is_autoincrement(&self) -> bool;
}

impl std::fmt::Debug for &dyn SQLiteColumnInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQLiteColumnInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("not_null", &self.is_not_null())
            .field("primary_key", &self.is_primary_key())
            .field("unique", &self.is_unique())
            .field("table", &self.table())
            .field("has_default", &self.has_default())
            .field("is_autoincrement", &self.is_autoincrement())
            .finish()
    }
}
