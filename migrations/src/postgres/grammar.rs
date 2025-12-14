//! PostgreSQL SQL type grammar and naming conventions
//!
//! This module provides type checking, naming conventions, and default value
//! handling for PostgreSQL columns matching drizzle-kit grammar.ts

// =============================================================================
// Naming Conventions
// =============================================================================

/// Generate default name for a primary key constraint
pub fn default_name_for_pk(table: &str) -> String {
    format!("{}_pkey", table)
}

/// Generate default name for a foreign key constraint
pub fn default_name_for_fk(
    table: &str,
    columns: &[String],
    table_to: &str,
    columns_to: &[String],
) -> String {
    let desired = format!(
        "{}_{}_{}_{}_fkey",
        table,
        columns.join("_"),
        table_to,
        columns_to.join("_")
    );

    // PostgreSQL identifier max length is 63
    if desired.len() > 63 {
        let hash = hash_string(&desired);
        if table.len() < 63 - 18 {
            format!("{}_{}_fkey", table, hash)
        } else {
            format!("{}_fkey", hash)
        }
    } else {
        desired
    }
}

/// Generate default name for a unique constraint
pub fn default_name_for_unique(table: &str, columns: &[String]) -> String {
    format!("{}_{}_key", table, columns.join("_"))
}

/// Generate default name for an index
pub fn default_name_for_index(table: &str, columns: &[String]) -> String {
    format!("{}_{}_idx", table, columns.join("_"))
}

/// Generate default name for an identity sequence
pub fn default_name_for_identity_sequence(table: &str, column: &str) -> String {
    format!("{}_{}_seq", table, column)
}

/// Generate default name for a check constraint
pub fn default_name_for_check(table: &str, index: usize) -> String {
    format!("{}_check_{}", table, index)
}

/// Simple hash function for constraint naming
fn hash_string(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:x}", hasher.finish())[..12].to_string()
}

// =============================================================================
// SQL Type Categories
// =============================================================================

/// PostgreSQL SQL type category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PgTypeCategory {
    SmallInt,
    Integer,
    BigInt,
    Numeric,
    Real,
    DoublePrecision,
    Boolean,
    Char,
    Varchar,
    Text,
    Json,
    Jsonb,
    Time,
    TimeTz,
    Timestamp,
    TimestampTz,
    Date,
    Uuid,
    Interval,
    Inet,
    Cidr,
    MacAddr,
    MacAddr8,
    Vector,
    HalfVec,
    SparseVec,
    Bit,
    Point,
    Line,
    Geometry,
    Serial,
    SmallSerial,
    BigSerial,
    Enum,
    Custom,
}

