//! From<T> and TryFrom<OwnedSQLiteValue> implementations

use super::OwnedSQLiteValue;
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

// --- Binary Data ---

impl From<&[u8]> for OwnedSQLiteValue {
    fn from(value: &[u8]) -> Self {
        OwnedSQLiteValue::Blob(value.to_vec().into_boxed_slice())
    }
}

impl From<Vec<u8>> for OwnedSQLiteValue {
    fn from(value: Vec<u8>) -> Self {
        OwnedSQLiteValue::Blob(value.into_boxed_slice())
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

#[cfg(feature = "arrayvec")]
impl<const N: usize> From<arrayvec::ArrayVec<u8, N>> for OwnedSQLiteValue {
    fn from(value: arrayvec::ArrayVec<u8, N>) -> Self {
        OwnedSQLiteValue::Blob(value.to_vec().into_boxed_slice())
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
    Vec<u8>,
);

#[cfg(feature = "uuid")]
impl_try_from_owned_sqlite_value!(Uuid);

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
    Vec<u8>,
);

#[cfg(feature = "uuid")]
impl_try_from_owned_sqlite_value_ref!(Uuid);

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
