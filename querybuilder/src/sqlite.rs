pub mod common;
pub mod traits;

use crate::{SQL, ToSQL};
use common::{SQLiteTableSchema, SQLiteValue};
use std::fmt::{self, Display};
use std::marker::PhantomData;

// Re-export common types through prelude
pub mod prelude {
    // Re-export SQLite-specific traits and types
    pub use super::traits::{column::*, table::*};

    // Re-export the SQLiteColumn type
    pub use super::SQLiteColumn;

    // Re-export SQLite-specific condition functions
    pub use super::common::*;

    // Re-export type aliases from common
    pub use super::common::{Blob, Integer, Real, Text};

    // Re-export core traits needed for SQLite functionality
    pub use crate::core::traits::*;
    pub use crate::core::{IntoValue, SQL, ToSQL};
}

// SQLite-specific column type with compile-time type information
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct SQLiteColumn<T, Tbl: SQLiteTableSchema + ?Sized, F, R>
where
    F: Fn() -> R,
    Option<T>: TryFrom<R>,
    <Option<T> as TryFrom<R>>::Error: std::fmt::Debug,
{
    name: &'static str,
    table: PhantomData<Tbl>,
    sql: &'static str,
    _t: PhantomData<T>,
    default_fn: Option<F>,
}

impl<T, Tbl: SQLiteTableSchema + ?Sized, F, R> SQLiteColumn<T, Tbl, F, R>
where
    F: Fn() -> R,
    Option<T>: TryFrom<R>,
    <Option<T> as TryFrom<R>>::Error: std::fmt::Debug,
{
    pub const fn new(name: &'static str, sql: &'static str, default_fn: Option<F>) -> Self {
        Self {
            name,
            table: PhantomData,
            _t: PhantomData,
            default_fn,
            sql,
        }
    }

    // pub const fn name(&self) -> &'static str {
    //     self.name
    // }

    // pub const fn sql(&self) -> &'static str {
    //     self.sql
    // }
}

impl<T, Tbl: SQLiteTableSchema + ?Sized, F, R> Display for SQLiteColumn<T, Tbl, F, R>
where
    F: Fn() -> R,
    Option<T>: TryFrom<R>,
    <Option<T> as TryFrom<R>>::Error: std::fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.name)
    }
}

impl<'a, T, Tbl: SQLiteTableSchema + ?Sized, F, R> ToSQL<'a, SQLiteValue<'a>>
    for SQLiteColumn<T, Tbl, F, R>
where
    T: Into<SQLiteValue<'a>>,
    F: Fn() -> R,
    Option<T>: TryFrom<R>,
    <Option<T> as TryFrom<R>>::Error: std::fmt::Debug,
{
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        SQL::new(self.sql, vec![])
    }
}
