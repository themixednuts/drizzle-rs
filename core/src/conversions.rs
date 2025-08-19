use crate::{
    sql::SQL,
    traits::{SQLColumnInfo, SQLParam, SQLTableInfo},
};
use std::borrow::Cow;

#[cfg(feature = "uuid")]
use uuid::Uuid;

pub trait ToSQL<'a, V: SQLParam> {
    fn to_sql(&self) -> SQL<'a, V>;
}

impl<'a, T, V> From<&T> for SQL<'a, V>
where
    T: ToSQL<'a, V>,
    V: SQLParam,
{
    fn from(value: &T) -> Self {
        value.to_sql()
    }
}

impl<'a, V: SQLParam, T> ToSQL<'a, V> for &T
where
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        (**self).to_sql()
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for () {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::empty()
    }
}

impl<'a, V, T> ToSQL<'a, V> for Vec<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), ", ")
    }
}

impl<'a, V, T> ToSQL<'a, V> for &'a [T]
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), ", ")
    }
}

impl<'a, V, T, const N: usize> ToSQL<'a, V> for [T; N]
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), ", ")
    }
}

// Implement ToSQL for SQLTableInfo and SQLColumnInfo trait objects
impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for &'static dyn SQLTableInfo {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::table(*self)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for &'static dyn SQLColumnInfo {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::column(*self)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Box<[&'static dyn SQLColumnInfo]> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(|&v| SQL::column(v)), ", ")
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Box<[&'static dyn SQLTableInfo]> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(|&v| SQL::table(v)), ", ")
    }
}

// Implement ToSQL for primitive types
impl<'a, V> ToSQL<'a, V> for &'a str
where
    V: SQLParam + 'a,
    V: From<&'a str>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(self))
    }
}

impl<'a, V> ToSQL<'a, V> for String
where
    V: SQLParam + 'a,
    V: From<String>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(self.clone()))
    }
}

impl<'a, V> ToSQL<'a, V> for i32
where
    V: SQLParam + 'a + From<i64>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self as i64))
    }
}

impl<'a, V> ToSQL<'a, V> for i64
where
    V: SQLParam + 'a + From<i64>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self))
    }
}

impl<'a, V> ToSQL<'a, V> for f64
where
    V: SQLParam + 'a + From<f64>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self))
    }
}

impl<'a, V> ToSQL<'a, V> for bool
where
    V: SQLParam + 'a + From<i64>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self as i64))
    }
}

impl<'a, V, T> ToSQL<'a, V> for Option<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match self {
            Some(value) => value.to_sql(), // Let the inner type handle parameterization
            None => SQL::raw("NULL"),      // NULL is a keyword, use raw
        }
    }
}

#[cfg(feature = "uuid")]
impl<'a, V> ToSQL<'a, V> for Uuid
where
    V: SQLParam + 'a,
    V: From<Uuid>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::parameter(V::from(*self))
    }
}