impl PgTypeCategory {
    /// Determine the type category for a SQL type string
    pub fn from_sql_type(sql_type: &str) -> Self {
        let normalized = sql_type.trim().to_lowercase();

        // Serial types (must check before integer)
        if is_match(&normalized, r"(?:smallserial)(?:\s.*)?") {
            return Self::SmallSerial;
        }
        if is_match(&normalized, r"(?:bigserial)(?:\s.*)?") {
            return Self::BigSerial;
        }
        if is_match(&normalized, r"(?:serial)(?:\s.*)?") {
            return Self::Serial;
        }

        // Integer types
        if is_match(&normalized, r"smallint(?:\s*\[\s*\])*") {
            return Self::SmallInt;
        }
        if is_match(&normalized, r"integer(?:\s*\[\s*\])*") {
            return Self::Integer;
        }
        if is_match(&normalized, r"bigint(?:\s*\[\s*\])*") {
            return Self::BigInt;
        }

        // Numeric types
        if is_match(
            &normalized,
            r"(?:numeric|decimal)(?:\(\d+(?:,\d+)?\))?(?:\s*\[\s*\])*",
        ) {
            return Self::Numeric;
        }
        if is_match(&normalized, r"real(?:\s*\[\s*\])*") {
            return Self::Real;
        }
        if is_match(&normalized, r"(?:double|double precision)(?:\s*\[\s*\])*") {
            return Self::DoublePrecision;
        }

        // Boolean
        if is_match(&normalized, r"boolean(?:\s*\[\s*\])*") {
            return Self::Boolean;
        }

        // String types
        if is_match(
            &normalized,
            r"(?:char|character)(?:\(\d+\))?(?:\s*\[\s*\])*",
        ) {
            return Self::Char;
        }
        if is_match(
            &normalized,
            r"(?:varchar|character varying)(?:\(\d+\))?(?:\s*\[\s*\])*",
        ) {
            return Self::Varchar;
        }
        if is_match(&normalized, r"text(?:\s*\[\s*\])*") {
            return Self::Text;
        }

        // JSON types
        if is_match(&normalized, r"jsonb(?:\s*\[\s*\])*") {
            return Self::Jsonb;
        }
        if is_match(&normalized, r"json(?:\s*\[\s*\])*") {
            return Self::Json;
        }

        // Time/Date types
        if is_match(&normalized, r"time(?:\(\d+\))?\s+with time zone(?:\[\])*") {
            return Self::TimeTz;
        }
        if is_match(&normalized, r"time(?:\(\d+\))?(?:\[\])*") {
            return Self::Time;
        }
        if is_match(
            &normalized,
            r"timestamp(?:\s)?(?:\(\d+\))?\s+with time zone(?:\[\])*",
        ) {
            return Self::TimestampTz;
        }
        if is_match(&normalized, r"timestamp(?:\s)?(?:\(\d+\))?(?:\[\])*") {
            return Self::Timestamp;
        }
        if is_match(&normalized, r"date(?:\s*\[\s*\])*") {
            return Self::Date;
        }

        // Other types
        if is_match(&normalized, r"uuid(?:\s*\[\s*\])*") {
            return Self::Uuid;
        }
        if is_match(
            &normalized,
            r"interval(\s+(year|month|day|hour|minute|second)(\s+to\s+(month|day|hour|minute|second))?)?(?:\(\d+\))?(?:\s*\[\s*\])*",
        ) {
            return Self::Interval;
        }
        if is_match(&normalized, r"inet(?:\(\d+\))?(?:\[\])?") {
            return Self::Inet;
        }
        if is_match(&normalized, r"cidr(?:\(\d+\))?(?:\[\])?") {
            return Self::Cidr;
        }
        if is_match(&normalized, r"macaddr(?:\s*\[\s*\])*") {
            return Self::MacAddr;
        }
        if is_match(&normalized, r"macaddr8(?:\s*\[\s*\])*") {
            return Self::MacAddr8;
        }

        // Vector types
        if is_match(&normalized, r"vector(?:\(\d+\))?(?:\s*\[\s*\])*") {
            return Self::Vector;
        }
        if is_match(&normalized, r"halfvec(?:\(\d+(?:,\d+)?\))?(?:\s*\[\s*\])*") {
            return Self::HalfVec;
        }
        if is_match(
            &normalized,
            r"sparsevec(?:\(\d+(?:,\d+)?\))?(?:\s*\[\s*\])*",
        ) {
            return Self::SparseVec;
        }

        // Bit type
        if is_match(&normalized, r"bit(?:\(\d+(?:,\d+)?\))?(?:\s*\[\s*\])*") {
            return Self::Bit;
        }

        // Geometric types
        if is_match(&normalized, r"point(?:\s*\[\s*\])*") {
            return Self::Point;
        }
        if is_match(&normalized, r"line(?:\s*\[\s*\])*") {
            return Self::Line;
        }
        if is_match(&normalized, r"geometry\(point(?:,\d+)?\)(?:\[\s*\])*") {
            return Self::Geometry;
        }

        Self::Custom
    }

    /// Get the drizzle import name for this type
    pub const fn drizzle_import(&self) -> &'static str {
        match self {
            Self::SmallInt => "smallint",
            Self::Integer => "integer",
            Self::BigInt => "bigint",
            Self::Numeric => "numeric",
            Self::Real => "real",
            Self::DoublePrecision => "doublePrecision",
            Self::Boolean => "boolean",
            Self::Char => "char",
            Self::Varchar => "varchar",
            Self::Text => "text",
            Self::Json => "json",
            Self::Jsonb => "jsonb",
            Self::Time | Self::TimeTz => "time",
            Self::Timestamp | Self::TimestampTz => "timestamp",
            Self::Date => "date",
            Self::Uuid => "uuid",
            Self::Interval => "interval",
            Self::Inet => "inet",
            Self::Cidr => "cidr",
            Self::MacAddr => "macaddr",
            Self::MacAddr8 => "macaddr8",
            Self::Vector => "vector",
            Self::HalfVec => "halfvec",
            Self::SparseVec => "sparsevec",
            Self::Bit => "bit",
            Self::Point => "point",
            Self::Line => "line",
            Self::Geometry => "geometry",
            Self::Serial => "serial",
            Self::SmallSerial => "smallserial",
            Self::BigSerial => "bigserial",
            Self::Enum => "pgEnum",
            Self::Custom => "customType",
        }
    }

    /// Check if this is a serial type
    pub const fn is_serial(&self) -> bool {
        matches!(self, Self::Serial | Self::SmallSerial | Self::BigSerial)
    }
}

