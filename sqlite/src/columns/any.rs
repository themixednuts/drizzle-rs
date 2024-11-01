use std::marker::PhantomData;

use common::{
    builders::column::ColumnBaseBuilder,
    traits::{Comparable, DefaultFn, DefaultValue, NotNull, PrimaryKey, Unique},
    ToSQL,
};

use crate::{common::Any, traits::column::SQLiteMode};

use super::{
    integer::NotAutoIncremented, DefaultFnNotSet, DefaultNotSet, NotPrimary, NotUnique, Nullable,
    SQLiteColumn, SQLiteColumnBuilder,
};

pub type SQLiteAnyColumnBuilder<
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

pub fn any(name: &'static str) -> SQLiteAnyColumnBuilder {
    SQLiteAnyColumnBuilder {
        base: ColumnBaseBuilder {
            name,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub trait SQLiteAnyMode {}

pub trait TSQLiteAny: SQLiteMode {}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SQLiteAny {}
impl SQLiteMode for SQLiteAny {}
impl TSQLiteAny for SQLiteAny {}
impl SQLiteAnyMode for SQLiteAny {}

pub type SQLiteAnyColumn<
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<Any, std::fmt::Error>,
> = SQLiteColumn<
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

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    From<SQLiteAnyColumnBuilder<P, N, U, D, F>> for SQLiteAnyColumn<P, N, U, D, F>
{
    fn from(value: SQLiteAnyColumnBuilder<P, N, U, D, F>) -> Self {
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
impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> ToSQL
    for SQLiteAnyColumn<P, N, U, D, F>
{
    fn to_sql(&self) -> String {
        let name = format!(r#""{}""#, self.name);
        let mut sql = vec![name.as_str(), "ANY"];

        if P::IS_PRIMARY && !U::IS_UNIQUE {
            sql.push("PRIMARY KEY");
        }

        if N::IS_NOT_NULL {
            sql.push("NOT NULL");
        }

        sql.join(" ").to_string()
    }
}

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
