use drizzle_core::{SQLTable, SQLTableInfo};

use crate::{PostgresColumnInfo, PostgresValue, common::PostgresSchemaType};

pub trait PostgresTable<'a>:
    SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> + PostgresTableInfo
{
}

pub trait PostgresTableInfo: SQLTableInfo {
    fn r#type(&self) -> &PostgresSchemaType;
    fn columns(&self) -> Box<[&'static dyn PostgresColumnInfo]>;

    /// Returns all tables this table depends on via foreign keys
    fn dependencies(&self) -> Box<[&'static dyn PostgresTableInfo]> {
        PostgresTableInfo::columns(self)
            .iter()
            .filter_map(|&col| PostgresColumnInfo::foreign_key(col))
            .map(|fk_col| PostgresColumnInfo::table(fk_col))
            .collect()
    }
}

impl std::fmt::Debug for dyn PostgresTableInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresTableInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("columns", &PostgresTableInfo::columns(self))
            .field("dependencies", &PostgresTableInfo::dependencies(self))
            .finish()
    }
}
