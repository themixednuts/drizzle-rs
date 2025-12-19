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

/// Deserialize Vec<String> into Cow<'static, [&'static str]>
///
/// Note: This converts to owned Vec<String> internally, then leaks to get &'static str.
/// For runtime deserialization, this is acceptable. For const conversions, use Cow::Borrowed
/// with a static slice directly.
#[cfg(feature = "serde")]
pub fn cow_vec_from_strings<'de, D>(
    deserializer: D,
) -> Result<Cow<'static, [&'static str]>, D::Error>
where
    D: Deserializer<'de>,
{
    let vec: Vec<String> = Vec::deserialize(deserializer)?;
    // Convert to owned Vec and leak to get &'static str
    // This is safe for deserialization as the strings will live for the program lifetime
    let leaked: Vec<&'static str> = vec
        .into_iter()
        .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
        .collect();
    Ok(Cow::Owned(leaked))
}

/// Deserialize Option<Vec<String>> into Option<Cow<'static, [&'static str]>>
#[cfg(feature = "serde")]
pub fn cow_option_vec_from_strings<'de, D>(
    deserializer: D,
) -> Result<Option<Cow<'static, [&'static str]>>, D::Error>
where
    D: Deserializer<'de>,
{
    let opt: Option<Vec<String>> = Option::deserialize(deserializer)?;
    Ok(opt.map(|vec| {
        let leaked: Vec<&'static str> = vec
            .into_iter()
            .map(|s| Box::leak(s.into_boxed_str()) as &'static str)
            .collect();
        Cow::Owned(leaked)
    }))
}

/// Deserialize Vec<IndexColumnDef> into Cow<'static, [IndexColumnDef]>
/// This is used for Index types that store columns as Cow<'static, [IndexColumnDef]>
#[cfg(feature = "serde")]
pub mod index_column_serde {
    use super::*;

    /// Owned version of OpclassDef for deserialization
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OwnedOpclass {
        name: String,
        #[serde(default)]
        default: bool,
    }

    /// Owned version of IndexColumnDef for deserialization
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OwnedIndexColumnDef {
        value: String,
        #[serde(default)]
        is_expression: bool,
        #[serde(default = "default_true")]
        asc: bool,
        #[serde(default)]
        nulls_first: bool,
        #[serde(default)]
        opclass: Option<OwnedOpclass>,
    }

    const fn default_true() -> bool {
        true
    }

    /// Postgres IndexColumnDef with all fields
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
    pub struct PostgresIndexColumnDef {
        pub value: &'static str,
        pub is_expression: bool,
        pub asc: bool,
        pub nulls_first: bool,
        pub opclass: Option<crate::postgres::ddl::OpclassDef>,
    }

    /// Deserialize Vec<IndexColumnDef> into Cow<'static, [PostgresIndexColumnDef]> for Postgres
    pub fn cow_index_columns_postgres<'de, D>(
        deserializer: D,
    ) -> Result<Cow<'static, [crate::postgres::ddl::IndexColumnDef]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<OwnedIndexColumnDef> = Vec::deserialize(deserializer)?;
        let leaked: Vec<crate::postgres::ddl::IndexColumnDef> = vec
            .into_iter()
            .map(|c| crate::postgres::ddl::IndexColumnDef {
                value: Box::leak(c.value.into_boxed_str()),
                is_expression: c.is_expression,
                asc: c.asc,
                nulls_first: c.nulls_first,
                opclass: c.opclass.map(|op| crate::postgres::ddl::OpclassDef {
                    name: Box::leak(op.name.into_boxed_str()),
                    default: op.default,
                }),
            })
            .collect();
        Ok(Cow::Owned(leaked))
    }

    /// Owned version of SQLite IndexColumnDef for deserialization
    #[derive(serde::Deserialize)]
    #[serde(rename_all = "camelCase")]
    struct OwnedSqliteIndexColumnDef {
        value: String,
        #[serde(default)]
        is_expression: bool,
    }

    /// Deserialize Vec<IndexColumnDef> into Cow<'static, [IndexColumnDef]> for SQLite
    pub fn cow_index_columns_sqlite<'de, D>(
        deserializer: D,
    ) -> Result<Cow<'static, [crate::sqlite::ddl::IndexColumnDef]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let vec: Vec<OwnedSqliteIndexColumnDef> = Vec::deserialize(deserializer)?;
        let leaked: Vec<crate::sqlite::ddl::IndexColumnDef> = vec
            .into_iter()
            .map(|c| crate::sqlite::ddl::IndexColumnDef {
                value: Box::leak(c.value.into_boxed_str()),
                is_expression: c.is_expression,
            })
            .collect();
        Ok(Cow::Owned(leaked))
    }
}
