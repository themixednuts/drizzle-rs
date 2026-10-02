//! `SQLite` unique constraints: [`UniqueConstraintDef`] (const) and
//! [`UniqueConstraint`] (runtime).

use crate::alloc_prelude::*;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// A unique constraint that can be built in a `const`.
///
/// [`TableSql`](super::TableSql) writes a single-column constraint without an
/// explicit name as `UNIQUE` on the column, and any other as a table
/// constraint.
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::UniqueConstraintDef;
/// use std::borrow::Cow;
///
/// const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("email"), Cow::Borrowed("tenant_id")];
/// const UNIQ: UniqueConstraintDef = UniqueConstraintDef::new("users", "uq_email_tenant")
///     .columns(COLS);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UniqueConstraintDef {
    /// Parent table name
    pub table: &'static str,
    /// Constraint name
    pub name: &'static str,
    /// Columns in the unique constraint
    pub columns: &'static [Cow<'static, str>],
    /// Whether the user gave the constraint name (rather than it being generated)
    pub name_explicit: bool,
}

impl UniqueConstraintDef {
    /// Creates a unique constraint with no columns; set them with
    /// [`columns`](Self::columns()).
    #[must_use]
    pub const fn new(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            columns: &[],
            name_explicit: false,
        }
    }

    /// Sets the constrained columns.
    #[must_use]
    pub const fn columns(self, cols: &'static [Cow<'static, str>]) -> Self {
        Self {
            columns: cols,
            ..self
        }
    }

    /// Marks the name as given by the user.
    #[must_use]
    pub const fn explicit_name(self) -> Self {
        Self {
            name_explicit: true,
            ..self
        }
    }

    /// Converts to the runtime [`UniqueConstraint`].
    #[must_use]
    pub const fn into_unique_constraint(self) -> UniqueConstraint {
        UniqueConstraint {
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            columns: Cow::Borrowed(self.columns),
            name_explicit: self.name_explicit,
        }
    }
}

impl Default for UniqueConstraintDef {
    fn default() -> Self {
        Self::new("", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// A unique constraint, as stored in migration snapshots.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniqueConstraint {
    /// Parent table name
    pub table: Cow<'static, str>,

    /// Constraint name
    pub name: Cow<'static, str>,

    /// Columns in the unique constraint
    pub columns: Cow<'static, [Cow<'static, str>]>,

    /// Whether the user gave the constraint name (rather than it being generated)
    pub name_explicit: bool,
}

impl UniqueConstraint {
    /// Creates a unique constraint.
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        columns: impl Into<Cow<'static, [Cow<'static, str>]>>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            columns: columns.into(),
            name_explicit: false,
        }
    }

    /// Creates a unique constraint from owned strings.
    #[cfg(feature = "std")]
    #[must_use]
    pub fn from_strings(table: String, name: String, columns: Vec<String>) -> Self {
        Self {
            table: Cow::Owned(table),
            name: Cow::Owned(name),
            columns: Cow::Owned(columns.into_iter().map(Cow::Owned).collect()),
            name_explicit: false,
        }
    }

    /// Returns the constraint name.
    #[inline]
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the table name.
    #[inline]
    #[must_use]
    pub fn table(&self) -> &str {
        &self.table
    }
}

impl Default for UniqueConstraint {
    fn default() -> Self {
        Self::new("", "", &[] as &[Cow<'static, str>])
    }
}

impl From<UniqueConstraintDef> for UniqueConstraint {
    fn from(def: UniqueConstraintDef) -> Self {
        def.into_unique_constraint()
    }
}

// =============================================================================
// Serde Implementation
// =============================================================================

#[cfg(feature = "serde")]
mod serde_impl {
    use super::{Cow, String, UniqueConstraint, Vec};
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for UniqueConstraint {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("UniqueConstraint", 4)?;
            state.serialize_field("table", &*self.table)?;
            state.serialize_field("name", &*self.name)?;
            // Serialize columns as Vec<&str>
            let cols: Vec<&str> = self.columns.iter().map(AsRef::as_ref).collect();
            state.serialize_field("columns", &cols)?;
            state.serialize_field("nameExplicit", &self.name_explicit)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for UniqueConstraint {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Helper {
                table: String,
                name: String,
                #[serde(default)]
                columns: Vec<String>,
                #[serde(default)]
                name_explicit: bool,
            }

            let helper = Helper::deserialize(deserializer)?;
            Ok(Self {
                table: Cow::Owned(helper.table),
                name: Cow::Owned(helper.name),
                columns: Cow::Owned(helper.columns.into_iter().map(Cow::Owned).collect()),
                name_explicit: helper.name_explicit,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_unique_def() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("email"), Cow::Borrowed("tenant_id")];
        const UNIQ: UniqueConstraintDef =
            UniqueConstraintDef::new("users", "uq_email_tenant").columns(COLS);

        assert_eq!(UNIQ.name, "uq_email_tenant");
        assert_eq!(UNIQ.table, "users");
        assert_eq!(UNIQ.columns.len(), 2);
    }

    #[test]
    fn test_unique_def_to_unique_constraint() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("email")];
        const DEF: UniqueConstraintDef =
            UniqueConstraintDef::new("users", "uq_email").columns(COLS);

        let uniq = DEF.into_unique_constraint();
        assert_eq!(uniq.name(), "uq_email");
        assert_eq!(uniq.columns.len(), 1);
    }

    #[test]
    fn test_const_into_unique_constraint() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("email")];
        const DEF: UniqueConstraintDef =
            UniqueConstraintDef::new("users", "uq_email").columns(COLS);
        const UNIQ: UniqueConstraint = DEF.into_unique_constraint();

        assert_eq!(&*UNIQ.name, "uq_email");
        assert_eq!(UNIQ.columns.len(), 1);
    }

    #[test]
    fn test_from_strings() {
        let uniq = UniqueConstraint::from_strings(
            "users".to_string(),
            "users_email_unique".to_string(),
            vec!["email".to_string()],
        );
        assert_eq!(uniq.table(), "users");
        assert_eq!(uniq.name(), "users_email_unique");
        assert_eq!(uniq.columns.len(), 1);
    }
}
