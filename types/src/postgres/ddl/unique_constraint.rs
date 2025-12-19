//! PostgreSQL Unique Constraint DDL types

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly unique constraint definition
///
/// # Examples
///
/// ```
/// use drizzle_types::postgres::ddl::UniqueConstraintDef;
/// use std::borrow::Cow;
///
/// const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("email"), Cow::Borrowed("tenant_id")];
/// const UNIQ: UniqueConstraintDef = UniqueConstraintDef::new("public", "users", "uq_email_tenant")
///     .columns(COLS);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct UniqueConstraintDef {
    /// Schema name
    pub schema: &'static str,
    /// Parent table name
    pub table: &'static str,
    /// Constraint name
    pub name: &'static str,
    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
    /// Columns in the unique constraint
    pub columns: &'static [Cow<'static, str>],
    /// NULLS NOT DISTINCT flag (PostgreSQL 15+)
    pub nulls_not_distinct: bool,
}

impl UniqueConstraintDef {
    /// Create a new unique constraint definition
    #[must_use]
    pub const fn new(schema: &'static str, table: &'static str, name: &'static str) -> Self {
        Self {
            schema,
            table,
            name,
            name_explicit: false,
            columns: &[],
            nulls_not_distinct: false,
        }
    }

    /// Set the columns in the unique constraint
    #[must_use]
    pub const fn columns(self, cols: &'static [Cow<'static, str>]) -> Self {
        Self {
            columns: cols,
            ..self
        }
    }

    /// Mark the name as explicitly specified
    #[must_use]
    pub const fn explicit_name(self) -> Self {
        Self {
            name_explicit: true,
            ..self
        }
    }

    /// Set NULLS NOT DISTINCT
    #[must_use]
    pub const fn nulls_not_distinct(self) -> Self {
        Self {
            nulls_not_distinct: true,
            ..self
        }
    }

    /// Convert to runtime [`UniqueConstraint`] type
    #[must_use]
    pub const fn into_unique_constraint(self) -> UniqueConstraint<'static> {
        UniqueConstraint {
            schema: Cow::Borrowed(self.schema),
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            columns: Cow::Borrowed(self.columns),
            name_explicit: self.name_explicit,
            nulls_not_distinct: self.nulls_not_distinct,
        }
    }
}

impl Default for UniqueConstraintDef {
    fn default() -> Self {
        Self::new("public", "", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime unique constraint entity
///
/// The lifetime parameter allows this type to work with both static (const) data
/// and owned (runtime) data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UniqueConstraint<'a> {
    /// Schema name
    pub schema: Cow<'a, str>,

    /// Parent table name
    pub table: Cow<'a, str>,

    /// Constraint name
    pub name: Cow<'a, str>,

    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,

    /// Columns in the unique constraint
    pub columns: Cow<'a, [Cow<'a, str>]>,

    /// NULLS NOT DISTINCT flag (PostgreSQL 15+)
    pub nulls_not_distinct: bool,
}

impl<'a> UniqueConstraint<'a> {
    /// Create a new unique constraint
    #[must_use]
    pub fn new(
        schema: impl Into<Cow<'a, str>>,
        table: impl Into<Cow<'a, str>>,
        name: impl Into<Cow<'a, str>>,
        columns: impl Into<Cow<'a, [Cow<'a, str>]>>,
    ) -> Self {
        Self {
            schema: schema.into(),
            table: table.into(),
            name: name.into(),
            columns: columns.into(),
            name_explicit: false,
            nulls_not_distinct: false,
        }
    }

