use common::{
    builders::column::ColumnBaseBuilder,
    traits::{Comparable, DefaultFn, DefaultValue, NotNull, PrimaryKey, Unique},
};

use crate::{common::Any, traits::column::SQLiteMode};

use super::{
    integer::NotAutoIncremented, DefaultFnNotSet, DefaultNotSet, NotPrimary, NotUnique, Nullable,
    SQLiteColumnBuilder,
};

pub type SQLiteAnyColumn<
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<Any, std::fmt::Error>,
> = SQLiteColumnBuilder<
    Any,
    Any,
    SQLiteAny,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    TFunc,
>;

pub fn any(name: &'static str) -> SQLiteAnyColumn {
    SQLiteAnyColumn {
        base: ColumnBaseBuilder {
            name,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub trait SQLiteAnyMode {}

pub trait TSQLiteAny: SQLiteMode {}
#[derive(Default, Clone, Copy, Debug)]
pub struct SQLiteAny {}
impl SQLiteMode for SQLiteAny {}
impl TSQLiteAny for SQLiteAny {}
impl SQLiteAnyMode for SQLiteAny {}

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<f64>
    for SQLiteAnyColumn<P, N, U, D, F>
{
}
impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<&f64>
    for SQLiteAnyColumn<P, N, U, D, F>
{
}

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<Self>
    for SQLiteAnyColumn<P, N, U, D, F>
{
}

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<Self>
    for &SQLiteAnyColumn<P, N, U, D, F>
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
