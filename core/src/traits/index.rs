use core::any::Any;

use crate::{SQLParam, SQLSchemaType, SQLTable, TableRef, ToSQL};

/// Compile-time index metadata.
///
/// Implementing this trait automatically provides [`SQLIndexInfo`] via a
/// blanket implementation.
pub trait DrizzleIndex: Send + Sync + 'static {
    /// Index name.
    const INDEX_NAME: &'static str;

    /// Column names included in this index, in definition order.
    const COLUMN_NAMES: &'static [&'static str];

    /// Whether this is a unique index.
    const IS_UNIQUE: bool = false;

    /// The table this index belongs to.
    fn table_ref() -> &'static TableRef;
}

/// Blanket: any `DrizzleIndex` automatically satisfies `SQLIndexInfo`.
impl<T: DrizzleIndex> SQLIndexInfo for T {
    fn table(&self) -> &'static TableRef {
        T::table_ref()
    }

    fn name(&self) -> &'static str {
        T::INDEX_NAME
    }

    fn columns(&self) -> &'static [&'static str] {
        T::COLUMN_NAMES
    }

    fn is_unique(&self) -> bool {
        T::IS_UNIQUE
    }
}

pub trait SQLIndexInfo: Any + Send + Sync {
    fn table(&self) -> &'static TableRef;
    /// The name of this index (for DROP INDEX statements)
    fn name(&self) -> &'static str;

    /// Column names included in this index, in definition order.
    fn columns(&self) -> &'static [&'static str];

    /// Whether this is a unique index
    fn is_unique(&self) -> bool {
        false
    }
}

impl core::fmt::Debug for dyn SQLIndexInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SQLIndexInfo")
            .field("name", &self.name())
            .field("is_unique", &self.is_unique())
            .field("columns", &self.columns())
            .field("table", &self.table().name)
            .finish()
    }
}

/// Trait for types that represent database indexes.
/// Implemented by tuple structs like `struct UserEmailIdx(User::email);`
#[diagnostic::on_unimplemented(
    message = "`{Self}` is not a SQL index for this dialect",
    label = "ensure this type was derived with #[SQLiteIndex] or #[PostgresIndex]"
)]
pub trait SQLIndex<'a, Type: SQLSchemaType, Value: SQLParam + 'a>:
    SQLIndexInfo + ToSQL<'a, Value>
{
    /// The table type this index is associated with
    type Table: SQLTable<'a, Type, Value>;
}
