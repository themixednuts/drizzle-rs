//! PostgreSQL Schema DDL types
//!
//! This module provides two complementary types:
//! - [`SchemaDef`] - A const-friendly definition type for compile-time schema definitions
//! - [`Schema`] - A runtime type for serde serialization/deserialization

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::cow_from_string;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly schema definition for compile-time schema definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SchemaDef {
    /// Schema name
    pub name: &'static str,
}

impl SchemaDef {
    /// Create a new schema definition
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// Convert to runtime [`Schema`] type
    #[must_use]
    pub const fn into_schema(self) -> Schema {
        Schema {
            name: Cow::Borrowed(self.name),
        }
    }
}

impl Default for SchemaDef {
    fn default() -> Self {
        Self::new("public")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime schema entity for serde serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Schema {
    /// Schema name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
}

impl Schema {
    /// Create a new schema (runtime)
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self { name: name.into() }
    }

    /// Get the schema name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new("public")
    }
}

impl From<SchemaDef> for Schema {
    fn from(def: SchemaDef) -> Self {
        def.into_schema()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_schema_def() {
        const SCHEMA: SchemaDef = SchemaDef::new("custom_schema");

        assert_eq!(SCHEMA.name, "custom_schema");
    }

    #[test]
    fn test_schema_def_to_schema() {
        const DEF: SchemaDef = SchemaDef::new("public");
        let schema = DEF.into_schema();
        assert_eq!(schema.name(), "public");
    }
}
