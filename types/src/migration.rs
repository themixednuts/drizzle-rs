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
    /// Returns the config spelling: `"camelCase"` or `"snake_case"`.
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

/// Where applied migrations are recorded: the tracking table and its schema.
///
/// Shared by the CLI and the runtime migrator. Each dialect has a default
/// constant; adjust it with the builder methods.
///
/// # Examples
///
/// ```
/// use drizzle_types::MigrationTracking;
///
/// let tracking = MigrationTracking::POSTGRES.table("schema_history");
/// assert_eq!(tracking.table, "schema_history");
/// assert_eq!(tracking.schema.as_deref(), Some("drizzle"));
/// ```
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MigrationTracking {
    /// Migrations tracking table name.
    pub table: Cow<'static, str>,
    /// Schema of the tracking table (used by `PostgreSQL`; `None` elsewhere).
    pub schema: Option<Cow<'static, str>>,
}

impl MigrationTracking {
    /// `SQLite` default: table `__drizzle_migrations`, no schema.
    pub const SQLITE: Self = Self {
        table: Cow::Borrowed("__drizzle_migrations"),
        schema: None,
    };

    /// `PostgreSQL` default: table `__drizzle_migrations` in schema `drizzle`.
    pub const POSTGRES: Self = Self {
        table: Cow::Borrowed("__drizzle_migrations"),
        schema: Some(Cow::Borrowed("drizzle")),
    };

    /// `MySQL` default: table `__drizzle_migrations` in the connection's database.
    pub const MYSQL: Self = Self {
        table: Cow::Borrowed("__drizzle_migrations"),
        schema: None,
    };

    /// Creates tracking metadata from a table name and an optional schema.
    pub fn new(
        table: impl Into<Cow<'static, str>>,
        schema: Option<impl Into<Cow<'static, str>>>,
    ) -> Self {
        Self {
            table: table.into(),
            schema: schema.map(Into::into),
        }
    }

    /// Replaces the table name, keeping the schema.
    #[must_use]
    pub fn table(mut self, table: impl Into<Cow<'static, str>>) -> Self {
        self.table = table.into();
        self
    }

    /// Sets the schema, keeping the table name.
    #[must_use]
    pub fn schema(mut self, schema: impl Into<Cow<'static, str>>) -> Self {
        self.schema = Some(schema.into());
        self
    }

    /// Removes the schema, keeping the table name.
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

/// A config value written inline or read from an environment variable.
///
/// With the `serde` feature it deserializes from `"literal"` or
/// `{ env = "VAR_NAME" }`, the same shape `drizzle-kit` and the CLI accept
/// for `dbCredentials.url`.
///
/// # Examples
///
/// ```
/// use drizzle_types::ConfigValue;
///
/// let url = ConfigValue::Inline("postgres://localhost/app".into());
/// assert_eq!(url.resolve().unwrap(), "postgres://localhost/app");
/// ```
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValue {
    /// Value written inline in the config file.
    Inline(String),
    /// Name of the environment variable to resolve.
    Env(String),
}

#[cfg(feature = "std")]
impl ConfigValue {
    /// Returns the value, reading the environment variable for [`ConfigValue::Env`].
    ///
    /// # Errors
    ///
    /// Returns [`ConfigValueError::NotPresent`] if this is a [`ConfigValue::Env`] pointing
    /// to a variable that is not set, or [`ConfigValueError::NotUnicode`] if the
    /// variable is set but contains invalid UTF-8.
    pub fn resolve(&self) -> Result<String, ConfigValueError> {
        match self {
            Self::Inline(v) => Ok(v.clone()),
            Self::Env(var) => match std::env::var(var) {
                Ok(v) => Ok(v),
                Err(std::env::VarError::NotPresent) => {
                    Err(ConfigValueError::NotPresent(var.clone()))
                }
                Err(std::env::VarError::NotUnicode(_)) => {
                    Err(ConfigValueError::NotUnicode(var.clone()))
                }
            },
        }
    }

    /// Returns the value, or `None` when a [`ConfigValue::Env`] variable is unset.
    ///
    /// # Errors
    ///
    /// Returns [`ConfigValueError::NotUnicode`] if the env var is set but contains
    /// invalid UTF-8. Missing env vars resolve to `Ok(None)`.
    pub fn resolve_optional(&self) -> Result<Option<String>, ConfigValueError> {
        match self {
            Self::Inline(v) => Ok(Some(v.clone())),
            Self::Env(var) => match std::env::var(var) {
                Ok(v) => Ok(Some(v)),
                Err(std::env::VarError::NotPresent) => Ok(None),
                Err(std::env::VarError::NotUnicode(_)) => {
                    Err(ConfigValueError::NotUnicode(var.clone()))
                }
            },
        }
    }
}

/// Failure resolving a [`ConfigValue::Env`] reference.
#[cfg(feature = "std")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigValueError {
    /// The named environment variable is not set in the process.
    NotPresent(String),
    /// The named environment variable is set but contains non-UTF-8 bytes.
    NotUnicode(String),
}

#[cfg(feature = "std")]
impl core::fmt::Display for ConfigValueError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self {
            Self::NotPresent(var) => write!(f, "env var `{var}` not set"),
            Self::NotUnicode(var) => write!(f, "env var `{var}` contains invalid unicode"),
        }
    }
}

#[cfg(feature = "std")]
impl std::error::Error for ConfigValueError {}

#[cfg(feature = "serde")]
impl<'de> serde::Deserialize<'de> for ConfigValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};

        struct ConfigValueVisitor;

        impl<'de> Visitor<'de> for ConfigValueVisitor {
            type Value = ConfigValue;

            fn expecting(&self, formatter: &mut core::fmt::Formatter) -> core::fmt::Result {
                formatter.write_str("a string or { env = \"VAR_NAME\" }")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                Ok(ConfigValue::Inline(value.to_string()))
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
                    .map(ConfigValue::Env)
                    .ok_or_else(|| de::Error::missing_field("env"))
            }
        }

        deserializer.deserialize_any(ConfigValueVisitor)
    }
}

#[cfg(feature = "schemars")]
impl schemars::JsonSchema for ConfigValue {
    fn schema_name() -> Cow<'static, str> {
        "ConfigValue".into()
    }

    fn json_schema(generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        use schemars::json_schema;

        // ConfigValue accepts either a plain string or { env: "VAR_NAME" }
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
