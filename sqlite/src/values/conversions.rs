//! From and TryFrom implementations for SQLiteValue

use super::{OwnedSQLiteValue, SQLiteValue};
use drizzle_core::{error::DrizzleError, sql::SQL, traits::ToSQL};
use std::{borrow::Cow, rc::Rc, sync::Arc};

#[cfg(feature = "uuid")]
use uuid::Uuid;

//------------------------------------------------------------------------------
// ToSQL Implementation
//------------------------------------------------------------------------------

impl<'a> ToSQL<'a, SQLiteValue<'a>> for SQLiteValue<'a> {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        SQL::param(self.clone())
    }
}

//------------------------------------------------------------------------------
// From OwnedSQLiteValue
//------------------------------------------------------------------------------

impl<'a> From<OwnedSQLiteValue> for SQLiteValue<'a> {
    fn from(value: OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(f) => SQLiteValue::Integer(f),
            OwnedSQLiteValue::Real(r) => SQLiteValue::Real(r),
            OwnedSQLiteValue::Text(v) => SQLiteValue::Text(Cow::Owned(v)),
            OwnedSQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Owned(v.into())),
            OwnedSQLiteValue::Null => SQLiteValue::Null,
        }
    }
}

impl<'a> From<&'a OwnedSQLiteValue> for SQLiteValue<'a> {
    fn from(value: &'a OwnedSQLiteValue) -> Self {
        match value {
            OwnedSQLiteValue::Integer(f) => SQLiteValue::Integer(*f),
            OwnedSQLiteValue::Real(r) => SQLiteValue::Real(*r),
            OwnedSQLiteValue::Text(v) => SQLiteValue::Text(Cow::Borrowed(v)),
            OwnedSQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Borrowed(v)),
            OwnedSQLiteValue::Null => SQLiteValue::Null,
        }
    }
}

impl<'a> From<&'a SQLiteValue<'a>> for SQLiteValue<'a> {
    fn from(value: &'a SQLiteValue<'a>) -> Self {
        match value {
            SQLiteValue::Integer(f) => SQLiteValue::Integer(*f),
            SQLiteValue::Real(r) => SQLiteValue::Real(*r),
            SQLiteValue::Text(v) => SQLiteValue::Text(Cow::Borrowed(v)),
            SQLiteValue::Blob(v) => SQLiteValue::Blob(Cow::Borrowed(v)),
            SQLiteValue::Null => SQLiteValue::Null,
        }
    }
}

impl<'a> From<Cow<'a, SQLiteValue<'a>>> for SQLiteValue<'a> {
    fn from(value: Cow<'a, SQLiteValue<'a>>) -> Self {
        match value {
            Cow::Borrowed(r) => r.into(),
            Cow::Owned(o) => o,
        }
    }
}

impl<'a> From<&'a Cow<'a, SQLiteValue<'a>>> for SQLiteValue<'a> {
    fn from(value: &'a Cow<'a, SQLiteValue<'a>>) -> Self {
        match value {
            Cow::Borrowed(r) => (*r).into(),
            Cow::Owned(o) => o.into(),
        }
    }
}

//------------------------------------------------------------------------------
// From<T> implementations
// Macro-based to reduce boilerplate
//------------------------------------------------------------------------------

/// Macro to implement From<integer> for SQLiteValue (converts to INTEGER)
macro_rules! impl_from_int_for_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> From<$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: $ty) -> Self {
                    SQLiteValue::Integer(value as i64)
                }
            }

            impl<'a> From<&$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: &$ty) -> Self {
                    SQLiteValue::Integer(*value as i64)
                }
            }
        )*
    };
}

/// Macro to implement From<float> for SQLiteValue (converts to REAL)
macro_rules! impl_from_float_for_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> From<$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: $ty) -> Self {
                    SQLiteValue::Real(value as f64)
                }
            }

            impl<'a> From<&$ty> for SQLiteValue<'a> {
                #[inline]
                fn from(value: &$ty) -> Self {
                    SQLiteValue::Real(*value as f64)
                }
            }
        )*
    };
}

