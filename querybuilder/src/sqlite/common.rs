use crate::core::SQLParam;
use std::borrow::Cow;

pub type Integer = i64;
pub type Real = f64;
pub type Text<'a> = Cow<'a, str>;
pub type Blob<'a> = Cow<'a, [u8]>;

// impl SQLParam for Integer {}
// impl SQLParam for Real {}
// impl SQLParam for Text<'_> {}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum SQLiteValue<'a> {
    Integer(Integer),
    Text(Text<'a>),
    Blob(Blob<'a>),
    Real(Real),
    Number(Number),
}

impl<'a> Default for SQLiteValue<'a> {
    fn default() -> Self {
        Self::Integer(Default::default())
    }
}

impl<'a> std::fmt::Display for SQLiteValue<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl<'a> From<&'a str> for SQLiteValue<'a> {
    fn from(value: &'a str) -> Self {
        Self::Text(Cow::Borrowed(value))
    }
}

impl<'a, T: AsRef<str> + 'a> From<&'a T> for SQLiteValue<'a> {
    fn from(value: &'a T) -> Self {
        Self::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl<'a> From<String> for SQLiteValue<'a> {
    fn from(value: String) -> Self {
        Self::Text(Cow::Owned(value))
    }
}

impl<'a> From<&'a [u8]> for SQLiteValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        Self::Blob(Cow::Borrowed(value))
    }
}

impl<'a> From<Vec<u8>> for SQLiteValue<'a> {
    fn from(value: Vec<u8>) -> Self {
        Self::Blob(Cow::Owned(value))
    }
}

impl<'a> From<f64> for SQLiteValue<'a> {
    fn from(value: f64) -> Self {
        Self::Real(value)
    }
}

impl<'a> From<i64> for SQLiteValue<'a> {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl<'a> SQLParam for SQLiteValue<'a> {}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Number {
    Integer(Integer),
    Real(Real),
}

impl Default for Number {
    fn default() -> Self {
        Self::Integer(Default::default())
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Real(value)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum SQLiteTableType {
    Table,
    View,
    Index,
    Trigger,
}

pub trait SQLiteTableSchema {
    const NAME: &'static str;
    const TYPE: SQLiteTableType;
    const SQL: &'static str;
}
