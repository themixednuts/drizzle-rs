//! From<T> and TryFrom<OwnedSQLiteValue> implementations

use super::OwnedSQLiteValue;
use crate::prelude::*;
use drizzle_core::error::DrizzleError;

#[cfg(feature = "uuid")]
use uuid::Uuid;

//------------------------------------------------------------------------------
// From<T> implementations
// Macro-based to reduce boilerplate
//------------------------------------------------------------------------------

/// Macro to implement From<integer> for OwnedSQLiteValue (converts to INTEGER)
macro_rules! impl_from_int_for_owned_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for OwnedSQLiteValue {
                #[inline]
                fn from(value: $ty) -> Self {
                    OwnedSQLiteValue::Integer(value as i64)
                }
            }

            impl From<&$ty> for OwnedSQLiteValue {
                #[inline]
                fn from(value: &$ty) -> Self {
                    OwnedSQLiteValue::Integer(*value as i64)
                }
            }
        )*
    };
}

/// Macro to implement From<float> for OwnedSQLiteValue (converts to REAL)
macro_rules! impl_from_float_for_owned_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl From<$ty> for OwnedSQLiteValue {
                #[inline]
                fn from(value: $ty) -> Self {
                    OwnedSQLiteValue::Real(value as f64)
                }
            }

            impl From<&$ty> for OwnedSQLiteValue {
                #[inline]
                fn from(value: &$ty) -> Self {
                    OwnedSQLiteValue::Real(*value as f64)
                }
            }
        )*
    };
}

// Integer types -> OwnedSQLiteValue::Integer
impl_from_int_for_owned_sqlite_value!(i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool);

// Float types -> OwnedSQLiteValue::Real
impl_from_float_for_owned_sqlite_value!(f32, f64);

// --- String Types ---

impl From<&str> for OwnedSQLiteValue {
    fn from(value: &str) -> Self {
        OwnedSQLiteValue::Text(value.to_string())
    }
}

impl From<Cow<'_, str>> for OwnedSQLiteValue {
    fn from(value: Cow<'_, str>) -> Self {
        OwnedSQLiteValue::Text(value.into_owned())
    }
}

impl From<String> for OwnedSQLiteValue {
    fn from(value: String) -> Self {
        OwnedSQLiteValue::Text(value)
    }
}

impl From<&String> for OwnedSQLiteValue {
    fn from(value: &String) -> Self {
        OwnedSQLiteValue::Text(value.clone())
    }
}

impl From<Box<String>> for OwnedSQLiteValue {
    fn from(value: Box<String>) -> Self {
        OwnedSQLiteValue::Text(*value)
    }
}

impl From<&Box<String>> for OwnedSQLiteValue {
    fn from(value: &Box<String>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().clone())
    }
}

impl From<Rc<String>> for OwnedSQLiteValue {
    fn from(value: Rc<String>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().clone())
    }
}

impl From<&Rc<String>> for OwnedSQLiteValue {
    fn from(value: &Rc<String>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().clone())
    }
}

impl From<Arc<String>> for OwnedSQLiteValue {
    fn from(value: Arc<String>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().clone())
    }
}

impl From<&Arc<String>> for OwnedSQLiteValue {
    fn from(value: &Arc<String>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().clone())
    }
}

impl From<Box<str>> for OwnedSQLiteValue {
    fn from(value: Box<str>) -> Self {
        OwnedSQLiteValue::Text(value.into())
    }
}

impl From<&Box<str>> for OwnedSQLiteValue {
    fn from(value: &Box<str>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().to_string())
    }
}

impl From<Rc<str>> for OwnedSQLiteValue {
    fn from(value: Rc<str>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().to_string())
    }
}

impl From<&Rc<str>> for OwnedSQLiteValue {
    fn from(value: &Rc<str>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().to_string())
    }
}

impl From<Arc<str>> for OwnedSQLiteValue {
    fn from(value: Arc<str>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().to_string())
    }
}

impl From<&Arc<str>> for OwnedSQLiteValue {
    fn from(value: &Arc<str>) -> Self {
        OwnedSQLiteValue::Text(value.as_ref().to_string())
    }
}

// --- Binary Data ---

impl From<&[u8]> for OwnedSQLiteValue {
    fn from(value: &[u8]) -> Self {
        OwnedSQLiteValue::Blob(value.to_vec().into_boxed_slice())
    }
}

