//! PostgreSQL Privilege DDL types
//!
//! See: <https://github.com/drizzle-team/drizzle-orm/blob/beta/drizzle-kit/src/dialects/postgres/ddl.ts>

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::cow_from_string;

// =============================================================================
// Privilege Type Enum
// =============================================================================

/// PostgreSQL privilege type
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "SCREAMING_SNAKE_CASE"))]
pub enum PrivilegeType {
    /// All privileges
    #[default]
    All,
    /// SELECT privilege
    Select,
    /// INSERT privilege
    Insert,
    /// UPDATE privilege
    Update,
    /// DELETE privilege
    Delete,
    /// TRUNCATE privilege
    Truncate,
    /// REFERENCES privilege
    References,
    /// TRIGGER privilege
    Trigger,
}

impl PrivilegeType {
    /// Get the SQL representation
    #[must_use]
    pub const fn as_sql(&self) -> &'static str {
        match self {
            Self::All => "ALL",
            Self::Select => "SELECT",
            Self::Insert => "INSERT",
            Self::Update => "UPDATE",
            Self::Delete => "DELETE",
            Self::Truncate => "TRUNCATE",
            Self::References => "REFERENCES",
            Self::Trigger => "TRIGGER",
        }
    }

    /// Parse from SQL string
    pub fn from_sql(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "ALL" => Some(Self::All),
            "SELECT" => Some(Self::Select),
            "INSERT" => Some(Self::Insert),
            "UPDATE" => Some(Self::Update),
            "DELETE" => Some(Self::Delete),
            "TRUNCATE" => Some(Self::Truncate),
            "REFERENCES" => Some(Self::References),
            "TRIGGER" => Some(Self::Trigger),
            _ => None,
        }
    }
}

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly privilege definition for compile-time schema definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrivilegeDef {
    /// Role granting the privilege
    pub grantor: &'static str,
    /// Role receiving the privilege
    pub grantee: &'static str,
    /// Schema name
    pub schema: &'static str,
    /// Table name
    pub table: &'static str,
    /// Privilege type
    pub privilege_type: PrivilegeType,
    /// Can the grantee grant this privilege to others?
    pub is_grantable: bool,
}

impl PrivilegeDef {
    /// Create a new privilege definition
    #[must_use]
    pub const fn new(
        schema: &'static str,
        table: &'static str,
        grantee: &'static str,
        privilege_type: PrivilegeType,
    ) -> Self {
        Self {
            grantor: "",
            grantee,
            schema,
            table,
            privilege_type,
            is_grantable: false,
        }
    }

    /// Set the grantor
    #[must_use]
    pub const fn grantor(self, grantor: &'static str) -> Self {
        Self { grantor, ..self }
    }

    /// Set is_grantable flag
    #[must_use]
    pub const fn grantable(self) -> Self {
        Self {
            is_grantable: true,
            ..self
        }
    }

    /// Convert to runtime [`Privilege`] type
    #[must_use]
    pub const fn into_privilege(self) -> Privilege {
        Privilege {
            grantor: Cow::Borrowed(self.grantor),
            grantee: Cow::Borrowed(self.grantee),
            schema: Cow::Borrowed(self.schema),
            table: Cow::Borrowed(self.table),
            privilege_type: self.privilege_type,
            is_grantable: self.is_grantable,
        }
    }
}

impl Default for PrivilegeDef {
    fn default() -> Self {
        Self::new("public", "", "", PrivilegeType::All)
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime privilege entity for serde serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Privilege {
    /// Role granting the privilege
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub grantor: Cow<'static, str>,

    /// Role receiving the privilege
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub grantee: Cow<'static, str>,

    /// Schema name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub schema: Cow<'static, str>,

    /// Table name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,

    /// Privilege type
    #[cfg_attr(feature = "serde", serde(rename = "type"))]
    pub privilege_type: PrivilegeType,

    /// Can the grantee grant this privilege to others?
    #[cfg_attr(feature = "serde", serde(default))]
    pub is_grantable: bool,
}

impl Privilege {
    /// Create a new privilege (runtime)
    #[must_use]
    pub fn new(
        schema: impl Into<Cow<'static, str>>,
        table: impl Into<Cow<'static, str>>,
        grantee: impl Into<Cow<'static, str>>,
        privilege_type: PrivilegeType,
    ) -> Self {
        Self {
            grantor: Cow::Borrowed(""),
            grantee: grantee.into(),
            schema: schema.into(),
            table: table.into(),
            privilege_type,
            is_grantable: false,
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

    /// Get the grantee
    #[inline]
    #[must_use]
    pub fn grantee(&self) -> &str {
        &self.grantee
    }

    /// Get the grantor
    #[inline]
    #[must_use]
    pub fn grantor(&self) -> &str {
        &self.grantor
    }
}

impl Default for Privilege {
    fn default() -> Self {
        Self::new("public", "", "", PrivilegeType::All)
    }
}

impl From<PrivilegeDef> for Privilege {
    fn from(def: PrivilegeDef) -> Self {
        def.into_privilege()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_privilege_def() {
        const PRIV: PrivilegeDef =
            PrivilegeDef::new("public", "users", "app_user", PrivilegeType::Select)
                .grantor("admin")
                .grantable();

        assert_eq!(PRIV.schema, "public");
        assert_eq!(PRIV.table, "users");
        assert_eq!(PRIV.grantee, "app_user");
        assert_eq!(PRIV.grantor, "admin");
        assert_eq!(PRIV.privilege_type, PrivilegeType::Select);
        assert!(PRIV.is_grantable);
    }

    #[test]
    fn test_privilege_def_to_privilege() {
        const DEF: PrivilegeDef =
            PrivilegeDef::new("public", "users", "reader", PrivilegeType::Select);
        let priv_ = DEF.into_privilege();
        assert_eq!(priv_.schema(), "public");
        assert_eq!(priv_.table(), "users");
        assert_eq!(priv_.grantee(), "reader");
    }

    #[test]
    fn test_privilege_type_sql() {
        assert_eq!(PrivilegeType::Select.as_sql(), "SELECT");
        assert_eq!(PrivilegeType::All.as_sql(), "ALL");
        assert_eq!(PrivilegeType::Truncate.as_sql(), "TRUNCATE");
    }
}
