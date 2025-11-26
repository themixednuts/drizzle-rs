use crate::prelude::*;
use core::any::Any;

mod column;
mod index;
mod param;
mod table;
mod tuple;

pub use column::*;
pub use index::*;
pub use param::*;
pub use table::*;

// Re-export enum traits from schema module
pub use crate::schema::{AsEnumInfo, SQLEnumInfo};

use crate::{SQL, ToSQL};

pub trait SQLSchema<'a, T, V: SQLParam + 'a>: ToSQL<'a, V> {
    const NAME: &'a str;
    const TYPE: T;
    const SQL: SQL<'a, V>;

    // Optional runtime SQL generation for tables with dynamic constraints
    fn sql(&self) -> SQL<'a, V> {
        Self::SQL
    }
}

pub trait SQLSchemaType: core::fmt::Debug + Any + Send + Sync {}

pub trait SQLSchemaImpl: Any + Send + Sync {
    fn create_statements(&self) -> Vec<String>;
}

/// Marker trait for types that can be compared in SQL expressions.
pub trait SQLComparable<'a, V: SQLParam, Rhs> {}

impl<'a, V, L, R> SQLComparable<'a, V, R> for L
where
    V: SQLParam + 'a,
    L: ToSQL<'a, V>,
    R: ToSQL<'a, V>,
{
}
