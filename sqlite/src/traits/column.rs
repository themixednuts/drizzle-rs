use drizzle_core::{SQLColumn, SQLColumnInfo};

use crate::{SQLiteValue, traits::SQLiteTableInfo};

pub trait SQLiteColumn<'a>: SQLColumn<'a, SQLiteValue<'a>> {
    const AUTOINCREMENT: bool = false;
}

pub trait SQLiteColumnInfo: SQLColumnInfo {
    fn is_autoincrement(&self) -> bool;
    fn table(&self) -> &dyn SQLiteTableInfo;

    /// Returns the foreign key reference if this column has one.
    fn foreign_key(&self) -> Option<&'static dyn SQLiteColumnInfo> {
        None
    }

    /// Erased access to the SQLite column info.
    fn as_sqlite_column(&self) -> &dyn SQLiteColumnInfo
    where
        Self: Sized,
    {
        self
    }

    /// Core-erased foreign key reference for call sites that only need generic info.
    fn foreign_key_core(&self) -> Option<&'static dyn SQLColumnInfo> {
        <Self as SQLiteColumnInfo>::foreign_key(self).map(|fk| fk as &dyn SQLColumnInfo)
    }
}

impl std::fmt::Debug for dyn SQLiteColumnInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQLiteColumnInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("not_null", &self.is_not_null())
            .field("primary_key", &self.is_primary_key())
            .field("unique", &self.is_unique())
            .field("table", &SQLiteColumnInfo::table(self))
            .field("has_default", &self.has_default())
            .field("is_autoincrement", &self.is_autoincrement())
            .finish()
    }
}
