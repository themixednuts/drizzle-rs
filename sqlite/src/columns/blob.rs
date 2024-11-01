use std::marker::PhantomData;

use common::{
    builders::column::ColumnBaseBuilder,
    traits::{Comparable, DefaultFn, DefaultValue, NotNull, PrimaryKey, Unique},
};

use crate::{common::Blob, traits::column::SQLiteMode};

use super::{
    integer::NotAutoIncremented, DefaultFnNotSet, DefaultNotSet, NotPrimary, NotUnique, Nullable,
    SQLiteColumn, SQLiteColumnBuilder,
};

pub type SQLiteBlobColumnBuilder<
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<Vec<u8>, std::fmt::Error>,
> = SQLiteColumnBuilder<
    Vec<u8>,
    Blob,
    SQLiteBlob,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    TFunc,
>;

pub trait BlobMode: SQLiteMode {}
#[derive(Default, Clone, Copy, Debug)]
pub struct SQLiteBlob {}
impl SQLiteMode for SQLiteBlob {}
impl BlobMode for SQLiteBlob {}

pub fn blob(name: &'static str) -> SQLiteBlobColumnBuilder {
    SQLiteBlobColumnBuilder {
        base: ColumnBaseBuilder {
            name,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub type SQLiteBlobColumn<
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<Vec<u8>, std::fmt::Error>,
> = SQLiteColumn<
    Vec<u8>,
    Blob,
    SQLiteBlob,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    TFunc,
>;

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    From<SQLiteBlobColumnBuilder<P, N, U, D, F>> for SQLiteBlobColumn<P, N, U, D, F>
{
    fn from(value: SQLiteBlobColumnBuilder<P, N, U, D, F>) -> Self {
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
impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<Vec<u8>>
    for SQLiteBlobColumn<P, N, U, D, F>
{
}
impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<&Vec<u8>>
    for SQLiteBlobColumn<P, N, U, D, F>
{
}

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<&[u8]>
    for SQLiteBlobColumn<P, N, U, D, F>
{
}

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<SQLiteBlobColumn<P, N, U, D, F>> for Vec<u8>
{
}

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<Self>
    for SQLiteBlobColumn<P, N, U, D, F>
{
}

impl<P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> Comparable<Self>
    for &SQLiteBlobColumn<P, N, U, D, F>
{
}

#[cfg(test)]
mod test {
    use super::blob;
    use core::panic;

    // #[test]
    // fn builder() {
    //     let b = blob("id").primary().not_null().default(vec![]);

    //     std::thread::spawn(move || {
    //         let b = b;
    //         assert_eq!(b.base.default, Some(vec![]));
    //     });

    //     let b = blob("id").default_fn(|| Ok(vec![]));

    //     // .autoincrement()
    //     // .not_null()
    //     // .default(42);
    // }
}
