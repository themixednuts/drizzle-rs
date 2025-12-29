use crate::traits::SQLSchemaType;
use crate::{SQL, SQLParam, SQLTable, SQLTableInfo};
use core::any::Any;

/// Trait for database views.
///
/// A View is essentially a named SQL query that can be queried like a table.
pub trait SQLView<'a, Type: SQLSchemaType, Value: SQLParam + 'a>:
    SQLTable<'a, Type, Value>
{
    /// Returns the SQL definition of this view (the SELECT statement).
    fn definition(&self) -> SQL<'a, Value>;

    /// Returns true if this is an existing view in the database not managed by Drizzle.
    fn is_existing(&self) -> bool {
        false
    }
}

/// Metadata information about a database view.
pub trait SQLViewInfo: SQLTableInfo + Any {
    /// Returns the SQL definition of this view as a static string.
    fn definition_sql(&self) -> &'static str;

    /// Returns true if this is a materialized view.
    fn is_materialized(&self) -> bool {
        false
    }

    /// Erased access to the view info.
    fn as_view_info(&self) -> &dyn SQLViewInfo
    where
        Self: Sized,
    {
        self
    }
}

impl core::fmt::Debug for dyn SQLViewInfo {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("SQLViewInfo")
            .field("name", &self.name())
            .field("definition", &self.definition_sql())
            .field("materialized", &self.is_materialized())
            .finish()
    }
}
