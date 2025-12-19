//! PostgreSQL Role DDL types
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

/// Const-friendly role definition for compile-time schema definitions.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RoleDef {
    /// Role name
    pub name: &'static str,
    /// Superuser privilege
    pub superuser: Option<bool>,
    /// Can create databases?
    pub create_db: Option<bool>,
    /// Can create roles?
    pub create_role: Option<bool>,
    /// Inherit privileges?
    pub inherit: Option<bool>,
    /// Can login?
    pub can_login: Option<bool>,
    /// Replication privilege
    pub replication: Option<bool>,
    /// Bypass row-level security
    pub bypass_rls: Option<bool>,
    /// Connection limit (-1 for unlimited)
    pub conn_limit: Option<i32>,
    /// Password (encrypted)
    pub password: Option<&'static str>,
    /// Password expiration date
    pub valid_until: Option<&'static str>,
}

impl RoleDef {
    /// Create a new role definition
    #[must_use]
    pub const fn new(name: &'static str) -> Self {
        Self {
            name,
            superuser: None,
            create_db: None,
            create_role: None,
            inherit: None,
            can_login: None,
            replication: None,
            bypass_rls: None,
            conn_limit: None,
            password: None,
            valid_until: None,
        }
    }

    /// Set superuser privilege
    #[must_use]
    pub const fn superuser(self, value: bool) -> Self {
        Self {
            superuser: Some(value),
            ..self
        }
    }

    /// Set create_db flag
    #[must_use]
    pub const fn create_db(self, value: bool) -> Self {
        Self {
            create_db: Some(value),
            ..self
        }
    }

    /// Set create_role flag
    #[must_use]
    pub const fn create_role(self, value: bool) -> Self {
        Self {
            create_role: Some(value),
            ..self
        }
    }

    /// Set inherit flag
    #[must_use]
    pub const fn inherit(self, value: bool) -> Self {
        Self {
            inherit: Some(value),
            ..self
        }
    }

    /// Set can_login flag
    #[must_use]
    pub const fn can_login(self, value: bool) -> Self {
        Self {
            can_login: Some(value),
            ..self
        }
    }

    /// Set replication privilege
    #[must_use]
    pub const fn replication(self, value: bool) -> Self {
        Self {
            replication: Some(value),
            ..self
        }
    }

    /// Set bypass_rls flag
    #[must_use]
    pub const fn bypass_rls(self, value: bool) -> Self {
        Self {
            bypass_rls: Some(value),
            ..self
        }
    }

    /// Set connection limit
    #[must_use]
    pub const fn conn_limit(self, limit: i32) -> Self {
        Self {
            conn_limit: Some(limit),
            ..self
        }
    }

    /// Set password
    #[must_use]
    pub const fn password(self, password: &'static str) -> Self {
        Self {
            password: Some(password),
            ..self
        }
    }

    /// Set valid_until date
    #[must_use]
    pub const fn valid_until(self, date: &'static str) -> Self {
        Self {
            valid_until: Some(date),
            ..self
        }
    }

    /// Convert to runtime [`Role`] type
    #[must_use]
    pub const fn into_role(self) -> Role {
        Role {
            name: Cow::Borrowed(self.name),
            superuser: self.superuser,
            create_db: self.create_db,
            create_role: self.create_role,
            inherit: self.inherit,
            can_login: self.can_login,
            replication: self.replication,
            bypass_rls: self.bypass_rls,
            conn_limit: self.conn_limit,
            password: match self.password {
                Some(p) => Some(Cow::Borrowed(p)),
                None => None,
            },
            valid_until: match self.valid_until {
                Some(d) => Some(Cow::Borrowed(d)),
                None => None,
            },
        }
    }
}

impl Default for RoleDef {
    fn default() -> Self {
        Self::new("")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime role entity for serde serialization.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct Role {
    /// Role name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// Superuser privilege
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub superuser: Option<bool>,

    /// Can create databases?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub create_db: Option<bool>,

    /// Can create roles?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub create_role: Option<bool>,

    /// Inherit privileges?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub inherit: Option<bool>,

    /// Can login?
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub can_login: Option<bool>,

    /// Replication privilege
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub replication: Option<bool>,

    /// Bypass row-level security
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub bypass_rls: Option<bool>,

    /// Connection limit (-1 for unlimited)
    #[cfg_attr(
        feature = "serde",
        serde(default, skip_serializing_if = "Option::is_none")
    )]
    pub conn_limit: Option<i32>,

    /// Password (encrypted)
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub password: Option<Cow<'static, str>>,

    /// Password expiration date
    #[cfg_attr(
        feature = "serde",
        serde(
            default,
            skip_serializing_if = "Option::is_none",
            deserialize_with = "cow_option_from_string"
        )
    )]
    pub valid_until: Option<Cow<'static, str>>,
}

impl Role {
    /// Create a new role (runtime)
    #[must_use]
    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self {
            name: name.into(),
            superuser: None,
            create_db: None,
            create_role: None,
            inherit: None,
            can_login: None,
            replication: None,
            bypass_rls: None,
            conn_limit: None,
            password: None,
            valid_until: None,
        }
    }

    /// Get the role name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}

impl Default for Role {
    fn default() -> Self {
        Self::new("")
    }
}

impl From<RoleDef> for Role {
    fn from(def: RoleDef) -> Self {
        def.into_role()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_role_def() {
        const ROLE: RoleDef = RoleDef::new("app_user")
            .create_db(true)
            .create_role(true)
            .can_login(true);

        assert_eq!(ROLE.name, "app_user");
        assert_eq!(ROLE.create_db, Some(true));
        assert_eq!(ROLE.create_role, Some(true));
        assert_eq!(ROLE.can_login, Some(true));
    }

    #[test]
    fn test_role_def_to_role() {
        const DEF: RoleDef = RoleDef::new("admin").superuser(true).create_db(true);
        let role = DEF.into_role();
        assert_eq!(role.name(), "admin");
        assert_eq!(role.superuser, Some(true));
        assert_eq!(role.create_db, Some(true));
    }

    #[test]
    fn test_role_with_conn_limit() {
        const ROLE: RoleDef = RoleDef::new("limited_user").conn_limit(5).can_login(true);
        let role = ROLE.into_role();
        assert_eq!(role.conn_limit, Some(5));
    }
}
