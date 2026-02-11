//! ToSQL trait for converting types to SQL fragments.

use crate::prelude::*;
use crate::{
    sql::{Token, SQL},
    traits::{SQLColumnInfo, SQLParam, SQLTableInfo},
};

#[cfg(feature = "std")]
use std::{rc::Rc, sync::Arc};

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::{rc::Rc, sync::Arc};

#[cfg(feature = "uuid")]
use uuid::Uuid;

/// Trait for types that can be converted to SQL fragments.
///
/// The `'a` lifetime ties any borrowed parameter values to the resulting SQL
/// fragment, allowing zero-copy SQL construction when inputs are already
/// borrowed.
pub trait ToSQL<'a, V: SQLParam> {
    fn to_sql(&self) -> SQL<'a, V>;

    /// Consume self and return SQL without cloning.
    /// Default delegates to `to_sql()` (which clones). Types that own their SQL
    /// (like `SQL` and `SQLExpr`) override this to avoid the clone.
    fn into_sql(self) -> SQL<'a, V>
    where
        Self: Sized,
    {
        self.to_sql()
    }

    fn alias(&self, alias: &'static str) -> SQL<'a, V> {
        self.to_sql().alias(alias)
    }
}

/// Wrapper for byte slices to avoid list semantics (`Vec<u8>` normally becomes a list).
///
/// Use this when you want a single BLOB/bytea parameter:
/// ```ignore
/// use drizzle_core::{SQLBytes, SQL};
///
/// let data = vec![1u8, 2, 3];
/// let sql = SQL::bytes(&data); // or SQL::param(SQLBytes::new(&data))
/// ```
#[derive(Debug, Clone)]
pub struct SQLBytes<'a>(pub Cow<'a, [u8]>);

/// Explicit SQL NULL marker.
#[derive(Debug, Clone, Copy, Default)]
pub struct SQLNull;

impl<'a> SQLBytes<'a> {
    #[inline]
    pub fn new(bytes: impl Into<Cow<'a, [u8]>>) -> Self {
        Self(bytes.into())
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for SQLNull {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::raw("NULL")
    }
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
        SQL::join(self.iter().map(ToSQL::to_sql), Token::COMMA)
    }
}

impl<'a, V, T> ToSQL<'a, V> for &'a [T]
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), Token::COMMA)
    }
}

impl<'a, V, T, const N: usize> ToSQL<'a, V> for [T; N]
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), Token::COMMA)
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
        SQL::join(self.iter().map(|&v| SQL::column(v)), Token::COMMA)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Box<[&'static dyn SQLTableInfo]> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(|&v| SQL::table(v)), Token::COMMA)
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
        SQL::param(V::from(self))
    }
}

impl<'a, V> ToSQL<'a, V> for Box<str>
where
    V: SQLParam + 'a,
    V: From<String>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.to_string()))
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, V> ToSQL<'a, V> for Rc<str>
where
    V: SQLParam + 'a,
    V: From<String>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.as_ref().to_string()))
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, V> ToSQL<'a, V> for Arc<str>
where
    V: SQLParam + 'a,
    V: From<String>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.as_ref().to_string()))
    }
}

impl<'a, V, T> ToSQL<'a, V> for Box<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        (**self).to_sql()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, V, T> ToSQL<'a, V> for Rc<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        (**self).to_sql()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, V, T> ToSQL<'a, V> for Arc<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        (**self).to_sql()
    }
}

impl<'a, V> ToSQL<'a, V> for String
where
    V: SQLParam + 'a,
    V: From<String>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.clone()))
    }
}

impl<'a, V> ToSQL<'a, V> for Cow<'a, str>
where
    V: SQLParam + 'a,
    V: From<&'a str>,
    V: From<String>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match self {
            Cow::Borrowed(value) => SQL::param(V::from(*value)),
            Cow::Owned(value) => SQL::param(V::from(value.clone())),
        }
    }
}

impl<'a, V> ToSQL<'a, V> for Cow<'a, [u8]>
where
    V: SQLParam + 'a,
    V: From<&'a [u8]>,
    V: From<Vec<u8>>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match self {
            Cow::Borrowed(value) => SQL::param(V::from(*value)),
            Cow::Owned(value) => SQL::param(V::from(value.clone())),
        }
    }
}

impl<'a, V> ToSQL<'a, V> for SQLBytes<'a>
where
    V: SQLParam + 'a,
    V: From<&'a [u8]>,
    V: From<Vec<u8>>,
    V: Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match &self.0 {
            Cow::Borrowed(value) => SQL::param(V::from(*value)),
            Cow::Owned(value) => SQL::param(V::from(value.clone())),
        }
    }
}

macro_rules! impl_tosql_param_copy {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl<'a, V> ToSQL<'a, V> for $ty
            where
                V: SQLParam + 'a + From<$ty>,
                V: Into<Cow<'a, V>>,
            {
                fn to_sql(&self) -> SQL<'a, V> {
                    SQL::param(V::from(*self))
                }
            }
        )+
    };
}

impl_tosql_param_copy!(i8, i16, i32, i64, f32, f64, bool, u8, u16, u32, u64, isize, usize);

impl<'a, V, T> ToSQL<'a, V> for Option<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match self {
            Some(value) => value.to_sql(),
            None => SQLNull.to_sql(),
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
        SQL::param(V::from(*self))
    }
}
