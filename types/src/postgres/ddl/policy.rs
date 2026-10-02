//! `PostgreSQL` row-level security policies: [`PolicyDef`] (const) and
//! [`Policy`] (runtime).

use crate::alloc_prelude::*;

#[cfg(feature = "serde")]
use crate::serde_helpers::{cow_from_string, cow_option_from_string, cow_option_vec_from_strings};

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// A row-level security policy that can be built in a `const`.
///
/// Clause values are SQL as written: `as_clause` is `PERMISSIVE` or
/// `RESTRICTIVE`, `for_clause` is `ALL`, `SELECT`, `INSERT`, `UPDATE` or
/// `DELETE`, and `using` / `with_check` are boolean expressions.
///
/// # Examples
///
/// ```
/// use drizzle_types::postgres::ddl::PolicyDef;
///
/// const OWNER_ONLY: PolicyDef = PolicyDef::new("public", "posts", "owner_only")
///     .for_clause("select")
///     .to(&["authenticated"])
///     .using("author_id = current_user_id()");
///
/// assert_eq!(
///     OWNER_ONLY.into_policy().create_policy_sql(),
///     "CREATE POLICY \"owner_only\" ON \"posts\" AS PERMISSIVE FOR SELECT \
///      TO \"authenticated\" USING (author_id = current_user_id());"
/// );
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PolicyDef {
    /// Schema name
    pub schema: &'static str,
    /// Table name
    pub table: &'static str,
    /// Policy name
    pub name: &'static str,
    /// `AS` clause: `PERMISSIVE` (the default when `None`) or `RESTRICTIVE`
    pub as_clause: Option<&'static str>,
    /// `FOR` clause: `ALL`, `SELECT`, `INSERT`, `UPDATE` or `DELETE`
    pub for_clause: Option<&'static str>,
    /// `TO` role names; `public` means `PUBLIC`
    pub to: Option<&'static [&'static str]>,
    /// USING expression
    pub using: Option<&'static str>,
    /// WITH CHECK expression
    pub with_check: Option<&'static str>,
}

impl PolicyDef {
    /// Creates a policy with no clauses set.
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

    /// Sets the `AS` clause.
    #[must_use]
    pub const fn as_clause(self, clause: &'static str) -> Self {
        Self {
            as_clause: Some(clause),
            ..self
        }
    }

    /// Sets the `FOR` clause.
    #[must_use]
    pub const fn for_clause(self, clause: &'static str) -> Self {
        Self {
            for_clause: Some(clause),
            ..self
        }
    }

    /// Sets the `TO` role names.
    #[must_use]
    pub const fn to(self, roles: &'static [&'static str]) -> Self {
        Self {
            to: Some(roles),
            ..self
        }
    }

    /// Sets the `USING` expression.
    #[must_use]
    pub const fn using(self, expr: &'static str) -> Self {
        Self {
            using: Some(expr),
            ..self
        }
    }

    /// Sets the `WITH CHECK` expression.
    #[must_use]
    pub const fn with_check(self, expr: &'static str) -> Self {
        Self {
            with_check: Some(expr),
            ..self
        }
    }

    /// Converts to the runtime [`Policy`].
    #[must_use]
    pub fn into_policy(self) -> Policy {
        Policy {
            schema: Cow::Borrowed(self.schema),
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            as_clause: self.as_clause.map(Cow::Borrowed),
            for_clause: self.for_clause.map(Cow::Borrowed),
            to: self
                .to
                .map(|roles| roles.iter().copied().map(Cow::Borrowed).collect()),
            using: self.using.map(Cow::Borrowed),
            with_check: self.with_check.map(Cow::Borrowed),
        }
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// A policy, as stored in migration snapshots.
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
            default,
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
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub for_clause: Option<Cow<'static, str>>,

    /// TO roles
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_vec_from_strings"
        )
    )]
    pub to: Option<Vec<Cow<'static, str>>>,

    /// USING expression
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub using: Option<Cow<'static, str>>,

    /// WITH CHECK expression
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub with_check: Option<Cow<'static, str>>,
}

impl Policy {
    /// Creates a policy.
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

    /// Returns the schema name.
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
    }

    /// Returns the table name.
    #[inline]
    #[must_use]
    pub fn table(&self) -> &str {
        &self.table
    }

    /// Returns the policy name.
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

    /// Optional fields are skipped when serializing; deserialization must
    /// treat the missing keys as `None` instead of erroring.
    #[cfg(feature = "serde")]
    #[test]
    fn test_serde_roundtrip_with_all_none_optionals() {
        let policy = Policy::new("public", "users", "users_policy");
        assert!(policy.as_clause.is_none());
        assert!(policy.using.is_none());

        let json = serde_json::to_string(&policy).expect("serialize");
        let parsed: Policy = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(parsed, policy);
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
