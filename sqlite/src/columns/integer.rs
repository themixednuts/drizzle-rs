use std::marker::PhantomData;

use common::{
    builders::column::ColumnBaseBuilder,
    traits::{Comparable, DefaultFn, DefaultValue, NotNull, PrimaryKey, Unique},
    ToSQL,
};

use crate::{
    common::Integer,
    traits::column::{Autoincrement, SQLAutoIncrement, SQLiteMode},
};

use super::{
    DefaultFnNotSet, DefaultNotSet, IsPrimary, NotPrimary, NotUnique, Nullable, SQLiteColumn,
    SQLiteColumnBuilder,
};

#[derive(Debug, Default, Clone, Copy)]
pub struct IsAutoIncremented;

impl Autoincrement for IsAutoIncremented {
    const AUTOINCREMENT: bool = true;
}
#[derive(Debug, Default, Clone, Copy)]
pub struct NotAutoIncremented;

impl Autoincrement for NotAutoIncremented {
    const AUTOINCREMENT: bool = false;
}

// #[derive(Debug, Default, Clone, Copy)]
// pub enum SQLiteIntegerMode {
//     #[default]
//     Number,
//     Timestamp,
//     TimestampMS,
//     Boolean,
// }

pub type SQLiteIntegerColumnBuilder<
    DataMode,
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TAutoincrement = NotAutoIncremented,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<Integer, std::fmt::Error>,
> = SQLiteColumnBuilder<
    i64,
    Integer,
    DataMode,
    TPrimary,
    TNotNull,
    TUnique,
    TAutoincrement,
    TDefault,
    TDefaultFn,
    TFunc,
>;

type SQLiteIntegerColumnBuilderAutoIncrementNotSet<M, N, U, D, F, Fun> =
    SQLiteIntegerColumnBuilder<M, IsPrimary, N, U, NotAutoIncremented, D, F, Fun>;

type SQLiteIntegerColumnBuilderAutoIncrementSet<M, N, U, D, F, Fun> =
    SQLiteIntegerColumnBuilder<M, IsPrimary, N, U, IsAutoIncremented, D, F, Fun>;

impl<
        M: SQLiteIntegerMode,
        N: NotNull,
        U: Unique,
        D: DefaultValue,
        F: DefaultFn,
        Fun: Fn() -> Result<Integer, std::fmt::Error> + Clone + Send + Sync,
    > SQLAutoIncrement for SQLiteIntegerColumnBuilderAutoIncrementNotSet<M, N, U, D, F, Fun>
{
    type Value = SQLiteIntegerColumnBuilderAutoIncrementSet<M, N, U, D, F, Fun>;

    fn autoincrement(self) -> Self::Value {
        SQLiteIntegerColumnBuilderAutoIncrementSet {
            base: self.base,
            default: self.default,
            default_fn: self.default_fn,
            unique_name: self.unique_name,
            _marker: PhantomData,
        }
    }
}

impl<
        M: SQLiteIntegerMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
        Fun: Fn() -> Result<Integer, std::fmt::Error> + Clone + Send + Sync,
    > ToSQL for SQLiteIntegerColumn<M, P, N, U, A, D, F, Fun>
{
    fn to_sql(&self) -> String {
        let name = format!(r#""{}""#, self.name);
        let mut sql = vec![name.as_str(), "INTEGER"];

        if P::IS_PRIMARY && !U::IS_UNIQUE {
            sql.push("PRIMARY KEY");
        }

        if A::AUTOINCREMENT {
            sql.push("AUTOINCREMENT");
        }

        println!("Checking NOT NULL");
        if N::IS_NOT_NULL {
            println!("IS NOT NULL");

            sql.push("NOT NULL");
        }

        sql.join(" ").into()
    }
}

pub fn integer<Mode: SQLiteIntegerMode>(
    name: &'static str,
    mode: Mode,
) -> SQLiteIntegerColumnBuilder<Mode> {
    SQLiteIntegerColumnBuilder {
        base: ColumnBaseBuilder {
            name,
            mode,
            ..Default::default()
        },
        ..Default::default()
    }
}

pub trait SQLiteIntegerMode: SQLiteMode {}

pub trait IntegerMode: SQLiteIntegerMode {}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SQLiteInteger {}
impl SQLiteMode for SQLiteInteger {}
impl IntegerMode for SQLiteInteger {}
impl SQLiteIntegerMode for SQLiteInteger {}

pub trait TimeStampMode: SQLiteMode {}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SQLiteTimeStamp {}
impl SQLiteMode for SQLiteTimeStamp {}
impl TimeStampMode for SQLiteTimeStamp {}
impl SQLiteIntegerMode for SQLiteTimeStamp {}

pub trait TimeStampMS: SQLiteMode {}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SQLiteTimeStampMS {}
impl SQLiteMode for SQLiteTimeStampMS {}
impl TimeStampMS for SQLiteTimeStampMS {}
impl SQLiteIntegerMode for SQLiteTimeStampMS {}

pub trait Boolean: SQLiteMode {}
#[derive(Default, Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct SQLiteBoolean {}
impl SQLiteMode for SQLiteBoolean {}
impl Boolean for SQLiteBoolean {}
impl SQLiteIntegerMode for SQLiteBoolean {}

pub type SQLiteIntegerColumn<
    DataMode,
    TPrimary = NotPrimary,
    TNotNull = Nullable,
    TUnique = NotUnique,
    TAutoincrement = NotAutoIncremented,
    TDefault = DefaultNotSet,
    TDefaultFn = DefaultFnNotSet,
    TFunc = fn() -> Result<Integer, std::fmt::Error>,
> = SQLiteColumn<
    i64,
    Integer,
    DataMode,
    TPrimary,
    TNotNull,
    TUnique,
    TAutoincrement,
    TDefault,
    TDefaultFn,
    TFunc,
>;

impl<
        M: SQLiteIntegerMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
    > From<SQLiteIntegerColumnBuilder<M, P, N, U, A, D, F>>
    for SQLiteIntegerColumn<M, P, N, U, A, D, F>
{
    fn from(value: SQLiteIntegerColumnBuilder<M, P, N, U, A, D, F>) -> Self {
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

impl<
        M: IntegerMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
    > Comparable<i64> for SQLiteIntegerColumn<M, P, N, U, A, D, F>
{
}

impl<
        M: IntegerMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
    > Comparable<&i64> for SQLiteIntegerColumn<M, P, N, U, A, D, F>
{
}

impl<
        M: IntegerMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
    > Comparable<Self> for SQLiteIntegerColumn<M, P, N, U, A, D, F>
{
}

impl<
        M: IntegerMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
    > Comparable<Self> for &SQLiteIntegerColumn<M, P, N, U, A, D, F>
{
}

#[cfg(test)]
mod test {

    use crate::prelude::*;

    #[test]
    fn builder() {
        let num = 42;
        let int = integer("id", SQLiteInteger {}).default(42).primary();

        // println!("{}", int.to_sql());

        // assert_eq!(int.base.default, Some(num))
        // .autoincrement()
        // .not_null()
        // .default(42);
    }
}
