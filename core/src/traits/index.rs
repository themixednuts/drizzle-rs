use core::any::Any;

use crate::{SQLParam, SQLSchemaType, SQLTable, SQLTableInfo, ToSQL};

pub trait SQLIndexInfo: Any + Send + Sync {
    fn table(&self) -> &dyn SQLTableInfo;
    /// The name of this index (for DROP INDEX statements)
    fn name(&self) -> &'static str;

    /// Column names included in this index, in definition order.
    fn columns(&self) -> &'static [&'static str] {
        &[]
    }

    /// Whether this is a unique index
    fn is_unique(&self) -> bool {
        false
    }
}

pub trait AsIndexInfo: Sized + SQLIndexInfo {
    fn as_index(&self) -> &dyn SQLIndexInfo {
        self
    }
}

impl core::fmt::Debug for dyn SQLIndexInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SQLIndexInfo")
            .field("name", &self.name())
            .field("is_unique", &self.is_unique())
            .field("columns", &self.columns())
            .field("table", &self.table())
            .finish()
    }
}
/// Trait for types that represent database indexes.
/// Implemented by tuple structs like `struct UserEmailIdx(User::email);`
pub trait SQLIndex<'a, Type: SQLSchemaType, Value: SQLParam + 'a>:
    SQLIndexInfo + ToSQL<'a, Value>
{
    /// The table type this index is associated with
    type Table: SQLTable<'a, Type, Value>;
}
