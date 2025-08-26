//! Owned PostgreSQL value types for static lifetime scenarios

use crate::PostgresValue;
use drizzle_core::{SQLParam, error::DrizzleError};
use std::borrow::Cow;
#[cfg(feature = "uuid")]
use uuid::Uuid;

/// Owned version of PostgresValue that doesn't borrow data
#[derive(Debug, Clone, PartialEq)]
pub enum OwnedPostgresValue {
    /// INTEGER, BIGINT values
    Integer(i64),
    /// REAL, DOUBLE PRECISION values  
    Real(f64),
    /// TEXT, VARCHAR, CHAR values (owned)
    Text(String),
    /// BYTEA values (owned binary data)
    Bytea(Vec<u8>),
    /// BOOLEAN values
    Boolean(bool),
    /// UUID values
    #[cfg(feature = "uuid")]
    Uuid(Uuid),
    /// JSON/JSONB values
    #[cfg(feature = "serde")]
    Json(serde_json::Value),
    /// NULL value
    Null,
}

impl SQLParam for OwnedPostgresValue {}

impl Default for OwnedPostgresValue {
    fn default() -> Self {
        OwnedPostgresValue::Null
    }
}

impl std::fmt::Display for OwnedPostgresValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            OwnedPostgresValue::Integer(i) => i.to_string(),
            OwnedPostgresValue::Real(r) => r.to_string(),
            OwnedPostgresValue::Text(s) => s.clone(),
            OwnedPostgresValue::Bytea(b) => format!(
                "\\x{}",
                b.iter().map(|b| format!("{:02x}", b)).collect::<String>()
            ),
            OwnedPostgresValue::Boolean(b) => b.to_string(),
            #[cfg(feature = "uuid")]
            OwnedPostgresValue::Uuid(uuid) => uuid.to_string(),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Json(json) => json.to_string(),
            OwnedPostgresValue::Null => String::new(),
        };
        write!(f, "{value}")
    }
}

// Conversions from PostgresValue to OwnedPostgresValue
impl<'a> From<PostgresValue<'a>> for OwnedPostgresValue {
    fn from(value: PostgresValue<'a>) -> Self {
        match value {
            PostgresValue::Integer(i) => OwnedPostgresValue::Integer(i),
            PostgresValue::Real(r) => OwnedPostgresValue::Real(r),
            PostgresValue::Text(cow) => OwnedPostgresValue::Text(cow.into_owned()),
            PostgresValue::Bytea(cow) => OwnedPostgresValue::Bytea(cow.into_owned()),
            PostgresValue::Boolean(b) => OwnedPostgresValue::Boolean(b),
            #[cfg(feature = "uuid")]
            PostgresValue::Uuid(uuid) => OwnedPostgresValue::Uuid(uuid),
            #[cfg(feature = "serde")]
            PostgresValue::Json(json) => OwnedPostgresValue::Json(json),
            PostgresValue::Enum(enum_val) => {
                OwnedPostgresValue::Text(enum_val.variant_name().to_string())
            }
            PostgresValue::Null => OwnedPostgresValue::Null,
            PostgresValue::Array(postgres_values) => todo!(),
        }
    }
}

// Conversions from OwnedPostgresValue to PostgresValue
impl<'a> From<OwnedPostgresValue> for PostgresValue<'a> {
    fn from(value: OwnedPostgresValue) -> Self {
        match value {
            OwnedPostgresValue::Integer(i) => PostgresValue::Integer(i),
            OwnedPostgresValue::Real(r) => PostgresValue::Real(r),
            OwnedPostgresValue::Text(s) => PostgresValue::Text(Cow::Owned(s)),
            OwnedPostgresValue::Bytea(b) => PostgresValue::Bytea(Cow::Owned(b)),
            OwnedPostgresValue::Boolean(b) => PostgresValue::Boolean(b),
            #[cfg(feature = "uuid")]
            OwnedPostgresValue::Uuid(uuid) => PostgresValue::Uuid(uuid),
            #[cfg(feature = "serde")]
            OwnedPostgresValue::Json(json) => PostgresValue::Json(json),
            OwnedPostgresValue::Null => PostgresValue::Null,
        }
    }
}

// Direct conversions from Rust types to OwnedPostgresValue
impl From<i64> for OwnedPostgresValue {
    fn from(value: i64) -> Self {
        OwnedPostgresValue::Integer(value)
    }
}

impl From<f64> for OwnedPostgresValue {
    fn from(value: f64) -> Self {
        OwnedPostgresValue::Real(value)
    }
}

impl From<String> for OwnedPostgresValue {
    fn from(value: String) -> Self {
        OwnedPostgresValue::Text(value)
    }
}

impl From<Vec<u8>> for OwnedPostgresValue {
    fn from(value: Vec<u8>) -> Self {
        OwnedPostgresValue::Bytea(value)
    }
}

impl From<bool> for OwnedPostgresValue {
    fn from(value: bool) -> Self {
        OwnedPostgresValue::Boolean(value)
    }
}

#[cfg(feature = "uuid")]
impl From<Uuid> for OwnedPostgresValue {
    fn from(value: Uuid) -> Self {
        OwnedPostgresValue::Uuid(value)
    }
}

#[cfg(feature = "serde")]
impl From<serde_json::Value> for OwnedPostgresValue {
    fn from(value: serde_json::Value) -> Self {
        OwnedPostgresValue::Json(value)
    }
}

// TryFrom conversions back to Rust types
impl TryFrom<OwnedPostgresValue> for i64 {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Integer(i) => Ok(i),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to i64",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for f64 {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Real(r) => Ok(r),
            OwnedPostgresValue::Integer(i) => Ok(i as f64),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to f64",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for String {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Text(s) => Ok(s),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to String",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for Vec<u8> {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Bytea(b) => Ok(b),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to Vec<u8>",
                value
            ))),
        }
    }
}

impl TryFrom<OwnedPostgresValue> for bool {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Boolean(b) => Ok(b),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to bool",
                value
            ))),
        }
    }
}

#[cfg(feature = "uuid")]
impl TryFrom<OwnedPostgresValue> for Uuid {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Uuid(uuid) => Ok(uuid),
            OwnedPostgresValue::Text(s) => Ok(Uuid::parse_str(&s)?),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to UUID",
                value
            ))),
        }
    }
}

#[cfg(feature = "serde")]
impl TryFrom<OwnedPostgresValue> for serde_json::Value {
    type Error = DrizzleError;

    fn try_from(value: OwnedPostgresValue) -> Result<Self, Self::Error> {
        match value {
            OwnedPostgresValue::Json(json) => Ok(json),
            OwnedPostgresValue::Text(s) => serde_json::from_str(&s)
                .map_err(|e| DrizzleError::ConversionError(format!("Failed to parse JSON: {}", e))),
            _ => Err(DrizzleError::ConversionError(format!(
                "Cannot convert {:?} to JSON",
                value
            ))),
        }
    }
}
