use core::any::Any;
use drizzle_core::{SQLColumn, SQLColumnInfo};

use crate::traits::PostgresTableInfo;
use crate::values::PostgresValue;
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

// Blanket implementation for static references
impl<T: PostgresColumnInfo> PostgresColumnInfo for &'static T {
    fn table(&self) -> &dyn PostgresTableInfo {
        <T as PostgresColumnInfo>::table(*self)
    }

    fn is_serial(&self) -> bool {
        <T as PostgresColumnInfo>::is_serial(*self)
    }

    fn is_bigserial(&self) -> bool {
        <T as PostgresColumnInfo>::is_bigserial(*self)
    }

    fn is_generated_identity(&self) -> bool {
        <T as PostgresColumnInfo>::is_generated_identity(*self)
    }

    fn postgres_type(&self) -> &'static str {
        <T as PostgresColumnInfo>::postgres_type(*self)
    }

    fn foreign_key(&self) -> Option<&'static dyn PostgresColumnInfo> {
        <T as PostgresColumnInfo>::foreign_key(*self)
    }
}

impl core::fmt::Debug for dyn PostgresColumnInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
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
