//! SQLite Primary Key DDL types

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly primary key definition
///
/// Used for composite primary keys (single-column PKs are defined on the column itself).
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::PrimaryKeyDef;
/// use std::borrow::Cow;
///
/// const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id"), Cow::Borrowed("role_id")];
/// const PK: PrimaryKeyDef = PrimaryKeyDef::new("user_roles", "pk_user_roles")
///     .columns(COLS);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct PrimaryKeyDef {
    /// Parent table name
    pub table: &'static str,
    /// Constraint name
    pub name: &'static str,
    /// Columns in the primary key
    pub columns: &'static [Cow<'static, str>],
    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
}

impl PrimaryKeyDef {
    /// Create a new primary key definition
    #[must_use]
    pub const fn new(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            columns: &[],
            name_explicit: false,
        }
    }

    /// Set the columns in the primary key
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

    /// Convert to runtime [`PrimaryKey`] type
    #[must_use]
    pub const fn into_primary_key(self) -> PrimaryKey<'static> {
        PrimaryKey {
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            columns: Cow::Borrowed(self.columns),
            name_explicit: self.name_explicit,
        }
    }
}

impl Default for PrimaryKeyDef {
    fn default() -> Self {
        Self::new("", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime primary key constraint entity
///
/// The lifetime parameter allows this type to work with both static (const) data
/// and owned (runtime) data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PrimaryKey<'a> {
    /// Parent table name
    pub table: Cow<'a, str>,

    /// Constraint name
    pub name: Cow<'a, str>,

    /// Columns in the primary key
    pub columns: Cow<'a, [Cow<'a, str>]>,

    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
}

impl<'a> PrimaryKey<'a> {
    /// Create a new primary key with owned data
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'a, str>>,
        name: impl Into<Cow<'a, str>>,
        columns: impl Into<Cow<'a, [Cow<'a, str>]>>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            columns: columns.into(),
            name_explicit: false,
        }
    }

    /// Create a new primary key from owned strings (convenience for runtime construction)
    #[cfg(feature = "std")]
    #[must_use]
    pub fn from_strings(table: String, name: String, columns: Vec<String>) -> PrimaryKey<'static> {
        PrimaryKey {
            table: Cow::Owned(table),
            name: Cow::Owned(name),
            columns: Cow::Owned(columns.into_iter().map(Cow::Owned).collect()),
            name_explicit: false,
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

    /// Convert to a static lifetime version by leaking owned data
    #[cfg(feature = "std")]
    #[must_use]
    pub fn into_static(self) -> PrimaryKey<'static> {
        PrimaryKey {
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
        }
    }
}

impl Default for PrimaryKey<'static> {
    fn default() -> Self {
        Self::new("", "", &[] as &[Cow<'static, str>])
    }
}

impl From<PrimaryKeyDef> for PrimaryKey<'static> {
    fn from(def: PrimaryKeyDef) -> Self {
        def.into_primary_key()
    }
}

// =============================================================================
// Serde Implementation
// =============================================================================

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for PrimaryKey<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("PrimaryKey", 4)?;
            state.serialize_field("table", &*self.table)?;
            state.serialize_field("name", &*self.name)?;
            // Serialize columns as Vec<&str>
            let cols: Vec<&str> = self.columns.iter().map(|c| c.as_ref()).collect();
            state.serialize_field("columns", &cols)?;
            state.serialize_field("nameExplicit", &self.name_explicit)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for PrimaryKey<'static> {
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
            Ok(PrimaryKey {
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
    fn test_const_primary_key_def() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id"), Cow::Borrowed("role_id")];
        const PK: PrimaryKeyDef = PrimaryKeyDef::new("user_roles", "pk_user_roles").columns(COLS);

        assert_eq!(PK.name, "pk_user_roles");
        assert_eq!(PK.table, "user_roles");
        assert_eq!(PK.columns.len(), 2);
    }

    #[test]
    fn test_primary_key_def_to_primary_key() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id"), Cow::Borrowed("role_id")];
        const DEF: PrimaryKeyDef = PrimaryKeyDef::new("user_roles", "pk_user_roles").columns(COLS);

        let pk = DEF.into_primary_key();
        assert_eq!(pk.name(), "pk_user_roles");
        assert_eq!(pk.columns.len(), 2);
    }

    #[test]
    fn test_const_into_primary_key() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id"), Cow::Borrowed("role_id")];
        const DEF: PrimaryKeyDef = PrimaryKeyDef::new("user_roles", "pk_user_roles").columns(COLS);
        const PK: PrimaryKey<'static> = DEF.into_primary_key();

        assert_eq!(&*PK.name, "pk_user_roles");
        assert_eq!(PK.columns.len(), 2);
    }

    #[test]
    fn test_from_strings() {
        let pk = PrimaryKey::from_strings(
            "users".to_string(),
            "users_pk".to_string(),
            vec!["id".to_string()],
        );
        assert_eq!(pk.table(), "users");
        assert_eq!(pk.name(), "users_pk");
        assert_eq!(pk.columns.len(), 1);
    }
}
