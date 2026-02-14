use core::any::Any;

use crate::expr::Expr;
use crate::{SQLParam, SQLSchema, SQLSchemaType, SQLTable, SQLTableInfo};

pub trait SQLColumnInfo: Any + Send + Sync {
    fn is_not_null(&self) -> bool;
    fn is_primary_key(&self) -> bool;
    fn is_unique(&self) -> bool;
    fn name(&self) -> &str;
    fn r#type(&self) -> &str;
    fn has_default(&self) -> bool;

    fn table(&self) -> &dyn SQLTableInfo;
    /// Returns the foreign key reference if this column has one.
    fn foreign_key(&self) -> Option<&'static dyn SQLColumnInfo> {
        None
    }

    /// Returns the foreign key table if this column references one.
    fn foreign_key_table(&self) -> Option<&'static dyn SQLTableInfo> {
        self.foreign_key().map(|fk| fk.table())
    }

    fn as_column(&self) -> &dyn SQLColumnInfo
    where
        Self: Sized,
    {
        self
    }
}

/// Column trait tying expression lifetimes to parameter values via `'a`.
pub trait SQLColumn<'a, Value: SQLParam + 'a>:
    SQLColumnInfo + Default + SQLSchema<'a, &'a str, Value> + Expr<'a, Value>
{
    type Table: SQLTable<'a, Self::TableType, Value>;
    type TableType: SQLSchemaType;
    type ForeignKeys;
    type Type: TryInto<Value>;

    const PRIMARY_KEY: bool = false;
    const NOT_NULL: bool = false;
    const UNIQUE: bool = false;
    const DEFAULT: Option<Self::Type> = None;

    fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
        None::<fn() -> Self::Type>
    }
}

// Blanket implementation for static references
impl<T: SQLColumnInfo> SQLColumnInfo for &'static T {
    fn is_not_null(&self) -> bool {
        (*self).is_not_null()
    }

    fn is_primary_key(&self) -> bool {
        (*self).is_primary_key()
    }

    fn is_unique(&self) -> bool {
        (*self).is_unique()
    }

    fn name(&self) -> &str {
        (*self).name()
    }

    fn r#type(&self) -> &str {
        (*self).r#type()
    }

    fn has_default(&self) -> bool {
        (*self).has_default()
    }

    fn table(&self) -> &dyn SQLTableInfo {
        (*self).table()
    }

    fn foreign_key(&self) -> Option<&'static dyn SQLColumnInfo> {
        (*self).foreign_key()
    }
}

impl core::fmt::Debug for dyn SQLColumnInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SQLColumnInfo")
            .field("name", &self.name())
            .field("type", &self.r#type())
            .field("not_null", &self.is_not_null())
            .field("primary_key", &self.is_primary_key())
            .field("unique", &self.is_unique())
            .field("table", &self.table().name())
            .field("has_default", &self.has_default())
            .finish()
    }
}
