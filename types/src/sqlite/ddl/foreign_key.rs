//! SQLite Foreign Key DDL types

#[cfg(feature = "std")]
use std::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::borrow::Cow;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::vec::Vec;

#[cfg(all(feature = "alloc", not(feature = "std")))]
use alloc::string::String;

// =============================================================================
// Shared Types
// =============================================================================

/// Foreign key referential action
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(rename_all = "snake_case"))]
pub enum ReferentialAction {
    /// No action (default)
    #[default]
    NoAction,
    /// Restrict deletion
    Restrict,
    /// Cascade changes
    Cascade,
    /// Set to NULL
    SetNull,
    /// Set to default value
    SetDefault,
}

impl ReferentialAction {
    /// Get the SQL representation
    #[must_use]
    pub const fn as_sql(&self) -> &'static str {
        match self {
            Self::NoAction => "NO ACTION",
            Self::Restrict => "RESTRICT",
            Self::Cascade => "CASCADE",
            Self::SetNull => "SET NULL",
            Self::SetDefault => "SET DEFAULT",
        }
    }

    /// Parse from SQL string
    pub fn from_sql(s: &str) -> Option<Self> {
        match s.to_uppercase().as_str() {
            "NO ACTION" => Some(Self::NoAction),
            "RESTRICT" => Some(Self::Restrict),
            "CASCADE" => Some(Self::Cascade),
            "SET NULL" => Some(Self::SetNull),
            "SET DEFAULT" => Some(Self::SetDefault),
            _ => None,
        }
    }
}

// =============================================================================
// Const-friendly Definition Type
// =============================================================================

/// Const-friendly foreign key definition
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::{ForeignKeyDef, ReferentialAction};
/// use std::borrow::Cow;
///
/// const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
/// const REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
///
/// const FK: ForeignKeyDef = ForeignKeyDef::new("posts", "fk_posts_user")
///     .columns(COLS)
///     .references("users", REFS)
///     .on_delete(ReferentialAction::Cascade);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ForeignKeyDef {
    /// Parent table name
    pub table: &'static str,
    /// Constraint name
    pub name: &'static str,
    /// Source columns
    pub columns: &'static [Cow<'static, str>],
    /// Referenced table name
    pub table_to: &'static str,
    /// Referenced columns
    pub columns_to: &'static [Cow<'static, str>],
    /// ON DELETE action
    pub on_delete: Option<ReferentialAction>,
    /// ON UPDATE action
    pub on_update: Option<ReferentialAction>,
    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
}

impl ForeignKeyDef {
    /// Create a new foreign key definition
    #[must_use]
    pub const fn new(table: &'static str, name: &'static str) -> Self {
        Self {
            table,
            name,
            columns: &[],
            table_to: "",
            columns_to: &[],
            on_delete: None,
            on_update: None,
            name_explicit: false,
        }
    }

