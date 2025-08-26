use std::any::Any;

use crate::{SQLParam, SQLSchema, SQLSchemaType, SQLTable, SQLTableInfo};

pub trait SQLColumnInfo: Any + Send + Sync {
    fn is_not_null(&self) -> bool;
    fn is_primary_key(&self) -> bool;
    fn is_unique(&self) -> bool;
    fn name(&self) -> &str;
    fn r#type(&self) -> &str;
    fn has_default(&self) -> bool;

    fn table(&self) -> &dyn SQLTableInfo;
    /// Returns the foreign key reference if this column has one
    fn foreign_key(&self) -> Option<&'static dyn SQLColumnInfo> {
        None
    }
}

pub trait AsColumnInfo: SQLColumnInfo {
    fn as_column(&self) -> &dyn SQLColumnInfo;
}

impl<T: SQLColumnInfo> AsColumnInfo for T {
    fn as_column(&self) -> &dyn SQLColumnInfo {
        self
    }
}

pub trait SQLColumn<'a, Value: SQLParam + 'a>:
    SQLColumnInfo + Default + SQLSchema<'a, &'a str, Value>
{
    type Table: SQLTable<'a, Self::TableType, Value>;
    type TableType: SQLSchemaType;
    type Type: TryInto<Value>;

    const PRIMARY_KEY: bool = false;
    const NOT_NULL: bool = false;
    const UNIQUE: bool = false;
    const DEFAULT: Option<Self::Type> = None;

    fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
        None::<fn() -> Self::Type>
    }
}

impl std::fmt::Debug for dyn SQLColumnInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQLColumnInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("not_null", &self.is_not_null())
            .field("primary_key", &self.is_primary_key())
            .field("unique", &self.is_unique())
            .field("table", &self.table())
            .field("has_default", &self.has_default())
            .finish()
    }
}