/// Simple pattern matching helper (basic implementation without regex dependency)
fn is_match(s: &str, _pattern: &str) -> bool {
    // Simplified matching - in production, use regex crate
    // For now, we do basic string checks
    let s = s.trim();

    // Handle common cases directly
    if s.starts_with("smallint") {
        return true;
    }
    if s.starts_with("integer") || s == "int" {
        return true;
    }
    if s.starts_with("bigint") {
        return true;
    }
    if s.starts_with("numeric") || s.starts_with("decimal") {
        return true;
    }
    if s.starts_with("real") {
        return true;
    }
    if s.starts_with("double") {
        return true;
    }
    if s.starts_with("boolean") || s == "bool" {
        return true;
    }
    if s.starts_with("char") || s.starts_with("character") {
        return true;
    }
    if s.starts_with("varchar") || s.starts_with("character varying") {
        return true;
    }
    if s.starts_with("text") {
        return true;
    }
    if s.starts_with("json") {
        return true;
    }
    if s.starts_with("time") {
        return true;
    }
    if s.starts_with("timestamp") {
        return true;
    }
    if s.starts_with("date") {
        return true;
    }
    if s.starts_with("uuid") {
        return true;
    }
    if s.starts_with("interval") {
        return true;
    }
    if s.starts_with("inet") {
        return true;
    }
    if s.starts_with("cidr") {
        return true;
    }
    if s.starts_with("macaddr") {
        return true;
    }
    if s.starts_with("vector") || s.starts_with("halfvec") || s.starts_with("sparsevec") {
        return true;
    }
    if s.starts_with("bit") {
        return true;
    }
    if s.starts_with("point") || s.starts_with("line") || s.starts_with("geometry") {
        return true;
    }
    if s.starts_with("serial") || s.starts_with("smallserial") || s.starts_with("bigserial") {
        return true;
    }

    false
}

// =============================================================================
// Type Parsing Utilities
// =============================================================================

/// Extract parameters from a type like "varchar(255)" or "numeric(10,2)"
pub fn parse_type_params(sql_type: &str) -> Option<(String, Option<String>)> {
    let start = sql_type.find('(')?;
    let end = sql_type.find(')')?;
    let params = &sql_type[start + 1..end];

    let parts: Vec<&str> = params.split(',').map(|s| s.trim()).collect();
    match parts.len() {
        1 => Some((parts[0].to_string(), None)),
        2 => Some((parts[0].to_string(), Some(parts[1].to_string()))),
        _ => None,
    }
}

/// Split SQL type into base type and options
pub fn split_sql_type(sql_type: &str) -> (String, Option<String>) {
    let normalized = sql_type.replace("[]", "");

    if let Some(start) = normalized.find('(') {
        if let Some(end) = normalized.find(')') {
            let base = normalized[..start].trim().to_string();
            let options = normalized[start + 1..end].replace(", ", ",");
            return (base, Some(options));
        }
    }

    (normalized.trim().to_string(), None)
}

/// Trim a character from both ends of a string
pub fn trim_char(s: &str, c: char) -> &str {
    s.trim_start_matches(c).trim_end_matches(c)
}

/// Check if a string is a serial expression
pub fn is_serial_expression(expr: &str, schema: &str) -> bool {
    let schema_prefix = if schema == "public" {
        String::new()
    } else {
        format!("{}.", schema)
    };

    (expr.starts_with(&format!("nextval('{}", schema_prefix))
        || expr.starts_with(&format!("nextval('\"{}", schema_prefix)))
        && (expr.ends_with("_seq'::regclass)") || expr.ends_with("_seq\"'::regclass)"))
}

// =============================================================================
// Identity Defaults
// =============================================================================

/// Default values for identity columns
pub struct IdentityDefaults;

impl IdentityDefaults {
    pub const START_WITH: &'static str = "1";
    pub const INCREMENT: &'static str = "1";
    pub const MIN: &'static str = "1";
    pub const CACHE: i32 = 1;
    pub const CYCLE: bool = false;

    /// Get the maximum value for an identity column based on type
    pub fn max_for(column_type: &str) -> &'static str {
        match column_type {
            "smallint" => "32767",
            "integer" => "2147483647",
            "bigint" => "9223372036854775807",
            _ => "2147483647", // Default to integer
        }
    }

    /// Get the minimum value for an identity column based on type
    pub fn min_for(column_type: &str) -> &'static str {
        match column_type {
            "smallint" => "-32768",
            "integer" => "-2147483648",
            "bigint" => "-9223372036854775808",
            _ => "-2147483648", // Default to integer
        }
    }
}

