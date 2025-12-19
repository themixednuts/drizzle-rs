//! PostgreSQL Foreign Key DDL types
//!
//! See: <https://github.com/drizzle-team/drizzle-orm/blob/beta/drizzle-kit/src/dialects/postgres/ddl.ts>

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
/// use drizzle_types::postgres::ddl::{ForeignKeyDef, ReferentialAction};
/// use std::borrow::Cow;
///
/// const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
/// const REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
///
/// const FK: ForeignKeyDef = ForeignKeyDef::new("public", "posts", "fk_posts_user")
///     .columns(COLS)
///     .references("public", "users", REFS)
///     .on_delete(ReferentialAction::Cascade);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct ForeignKeyDef {
    /// Schema name
    pub schema: &'static str,
    /// Parent table name
    pub table: &'static str,
    /// Constraint name
    pub name: &'static str,
    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
    /// Source columns
    pub columns: &'static [Cow<'static, str>],
    /// Referenced schema name
    pub schema_to: &'static str,
    /// Referenced table name
    pub table_to: &'static str,
    /// Referenced columns
    pub columns_to: &'static [Cow<'static, str>],
    /// ON DELETE action
    pub on_delete: Option<ReferentialAction>,
    /// ON UPDATE action
    pub on_update: Option<ReferentialAction>,
}

impl ForeignKeyDef {
    /// Create a new foreign key definition
    #[must_use]
    pub const fn new(schema: &'static str, table: &'static str, name: &'static str) -> Self {
        Self {
            schema,
            table,
            name,
            name_explicit: false,
            columns: &[],
            schema_to: "public",
            table_to: "",
            columns_to: &[],
            on_delete: None,
            on_update: None,
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

    /// Set referenced table and columns
    #[must_use]
    pub const fn references(
        self,
        schema_to: &'static str,
        table_to: &'static str,
        cols_to: &'static [Cow<'static, str>],
    ) -> Self {
        Self {
            schema_to,
            table_to,
            columns_to: cols_to,
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
    pub const fn into_foreign_key(self) -> ForeignKey<'static> {
        ForeignKey {
            schema: Cow::Borrowed(self.schema),
            table: Cow::Borrowed(self.table),
            name: Cow::Borrowed(self.name),
            name_explicit: self.name_explicit,
            columns: Cow::Borrowed(self.columns),
            schema_to: Cow::Borrowed(self.schema_to),
            table_to: Cow::Borrowed(self.table_to),
            columns_to: Cow::Borrowed(self.columns_to),
            on_update: match self.on_update {
                Some(a) => Some(Cow::Borrowed(a.as_sql())),
                None => None,
            },
            on_delete: match self.on_delete {
                Some(a) => Some(Cow::Borrowed(a.as_sql())),
                None => None,
            },
        }
    }
}

impl Default for ForeignKeyDef {
    fn default() -> Self {
        Self::new("public", "", "")
    }
}

// =============================================================================
// Runtime Type for Serde
// =============================================================================

/// Runtime foreign key constraint entity
///
/// The lifetime parameter allows this type to work with both static (const) data
/// and owned (runtime) data.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ForeignKey<'a> {
    /// Schema name
    pub schema: Cow<'a, str>,

    /// Parent table name
    pub table: Cow<'a, str>,

    /// Constraint name
    pub name: Cow<'a, str>,

    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,

    /// Source columns
    pub columns: Cow<'a, [Cow<'a, str>]>,

    /// Referenced schema name
    pub schema_to: Cow<'a, str>,

    /// Referenced table name
    pub table_to: Cow<'a, str>,

    /// Referenced columns
    pub columns_to: Cow<'a, [Cow<'a, str>]>,

    /// ON UPDATE action
    pub on_update: Option<Cow<'a, str>>,

    /// ON DELETE action
    pub on_delete: Option<Cow<'a, str>>,
}

impl<'a> ForeignKey<'a> {
    /// Create a new foreign key
    #[must_use]
    pub fn new(
        schema: impl Into<Cow<'a, str>>,
        table: impl Into<Cow<'a, str>>,
        name: impl Into<Cow<'a, str>>,
    ) -> Self {
        Self {
            schema: schema.into(),
            table: table.into(),
            name: name.into(),
            name_explicit: false,
            columns: Cow::Borrowed(&[]),
            schema_to: Cow::Borrowed("public"),
            table_to: Cow::Borrowed(""),
            columns_to: Cow::Borrowed(&[]),
            on_update: None,
            on_delete: None,
        }
    }

