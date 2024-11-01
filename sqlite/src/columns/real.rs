use std::marker::PhantomData;

use common::{
    builders::column::ColumnBaseBuilder,
    traits::{Comparable, DefaultFn, DefaultValue, NotNull, PrimaryKey, Unique},
};

use crate::{common::Real, traits::column::SQLiteMode};

use super::{
    integer::NotAutoIncremented, DefaultFnNotSet, DefaultNotSet, NotPrimary, NotUnique, Nullable,
    SQLiteColumn, SQLiteColumnBuilder,
};

pub type SQLiteRealColumnBuilder<
    DataMode = SQLiteReal,
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<f64, std::fmt::Error>,
> = SQLiteColumnBuilder<
    f64,
    Real,
    DataMode,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    TFunc,
>;

pub trait SQLiteRealMode: SQLiteMode {}

pub trait RealMode: SQLiteRealMode {}
#[derive(Default, Clone, Copy, Debug)]
pub struct SQLiteReal {}
impl SQLiteMode for SQLiteReal {}
impl RealMode for SQLiteReal {}
impl SQLiteRealMode for SQLiteReal {}

pub fn real(name: &'static str) -> SQLiteRealColumnBuilder<SQLiteReal> {
    SQLiteRealColumnBuilder {
        base: ColumnBaseBuilder {
            name,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub type SQLiteRealColumn<
    DataMode = SQLiteReal,
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<f64, std::fmt::Error>,
> = SQLiteColumn<
    f64,
    Real,
    DataMode,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    TFunc,
>;

impl<M: SQLiteRealMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    From<SQLiteRealColumnBuilder<M, P, N, U, D, F>> for SQLiteRealColumn<M, P, N, U, D, F>
{
    fn from(value: SQLiteRealColumnBuilder<M, P, N, U, D, F>) -> Self {
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

impl<M: SQLiteRealMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<f64> for SQLiteRealColumn<M, P, N, U, D, F>
{
}
impl<M: SQLiteRealMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<&f64> for SQLiteRealColumn<M, P, N, U, D, F>
{
}

impl<M: SQLiteRealMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<Self> for SQLiteRealColumn<M, P, N, U, D, F>
{
}

impl<M: SQLiteRealMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<Self> for &SQLiteRealColumn<M, P, N, U, D, F>
{
}

#[cfg(test)]
mod test {
    use super::real;
    use core::panic;

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
