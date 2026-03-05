//! Serde helpers for Cow<'static, str> deserialization
//!
//! These helpers allow DDL types to use `Cow<'static, str>` while still
//! being deserializable from JSON (where strings become `Cow::Owned`).

#[allow(unused_imports)]
use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Deserializer};

/// Deserialize a String into Cow<'static, str>
#[cfg(feature = "serde")]
pub fn cow_from_string<'de, D>(deserializer: D) -> Result<Cow<'static, str>, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Ok(Cow::Owned(s))
}

/// Deserialize an Option<String> into Option<Cow<'static, str>>
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

/// Deserialize `Vec<String>` into `Vec<Cow<'static, str>>`.
#[cfg(feature = "serde")]
pub fn cow_vec_from_strings<'de, D>(deserializer: D) -> Result<Vec<Cow<'static, str>>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec: Vec<String> = Vec::deserialize(deserializer)?;
    Ok(vec.into_iter().map(Cow::Owned).collect())
}

/// Deserialize `Option<Vec<String>>` into `Option<Vec<Cow<'static, str>>>`.
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
