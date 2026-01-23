use drizzle_core::{SQLTable, SQLTableInfo};

use crate::{PostgresColumnInfo, PostgresValue, common::PostgresSchemaType};

pub trait PostgresTable<'a>:
    SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> + PostgresTableInfo
{
}

pub trait PostgresTableInfo: SQLTableInfo {
    fn r#type(&self) -> &PostgresSchemaType;
    fn postgres_columns(&self) -> &'static [&'static dyn PostgresColumnInfo];

    /// Returns all tables this table depends on via foreign keys.
    fn postgres_dependencies(&self) -> &'static [&'static dyn PostgresTableInfo];
}

// Blanket implementation for static references
impl<T: PostgresTableInfo> PostgresTableInfo for &'static T {
    fn r#type(&self) -> &PostgresSchemaType {
        (*self).r#type()
    }

    fn postgres_columns(&self) -> &'static [&'static dyn PostgresColumnInfo] {
        (*self).postgres_columns()
    }

    fn postgres_dependencies(&self) -> &'static [&'static dyn PostgresTableInfo] {
        (*self).postgres_dependencies()
    }
}

impl std::fmt::Debug for dyn PostgresTableInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresTableInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("columns", &PostgresTableInfo::postgres_columns(self))
            .field(
                "dependencies",
                &PostgresTableInfo::postgres_dependencies(self),
            )
            .finish()
    }
}
