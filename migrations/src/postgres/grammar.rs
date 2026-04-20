//! `PostgreSQL` SQL type grammar and naming conventions
//!
//! This module provides type checking, naming conventions, and default value
//! handling for `PostgreSQL` columns matching drizzle-kit grammar.ts

// =============================================================================
// Naming Conventions
// =============================================================================

/// Generate default name for a primary key constraint
#[must_use]
pub fn default_name_for_pk(table: &str) -> String {
    format!("{table}_pkey")
}

/// Generate default name for a foreign key constraint
#[must_use]
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
            format!("{table}_{hash}_fkey")
        } else {
            format!("{hash}_fkey")
        }
    } else {
        desired
    }
}

/// Generate default name for a unique constraint
#[must_use]
pub fn default_name_for_unique(table: &str, columns: &[String]) -> String {
    format!("{}_{}_key", table, columns.join("_"))
}

/// Generate default name for an index
#[must_use]
pub fn default_name_for_index(table: &str, columns: &[String]) -> String {
    format!("{}_{}_idx", table, columns.join("_"))
}

/// Generate default name for an identity sequence
#[must_use]
pub fn default_name_for_identity_sequence(table: &str, column: &str) -> String {
    format!("{table}_{column}_seq")
}

/// Generate default name for a check constraint
#[must_use]
pub fn default_name_for_check(table: &str, index: usize) -> String {
    format!("{table}_check_{index}")
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

/// `PostgreSQL` SQL type category
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
    /// Match serial and integer types. Serial aliases must be checked first so
    /// `smallserial` isn't misclassified as `smallint`.
    fn match_numeric(s: &str) -> Option<Self> {
        // Serial aliases first (prefix collides with integer types).
        if s.starts_with("smallserial") {
            return Some(Self::SmallSerial);
        }
        if s.starts_with("bigserial") {
            return Some(Self::BigSerial);
        }
        if s.starts_with("serial") {
            return Some(Self::Serial);
        }

        if s.starts_with("smallint") || s == "int2" {
            return Some(Self::SmallInt);
        }
        if s.starts_with("integer") || s == "int" || s == "int4" {
            return Some(Self::Integer);
        }
        if s.starts_with("bigint") || s == "int8" {
            return Some(Self::BigInt);
        }
        if s.starts_with("numeric") || s.starts_with("decimal") {
            return Some(Self::Numeric);
        }
        if s.starts_with("real") || s == "float4" {
            return Some(Self::Real);
        }
        if s.starts_with("double") {
            return Some(Self::DoublePrecision);
        }
        if s.starts_with("boolean") || s == "bool" {
            return Some(Self::Boolean);
        }
        None
    }

    /// Match string and JSON types. `varchar`/`character varying` must be
    /// checked before `char`/`character`; `jsonb` before `json`.
    fn match_string_or_json(s: &str) -> Option<Self> {
        if s.starts_with("varchar") || s.starts_with("character varying") {
            return Some(Self::Varchar);
        }
        if s.starts_with("char") || s.starts_with("character") {
            return Some(Self::Char);
        }
        if s.starts_with("text") {
            return Some(Self::Text);
        }
        if s.starts_with("jsonb") {
            return Some(Self::Jsonb);
        }
        if s.starts_with("json") {
            return Some(Self::Json);
        }
        None
    }

    /// Match time/date types. The `with time zone` variants are checked before
    /// the base `timestamp` / `time` prefixes.
    fn match_temporal(s: &str) -> Option<Self> {
        if s.starts_with("timestamp") && s.contains("with time zone") {
            return Some(Self::TimestampTz);
        }
        if s.starts_with("timestamp") {
            return Some(Self::Timestamp);
        }
        if s.starts_with("time") && s.contains("with time zone") {
            return Some(Self::TimeTz);
        }
        if s.starts_with("time") {
            return Some(Self::Time);
        }
        if s.starts_with("date") {
            return Some(Self::Date);
        }
        if s.starts_with("interval") {
            return Some(Self::Interval);
        }
        None
    }

    /// Match network, vector, bit, geometric and other specialized types.
    fn match_specialized(s: &str) -> Option<Self> {
        if s.starts_with("uuid") {
            return Some(Self::Uuid);
        }
        if s.starts_with("inet") {
            return Some(Self::Inet);
        }
        if s.starts_with("cidr") {
            return Some(Self::Cidr);
        }
        // macaddr8 must be matched before macaddr
        if s.starts_with("macaddr8") {
            return Some(Self::MacAddr8);
        }
        if s.starts_with("macaddr") {
            return Some(Self::MacAddr);
        }
        if s.starts_with("vector") {
            return Some(Self::Vector);
        }
        if s.starts_with("halfvec") {
            return Some(Self::HalfVec);
        }
        if s.starts_with("sparsevec") {
            return Some(Self::SparseVec);
        }
        if s.starts_with("bit") {
            return Some(Self::Bit);
        }
        if s.starts_with("geometry") {
            return Some(Self::Geometry);
        }
        if s.starts_with("point") {
            return Some(Self::Point);
        }
        if s.starts_with("line") {
            return Some(Self::Line);
        }
        None
    }

    /// Determine the type category for a SQL type string
    #[must_use]
    pub fn from_sql_type(sql_type: &str) -> Self {
        let s = sql_type.trim().to_lowercase();

        Self::match_numeric(&s)
            .or_else(|| Self::match_string_or_json(&s))
            .or_else(|| Self::match_temporal(&s))
            .or_else(|| Self::match_specialized(&s))
            .unwrap_or(Self::Custom)
    }

    /// Get the drizzle import name for this type
    #[must_use]
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
    #[must_use]
    pub const fn is_serial(&self) -> bool {
        matches!(self, Self::Serial | Self::SmallSerial | Self::BigSerial)
    }
}

