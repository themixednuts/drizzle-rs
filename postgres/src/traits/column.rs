use drizzle_core::{SQLColumn, SQLColumnInfo};
use std::any::Any;

use crate::{PostgresValue, traits::PostgresTableInfo};
pub trait PostgresColumn<'a>: SQLColumn<'a, PostgresValue<'a>> {
    const SERIAL: bool = false;
    const BIGSERIAL: bool = false;
    const GENERATED_IDENTITY: bool = false;
}
pub trait PostgresColumnInfo: SQLColumnInfo + Any {
    fn table(&self) -> &dyn PostgresTableInfo;

    fn is_serial(&self) -> bool;
    fn is_bigserial(&self) -> bool;
    fn is_generated_identity(&self) -> bool;
    fn postgres_type(&self) -> &'static str;

    /// Returns the foreign key reference if this column has one
    fn foreign_key(&self) -> Option<&'static dyn PostgresColumnInfo> {
        None
    }

    /// Erased access to the Postgres column info.
    fn as_postgres_column(&self) -> &dyn PostgresColumnInfo
    where
        Self: Sized,
    {
        self
    }

    /// Core-erased foreign key reference for call sites that only need generic info.
    fn foreign_key_core(&self) -> Option<&'static dyn SQLColumnInfo> {
        <Self as PostgresColumnInfo>::foreign_key(self).map(|fk| fk as &dyn SQLColumnInfo)
    }
}

impl std::fmt::Debug for dyn PostgresColumnInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostgresColumnInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("not_null", &self.is_not_null())
            .field("primary_key", &self.is_primary_key())
            .field("unique", &self.is_unique())
            .field("table", &PostgresColumnInfo::table(self))
            .field("has_default", &self.has_default())
            .field("is_serial", &self.is_serial())
            .field("is_bigserial", &self.is_bigserial())
            .field("is_generated_identity", &self.is_generated_identity())
            .finish()
    }
}
