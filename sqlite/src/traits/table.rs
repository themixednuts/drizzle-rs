use drizzle_core::{SQLTable, SQLTableInfo};

use crate::common::SQLiteSchemaType;
use crate::traits::SQLiteColumnInfo;
use crate::values::SQLiteValue;

pub trait SQLiteTable<'a>:
    SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>> + SQLiteTableInfo
{
    const WITHOUT_ROWID: bool;
    const STRICT: bool;
}

pub trait SQLiteTableInfo: SQLTableInfo {
    fn r#type(&self) -> &SQLiteSchemaType;
    fn without_rowid(&self) -> bool;
    fn strict(&self) -> bool;

    fn sqlite_columns(&self) -> &'static [&'static dyn SQLiteColumnInfo];

    /// Returns all tables this table depends on via foreign keys
    fn sqlite_dependencies(&self) -> &'static [&'static dyn SQLiteTableInfo];
}

// Blanket implementation for static references
impl<T: SQLiteTableInfo> SQLiteTableInfo for &'static T {
    fn r#type(&self) -> &SQLiteSchemaType {
        (*self).r#type()
    }

    fn without_rowid(&self) -> bool {
        (*self).without_rowid()
    }

    fn strict(&self) -> bool {
        (*self).strict()
    }

    fn sqlite_columns(&self) -> &'static [&'static dyn SQLiteColumnInfo] {
        (*self).sqlite_columns()
    }

    fn sqlite_dependencies(&self) -> &'static [&'static dyn SQLiteTableInfo] {
        (*self).sqlite_dependencies()
    }
}

impl std::fmt::Debug for dyn SQLiteTableInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQLiteTableInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("columns", &SQLiteTableInfo::sqlite_columns(self))
            .field("without_rowid", &self.without_rowid())
            .field("strict", &self.strict())
            .field("dependencies", &SQLiteTableInfo::sqlite_dependencies(self))
            .finish()
    }
}
