use crate::alloc_prelude::*;

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
    #[must_use]
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
    /// Optional schema name for the tracking table (`PostgreSQL`).
    pub schema: Option<Cow<'static, str>>,
}

impl MigrationTracking {
    /// Default `SQLite` migration tracking metadata.
    pub const SQLITE: Self = Self {
        table: Cow::Borrowed("__drizzle_migrations"),
        schema: None,
    };

    /// Default `PostgreSQL` migration tracking metadata.
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
    #[must_use]
    pub fn table(mut self, table: impl Into<Cow<'static, str>>) -> Self {
        self.table = table.into();
        self
    }

    /// Override schema while preserving table name.
    #[must_use]
    pub fn schema(mut self, schema: impl Into<Cow<'static, str>>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Clear the schema while preserving table name.
    #[must_use]
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

/// A value that's either a literal string or an env-var reference.
///
/// In TOML this deserializes from `"literal"` or `{ env = "VAR_NAME" }` — the
/// same shape `drizzle-kit` and the CLI accept for `dbCredentials.url`. Used
/// anywhere a config value can be either inline or pulled from the
/// environment at runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvOr {
    /// Literal value taken from the config file.
    Value(String),
    /// Name of the environment variable to resolve.
    Env(String),
}

#[cfg(feature = "std")]
impl EnvOr {
    /// Resolve to a concrete string, reading the environment if needed.
    ///
    /// # Errors
    ///
    /// Returns [`EnvOrError::NotPresent`] if this is an [`EnvOr::Env`] pointing
    /// to a variable that is not set, or [`EnvOrError::NotUnicode`] if the
    /// variable is set but contains invalid UTF-8.
    pub fn resolve(&self) -> Result<String, EnvOrError> {
        match self {
            Self::Value(v) => Ok(v.clone()),
            Self::Env(var) => match std::env::var(var) {
                Ok(v) => Ok(v),
                Err(std::env::VarError::NotPresent) => Err(EnvOrError::NotPresent(var.clone())),
                Err(std::env::VarError::NotUnicode(_)) => Err(EnvOrError::NotUnicode(var.clone())),
            },
        }
    }

    /// Resolve to an optional value (returns `None` when an `Env` var is unset).
    ///
    /// # Errors
    ///
    /// Returns [`EnvOrError::NotUnicode`] if the env var is set but contains
    /// invalid UTF-8. Missing env vars resolve to `Ok(None)`.
    pub fn resolve_optional(&self) -> Result<Option<String>, EnvOrError> {
        match self {
            Self::Value(v) => Ok(Some(v.clone())),
            Self::Env(var) => match std::env::var(var) {
                Ok(v) => Ok(Some(v)),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(std::env::VarError::NotUnicode(_)) => Err(EnvOrError::NotUnicode(var.clone())),
            },
        }
    }
}

/// Failure resolving an [`EnvOr::Env`] reference.
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnvOrError {
    /// The named environment variable is not set in the process.
    NotPresent(String),
    /// The named environment variable is set but contains non-UTF-8 bytes.
    NotUnicode(String),
}

#[cfg(feature = "std")]
impl core::fmt::Display for EnvOrError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotPresent(var) => write!(f, "env var `{var}` not set"),
            Self::NotUnicode(var) => write!(f, "env var `{var}` contains invalid unicode"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for EnvOrError {}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for EnvOr {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct EnvOrVisitor;

        impl<'de> Visitor<'de> for EnvOrVisitor {
            type Value = EnvOr;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a string or { env = \"VAR_NAME\" }")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(EnvOr::Value(value.to_string()))
            }

            fn visit_map<M>(self, mut map: M) -> Result<Self::Value, M::Error>
            where
                M: MapAccess<'de>,
            {
                let mut env_var: Option<String> = None;

                while let Some(key) = map.next_key::<String>()? {
                    if key == "env" {
                        env_var = Some(map.next_value()?);
                    } else {
                        return Err(de::Error::unknown_field(&key, &["env"]));
                    }
                }

                env_var
                    .map(EnvOr::Env)
                    .ok_or_else(|| de::Error::missing_field("env"))
            }
        }

        deserializer.deserialize_any(EnvOrVisitor)
    }
}

#[cfg(feature = "schemars")]
impl schemars::JsonSchema for EnvOr {
    fn schema_name() -> Cow<'static, str> {
        "EnvOr".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        use schemars::json_schema;

        // EnvOr accepts either a plain string or { env: "VAR_NAME" }
        json_schema!({
            "oneOf": [
                generator.subschema_for::<String>(),
                {
                    "type": "object",
                    "properties": {
                        "env": { "type": "string" }
                    },
                    "required": ["env"],
                    "additionalProperties": false
                }
            ]
        })
    }
}
