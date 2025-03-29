pub mod common;
pub mod query_builder;

pub use self::common::SQLiteTableType;
pub use self::query_builder::SQLiteQueryBuilder;
pub use drivers::SQLiteValue;

// Re-export the SQLiteColumn type for convenience
pub use self::common::IntoSQLiteValue;
pub use self::query_builder::{
    Columns, DeleteBuilder, InsertBuilder, JoinType, QueryBuilder, SelectBuilder, SortDirection,
    UpdateBuilder, alias,
};

/// A column in a SQLite table
#[derive(Clone, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SQLiteColumn<'a, T: IntoSQLiteValue<'a>, Tbl: Clone> {
    pub name: &'a str,
    pub sql: &'a str,
    pub default_fn: Option<fn() -> T>,
    pub _phantom: std::marker::PhantomData<Tbl>,
}

impl<'a, T: IntoSQLiteValue<'a>, Tbl: Clone> SQLiteColumn<'a, T, Tbl> {
    /// Create a new column definition
    pub const fn new(name: &'a str, sql: &'a str, default_fn: Option<fn() -> T>) -> Self {
        Self {
            name,
            sql,
            default_fn,
            _phantom: std::marker::PhantomData,
        }
    }

    /// Alias a column
    ///
    /// # Arguments
    /// * `alias` - The alias to use for the column
    ///
    /// # Returns
    /// A SQL expression representing "column AS alias"
    pub fn as_(&self, alias: &str) -> crate::SQL<'a, SQLiteValue<'a>> {
        use crate::core::ToSQL;
        use std::borrow::Cow;
        let sql = self.to_sql();
        crate::SQL(Cow::Owned(format!("{} AS {}", sql.0, alias)), sql.1)
    }
}

impl<'a, T: IntoSQLiteValue<'a>, Tbl: Clone> crate::core::ToSQL<'a, SQLiteValue<'a>>
    for SQLiteColumn<'a, T, Tbl>
{
    fn to_sql(&self) -> crate::SQL<'a, SQLiteValue<'a>> {
        use std::borrow::Cow;
        crate::SQL(Cow::Owned(self.name.to_string()), Vec::new())
    }
}

// Create a type helper trait to get the type of a column
pub trait ColumnType<T> {
    type OutputType;
}

impl<'a, T: IntoSQLiteValue<'a>, Tbl: Clone> ColumnType<T> for SQLiteColumn<'a, T, Tbl> {
    type OutputType = T;
}

/// Module containing types that are exported as part of the prelude
pub mod prelude {
    pub use super::ColumnType;
    pub use super::SQLiteColumn;
    pub use super::common::{IntoSQLiteValue, SQLiteEnum, SQLiteEnumRepr, SQLiteTableType};
    pub use super::query_builder::{
        Columns, DeleteBuilder, InsertBuilder, JoinType, QueryBuilder, SelectBuilder,
        SortDirection, UpdateBuilder, alias,
    };
}