    /// Create a new foreign key from owned strings (convenience for runtime construction)
    #[cfg(feature = "std")]
    #[must_use]
    pub fn from_strings(
        schema: String,
        table: String,
        name: String,
        columns: Vec<String>,
        schema_to: String,
        table_to: String,
        columns_to: Vec<String>,
    ) -> ForeignKey<'static> {
        ForeignKey {
            schema: Cow::Owned(schema),
            table: Cow::Owned(table),
            name: Cow::Owned(name),
            name_explicit: false,
            columns: Cow::Owned(columns.into_iter().map(Cow::Owned).collect()),
            schema_to: Cow::Owned(schema_to),
            table_to: Cow::Owned(table_to),
            columns_to: Cow::Owned(columns_to.into_iter().map(Cow::Owned).collect()),
            on_update: None,
            on_delete: None,
        }
    }

    /// Set ON DELETE action
    #[must_use]
    pub fn on_delete(mut self, action: impl Into<Cow<'a, str>>) -> Self {
        self.on_delete = Some(action.into());
        self
    }

    /// Set ON UPDATE action
    #[must_use]
    pub fn on_update(mut self, action: impl Into<Cow<'a, str>>) -> Self {
        self.on_update = Some(action.into());
        self
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

    /// Get the referenced table name
    #[inline]
    #[must_use]
    pub fn table_to(&self) -> &str {
        &self.table_to
    }

    /// Get the referenced schema name
    #[inline]
    #[must_use]
    pub fn schema_to(&self) -> &str {
        &self.schema_to
    }

    /// Convert to a static lifetime version by converting to owned data
    #[cfg(feature = "std")]
    #[must_use]
    pub fn into_static(self) -> ForeignKey<'static> {
        ForeignKey {
            schema: Cow::Owned(self.schema.into_owned()),
            table: Cow::Owned(self.table.into_owned()),
            name: Cow::Owned(self.name.into_owned()),
            name_explicit: self.name_explicit,
            columns: Cow::Owned(
                self.columns
                    .into_owned()
                    .into_iter()
                    .map(|c| Cow::Owned(c.into_owned()))
                    .collect(),
            ),
            schema_to: Cow::Owned(self.schema_to.into_owned()),
            table_to: Cow::Owned(self.table_to.into_owned()),
            columns_to: Cow::Owned(
                self.columns_to
                    .into_owned()
                    .into_iter()
                    .map(|c| Cow::Owned(c.into_owned()))
                    .collect(),
            ),
            on_update: self.on_update.map(|c| Cow::Owned(c.into_owned())),
            on_delete: self.on_delete.map(|c| Cow::Owned(c.into_owned())),
        }
    }
}

impl Default for ForeignKey<'static> {
    fn default() -> Self {
        Self::new("public", "", "")
    }
}

impl From<ForeignKeyDef> for ForeignKey<'static> {
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

    impl Serialize for ForeignKey<'_> {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            use serde::ser::SerializeStruct;
            let mut state = serializer.serialize_struct("ForeignKey", 10)?;
            state.serialize_field("schema", &*self.schema)?;
            state.serialize_field("table", &*self.table)?;
            state.serialize_field("name", &*self.name)?;
            state.serialize_field("nameExplicit", &self.name_explicit)?;
            let cols: Vec<&str> = self.columns.iter().map(|c| c.as_ref()).collect();
            state.serialize_field("columns", &cols)?;
            state.serialize_field("schemaTo", &*self.schema_to)?;
            state.serialize_field("tableTo", &*self.table_to)?;
            let cols_to: Vec<&str> = self.columns_to.iter().map(|c| c.as_ref()).collect();
            state.serialize_field("columnsTo", &cols_to)?;
            if let Some(ref action) = self.on_update {
                state.serialize_field("onUpdate", &**action)?;
            }
            if let Some(ref action) = self.on_delete {
                state.serialize_field("onDelete", &**action)?;
            }
            state.end()
        }
    }

    impl<'de> Deserialize<'de> for ForeignKey<'static> {
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
                name_explicit: bool,
                #[serde(default)]
                columns: Vec<String>,
                schema_to: String,
                table_to: String,
                #[serde(default)]
                columns_to: Vec<String>,
                #[serde(default)]
                on_update: Option<String>,
                #[serde(default)]
                on_delete: Option<String>,
            }

            let helper = Helper::deserialize(deserializer)?;
            Ok(ForeignKey {
                schema: Cow::Owned(helper.schema),
                table: Cow::Owned(helper.table),
                name: Cow::Owned(helper.name),
                name_explicit: helper.name_explicit,
                columns: Cow::Owned(helper.columns.into_iter().map(Cow::Owned).collect()),
                schema_to: Cow::Owned(helper.schema_to),
                table_to: Cow::Owned(helper.table_to),
                columns_to: Cow::Owned(helper.columns_to.into_iter().map(Cow::Owned).collect()),
                on_update: helper.on_update.map(Cow::Owned),
                on_delete: helper.on_delete.map(Cow::Owned),
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

        const FK: ForeignKeyDef = ForeignKeyDef::new("public", "posts", "fk_posts_user")
            .columns(COLS)
            .references("public", "users", REFS)
            .on_delete(ReferentialAction::Cascade);

        assert_eq!(FK.name, "fk_posts_user");
        assert_eq!(FK.table, "posts");
        assert_eq!(FK.columns.len(), 1);
    }

    #[test]
    fn test_foreign_key_def_to_foreign_key() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
        const REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];

        const DEF: ForeignKeyDef = ForeignKeyDef::new("public", "posts", "fk")
            .columns(COLS)
            .references("public", "users", REFS);

        let fk = DEF.into_foreign_key();
        assert_eq!(fk.name(), "fk");
        assert_eq!(fk.columns.len(), 1);
    }

    #[test]
    fn test_const_into_foreign_key() {
        const COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
        const REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];

        const DEF: ForeignKeyDef = ForeignKeyDef::new("public", "posts", "fk_posts_user")
            .columns(COLS)
            .references("public", "users", REFS);
        const FK: ForeignKey<'static> = DEF.into_foreign_key();

        assert_eq!(&*FK.name, "fk_posts_user");
        assert_eq!(FK.columns.len(), 1);
    }

    #[test]
    fn test_from_strings() {
        let fk = ForeignKey::from_strings(
            "public".to_string(),
            "posts".to_string(),
            "fk_posts_user".to_string(),
            vec!["user_id".to_string()],
            "public".to_string(),
            "users".to_string(),
            vec!["id".to_string()],
        );
        assert_eq!(fk.schema(), "public");
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
