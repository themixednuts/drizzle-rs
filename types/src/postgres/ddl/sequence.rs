//! PostgreSQL Sequence DDL types
//!
//! See: <https://github.com/drizzle-team/drizzle-orm/blob/beta/drizzle-kit/src/dialects/postgres/ddl.ts>

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly sequence definition for compile-time schema definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SequenceDef {
    /// Schema name
    pub schema: &'static str,
    /// Sequence name
    pub name: &'static str,
    /// Increment value (as string for const compatibility)
    pub increment_by: Option<&'static str>,
    /// Minimum value (as string)
    pub min_value: Option<&'static str>,
    /// Maximum value (as string)
    pub max_value: Option<&'static str>,
    /// Start value (as string)
    pub start_with: Option<&'static str>,
    /// Cache size
    pub cache_size: Option<i32>,
    /// Cycle flag
    pub cycle: Option<bool>,
}

impl SequenceDef {
    /// Create a new sequence definition
    #[must_use]
    pub const fn new(schema: &'static str, name: &'static str) -> Self {
        Self {
            schema,
            name,
            increment_by: None,
            min_value: None,
            max_value: None,
            start_with: None,
            cache_size: None,
            cycle: None,
        }
    }

    /// Set increment value
    #[must_use]
    pub const fn increment_by(self, value: &'static str) -> Self {
        Self {
            increment_by: Some(value),
            ..self
        }
    }

    /// Set minimum value
    #[must_use]
    pub const fn min_value(self, value: &'static str) -> Self {
        Self {
            min_value: Some(value),
            ..self
        }
    }

    /// Set maximum value
    #[must_use]
    pub const fn max_value(self, value: &'static str) -> Self {
        Self {
            max_value: Some(value),
            ..self
        }
    }

    /// Set start value
    #[must_use]
    pub const fn start_with(self, value: &'static str) -> Self {
        Self {
            start_with: Some(value),
            ..self
        }
    }

    /// Set cache size
    #[must_use]
    pub const fn cache_size(self, value: i32) -> Self {
        Self {
            cache_size: Some(value),
            ..self
        }
    }

    /// Set cycle flag
    #[must_use]
    pub const fn cycle(self, value: bool) -> Self {
        Self {
            cycle: Some(value),
            ..self
        }
    }

    /// Convert to runtime [`Sequence`] type
    #[must_use]
    pub const fn into_sequence(self) -> Sequence {
        Sequence {
            schema: Cow::Borrowed(self.schema),
            name: Cow::Borrowed(self.name),
            increment_by: match self.increment_by {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            min_value: match self.min_value {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            max_value: match self.max_value {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            start_with: match self.start_with {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            cache_size: self.cache_size,
            cycle: self.cycle,
        }
    }
}

impl Default for SequenceDef {
    fn default() -> Self {
        Self::new("public", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime sequence entity for serde serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Sequence {
    /// Schema name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub schema: Cow<'static, str>,

    /// Sequence name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// Increment value
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub increment_by: Option<Cow<'static, str>>,

    /// Minimum value
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub min_value: Option<Cow<'static, str>>,

    /// Maximum value
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub max_value: Option<Cow<'static, str>>,

    /// Start value
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub start_with: Option<Cow<'static, str>>,

    /// Cache size
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub cache_size: Option<i32>,

    /// Cycle flag
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub cycle: Option<bool>,
}

impl Sequence {
    /// Create a new sequence (runtime)
    #[must_use]
    pub fn new(schema: impl Into<Cow<'static, str>>, name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            schema: schema.into(),
            name: name.into(),
            increment_by: None,
            min_value: None,
            max_value: None,
            start_with: None,
            cache_size: None,
            cycle: None,
        }
    }

    /// Get the schema name
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Get the sequence name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for Sequence {
    fn default() -> Self {
        Self::new("public", "")
    }
}

impl From<SequenceDef> for Sequence {
    fn from(def: SequenceDef) -> Self {
        def.into_sequence()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_sequence_def() {
        const SEQ: SequenceDef = SequenceDef::new("public", "users_id_seq")
            .increment_by("1")
            .start_with("1")
            .cycle(true);

        assert_eq!(SEQ.schema, "public");
        assert_eq!(SEQ.name, "users_id_seq");
        assert_eq!(SEQ.increment_by, Some("1"));
        assert_eq!(SEQ.cycle, Some(true));
    }

    #[test]
    fn test_sequence_def_to_sequence() {
        const DEF: SequenceDef = SequenceDef::new("public", "seq").increment_by("1");
        let seq = DEF.into_sequence();
        assert_eq!(seq.schema(), "public");
        assert_eq!(seq.name(), "seq");
        assert_eq!(seq.increment_by.as_ref().map(|s| s.as_ref()), Some("1"));
    }

    #[test]
    fn test_sequence_with_cache() {
        const SEQ: SequenceDef = SequenceDef::new("public", "seq").cache_size(100);
        let seq = SEQ.into_sequence();
        assert_eq!(seq.cache_size, Some(100));
    }
}