    /// Create a new unique constraint from owned strings (convenience for runtime construction)
    #[cfg(feature = "std")]
    #[must_use]
    pub fn from_strings(
        schema: String,
        table: String,
        name: String,
        columns: Vec<String>,
    ) -> UniqueConstraint<'static> {
        UniqueConstraint {
            schema: Cow::Owned(schema),
            table: Cow::Owned(table),
            name: Cow::Owned(name),
            columns: Cow::Owned(columns.into_iter().map(Cow::Owned).collect()),
            name_explicit: false,
            nulls_not_distinct: false,
        }
    }

    /// Get the schema name
    #[inline]
    #[must_use]
    pub fn schema(&self) -> &str {
        &self.schema
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

    /// Mark the name as explicitly specified
    #[must_use]
    pub fn explicit_name(mut self) -> Self {
        self.name_explicit = true;
        self
    }

    /// Convert to a static lifetime version by converting to owned data
    #[cfg(feature = "std")]
    #[must_use]
    pub fn into_static(self) -> UniqueConstraint<'static> {
        UniqueConstraint {
            schema: Cow::Owned(self.schema.into_owned()),
            table: Cow::Owned(self.table.into_owned()),
            name: Cow::Owned(self.name.into_owned()),
            columns: Cow::Owned(
                self.columns
                    .into_owned()
                    .into_iter()
                    .map(|c| Cow::Owned(c.into_owned()))
                    .collect(),
            ),
            name_explicit: self.name_explicit,
            nulls_not_distinct: self.nulls_not_distinct,
        }
    }
}

impl Default for UniqueConstraint<'static> {
    fn default() -> Self {
        Self::new("public", "", "", &[] as &[Cow<'static, str>])
    }
}

impl From<UniqueConstraintDef> for UniqueConstraint<'static> {
    fn from(def: UniqueConstraintDef) -> Self {
        def.into_unique_constraint()
    }
}

// =============================================================================
// Serde Implementation
// =============================================================================

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for UniqueConstraint<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("UniqueConstraint", 6)?;
            state.serialize_field("schema", &*self.schema)?;
            state.serialize_field("table", &*self.table)?;
            state.serialize_field("name", &*self.name)?;
            // Serialize columns as Vec<&str>
            let cols: Vec<&str> = self.columns.iter().map(|c| c.as_ref()).collect();
            state.serialize_field("columns", &cols)?;
            state.serialize_field("nameExplicit", &self.name_explicit)?;
            state.serialize_field("nullsNotDistinct", &self.nulls_not_distinct)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for UniqueConstraint<'static> {
        fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
        where
            D: Deserializer<'de>,
        {
            #[derive(Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct Helper {
                schema: String,
                table: String,
                name: String,
                #[serde(default)]
                columns: Vec<String>,
                #[serde(default)]
                name_explicit: bool,
                #[serde(default)]
                nulls_not_distinct: bool,
            }

            let helper = Helper::deserialize(deserializer)?;
            Ok(UniqueConstraint {
                schema: Cow::Owned(helper.schema),
                table: Cow::Owned(helper.table),
                name: Cow::Owned(helper.name),
                columns: Cow::Owned(helper.columns.into_iter().map(Cow::Owned).collect()),
                name_explicit: helper.name_explicit,
                nulls_not_distinct: helper.nulls_not_distinct,
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
            UniqueConstraintDef::new("public", "users", "uq_email_tenant")
                .columns(COLS)
                .nulls_not_distinct();

        assert_eq!(UNIQ.name, "uq_email_tenant");
        assert_eq!(UNIQ.table, "users");
        assert_eq!(UNIQ.columns.len(), 2);
        assert!(UNIQ.nulls_not_distinct);
    }

    #[test]
    fn test_unique_def_to_unique_constraint() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("email")];
        const DEF: UniqueConstraintDef =
            UniqueConstraintDef::new("public", "users", "uq_email").columns(COLS);

        let uniq = DEF.into_unique_constraint();
        assert_eq!(uniq.name(), "uq_email");
        assert_eq!(uniq.columns.len(), 1);
    }

    #[test]
    fn test_const_into_unique_constraint() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("email")];
        const DEF: UniqueConstraintDef =
            UniqueConstraintDef::new("public", "users", "uq_email").columns(COLS);
        const UNIQ: UniqueConstraint<'static> = DEF.into_unique_constraint();

        assert_eq!(&*UNIQ.name, "uq_email");
        assert_eq!(UNIQ.columns.len(), 1);
    }

    #[test]
    fn test_from_strings() {
        let uniq = UniqueConstraint::from_strings(
            "public".to_string(),
            "users".to_string(),
            "users_email_unique".to_string(),
            vec!["email".to_string()],
        );
        assert_eq!(uniq.schema(), "public");
        assert_eq!(uniq.table(), "users");
        assert_eq!(uniq.name(), "users_email_unique");
        assert_eq!(uniq.columns.len(), 1);
    }
}
