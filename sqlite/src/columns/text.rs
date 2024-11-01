use std::{fmt, marker::PhantomData};

use common::{
    builders::column::ColumnBaseBuilder,
    traits::{Comparable, DefaultFn, DefaultValue, NotNull, PrimaryKey, Unique},
    ToSQL,
};

use crate::{common::Text, traits::column::SQLiteMode};

use super::{
    integer::NotAutoIncremented, DefaultFnNotSet, DefaultNotSet, NotPrimary, NotUnique, Nullable,
    SQLiteColumn, SQLiteColumnBuilder,
};

pub type SQLiteTextColumnBuilder<
    DataMode = SQLiteText,
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    Func = fn() -> Result<String, std::fmt::Error>,
> = SQLiteColumnBuilder<
    String,
    Text,
    DataMode,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    Func,
>;

pub trait SQLiteTextMode: SQLiteMode {}

pub trait StringMode: SQLiteTextMode {}
#[derive(Default, Clone, Copy, Debug)]
pub struct SQLiteText {}
impl SQLiteMode for SQLiteText {}
impl StringMode for SQLiteText {}
impl SQLiteTextMode for SQLiteText {}

pub trait TextEnumMode: SQLiteTextMode {}
#[derive(Default, Clone, Copy, Debug)]
pub struct SQLiteTextEnum(&'static [&'static str]);
impl SQLiteMode for SQLiteTextEnum {}
impl TextEnumMode for SQLiteTextEnum {}
impl SQLiteTextMode for SQLiteTextEnum {}

pub trait JSONMode: SQLiteTextMode {}
#[derive(Default, Clone, Copy, Debug)]
pub struct SQLiteJSON {}
impl SQLiteMode for SQLiteJSON {}
impl JSONMode for SQLiteJSON {}
impl SQLiteTextMode for SQLiteJSON {}

pub fn text<Mode: SQLiteTextMode>(name: &'static str, mode: Mode) -> SQLiteTextColumnBuilder<Mode> {
    SQLiteTextColumnBuilder {
        base: ColumnBaseBuilder {
            name,
            mode,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub type SQLiteTextColumn<
    DataMode = SQLiteText,
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    Func = fn() -> Result<String, std::fmt::Error>,
> = SQLiteColumn<
    String,
    Text,
    DataMode,
    TPrimary,
    TNotNull,
    TUnique,
    NotAutoIncremented,
    TDefault,
    TDefaultFn,
    Func,
>;

impl<M: SQLiteTextMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    From<SQLiteTextColumnBuilder<M, P, N, U, D, F>> for SQLiteTextColumn<M, P, N, U, D, F>
{
    fn from(value: SQLiteTextColumnBuilder<M, P, N, U, D, F>) -> Self {
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

impl<M: SQLiteTextMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn> ToSQL
    for SQLiteTextColumn<M, P, N, U, D, F>
{
    fn to_sql(self) -> String {
        let name = format!(r#""{}""#, self.name);
        let mut sql = vec![name.as_str(), "INTEGER"];

        if P::IS_PRIMARY && !U::IS_UNIQUE {
            sql.push("PRIMARY KEY");
        }

        if N::IS_NOT_NULL {
            sql.push("NOT NULL");
        }

        sql.join(" ").to_string()
    }
}
impl<M: SQLiteTextMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<String> for SQLiteTextColumn<M, P, N, U, D, F>
{
}
impl<M: SQLiteTextMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<&String> for SQLiteTextColumn<M, P, N, U, D, F>
{
}

impl<M: SQLiteTextMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<&str> for SQLiteTextColumn<M, P, N, U, D, F>
{
}

impl<M: SQLiteTextMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<Self> for SQLiteTextColumn<M, P, N, U, D, F>
{
}

impl<M: SQLiteTextMode, P: PrimaryKey, N: NotNull, U: Unique, D: DefaultValue, F: DefaultFn>
    Comparable<Self> for &SQLiteTextColumn<M, P, N, U, D, F>
{
}

#[cfg(test)]
mod test {
    use crate::prelude::*;

    #[test]
    fn builder() {
        let str = "my text";
        let int = text("id", SQLiteText {})
            .primary()
            .not_null()
            .default(str.into());

        std::thread::spawn(move || {
            let int = int;
            assert_eq!(int.default, Some(str.into()));
        });

        // .autoincrement()
        // .not_null()
        // .default(42);
    }
}
