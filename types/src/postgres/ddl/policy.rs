//! PostgreSQL Policy DDL types
//!
//! This module provides two complementary types:
//! - [`PolicyDef`] - A const-friendly definition type for compile-time schema definitions
//! - [`Policy`] - A runtime type for serde serialization/deserialization

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string, cow_option_vec_from_strings};

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly policy definition for compile-time schema definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PolicyDef {
    /// Schema name
    pub schema: &'static str,
    /// Table name
    pub table: &'static str,
    /// Policy name
    pub name: &'static str,
    /// AS clause (PERMISSIVE/RESTRICTIVE)
    pub as_clause: Option<&'static str>,
    /// FOR clause (ALL/SELECT/INSERT/UPDATE/DELETE)
    pub for_clause: Option<&'static str>,
    /// TO roles (comma-separated)
    pub to: Option<&'static [&'static str]>,
    /// USING expression
    pub using: Option<&'static str>,
    /// WITH CHECK expression
    pub with_check: Option<&'static str>,
}

impl PolicyDef {
    /// Create a new policy definition
    #[must_use]
    pub const fn new(schema: &'static str, table: &'static str, name: &'static str) -> Self {
        Self {
            schema,
            table,
            name,
            as_clause: None,
            for_clause: None,
            to: None,
            using: None,
            with_check: None,
        }
    }

    /// Set AS clause
    #[must_use]
    pub const fn as_clause(self, clause: &'static str) -> Self {
        Self {
            as_clause: Some(clause),
            ..self
        }
    }

    /// Set FOR clause
    #[must_use]
    pub const fn for_clause(self, clause: &'static str) -> Self {
        Self {
            for_clause: Some(clause),
            ..self
        }
    }

    /// Set TO roles
    #[must_use]
    pub const fn to(self, roles: &'static [&'static str]) -> Self {
        Self {
            to: Some(roles),
            ..self
        }
    }

    /// Set USING expression
    #[must_use]
    pub const fn using(self, expr: &'static str) -> Self {
        Self {
            using: Some(expr),
            ..self
        }
    }

    /// Set WITH CHECK expression
    #[must_use]
    pub const fn with_check(self, expr: &'static str) -> Self {
        Self {
            with_check: Some(expr),
            ..self
        }
    }

    /// Convert to runtime [`Policy`] type
    #[must_use]
    pub const fn into_policy(self) -> Policy {
        Policy {
            schema: Cow::Borrowed(self.schema),
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            as_clause: match self.as_clause {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            for_clause: match self.for_clause {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            to: match self.to {
                Some(roles) => Some(Cow::Borrowed(roles)),
                None => None,
            },
            using: match self.using {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
            with_check: match self.with_check {
                Some(s) => Some(Cow::Borrowed(s)),
                None => None,
            },
        }
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime policy entity for serde serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Policy {
    /// Schema name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub schema: Cow<'static, str>,

    /// Table name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,

    /// Policy name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// AS clause (PERMISSIVE/RESTRICTIVE)
    #[cfg_attr(
        feature = "serde",
        serde(
            rename = "as",
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub as_clause: Option<Cow<'static, str>>,

    /// FOR clause (ALL/SELECT/INSERT/UPDATE/DELETE)
    #[cfg_attr(
        feature = "serde",
        serde(
            rename = "for",
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub for_clause: Option<Cow<'static, str>>,

    /// TO roles
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_vec_from_strings"
        )
    )]
    pub to: Option<Cow<'static, [&'static str]>>,

    /// USING expression
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub using: Option<Cow<'static, str>>,

    /// WITH CHECK expression
    #[cfg_attr(
        feature = "serde",
        serde(
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub with_check: Option<Cow<'static, str>>,
}

impl Policy {
    /// Create a new policy (runtime)
    #[must_use]
    pub fn new(
        schema: impl Into<Cow<'static, str>>,
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            schema: schema.into(),
            table: table.into(),
            name: name.into(),
            as_clause: None,
            for_clause: None,
            to: None,
            using: None,
            with_check: None,
        }
    }

    /// Get the schema name
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Get the table name
    #[inline]
    #[must_use]
    pub fn table(&self) -> &str {
        &self.table
    }

    /// Get the policy name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl From<PolicyDef> for Policy {
    fn from(def: PolicyDef) -> Self {
        def.into_policy()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_policy_def() {
        const POLICY: PolicyDef = PolicyDef::new("public", "users", "users_policy")
            .for_clause("SELECT")
            .using("user_id = current_user_id()");

        assert_eq!(POLICY.schema, "public");
        assert_eq!(POLICY.table, "users");
        assert_eq!(POLICY.name, "users_policy");
    }

    #[test]
    fn test_policy_def_to_policy() {
        const DEF: PolicyDef = PolicyDef::new("public", "users", "policy");
        let policy = DEF.into_policy();
        assert_eq!(policy.schema(), "public");
        assert_eq!(policy.table(), "users");
        assert_eq!(policy.name(), "policy");
    }
}
