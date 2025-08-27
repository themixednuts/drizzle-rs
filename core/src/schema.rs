use crate::{ToSQL, sql::SQL, traits::SQLParam};
use std::any::Any;

/// Trait for database enum types that can be part of a schema
pub trait SQLEnumInfo: Any + Send + Sync {
    /// The name of this enum type
    fn name(&self) -> &'static str;

    /// The SQL CREATE TYPE statement for this enum
    fn create_type_sql(&self) -> String;

    /// All possible values of this enum
    fn variants(&self) -> &'static [&'static str];
}

/// Helper trait for converting enum info objects to trait objects
pub trait AsEnumInfo: SQLEnumInfo {
    fn as_enum(&self) -> &dyn SQLEnumInfo;
}

impl<T: SQLEnumInfo> AsEnumInfo for T {
    fn as_enum(&self) -> &dyn SQLEnumInfo {
        self
    }
}

impl std::fmt::Debug for dyn SQLEnumInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SQLEnumInfo")
            .field("name", &self.name())
            .field("variants", &self.variants())
            .finish()
    }
}

/// Sort direction for ORDER BY clauses
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderBy {
    Asc,
    Desc,
}

impl OrderBy {
    /// Creates an ascending ORDER BY clause: "column ASC"
    pub fn asc<'a, V, T>(column: T) -> SQL<'a, V>
    where
        V: SQLParam + 'a,
        T: ToSQL<'a, V>,
    {
        column.to_sql().append(Self::Asc.to_sql())
    }

    /// Creates a descending ORDER BY clause: "column DESC"
    pub fn desc<'a, V, T>(column: T) -> SQL<'a, V>
    where
        V: SQLParam + 'a,
        T: ToSQL<'a, V>,
    {
        column.to_sql().append(Self::Desc.to_sql())
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for OrderBy {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql_str = match self {
            OrderBy::Asc => "ASC",
            OrderBy::Desc => "DESC",
        };
        SQL::raw(sql_str)
    }
}