impl From<Cow<'_, [u8]>> for OwnedSQLiteValue {
    fn from(value: Cow<'_, [u8]>) -> Self {
        OwnedSQLiteValue::Blob(value.into_owned().into_boxed_slice())
    }
}

impl From<Vec<u8>> for OwnedSQLiteValue {
    fn from(value: Vec<u8>) -> Self {
        OwnedSQLiteValue::Blob(value.into_boxed_slice())
    }
}

impl From<Box<Vec<u8>>> for OwnedSQLiteValue {
    fn from(value: Box<Vec<u8>>) -> Self {
        OwnedSQLiteValue::Blob(value.into_boxed_slice())
    }
}

impl From<&Box<Vec<u8>>> for OwnedSQLiteValue {
    fn from(value: &Box<Vec<u8>>) -> Self {
        OwnedSQLiteValue::Blob(value.as_slice().to_vec().into_boxed_slice())
    }
}

impl From<Rc<Vec<u8>>> for OwnedSQLiteValue {
    fn from(value: Rc<Vec<u8>>) -> Self {
        OwnedSQLiteValue::Blob(value.as_slice().to_vec().into_boxed_slice())
    }
}

impl From<&Rc<Vec<u8>>> for OwnedSQLiteValue {
    fn from(value: &Rc<Vec<u8>>) -> Self {
        OwnedSQLiteValue::Blob(value.as_slice().to_vec().into_boxed_slice())
    }
}

impl From<Arc<Vec<u8>>> for OwnedSQLiteValue {
    fn from(value: Arc<Vec<u8>>) -> Self {
        OwnedSQLiteValue::Blob(value.as_slice().to_vec().into_boxed_slice())
    }
}

impl From<&Arc<Vec<u8>>> for OwnedSQLiteValue {
    fn from(value: &Arc<Vec<u8>>) -> Self {
        OwnedSQLiteValue::Blob(value.as_slice().to_vec().into_boxed_slice())
    }
}

// --- UUID ---

#[cfg(feature = "uuid")]
impl From<Uuid> for OwnedSQLiteValue {
    fn from(value: Uuid) -> Self {
        OwnedSQLiteValue::Blob(value.as_bytes().to_vec().into_boxed_slice())
    }
}

#[cfg(feature = "uuid")]
impl From<&Uuid> for OwnedSQLiteValue {
    fn from(value: &Uuid) -> Self {
        OwnedSQLiteValue::Blob(value.as_bytes().to_vec().into_boxed_slice())
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayString<N>> for OwnedSQLiteValue {
    fn from(value: arrayvec::ArrayString<N>) -> Self {
        OwnedSQLiteValue::Text(value.to_string())
    }
}

impl From<compact_str::CompactString> for OwnedSQLiteValue {
    fn from(value: compact_str::CompactString) -> Self {
        OwnedSQLiteValue::Text(value.to_string())
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayVec<u8, N>> for OwnedSQLiteValue {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        OwnedSQLiteValue::Blob(value.to_vec().into_boxed_slice())
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::Bytes> for OwnedSQLiteValue {
    fn from(value: bytes::Bytes) -> Self {
        OwnedSQLiteValue::Blob(value.to_vec().into_boxed_slice())
    }
}

#[cfg(feature = "bytes")]
impl From<bytes::BytesMut> for OwnedSQLiteValue {
    fn from(value: bytes::BytesMut) -> Self {
        OwnedSQLiteValue::Blob(value.to_vec().into_boxed_slice())
    }
}

#[cfg(feature = "smallvec")]
impl<const N: usize> From<smallvec::SmallVec<[u8; N]>> for OwnedSQLiteValue {
    fn from(value: smallvec::SmallVec<[u8; N]>) -> Self {
        OwnedSQLiteValue::Blob(value.into_vec().into_boxed_slice())
    }
}

// --- Option Types ---
impl<T> From<Option<T>> for OwnedSQLiteValue
where
    T: TryInto<OwnedSQLiteValue>,
{
    fn from(value: Option<T>) -> Self {
        match value {
            Some(value) => value.try_into().unwrap_or(OwnedSQLiteValue::Null),
            None => OwnedSQLiteValue::Null,
        }
    }
}

//------------------------------------------------------------------------------
// TryFrom<OwnedSQLiteValue> implementations
// Uses the FromSQLiteValue trait via convert() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement TryFrom<OwnedSQLiteValue> for types implementing FromSQLiteValue
macro_rules! impl_try_from_owned_sqlite_value {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TryFrom<OwnedSQLiteValue> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
                    value.convert()
                }
            }
        )*
    };
}

impl_try_from_owned_sqlite_value!(
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
    compact_str::CompactString,
);

#[cfg(feature = "uuid")]
impl_try_from_owned_sqlite_value!(Uuid);

#[cfg(feature = "bytes")]
impl_try_from_owned_sqlite_value!(bytes::Bytes, bytes::BytesMut);

#[cfg(feature = "smallvec")]
impl<const N: usize> TryFrom<OwnedSQLiteValue> for smallvec::SmallVec<[u8; N]> {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        value.convert()
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> TryFrom<OwnedSQLiteValue> for arrayvec::ArrayString<N> {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Text(s) => arrayvec::ArrayString::from(&s).map_err(|_| {
                DrizzleError::ConversionError(
                    format!("Text length {} exceeds ArrayString capacity {}", s.len(), N).into(),
                )
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to ArrayString", value).into(),
            )),
        }
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> TryFrom<OwnedSQLiteValue> for arrayvec::ArrayVec<u8, N> {
    type Error = DrizzleError;

    fn try_from(value: OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(bytes) => {
                arrayvec::ArrayVec::try_from(bytes.as_ref()).map_err(|_| {
                    DrizzleError::ConversionError(
                        format!(
                            "Blob length {} exceeds ArrayVec capacity {}",
                            bytes.len(),
                            N
                        )
                        .into(),
                    )
                })
            }
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to ArrayVec<u8>", value).into(),
            )),
        }
    }
}

