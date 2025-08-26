use drizzle_core::{SQLTable, SQLTableInfo};

use crate::{SQLiteValue, common::SQLiteSchemaType, traits::SQLiteColumnInfo};

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

    fn columns(&self) -> Box<[&'static dyn SQLiteColumnInfo]>;

    /// Returns all tables this table depends on via foreign keys
    fn dependencies(&self) -> Box<[&'static dyn SQLiteTableInfo]> {
        SQLiteTableInfo::columns(self)
            .iter()
            .filter_map(|&col| SQLiteColumnInfo::foreign_key(col))
            .map(|fk_col| SQLiteColumnInfo::table(fk_col))
            .collect()
    }
}

impl std::fmt::Debug for dyn SQLiteTableInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQLiteTableInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("columns", &SQLiteTableInfo::columns(self))
            .field("without_rowid", &self.without_rowid())
            .field("strict", &self.strict())
            .field("dependencies", &SQLiteTableInfo::dependencies(self))
            .finish()
    }
}
