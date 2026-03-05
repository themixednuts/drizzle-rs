use crate::alloc_prelude::Cow;

/// Identifier casing strategy for inferred names.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]
pub enum Casing {
    /// `camelCase` (e.g. `userId`, `createdAt`).
    #[default]
    #[cfg_attr(feature = "serde", serde(rename = "camelCase"))]
    CamelCase,
    /// `snake_case` (e.g. `user_id`, `created_at`).
    #[cfg_attr(feature = "serde", serde(rename = "snake_case"))]
    SnakeCase,
}

impl Casing {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CamelCase => "camelCase",
            Self::SnakeCase => "snake_case",
        }
    }
}

impl core::fmt::Display for Casing {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl core::str::FromStr for Casing {
    type Err = crate::alloc_prelude::String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "camelCase" | "camel" => Ok(Self::CamelCase),
            "snake_case" | "snake" => Ok(Self::SnakeCase),
            _ => Err(format!(
                "invalid casing '{s}', expected 'camelCase' or 'snake_case'"
            )),
        }
    }
}

/// Shared migration metadata configuration.
///
/// This contains only tracking metadata and can be reused by higher-level crates
/// (CLI, runtime migrator, etc.) without pulling in migration runtime logic.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MigrationTracking {
    /// Migrations tracking table name.
    pub table: Cow<'static, str>,
    /// Optional schema name for the tracking table (PostgreSQL).
    pub schema: Option<Cow<'static, str>>,
}

impl MigrationTracking {
    /// Default SQLite migration tracking metadata.
    pub const SQLITE: Self = Self {
        table: Cow::Borrowed("__drizzle_migrations"),
        schema: None,
    };

    /// Default PostgreSQL migration tracking metadata.
    pub const POSTGRES: Self = Self {
        table: Cow::Borrowed("__drizzle_migrations"),
        schema: Some(Cow::Borrowed("drizzle")),
    };

    /// Create tracking metadata from table/schema values.
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        schema: Option<impl Into<Cow<'static, str>>>,
    ) -> Self {
        Self {
            table: table.into(),
            schema: schema.map(Into::into),
        }
    }

    /// Override table name while preserving schema.
    pub fn table(mut self, table: impl Into<Cow<'static, str>>) -> Self {
        self.table = table.into();
        self
    }

    /// Override schema while preserving table name.
    pub fn schema(mut self, schema: impl Into<Cow<'static, str>>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Clear the schema while preserving table name.
    pub fn without_schema(mut self) -> Self {
        self.schema = None;
        self
    }
}
impl Default for MigrationTracking {
    fn default() -> Self {
        Self::SQLITE
    }
}
