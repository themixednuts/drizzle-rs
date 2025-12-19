//! SQLite Check Constraint DDL types

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(feature = "serde")]
use crate::serde_helpers::cow_from_string;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly check constraint definition
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::CheckConstraintDef;
///
/// const CHECK: CheckConstraintDef = CheckConstraintDef::new("users", "ck_age")
///     .value("age >= 0");
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct CheckConstraintDef {
    /// Parent table name
    pub table: &'static str,
    /// Constraint name
    pub name: &'static str,
    /// Check expression
    pub value: &'static str,
}

impl CheckConstraintDef {
    /// Create a new check constraint definition
    #[must_use]
    pub const fn new(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            value: "",
        }
    }

    /// Set the check expression
    #[must_use]
    pub const fn value(self, expression: &'static str) -> Self {
        Self {
            value: expression,
            ..self
        }
    }

    /// Convert to runtime [`CheckConstraint`] type
    #[must_use]
    pub const fn into_check_constraint(self) -> CheckConstraint {
        CheckConstraint {
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            value: Cow::Borrowed(self.value),
        }
    }
}

impl Default for CheckConstraintDef {
    fn default() -> Self {
        Self::new("", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime check constraint entity
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "camelCase"))]
pub struct CheckConstraint {
    /// Parent table name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub table: Cow<'static, str>,

    /// Constraint name
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub name: Cow<'static, str>,

    /// Check expression
    #[cfg_attr(feature = "serde", serde(deserialize_with = "cow_from_string"))]
    pub value: Cow<'static, str>,
}

impl CheckConstraint {
    /// Create a new check constraint
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        value: impl Into<Cow<'static, str>>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            value: value.into(),
        }
    }

    /// Get the constraint name
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the table name
    #[inline]
    #[must_use]
    pub fn table(&self) -> &str {
        &self.table
    }
}

impl Default for CheckConstraint {
    fn default() -> Self {
        Self::new("", "", "")
    }
}

impl From<CheckConstraintDef> for CheckConstraint {
    fn from(def: CheckConstraintDef) -> Self {
        def.into_check_constraint()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_check_def() {
        const CHECK: CheckConstraintDef =
            CheckConstraintDef::new("users", "ck_age").value("age >= 0");

        assert_eq!(CHECK.name, "ck_age");
        assert_eq!(CHECK.table, "users");
        assert_eq!(CHECK.value, "age >= 0");
    }

    #[test]
    fn test_check_def_to_check_constraint() {
        const DEF: CheckConstraintDef =
            CheckConstraintDef::new("users", "ck_age").value("age >= 0");

        let check = DEF.into_check_constraint();
        assert_eq!(check.name(), "ck_age");
        assert_eq!(check.value.as_ref(), "age >= 0");
    }
}