// Integer types -> SQLiteValue::Integer
impl_from_int_for_sqlite_value!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool);

// Float types -> SQLiteValue::Real
impl_from_float_for_sqlite_value!(f32, f64);

// --- String Types ---

impl<'a> From<&'a str> for SQLiteValue<'a> {
    fn from(value: &'a str) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, str>> for SQLiteValue<'a> {
    fn from(value: Cow<'a, str>) -> Self {
        SQLiteValue::Text(value)
    }
}

impl<'a> From<String> for SQLiteValue<'a> {
    fn from(value: String) -> Self {
        SQLiteValue::Text(Cow::Owned(value))
    }
}

impl<'a> From<&'a String> for SQLiteValue<'a> {
    fn from(value: &'a String) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value))
    }
}

impl<'a> From<Box<String>> for SQLiteValue<'a> {
    fn from(value: Box<String>) -> Self {
        SQLiteValue::Text(Cow::Owned(*value))
    }
}

impl<'a> From<&'a Box<String>> for SQLiteValue<'a> {
    fn from(value: &'a Box<String>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl<'a> From<Rc<String>> for SQLiteValue<'a> {
    fn from(value: Rc<String>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Rc<String>> for SQLiteValue<'a> {
    fn from(value: &'a Rc<String>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl<'a> From<Arc<String>> for SQLiteValue<'a> {
    fn from(value: Arc<String>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Arc<String>> for SQLiteValue<'a> {
    fn from(value: &'a Arc<String>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_str()))
    }
}

impl<'a> From<Box<str>> for SQLiteValue<'a> {
    fn from(value: Box<str>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.into()))
    }
}

impl<'a> From<&'a Box<str>> for SQLiteValue<'a> {
    fn from(value: &'a Box<str>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl<'a> From<Rc<str>> for SQLiteValue<'a> {
    fn from(value: Rc<str>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_ref().to_string()))
    }
}

impl<'a> From<&'a Rc<str>> for SQLiteValue<'a> {
    fn from(value: &'a Rc<str>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

impl<'a> From<Arc<str>> for SQLiteValue<'a> {
    fn from(value: Arc<str>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_ref().to_string()))
    }
}

impl<'a> From<&'a Arc<str>> for SQLiteValue<'a> {
    fn from(value: &'a Arc<str>) -> Self {
        SQLiteValue::Text(Cow::Borrowed(value.as_ref()))
    }
}

// --- ArrayString ---

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<arrayvec::ArrayString<N>> for SQLiteValue<'a> {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.to_string()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<&arrayvec::ArrayString<N>> for SQLiteValue<'a> {
    fn from(value: &arrayvec::ArrayString<N>) -> Self {
        SQLiteValue::Text(Cow::Owned(value.as_str().to_owned()))
    }
}

// --- Binary Data ---

impl<'a> From<&'a [u8]> for SQLiteValue<'a> {
    fn from(value: &'a [u8]) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value))
    }
}

impl<'a> From<Cow<'a, [u8]>> for SQLiteValue<'a> {
    fn from(value: Cow<'a, [u8]>) -> Self {
        SQLiteValue::Blob(value)
    }
}

impl<'a> From<Vec<u8>> for SQLiteValue<'a> {
    fn from(value: Vec<u8>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value))
    }
}

impl<'a> From<Box<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: Box<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Owned(*value))
    }
}

impl<'a> From<&'a Box<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: &'a Box<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_slice()))
    }
}

impl<'a> From<Rc<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: Rc<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Rc<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: &'a Rc<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_slice()))
    }
}

impl<'a> From<Arc<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: Arc<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.as_ref().clone()))
    }
}

impl<'a> From<&'a Arc<Vec<u8>>> for SQLiteValue<'a> {
    fn from(value: &'a Arc<Vec<u8>>) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_slice()))
    }
}

// --- ArrayVec<u8, N> ---

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<arrayvec::ArrayVec<u8, N>> for SQLiteValue<'a> {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, const N: usize> From<&arrayvec::ArrayVec<u8, N>> for SQLiteValue<'a> {
    fn from(value: &arrayvec::ArrayVec<u8, N>) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.to_vec()))
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl<'a> From<Uuid> for SQLiteValue<'a> {
    fn from(value: Uuid) -> Self {
        SQLiteValue::Blob(Cow::Owned(value.as_bytes().to_vec()))
    }
}