// =============================================================================
// Type Parsing Utilities
// =============================================================================

/// Extract parameters from a type like "varchar(255)" or "numeric(10,2)"
#[must_use]
pub fn parse_type_params(sql_type: &str) -> Option<(String, Option<String>)> {
    let start = sql_type.find('(')?;
    let end = sql_type.find(')')?;
    let params = &sql_type[start + 1..end];

    let parts: Vec<&str> = params.split(',').map(str::trim).collect();
    match parts.len() {
        1 => Some((parts[0].to_string(), None)),
        2 => Some((parts[0].to_string(), Some(parts[1].to_string()))),
        _ => None,
    }
}

/// Split SQL type into base type and options
#[must_use]
pub fn split_sql_type(sql_type: &str) -> (String, Option<String>) {
    let normalized = sql_type.replace("[]", "");

    if let Some(start) = normalized.find('(')
        && let Some(end) = normalized.find(')')
    {
        let base = normalized[..start].trim().to_string();
        let options = normalized[start + 1..end].replace(", ", ",");
        return (base, Some(options));
    }

    (normalized.trim().to_string(), None)
}

/// Trim a character from both ends of a string
#[must_use]
pub fn trim_char(s: &str, c: char) -> &str {
    s.trim_start_matches(c).trim_end_matches(c)
}

/// Check if a string is a serial expression
#[must_use]
pub fn is_serial_expression(expr: &str, schema: &str) -> bool {
    let schema_prefix = if schema == "public" {
        String::new()
    } else {
        format!("{schema}.")
    };

    (expr.starts_with(&format!("nextval('{schema_prefix}"))
        || expr.starts_with(&format!("nextval('\"{schema_prefix}")))
        && (expr.ends_with("_seq'::regclass)") || expr.ends_with("_seq\"'::regclass)"))
}

/// Extract the sequence name from a `nextval('...'::regclass)` expression.
///
/// Returns just the sequence name (without schema prefix or quotes):
/// - `nextval('users_id_seq'::regclass)` → `users_id_seq`
/// - `nextval('public.users_id_seq'::regclass)` → `users_id_seq`
/// - `nextval('"myschema"."users_id_seq"'::regclass)` → `users_id_seq`
#[must_use]
pub fn extract_nextval_sequence(expr: &str) -> Option<String> {
    let inner = expr
        .strip_prefix("nextval('")?
        .strip_suffix("'::regclass)")?;
    let name_part = inner.rfind('.').map_or(inner, |pos| &inner[pos + 1..]);
    let name = name_part.trim_matches('"');
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
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

    /// Get the maximum value for an identity column based on type.
    ///
    /// Falls back to the `integer` range for unknown/unspecified types.
    #[must_use]
    pub fn max_for(column_type: &str) -> &'static str {
        match column_type {
            "smallint" => "32767",
            "bigint" => "9223372036854775807",
            // "integer" and fallback share the same range
            _ => "2147483647",
        }
    }

    /// Get the minimum value for an identity column based on type.
    ///
    /// Falls back to the `integer` range for unknown/unspecified types.
    #[must_use]
    pub fn min_for(column_type: &str) -> &'static str {
        match column_type {
            "smallint" => "-32768",
            "bigint" => "-9223372036854775808",
            // "integer" and fallback share the same range
            _ => "-2147483648",
        }
    }
}

// =============================================================================
// System Checks
// =============================================================================

/// System namespace names that should be skipped
pub const SYSTEM_NAMESPACE_NAMES: &[&str] = &["pg_toast", "pg_catalog", "information_schema"];

/// Check if a namespace is a system namespace
#[must_use]
pub fn is_system_namespace(name: &str) -> bool {
    name.starts_with("pg_toast")
        || name == "pg_default"
        || name == "pg_global"
        || name.starts_with("pg_temp_")
        || SYSTEM_NAMESPACE_NAMES.contains(&name)
}