//------------------------------------------------------------------------------
// TryFrom<&OwnedSQLiteValue> implementations for borrowing without consuming
// Uses the FromSQLiteValue trait via convert_ref() for unified conversion logic
//------------------------------------------------------------------------------

/// Macro to implement TryFrom<&OwnedSQLiteValue> for types implementing FromSQLiteValue
macro_rules! impl_try_from_owned_sqlite_value_ref {
    ($($ty:ty),* $(,)?) => {
        $(
            impl TryFrom<&OwnedSQLiteValue> for $ty {
                type Error = DrizzleError;

                #[inline]
                fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
                    value.convert_ref()
                }
            }
        )*
    };
}

impl_try_from_owned_sqlite_value_ref!(
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
    compact_str::CompactString,
);

#[cfg(feature = "uuid")]
impl_try_from_owned_sqlite_value_ref!(Uuid);

#[cfg(feature = "bytes")]
impl_try_from_owned_sqlite_value_ref!(bytes::Bytes, bytes::BytesMut);

#[cfg(feature = "smallvec")]
impl<const N: usize> TryFrom<&OwnedSQLiteValue> for smallvec::SmallVec<[u8; N]> {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        value.convert_ref()
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> TryFrom<&OwnedSQLiteValue> for arrayvec::ArrayString<N> {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Text(s) => arrayvec::ArrayString::from(s.as_str()).map_err(|_| {
                DrizzleError::ConversionError(
                    format!("Text length {} exceeds ArrayString capacity {}", s.len(), N).into(),
                )
            }),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to ArrayString", value).into(),
            )),
        }
    }
}

#[cfg(feature = "arrayvec")]
impl<const N: usize> TryFrom<&OwnedSQLiteValue> for arrayvec::ArrayVec<u8, N> {
    type Error = DrizzleError;

    fn try_from(value: &OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(bytes) => {
                arrayvec::ArrayVec::try_from(bytes.as_ref()).map_err(|_| {
                    DrizzleError::ConversionError(
                        format!(
                            "Blob length {} exceeds ArrayVec capacity {}",
                            bytes.len(),
                            N
                        )
                        .into(),
                    )
                })
            }
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to ArrayVec<u8>", value).into(),
            )),
        }
    }
}

// --- Borrowed reference types (cannot use FromSQLiteValue) ---

impl<'a> TryFrom<&'a OwnedSQLiteValue> for &'a str {
    type Error = DrizzleError;

    fn try_from(value: &'a OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Text(s) => Ok(s.as_str()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to &str", value).into(),
            )),
        }
    }
}

impl<'a> TryFrom<&'a OwnedSQLiteValue> for &'a [u8] {
    type Error = DrizzleError;

    fn try_from(value: &'a OwnedSQLiteValue) -> Result<Self, Self::Error> {
        match value {
            OwnedSQLiteValue::Blob(b) => Ok(b.as_ref()),
            _ => Err(DrizzleError::ConversionError(
                format!("Cannot convert {:?} to &[u8]", value).into(),
            )),
        }
    }
}