#[cfg(feature = "uuid")]
impl<'a> From<&'a Uuid> for SQLiteValue<'a> {
    fn from(value: &'a Uuid) -> Self {
        SQLiteValue::Blob(Cow::Borrowed(value.as_bytes()))
    }
}

// --- Option Types ---
impl<'a, T> From<Option<T>> for SQLiteValue<'a>
where
    T: TryInto<SQLiteValue<'a>>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.try_into().unwrap_or(SQLiteValue::Null),
            None => SQLiteValue::Null,
        }
    }
}

// --- Cow integration for SQL struct ---
impl<'a> From<SQLiteValue<'a>> for Cow<'a, SQLiteValue<'a>> {
    fn from(value: SQLiteValue<'a>) -> Self {
        Cow::Owned(value)
    }
}

impl<'a> From<&'a SQLiteValue<'a>> for Cow<'a, SQLiteValue<'a>> {
    fn from(value: &'a SQLiteValue<'a>) -> Self {
        Cow::Borrowed(value)
    }
}

//------------------------------------------------------------------------------
// TryFrom<SQLiteValue> implementations
// Uses the FromSQLiteValue trait via convert() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement TryFrom<SQLiteValue> for types implementing FromSQLiteValue
macro_rules! impl_try_from_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> TryFrom<SQLiteValue<'a>> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: SQLiteValue<'a>) -> Result<Self, Self::Error> {
                    value.convert()
                }
            }
        )*
    };
}

impl_try_from_sqlite_value!(
    i8,
    i16,
    i32,
    i64,
    isize,
    u8,
    u16,
    u32,
    u64,
    usize,
    f32,
    f64,
    bool,
    String,
    Box<String>,
    Rc<String>,
    Arc<String>,
    Box<str>,
    Rc<str>,
    Arc<str>,
    Box<Vec<u8>>,
    Rc<Vec<u8>>,
    Arc<Vec<u8>>,
    Vec<u8>,
);

#[cfg(feature = "uuid")]
impl_try_from_sqlite_value!(Uuid);

//------------------------------------------------------------------------------
// TryFrom<&SQLiteValue> implementations for borrowing without consuming
// Uses the FromSQLiteValue trait via convert_ref() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement TryFrom<&SQLiteValue> for types implementing FromSQLiteValue
macro_rules! impl_try_from_sqlite_value_ref {
    ($($ty:ty),* $(,)?) => {
        $(
            impl<'a> TryFrom<&SQLiteValue<'a>> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: &SQLiteValue<'a>) -> Result<Self, Self::Error> {
                    value.convert_ref()
                }
            }
        )*
    };
}

impl_try_from_sqlite_value_ref!(
    i8,
    i16,
    i32,
    i64,
    isize,
    u8,
    u16,
    u32,
    u64,
    usize,
    f32,
    f64,
    bool,
    String,
    Box<String>,
    Rc<String>,
    Arc<String>,
    Box<str>,
    Rc<str>,
    Arc<str>,
    Box<Vec<u8>>,
    Rc<Vec<u8>>,
    Arc<Vec<u8>>,
    Vec<u8>,
);

#[cfg(feature = "uuid")]
impl_try_from_sqlite_value_ref!(Uuid);

// --- Borrowed reference types (cannot use FromSQLiteValue) ---

impl<'a> TryFrom<&'a SQLiteValue<'a>> for &'a str {
    type Error = DrizzleError;

    fn try_from(value: &'a SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Text(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to &str", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<&'a SQLiteValue<'a>> for &'a [u8] {
    type Error = DrizzleError;

    fn try_from(value: &'a SQLiteValue<'a>) -> Result<Self, Self::Error> {
        match value {
            SQLiteValue::Blob(cow) => Ok(cow.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to &[u8]", value).into(),
            )),
        }
    }
}
