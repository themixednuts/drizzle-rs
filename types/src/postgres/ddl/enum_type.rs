//! `PostgreSQL` enums: [`EnumDef`] (const) and [`Enum`] (runtime).

use crate::alloc_prelude::*;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// An enum definition that can be built in a `const`.
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
    /// Creates an enum definition.
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

    /// Converts to the runtime [`Enum`].
    #[must_use]
    pub const fn into_enum(self) -> Enum {
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

/// An enum, as stored in migration snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Enum {
    /// Schema name
    pub schema: Cow<'static, str>,

    /// Enum name
    pub name: Cow<'static, str>,

    /// Enum values
    pub values: Cow<'static, [Cow<'static, str>]>,
}

impl Enum {
    /// Creates an enum.
    #[must_use]
    pub fn new(
        schema: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        values: impl Into<Cow<'static, [Cow<'static, str>]>>,
    ) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
            values: values.into(),
        }
    }

    /// Creates an enum from owned strings.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn from_strings(schema: String, name: String, values: Vec<String>) -> Self {
        Self {
            schema: Cow::Owned(schema),
            name: Cow::Owned(name),
            values: Cow::Owned(values.into_iter().map(Cow::Owned).collect()),
        }
    }

    /// Returns the schema name.
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Returns the enum name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for Enum {
    fn default() -> Self {
        Self::new("public", "", &[] as &[Cow<'static, str>])
    }
}

impl From<EnumDef> for Enum {
    fn from(def: EnumDef) -> Self {
        def.into_enum()
    }
}

// =============================================================================
// Serde Implementation
// =============================================================================

#[cfg(feature = "serde")]
mod serde_impl {
    use super::{Cow, Enum, String, Vec};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for Enum {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("Enum", 3)?;
            state.serialize_field("schema", &*self.schema)?;
            state.serialize_field("name", &*self.name)?;
            // Serialize values as Vec<&str>
            let vals: Vec<&str> = self.values.iter().map(AsRef::as_ref).collect();
            state.serialize_field("values", &vals)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for Enum {
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
            Ok(Self {
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
