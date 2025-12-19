//! SQLite View DDL types
//!
//! See: <https://github.com/drizzle-team/drizzle-orm/blob/beta/drizzle-kit/src/dialects/sqlite/ddl.ts>

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly view definition
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::ViewDef;
///
/// const VIEW: ViewDef = ViewDef::new("active_users")
///     .definition("SELECT * FROM users WHERE active = 1");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ViewDef {
    /// View name
    pub name: &'static str,
    /// View definition (AS SELECT ...)
    pub definition: Option<&'static str>,
    /// Whether this is an existing view (not managed by drizzle)
    pub is_existing: bool,
    /// Error message if the view failed to parse/validate
    pub error: Option<&'static str>,
}

impl ViewDef {
    /// Create a new view definition
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            definition: None,
            is_existing: false,
            error: None,
        }
    }

    /// Set the view definition
    #[must_use]
    pub const fn definition(self, sql: &'static str) -> Self {
        Self {
            definition: Some(sql),
            ..self
        }
    }

    /// Mark as existing (not managed by drizzle)
    #[must_use]
    pub const fn existing(self) -> Self {
        Self {
            is_existing: true,
            ..self
        }
    }

    /// Set an error message
    #[must_use]
    pub const fn error(self, error: &'static str) -> Self {
        Self {
            error: Some(error),
            ..self
        }
    }

    /// Convert to runtime [`View`] type
    #[must_use]
    pub const fn into_view(self) -> View {
        View {
            name: Cow::Borrowed(self.name),
            definition: match self.definition {
                Some(d) => Some(Cow::Borrowed(d)),
                None => None,
            },
            is_existing: self.is_existing,
            error: match self.error {
                Some(e) => Some(Cow::Borrowed(e)),
                None => None,
            },
        }
    }
}

impl Default for ViewDef {
    fn default() -> Self {
        Self::new("")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime view entity
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct View {
    /// View name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// View definition (AS SELECT ...)
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub definition: Option<Cow<'static, str>>,

    /// Whether this is an existing view (not managed by drizzle)
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_existing: bool,

    /// Error message if the view failed to parse/validate
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub error: Option<Cow<'static, str>>,
}

impl View {
    /// Create a new view
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            definition: None,
            is_existing: false,
            error: None,
        }
    }

    /// Get the view name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for View {
    fn default() -> Self {
        Self::new("")
    }
}

impl From<ViewDef> for View {
    fn from(def: ViewDef) -> Self {
        def.into_view()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_view_def() {
        const VIEW: ViewDef =
            ViewDef::new("active_users").definition("SELECT * FROM users WHERE active = 1");

        assert_eq!(VIEW.name, "active_users");
        assert!(VIEW.definition.is_some());
        assert!(!VIEW.is_existing);
    }

    #[test]
    fn test_view_def_to_view() {
        const DEF: ViewDef = ViewDef::new("active_users").existing();

        let view = DEF.into_view();
        assert_eq!(view.name(), "active_users");
        assert!(view.is_existing);
    }
}
