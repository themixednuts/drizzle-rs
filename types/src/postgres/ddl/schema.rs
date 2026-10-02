//! `PostgreSQL` schemas: [`SchemaDef`] (const) and [`Schema`] (runtime).

use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use crate::serde_helpers::cow_from_string;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// A schema definition that can be built in a `const`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SchemaDef {
    /// Schema name
    pub name: &'static str,
}

impl SchemaDef {
    /// Creates a schema definition.
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self { name }
    }

    /// Converts to the runtime [`Schema`].
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

/// A schema, as stored in migration snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Schema {
    /// Schema name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,
}

impl Schema {
    /// Creates a schema.
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self { name: name.into() }
    }

    /// Returns the schema name.
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
