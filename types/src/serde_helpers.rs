//! `#[serde(deserialize_with = ...)]` helpers for `Cow<'static, str>` fields.
//!
//! DDL types store names as `Cow<'static, str>` so they can be `const`.
//! These functions deserialize owned strings into `Cow::Owned`.

#[allow(unused_imports)]
use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer};

/// Deserializes a `String` into a `Cow<'static, str>`.
///
/// # Errors
///
/// Returns an error if the deserializer fails to produce a `String`.
#[cfg(feature = "serde")]
pub fn cow_from_string<'de, D>(deserializer: D) -> Result<Cow<'static, str>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(Cow::Owned(s))
}

/// Deserializes an `Option<String>` into an `Option<Cow<'static, str>>`.
///
/// # Errors
///
/// Returns an error if the deserializer fails to produce an `Option<String>`.
#[cfg(feature = "serde")]
pub fn cow_option_from_string<'de, D>(
    deserializer: D,
) -> Result<Option<Cow<'static, str>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt = Option::<String>::deserialize(deserializer)?;
    Ok(opt.map(Cow::Owned))
}

/// Deserializes a `Vec<String>` into a `Vec<Cow<'static, str>>`.
///
/// # Errors
///
/// Returns an error if the deserializer fails to produce a `Vec<String>`.
#[cfg(feature = "serde")]
pub fn cow_vec_from_strings<'de, D>(deserializer: D) -> Result<Vec<Cow<'static, str>>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec: Vec<String> = Vec::deserialize(deserializer)?;
    Ok(vec.into_iter().map(Cow::Owned).collect())
}

/// Deserializes an `Option<Vec<String>>` into an `Option<Vec<Cow<'static, str>>>`.
///
/// # Errors
///
/// Returns an error if the deserializer fails to produce an `Option<Vec<String>>`.
#[cfg(feature = "serde")]
pub fn cow_option_vec_from_strings<'de, D>(
    deserializer: D,
) -> Result<Option<Vec<Cow<'static, str>>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<Vec<String>> = Option::deserialize(deserializer)?;
    Ok(opt.map(|vec| vec.into_iter().map(Cow::Owned).collect()))
}