    /// Set source columns
    #[must_use]
    pub const fn columns(self, cols: &'static [Cow<'static, str>]) -> Self {
        Self {
            columns: cols,
            ..self
        }
    }

    /// Set reference table and columns
    #[must_use]
    pub const fn references(
        self,
        table: &'static str,
        columns: &'static [Cow<'static, str>],
    ) -> Self {
        Self {
            table_to: table,
            columns_to: columns,
            ..self
        }
    }

    /// Set ON DELETE action
    #[must_use]
    pub const fn on_delete(self, action: ReferentialAction) -> Self {
        Self {
            on_delete: Some(action),
            ..self
        }
    }

    /// Set ON UPDATE action
    #[must_use]
    pub const fn on_update(self, action: ReferentialAction) -> Self {
        Self {
            on_update: Some(action),
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

    /// Convert to runtime [`ForeignKey`] type
    #[must_use]
    pub const fn into_foreign_key(self) -> ForeignKey {
        ForeignKey {
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            columns: Cow::Borrowed(self.columns),
            table_to: Cow::Borrowed(self.table_to),
            columns_to: Cow::Borrowed(self.columns_to),
            on_delete: match self.on_delete {
                Some(a) => Some(Cow::Borrowed(a.as_sql())),
                None => None,
            },
            on_update: match self.on_update {
                Some(a) => Some(Cow::Borrowed(a.as_sql())),
                None => None,
            },
            name_explicit: self.name_explicit,
        }
    }
}

impl Default for ForeignKeyDef {
    fn default() -> Self {
        Self::new("", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime foreign key constraint entity
///
/// Uses `Cow<'static, str>` for all string fields, which works with both:
/// - Borrowed data from const definitions (`Cow::Borrowed`)
/// - Owned data from deserialization/introspection (`Cow::Owned`)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignKey {
    /// Parent table name
    pub table: Cow<'static, str>,

    /// Constraint name
    pub name: Cow<'static, str>,

    /// Source columns
    pub columns: Cow<'static, [Cow<'static, str>]>,

    /// Referenced table name
    pub table_to: Cow<'static, str>,

    /// Referenced columns
    pub columns_to: Cow<'static, [Cow<'static, str>]>,

    /// ON DELETE action (as SQL string)
    pub on_delete: Option<Cow<'static, str>>,

    /// ON UPDATE action (as SQL string)
    pub on_update: Option<Cow<'static, str>>,

    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
}

impl ForeignKey {
    /// Create a new foreign key
    #[must_use]
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        name: impl Into<Cow<'static, str>>,
        columns: impl Into<Cow<'static, [Cow<'static, str>]>>,
        table_to: impl Into<Cow<'static, str>>,
        columns_to: impl Into<Cow<'static, [Cow<'static, str>]>>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            columns: columns.into(),
            table_to: table_to.into(),
            columns_to: columns_to.into(),
            on_delete: None,
            on_update: None,
            name_explicit: false,
        }
    }

    /// Create a new foreign key from owned strings (convenience for runtime construction)
    #[cfg(feature = "std")]
    #[must_use]
    pub fn from_strings(
        table: String,
        name: String,
        columns: Vec<String>,
        table_to: String,
        columns_to: Vec<String>,
    ) -> ForeignKey {
        ForeignKey {
            table: Cow::Owned(table),
            name: Cow::Owned(name),
            columns: Cow::Owned(columns.into_iter().map(Cow::Owned).collect()),
            table_to: Cow::Owned(table_to),
            columns_to: Cow::Owned(columns_to.into_iter().map(Cow::Owned).collect()),
            on_delete: None,
            on_update: None,
            name_explicit: false,
        }
    }

    /// Set ON DELETE action
    #[must_use]
    pub fn on_delete(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        self.on_delete = Some(action.into());
        self
    }

    /// Set ON UPDATE action
    #[must_use]
    pub fn on_update(mut self, action: impl Into<Cow<'static, str>>) -> Self {
        self.on_update = Some(action.into());
        self
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

impl Default for ForeignKey {
    fn default() -> Self {
        Self::new(
            "",
            "",
            &[] as &[Cow<'static, str>],
            "",
            &[] as &[Cow<'static, str>],
        )
    }
}

impl From<ForeignKeyDef> for ForeignKey {
    fn from(def: ForeignKeyDef) -> Self {
        def.into_foreign_key()
    }
}

// =============================================================================
// Serde Implementation
// =============================================================================

#[cfg(feature = "serde")]
mod serde_impl {
    use super::*;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    impl Serialize for ForeignKey {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("ForeignKey", 8)?;
            state.serialize_field("table", &*self.table)?;
            state.serialize_field("name", &*self.name)?;
            let cols: Vec<&str> = self.columns.iter().map(|c| c.as_ref()).collect();
            state.serialize_field("columns", &cols)?;
            state.serialize_field("tableTo", &*self.table_to)?;
            let cols_to: Vec<&str> = self.columns_to.iter().map(|c| c.as_ref()).collect();
            state.serialize_field("columnsTo", &cols_to)?;
            if let Some(ref action) = self.on_delete {
                state.serialize_field("onDelete", &**action)?;
            }
            if let Some(ref action) = self.on_update {
                state.serialize_field("onUpdate", &**action)?;
            }
            state.serialize_field("nameExplicit", &self.name_explicit)?;
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for ForeignKey {
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
                table_to: String,
                #[serde(default)]
                columns_to: Vec<String>,
                #[serde(default)]
                on_delete: Option<String>,
                #[serde(default)]
                on_update: Option<String>,
                #[serde(default)]
                name_explicit: bool,
            }

            let helper = Helper::deserialize(deserializer)?;
            Ok(ForeignKey {
                table: Cow::Owned(helper.table),
                name: Cow::Owned(helper.name),
                columns: Cow::Owned(helper.columns.into_iter().map(Cow::Owned).collect()),
                table_to: Cow::Owned(helper.table_to),
                columns_to: Cow::Owned(helper.columns_to.into_iter().map(Cow::Owned).collect()),
                on_delete: helper.on_delete.map(Cow::Owned),
                on_update: helper.on_update.map(Cow::Owned),
                name_explicit: helper.name_explicit,
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_const_foreign_key_def() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
        const REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];

        const FK: ForeignKeyDef = ForeignKeyDef::new("posts", "fk_posts_user")
            .columns(COLS)
            .references("users", REFS)
            .on_delete(ReferentialAction::Cascade);

        assert_eq!(FK.name, "fk_posts_user");
        assert_eq!(FK.table, "posts");
        assert_eq!(FK.on_delete, Some(ReferentialAction::Cascade));
    }

    #[test]
    fn test_foreign_key_def_to_foreign_key() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
        const REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];

        const DEF: ForeignKeyDef = ForeignKeyDef::new("posts", "fk_posts_user")
            .columns(COLS)
            .references("users", REFS);

        let fk = DEF.into_foreign_key();
        assert_eq!(fk.name(), "fk_posts_user");
        assert_eq!(fk.columns.len(), 1);
    }

    #[test]
    fn test_const_into_foreign_key() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
        const REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];

        const DEF: ForeignKeyDef = ForeignKeyDef::new("posts", "fk_posts_user")
            .columns(COLS)
            .references("users", REFS);
        const FK: ForeignKey = DEF.into_foreign_key();

        assert_eq!(&*FK.name, "fk_posts_user");
        assert_eq!(FK.columns.len(), 1);
    }

    #[test]
    fn test_from_strings() {
        let fk = ForeignKey::from_strings(
            "posts".to_string(),
            "fk_posts_user".to_string(),
            vec!["user_id".to_string()],
            "users".to_string(),
            vec!["id".to_string()],
        );
        assert_eq!(fk.table(), "posts");
        assert_eq!(fk.name(), "fk_posts_user");
        assert_eq!(fk.columns.len(), 1);
    }

    #[test]
    fn test_referential_action_sql() {
        assert_eq!(ReferentialAction::Cascade.as_sql(), "CASCADE");
        assert_eq!(ReferentialAction::SetNull.as_sql(), "SET NULL");
    }
}
