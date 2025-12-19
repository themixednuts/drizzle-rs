//! PostgreSQL Enum DDL types
//!
//! This module provides two complementary types:
//! - [`EnumDef`] - A const-friendly definition type for compile-time schema definitions
//! - [`Enum`] - A runtime type for serde serialization/deserialization

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly enum definition for compile-time schema definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct EnumDef {
    /// Schema name
    pub schema: &'static str,
    /// Enum name
    pub name: &'static str,
    /// Enum values
    pub values: &'static [Cow<'static, str>],
}

impl EnumDef {
    /// Create a new enum definition
    #[must_use]
    pub const fn new(
        schema: &'static str,
        name: &'static str,
        values: &'static [Cow<'static, str>],
    ) -> Self {
        Self {
            schema,
            name,
            values,
        }
    }

    /// Convert to runtime [`Enum`] type
    #[must_use]
    pub const fn into_enum(self) -> Enum<'static> {
        Enum {
            schema: Cow::Borrowed(self.schema),
            name: Cow::Borrowed(self.name),
            values: Cow::Borrowed(self.values),
        }
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime enum entity for serde serialization.
///
/// The lifetime parameter allows this type to work with both static (const) data
/// and owned (runtime) data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enum<'a> {
    /// Schema name
    pub schema: Cow<'a, str>,

    /// Enum name
    pub name: Cow<'a, str>,

    /// Enum values
    pub values: Cow<'a, [Cow<'a, str>]>,
}

impl<'a> Enum<'a> {
    /// Create a new enum (runtime)
    #[must_use]
    pub fn new(
        schema: impl Into<Cow<'a, str>>,
        name: impl Into<Cow<'a, str>>,
        values: impl Into<Cow<'a, [Cow<'a, str>]>>,
    ) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
            values: values.into(),
        }
    }

    /// Create a new enum from owned strings (convenience for runtime construction)
    #[cfg(feature = "std")]
    #[must_use]
    pub fn from_strings(schema: String, name: String, values: Vec<String>) -> Enum<'static> {
        Enum {
            schema: Cow::Owned(schema),
            name: Cow::Owned(name),
            values: Cow::Owned(values.into_iter().map(Cow::Owned).collect()),
        }
    }

    /// Get the schema name
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Get the enum name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Convert to a static lifetime version by converting to owned data
    #[cfg(feature = "std")]
    #[must_use]
    pub fn into_static(self) -> Enum<'static> {
        Enum {
            schema: Cow::Owned(self.schema.into_owned()),
            name: Cow::Owned(self.name.into_owned()),
            values: Cow::Owned(
                self.values
                    .into_owned()
                    .into_iter()
                    .map(|c| Cow::Owned(c.into_owned()))
                    .collect(),
            ),
        }
    }
}

impl Default for Enum<'static> {
    fn default() -> Self {
        Self::new("public", "", &[] as &[Cow<'static, str>])
    }
}

impl From<EnumDef> for Enum<'static> {
    fn from(def: EnumDef) -> Self {
        def.into_enum()
    }
}

// =============================================================================
// Serde Implementation
// =============================================================================

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for Enum<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("Enum", 3)?;
            state.serialize_field("schema", &*self.schema)?;
            state.serialize_field("name", &*self.name)?;
            // Serialize values as Vec<&str>
            let vals: Vec<&str> = self.values.iter().map(|v| v.as_ref()).collect();
            state.serialize_field("values", &vals)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for Enum<'static> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Helper {
                schema: String,
                name: String,
                #[serde(default)]
                values: Vec<String>,
            }

            let helper = Helper::deserialize(deserializer)?;
            Ok(Enum {
                schema: Cow::Owned(helper.schema),
                name: Cow::Owned(helper.name),
                values: Cow::Owned(helper.values.into_iter().map(Cow::Owned).collect()),
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_enum_def() {
        const VALUES: &[Cow<'static, str>] = &[
            Cow::Borrowed("active"),
            Cow::Borrowed("inactive"),
            Cow::Borrowed("pending"),
        ];
        const STATUS_ENUM: EnumDef = EnumDef::new("public", "status", VALUES);

        assert_eq!(STATUS_ENUM.schema, "public");
        assert_eq!(STATUS_ENUM.name, "status");
        assert_eq!(STATUS_ENUM.values.len(), 3);
    }

    #[test]
    fn test_enum_def_to_enum() {
        const VALUES: &[Cow<'static, str>] = &[Cow::Borrowed("active"), Cow::Borrowed("inactive")];
        const DEF: EnumDef = EnumDef::new("public", "status", VALUES);
        let enum_ty = DEF.into_enum();
        assert_eq!(enum_ty.schema(), "public");
        assert_eq!(enum_ty.name(), "status");
        assert_eq!(enum_ty.values.len(), 2);
    }

    #[test]
    fn test_from_strings() {
        let enum_ty = Enum::from_strings(
            "public".to_string(),
            "status".to_string(),
            vec!["active".to_string(), "inactive".to_string()],
        );
        assert_eq!(enum_ty.schema(), "public");
        assert_eq!(enum_ty.name(), "status");
        assert_eq!(enum_ty.values.len(), 2);
    }
}
