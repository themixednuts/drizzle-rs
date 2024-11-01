use std::{fmt::Display, marker::PhantomData, ops::Deref};

use common::{
    builders::column::ColumnBaseBuilder,
    traits::{
        ColumnBuilder, DefaultFn, DefaultValue, NotNull, PrimaryKey, SQLDefault, SQLDefaultFn,
        SQLNotNull, SQLPrimary, SQLUnique, Unique,
    },
    ToSQL,
};
use integer::NotAutoIncremented;

use crate::traits::column::{Autoincrement, Column, SQLiteMode};

pub mod any;
pub mod blob;
pub mod integer;
pub mod number;
pub mod real;
pub mod text;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotSet;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NoDefaultFn;

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IsPrimary;

impl PrimaryKey for IsPrimary {
    const IS_PRIMARY: bool = true;
}
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotPrimary;

impl PrimaryKey for NotPrimary {
    const IS_PRIMARY: bool = false;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotNullable;

impl NotNull for NotNullable {
    const IS_NOT_NULL: bool = true;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Nullable;

impl NotNull for Nullable {
    const IS_NOT_NULL: bool = false;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct IsUnique;

impl Unique for IsUnique {
    const IS_UNIQUE: bool = true;
}
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct NotUnique;

impl Unique for NotUnique {
    const IS_UNIQUE: bool = false;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefaultSet;

impl DefaultValue for DefaultSet {
    const HAS_DEFAULT: bool = true;
}
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefaultNotSet;

impl DefaultValue for DefaultNotSet {
    const HAS_DEFAULT: bool = false;
}

#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefaultFnSet;

impl DefaultFn for DefaultFnSet {
    const HAS_DEFAULT_FN: bool = true;
}
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DefaultFnNotSet;

impl DefaultFn for DefaultFnNotSet {
    const HAS_DEFAULT_FN: bool = false;
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub struct SQLiteColumnBuilder<
    DataType: Default + Clone + Sync + Send + PartialEq,
    ColumnType: Default + Clone + Sync + Send + PartialEq,
    DataMode: SQLiteMode,
    TPrimary: PrimaryKey = NotPrimary,
    TNotNull: NotNull = Nullable,
    TUnique: Unique = NotUnique,
    TAutoincrement: Autoincrement = NotAutoIncremented,
    TDefault: DefaultValue = DefaultNotSet,
    TDefaultFn: DefaultFn = DefaultFnNotSet,
    TFunc: Fn() -> Result<DataType, std::fmt::Error> + Clone + Sync + Send = fn() -> Result<
        DataType,
        std::fmt::Error,
    >,
    // TUpdate,
    // TUpdateFn,
    // TUpdateFunc,
    // TConflict,
    // TConflictFn,
    // TConflictFunc,
> {
    pub(crate) base: ColumnBaseBuilder<DataType, ColumnType, DataMode>,
    pub(crate) unique_name: Option<&'static str>,
    pub(crate) default: Option<DataType>,
    pub(crate) default_fn: Option<TFunc>,
    pub(crate) _marker: PhantomData<(
        TPrimary,
        TNotNull,
        TUnique,
        TAutoincrement,
        TDefault,
        TDefaultFn,
    )>,
    // onUpdateFn: Option<Fn2>,
    // uniqueName: Option<String>,
}

impl<
        DataType: Default + Clone + Sync + Send + PartialEq,
        ColumnType: Default + Clone + Sync + Send + PartialEq,
        DataMode: SQLiteMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
    > Default for SQLiteColumnBuilder<DataType, ColumnType, DataMode, P, N, U, A, D, F>
{
    fn default() -> Self {
        Self {
            base: ColumnBaseBuilder::default(),
            unique_name: None,
            default: None,
            default_fn: None,
            _marker: PhantomData,
        }
    }
}

// Primary FieldSet
type SQLiteColumnBuilderPrimaryNotSet<DataType, ColumnType, DataMode, N, U, A, D, F, Fun> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, NotPrimary, N, U, A, D, F, Fun>;

type SQLiteColumnBuilderPrimarySet<DataType, ColumnType, DataMode, N, U, A, D, F, Fun> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, IsPrimary, N, U, A, D, F, Fun>;

impl<
        DataType: Default + Clone + Sync + Send + Eq,
        ColumnType: Default + Clone + Sync + Send + Eq,
        DataMode: SQLiteMode,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
        Fun: Fn() -> Result<DataType, std::fmt::Error> + Sized + Clone + Sync + Send,
    > SQLPrimary
    for SQLiteColumnBuilderPrimaryNotSet<DataType, ColumnType, DataMode, N, U, A, D, F, Fun>
{
    type Value = SQLiteColumnBuilderPrimarySet<DataType, ColumnType, DataMode, N, U, A, D, F, Fun>;
    fn primary(self) -> Self::Value {
        SQLiteColumnBuilder {
            base: self.base,
            default: self.default,
            default_fn: self.default_fn,
            unique_name: self.unique_name,
            _marker: PhantomData,
        }
    }
}

// Not Null FieldSet
type SQLiteColumnBuilderNotNullNotSet<DataType, ColumnType, DataMode, P, U, A, D, F, Fun> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, P, Nullable, U, A, D, F, Fun>;

type SQLiteColumnBuilderNotNullSet<DataType, ColumnType, DataMode, P, U, A, D, F, Fun> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, P, NotNullable, U, A, D, F, Fun>;

impl<
        DataType: Default + Clone + Sync + Send + Eq,
        ColumnType: Default + Clone + Sync + Send + Eq,
        DataMode: SQLiteMode,
        P: PrimaryKey,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
        Fun: Fn() -> Result<DataType, std::fmt::Error> + Clone + Sync + Send,
    > SQLNotNull
    for SQLiteColumnBuilderNotNullNotSet<DataType, ColumnType, DataMode, P, U, A, D, F, Fun>
{
    type Value = SQLiteColumnBuilderNotNullSet<DataType, ColumnType, DataMode, P, U, A, D, F, Fun>;
    fn not_null(self) -> Self::Value {
        SQLiteColumnBuilder {
            base: self.base,
            default: self.default,
            default_fn: self.default_fn,
            unique_name: self.unique_name,
            _marker: PhantomData,
        }
    }
}

// Default FieldSet
type SQLiteColumnBuilderDefaultNotSet<DataType, ColumnType, DataMode, P, N, U, A> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, P, N, U, A, DefaultNotSet, DefaultFnNotSet>;

type SQLiteColumnBuilderDefaultSet<DataType, ColumnType, DataMode, P, N, U, A> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, P, N, U, A, DefaultSet, DefaultFnNotSet>;

impl<
        DataType: Default + Clone + Sync + Send + Eq,
        ColumnType: Default + Clone + Sync + Send + Eq,
        DataMode: SQLiteMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
    > SQLDefault for SQLiteColumnBuilderDefaultNotSet<DataType, ColumnType, DataMode, P, N, U, A>
{
    type Value = SQLiteColumnBuilderDefaultSet<DataType, ColumnType, DataMode, P, N, U, A>;
    type DataType = DataType;

    fn default(self, value: Self::DataType) -> Self::Value {
        SQLiteColumnBuilder {
            base: self.base,
            default: Some(value),
            default_fn: self.default_fn,
            unique_name: self.unique_name,
            _marker: PhantomData,
        }
    }
}

// DefaultFn FieldSet
type SQLiteColumnBuilderDefaultFnNotSet<DataType, ColumnType, DataMode, P, N, U, A, Func> =
    SQLiteColumnBuilder<
        DataType,
        ColumnType,
        DataMode,
        P,
        N,
        U,
        A,
        DefaultNotSet,
        DefaultFnNotSet,
        Func,
    >;

type SQLiteColumnBuilderDefaultFnSet<DataType, ColumnType, DataMode, P, N, U, A, F> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, P, N, U, A, DefaultNotSet, DefaultFnSet, F>;

impl<
        DataType: Default + Clone + Sync + Send + Eq,
        ColumnType: Default + Clone + Sync + Send + Eq,
        DataMode: SQLiteMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        Fun: Fn() -> Result<DataType, std::fmt::Error> + Clone + Sync + Send,
    > SQLDefaultFn<DataType, Fun>
    for SQLiteColumnBuilderDefaultFnNotSet<DataType, ColumnType, DataMode, P, N, U, A, Fun>
{
    type Value = SQLiteColumnBuilderDefaultFnSet<DataType, ColumnType, DataMode, P, N, U, A, Fun>;

    fn default_fn(self, value: Fun) -> Self::Value {
        SQLiteColumnBuilder {
            base: self.base,
            default: self.default,
            default_fn: Some(value),
            unique_name: self.unique_name,
            _marker: PhantomData,
        }
    }
}

// Unique FieldSet
type SQLiteColumnBuilderUniqueNotSet<DataType, ColumnType, DataMode, N, A, D, F, Fun> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, NotPrimary, N, NotUnique, A, D, F, Fun>;

type SQLiteColumnBuilderUniqueSet<DataType, ColumnType, DataMode, N, A, D, F, Fun> =
    SQLiteColumnBuilder<DataType, ColumnType, DataMode, NotPrimary, N, IsUnique, A, D, F, Fun>;

impl<
        DataType: Default + Clone + Sync + Send + Eq,
        ColumnType: Default + Clone + Sync + Send + Eq,
        DataMode: SQLiteMode,
        N: NotNull,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
        Fun: Fn() -> Result<DataType, std::fmt::Error> + Clone + Sync + Send,
    > SQLUnique
    for SQLiteColumnBuilderUniqueNotSet<DataType, ColumnType, DataMode, N, A, D, F, Fun>
{
    type Value = SQLiteColumnBuilderUniqueSet<DataType, ColumnType, DataMode, N, A, D, F, Fun>;

    fn unique(self, value: &'static str) -> Self::Value {
        SQLiteColumnBuilder {
            base: self.base,
            default: self.default,
            default_fn: self.default_fn,
            unique_name: Some(value),
            _marker: PhantomData,
        }
    }
}

// impl<
//         DataType: Default + Clone + Sync + Send,
//         ColumnType: Default + Clone + Sync + Send,
//         DataMode: Default + Clone + Sync + Send,
//         TPrimary: PrimaryKey,
//         TNotNull: NotNull,
//         TUnique: Unique,
//         TAutoincrement: Autoincrement,
//         TDefault: DefaultValue,
//         TDefaultFn: DefaultFn,
//         TFunc: Fn() -> Result<DataType, std::fmt::Error> + Clone + Sync + Send,
//     > Display
//     for SQLiteColumn<
//         DataType,
//         ColumnType,
//         DataMode,
//         TPrimary,
//         TNotNull,
//         TUnique,
//         TAutoincrement,
//         TDefault,
//         TDefaultFn,
//         TFunc,
//     >
// where
//     Self: ToSQL,
// {
//     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
//         write!(f, "{}", self.clone().to_sql())
//     }
// }
#[derive(Debug, Clone, Copy, PartialEq, PartialOrd)]
pub struct SQLiteColumn<
    DataType: Default + Clone + Sync + Send + PartialEq,
    ColumnType: Default + Clone + Sync + Send + PartialEq,
    DataMode: SQLiteMode,
    TPrimary: PrimaryKey = NotPrimary,
    TNotNull: NotNull = Nullable,
    TUnique: Unique = NotUnique,
    TAutoincrement: Autoincrement = NotAutoIncremented,
    TDefault: DefaultValue = DefaultNotSet,
    TDefaultFn: DefaultFn = DefaultFnNotSet,
    TFunc: Fn() -> Result<DataType, std::fmt::Error> + Clone + Sync + Send = fn() -> Result<
        DataType,
        std::fmt::Error,
    >,
    // TUpdate,
    // TUpdateFn,
    // TUpdateFunc,
    // TConflict,
    // TConflictFn,
    // TConflictFunc,
> {
    pub(crate) name: &'static str,
    pub(crate) data_type: DataType,
    pub(crate) column_type: ColumnType,
    pub(crate) unique_name: Option<&'static str>,
    pub(crate) default: Option<DataType>,
    pub(crate) default_fn: Option<TFunc>,
    pub(crate) _marker: PhantomData<(
        TPrimary,
        TNotNull,
        TUnique,
        TAutoincrement,
        TDefault,
        TDefaultFn,
        DataMode,
    )>,
    // onUpdateFn: Option<Fn2>,
    // uniqueName: Option<String>,
}

// impl<
//         DataType: Default + Clone + Sync + Send,
//         ColumnType: Default + Clone + Sync + Send,
//         DataMode: SQLiteMode,
//         P: PrimaryKey,
//         N: NotNull,
//         U: Unique,
//         A: Autoincrement,
//         D: DefaultValue,
//         F: DefaultFn,
//     > Deref for SQLiteColumn<DataType, ColumnType, DataMode, P, N, U, A, D, F>
// {
//     type Target;

//     fn deref(&self) -> &Self::Target {
//         todo!()
//     }
// }

impl<
        DataType: Default + Clone + Sync + Send + Eq,
        ColumnType: Default + Clone + Sync + Send + Eq,
        DataMode: SQLiteMode,
        P: PrimaryKey,
        N: NotNull,
        U: Unique,
        A: Autoincrement,
        D: DefaultValue,
        F: DefaultFn,
    > Column for SQLiteColumn<DataType, ColumnType, DataMode, P, N, U, A, D, F>
{
    fn name(&self) -> &'static str {
        self.name
    }
}