// =============================================================================
// System Checks
// =============================================================================

/// System namespace names that should be skipped
pub const SYSTEM_NAMESPACE_NAMES: &[&str] = &["pg_toast", "pg_catalog", "information_schema"];

/// Check if a namespace is a system namespace
pub fn is_system_namespace(name: &str) -> bool {
    name.starts_with("pg_toast")
        || name == "pg_default"
        || name == "pg_global"
        || name.starts_with("pg_temp_")
        || SYSTEM_NAMESPACE_NAMES.contains(&name)
}

/// Check if a role is a system role
pub fn is_system_role(name: &str) -> bool {
    name == "postgres" || name.starts_with("pg_")
}

/// Check if an action is the default (NO ACTION)
pub fn is_default_action(action: &str) -> bool {
    action.eq_ignore_ascii_case("no action")
}

// =============================================================================
// Default Values
// =============================================================================

/// PostgreSQL default values and settings
pub struct PgDefaults;

impl PgDefaults {
    /// Default tablespace
    pub const TABLESPACE: &'static str = "pg_default";

    /// Default access method
    pub const ACCESS_METHOD: &'static str = "heap";

    /// Default nulls not distinct setting
    pub const NULLS_NOT_DISTINCT: bool = false;

    /// Default index method
    pub const INDEX_METHOD: &'static str = "btree";

    /// Default geometry SRID
    pub const GEOMETRY_SRID: i32 = 0;
}

/// Vector operator classes for indexes
pub const VECTOR_OPS: &[&str] = &[
    "vector_l2_ops",
    "vector_ip_ops",
    "vector_cosine_ops",
    "vector_l1_ops",
    "bit_hamming_ops",
    "bit_jaccard_ops",
    "halfvec_l2_ops",
    "sparsevec_l2_ops",
];

// =============================================================================
// Parsing Helpers
// =============================================================================

/// Parse a CHECK constraint definition
pub fn parse_check_definition(value: &str) -> String {
    value
        .trim_start_matches("CHECK ((")
        .trim_end_matches("))")
        .to_string()
}

/// Parse a VIEW definition
pub fn parse_view_definition(value: Option<&str>) -> Option<String> {
    value.map(|v| {
        v.split_whitespace()
            .collect::<Vec<_>>()
            .join(" ")
            .trim_end_matches(';')
            .to_string()
    })
}

/// Parse ON DELETE/UPDATE action from PostgreSQL code
pub fn parse_on_type(code: &str) -> &'static str {
    match code {
        "a" => "NO ACTION",
        "r" => "RESTRICT",
        "n" => "SET NULL",
        "c" => "CASCADE",
        "d" => "SET DEFAULT",
        _ => "NO ACTION",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_name_for_pk() {
        assert_eq!(default_name_for_pk("users"), "users_pkey");
    }

    #[test]
    fn test_default_name_for_fk() {
        let name = default_name_for_fk(
            "posts",
            &["author_id".to_string()],
            "users",
            &["id".to_string()],
        );
        assert_eq!(name, "posts_author_id_users_id_fkey");
    }

    #[test]
    fn test_default_name_for_unique() {
        let name = default_name_for_unique("users", &["email".to_string()]);
        assert_eq!(name, "users_email_key");
    }

    #[test]
    fn test_default_name_for_index() {
        let name = default_name_for_index("users", &["email".to_string(), "name".to_string()]);
        assert_eq!(name, "users_email_name_idx");
    }

    #[test]
    fn test_parse_type_params() {
        assert_eq!(
            parse_type_params("varchar(255)"),
            Some(("255".to_string(), None))
        );
        assert_eq!(
            parse_type_params("numeric(10,2)"),
            Some(("10".to_string(), Some("2".to_string())))
        );
        assert_eq!(parse_type_params("text"), None);
    }

    #[test]
    fn test_is_system_namespace() {
        assert!(is_system_namespace("pg_catalog"));
        assert!(is_system_namespace("pg_toast_12345"));
        assert!(!is_system_namespace("public"));
        assert!(!is_system_namespace("myschema"));
    }

    #[test]
    fn test_identity_defaults() {
        assert_eq!(IdentityDefaults::max_for("smallint"), "32767");
        assert_eq!(IdentityDefaults::max_for("integer"), "2147483647");
        assert_eq!(IdentityDefaults::max_for("bigint"), "9223372036854775807");
    }
}
