use std::marker::PhantomData;

use common::{
    builders::column::ColumnBaseBuilder,
    traits::{Comparable, DefaultFn, DefaultValue, NotNull, PrimaryKey, Unique},
};

use crate::{common::Number, traits::column::SQLiteMode};

use super::{
    integer::NotAutoIncremented, DefaultFnNotSet, DefaultNotSet, NotPrimary, NotUnique, Nullable,
    SQLiteColumn, SQLiteColumnBuilder,
};

pub type SQLiteNumberColumnBuilder<
    DataMode = SQLiteNumber,
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<Number, std::fmt::Error>,
> = SQLiteColumnBuilder<
    Number,
    Number,
    DataMode,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    TFunc,
>;

pub trait SQLiteNumberMode: SQLiteMode {}

pub trait NumberMode: SQLiteNumberMode {}
#[derive(Default, Clone, Copy, Debug)]
pub struct SQLiteNumber {}
impl SQLiteMode for SQLiteNumber {}
impl NumberMode for SQLiteNumber {}
impl SQLiteNumberMode for SQLiteNumber {}

pub fn number(name: &'static str) -> SQLiteNumberColumnBuilder {
    SQLiteNumberColumnBuilder {
        base: ColumnBaseBuilder {
            name,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub type SQLiteNumberColumn<
    DataMode,
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<Number, std::fmt::Error>,
> = SQLiteColumn<
    Number,
    Number,
    DataMode,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    TFunc,
>;

impl<M: SQLiteNumberMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    From<SQLiteNumberColumnBuilder<M, P, N, U, D, F>> for SQLiteNumberColumn<M, P, N, U, D, F>
{
    fn from(value: SQLiteNumberColumnBuilder<M, P, N, U, D, F>) -> Self {
        Self {
            name: value.base.name,
            data_type: value.base.data_type,
            column_type: value.base.column_type,
            unique_name: value.unique_name,
            default: value.default,
            default_fn: value.default_fn,
            _marker: PhantomData,
        }
    }
}
impl<M: SQLiteNumberMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<f64> for SQLiteNumberColumn<M, P, N, U, D, F>
{
}
impl<M: SQLiteNumberMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<&f64> for SQLiteNumberColumn<M, P, N, U, D, F>
{
}

impl<M: SQLiteNumberMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<Self> for SQLiteNumberColumn<M, P, N, U, D, F>
{
}

impl<M: SQLiteNumberMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<Self> for &SQLiteNumberColumn<M, P, N, U, D, F>
{
}

#[cfg(test)]
mod test {

    // #[test]
    // fn builder() {
    //     let str = 12.0;
    //     let int = real("id").primary().not_null().default(str);

    //     std::thread::spawn(move || {
    //         let int = int;
    //         assert_eq!(int.base.default, Some(12.0));
    //     });

    // .autoincrement()
    // .not_null()
    // .default(42);
    // }
}