/// Check if a role is a system role
#[must_use]
pub fn is_system_role(name: &str) -> bool {
    name == "postgres" || name.starts_with("pg_")
}

/// Check if an action is the default (NO ACTION)
#[must_use]
pub const fn is_default_action(action: &str) -> bool {
    action.eq_ignore_ascii_case("no action")
}

// =============================================================================
// Default Values
// =============================================================================

/// `PostgreSQL` default values and settings
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
#[must_use]
pub fn parse_check_definition(value: &str) -> String {
    value
        .trim_start_matches("CHECK ((")
        .trim_end_matches("))")
        .to_string()
}

/// Parse a VIEW definition.
///
/// Callers with `Option<&str>` can pair this with [`Option::map`].
#[must_use]
pub fn parse_view_definition(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .trim_end_matches(';')
        .to_string()
}

/// Parse ON DELETE/UPDATE action from `PostgreSQL` code.
///
/// Unknown codes and the canonical `"a"` (no action) both map to `"NO ACTION"`.
#[must_use]
pub fn parse_on_type(code: &str) -> &'static str {
    match code {
        "r" => "RESTRICT",
        "n" => "SET NULL",
        "c" => "CASCADE",
        "d" => "SET DEFAULT",
        // "a" and any unknown code default to "NO ACTION"
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

    #[test]
    fn test_from_sql_type_serial_vs_integer() {
        // These must NOT be classified as serial
        assert_eq!(
            PgTypeCategory::from_sql_type("integer"),
            PgTypeCategory::Integer
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("int"),
            PgTypeCategory::Integer
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("int4"),
            PgTypeCategory::Integer
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("bigint"),
            PgTypeCategory::BigInt
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("int8"),
            PgTypeCategory::BigInt
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("smallint"),
            PgTypeCategory::SmallInt
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("int2"),
            PgTypeCategory::SmallInt
        );

        // These must be serial
        assert_eq!(
            PgTypeCategory::from_sql_type("serial"),
            PgTypeCategory::Serial
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("SERIAL"),
            PgTypeCategory::Serial
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("bigserial"),
            PgTypeCategory::BigSerial
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("smallserial"),
            PgTypeCategory::SmallSerial
        );

        assert!(PgTypeCategory::Serial.is_serial());
        assert!(PgTypeCategory::BigSerial.is_serial());
        assert!(PgTypeCategory::SmallSerial.is_serial());
        assert!(!PgTypeCategory::Integer.is_serial());
        assert!(!PgTypeCategory::BigInt.is_serial());
    }

    #[test]
    fn test_from_sql_type_common() {
        assert_eq!(PgTypeCategory::from_sql_type("text"), PgTypeCategory::Text);
        assert_eq!(
            PgTypeCategory::from_sql_type("varchar(255)"),
            PgTypeCategory::Varchar
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("boolean"),
            PgTypeCategory::Boolean
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("bool"),
            PgTypeCategory::Boolean
        );
        assert_eq!(PgTypeCategory::from_sql_type("uuid"), PgTypeCategory::Uuid);
        assert_eq!(
            PgTypeCategory::from_sql_type("jsonb"),
            PgTypeCategory::Jsonb
        );
        assert_eq!(PgTypeCategory::from_sql_type("json"), PgTypeCategory::Json);
        assert_eq!(
            PgTypeCategory::from_sql_type("timestamp with time zone"),
            PgTypeCategory::TimestampTz
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("timestamp"),
            PgTypeCategory::Timestamp
        );
        assert_eq!(PgTypeCategory::from_sql_type("date"), PgTypeCategory::Date);
        assert_eq!(
            PgTypeCategory::from_sql_type("numeric(10,2)"),
            PgTypeCategory::Numeric
        );
        assert_eq!(PgTypeCategory::from_sql_type("real"), PgTypeCategory::Real);
        assert_eq!(
            PgTypeCategory::from_sql_type("double precision"),
            PgTypeCategory::DoublePrecision
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("macaddr8"),
            PgTypeCategory::MacAddr8
        );
        assert_eq!(
            PgTypeCategory::from_sql_type("macaddr"),
            PgTypeCategory::MacAddr
        );
    }

    #[test]
    fn test_extract_nextval_sequence() {
        assert_eq!(
            extract_nextval_sequence("nextval('users_id_seq'::regclass)"),
            Some("users_id_seq".to_string())
        );
        assert_eq!(
            extract_nextval_sequence("nextval('public.users_id_seq'::regclass)"),
            Some("users_id_seq".to_string())
        );
        assert_eq!(
            extract_nextval_sequence("nextval('\"myschema\".\"users_id_seq\"'::regclass)"),
            Some("users_id_seq".to_string())
        );
        assert_eq!(extract_nextval_sequence("not_a_nextval"), None);
        assert_eq!(extract_nextval_sequence("nextval(''::regclass)"), None);
    }
}
