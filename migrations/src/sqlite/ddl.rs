//! SQLite DDL entity types for drizzle-kit beta v7 format
//!
//! These types represent the flat DDL entity array format used in drizzle-kit beta.
//! Each entity has an `entityType` discriminator field and belongs to a table via reference.

use crate::traits::{Entity, EntityKey, EntityKind};
use serde::{Deserialize, Serialize};

// =============================================================================
// Entity Type Constants
// =============================================================================

/// Entity type discriminator for tables
pub const ENTITY_TYPE_TABLES: &str = "tables";
/// Entity type discriminator for columns  
pub const ENTITY_TYPE_COLUMNS: &str = "columns";
/// Entity type discriminator for indexes
pub const ENTITY_TYPE_INDEXES: &str = "indexes";
/// Entity type discriminator for foreign keys
pub const ENTITY_TYPE_FKS: &str = "fks";
/// Entity type discriminator for primary keys
pub const ENTITY_TYPE_PKS: &str = "pks";
/// Entity type discriminator for unique constraints
pub const ENTITY_TYPE_UNIQUES: &str = "uniques";
/// Entity type discriminator for check constraints
pub const ENTITY_TYPE_CHECKS: &str = "checks";
/// Entity type discriminator for views
pub const ENTITY_TYPE_VIEWS: &str = "views";

// =============================================================================
// DDL Entity Types
// =============================================================================

/// Table entity - represents a table definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    /// Table name
    pub name: String,
    /// Is this a STRICT table?
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub strict: bool,
    /// Is this a WITHOUT ROWID table?
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub without_rowid: bool,
}

impl Table {
    /// Create a new table entity
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            strict: false,
            without_rowid: false,
        }
    }

    /// Set STRICT mode
    pub fn strict(mut self) -> Self {
        self.strict = true;
        self
    }

    /// Set WITHOUT ROWID mode
    pub fn without_rowid(mut self) -> Self {
        self.without_rowid = true;
        self
    }
}

/// Column entity - represents a column definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    /// Parent table name
    pub table: String,
    /// Column name
    pub name: String,
    /// SQL type (e.g., "integer", "text", "real", "blob")
    #[serde(rename = "type")]
    pub sql_type: String,
    /// Is this column NOT NULL?
    pub not_null: bool,
    /// Is this column AUTOINCREMENT?
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autoincrement: Option<bool>,
    /// Default value as string
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Generated column configuration
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<Generated>,
}

impl Column {
    /// Create a new column entity
    pub fn new(
        table: impl Into<String>,
        name: impl Into<String>,
        sql_type: impl Into<String>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            sql_type: sql_type.into(),
            not_null: false,
            autoincrement: None,
            default: None,
            generated: None,
        }
    }

    /// Set NOT NULL
    pub fn not_null(mut self) -> Self {
        self.not_null = true;
        self
    }

    /// Set AUTOINCREMENT
    pub fn autoincrement(mut self) -> Self {
        self.autoincrement = Some(true);
        self
    }

    /// Set default value
    pub fn default_value(mut self, value: impl Into<String>) -> Self {
        self.default = Some(value.into());
        self
    }
}

/// Generated column configuration
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    /// SQL expression for generation
    #[serde(rename = "as")]
    pub expression: String,
    /// Generation type: "stored" or "virtual"
    #[serde(rename = "type")]
    pub gen_type: GeneratedType,
}

/// Generated column type
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum GeneratedType {
    Stored,
    Virtual,
}

/// Index column definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexColumn {
    /// Column name or expression
    pub value: String,
    /// Whether this is an expression (vs column name)
    pub is_expression: bool,
}

/// Index origin - how the index was created
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum IndexOrigin {
    /// Manually created via CREATE INDEX
    Manual,
    /// Auto-created for UNIQUE constraint
    Auto,
}

/// Index entity - represents an index definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    /// Parent table name
    pub table: String,
    /// Index name
    pub name: String,
    /// Columns included in the index
    pub columns: Vec<IndexColumn>,
    /// Is this a unique index?
    pub is_unique: bool,
    /// WHERE clause for partial indexes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    /// How the index was created
    pub origin: IndexOrigin,
}

impl Index {
    /// Create a new index entity
    pub fn new(
        table: impl Into<String>,
        name: impl Into<String>,
        columns: Vec<IndexColumn>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            columns,
            is_unique: false,
            r#where: None,
            origin: IndexOrigin::Manual,
        }
    }

    /// Make this a unique index
    pub fn unique(mut self) -> Self {
        self.is_unique = true;
        self
    }
}

/// Foreign key entity - represents a foreign key constraint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKey {
    /// Parent table name
    pub table: String,
    /// Constraint name
    pub name: String,
    /// Columns in the source table
    pub columns: Vec<String>,
    /// Referenced table name
    pub table_to: String,
    /// Columns in the referenced table
    pub columns_to: Vec<String>,
    /// ON DELETE action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<String>,
    /// ON UPDATE action
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<String>,
    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
}

impl ForeignKey {
    /// Create a new foreign key entity
    pub fn new(
        table: impl Into<String>,
        name: impl Into<String>,
        columns: Vec<String>,
        table_to: impl Into<String>,
        columns_to: Vec<String>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            columns,
            table_to: table_to.into(),
            columns_to,
            on_delete: None,
            on_update: None,
            name_explicit: false,
        }
    }
}

/// Primary key entity - represents a primary key constraint
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryKey {
    /// Parent table name
    pub table: String,
    /// Constraint name
    pub name: String,
    /// Columns in the primary key
    pub columns: Vec<String>,
    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
}

impl PrimaryKey {
    /// Create a new primary key entity
    pub fn new(table: impl Into<String>, name: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            columns,
            name_explicit: false,
        }
    }
}

/// Unique constraint entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UniqueConstraint {
    /// Parent table name
    pub table: String,
    /// Constraint name
    pub name: String,
    /// Columns in the unique constraint
    pub columns: Vec<String>,
    /// Whether the constraint name was explicitly specified
    pub name_explicit: bool,
}

impl UniqueConstraint {
    /// Create a new unique constraint entity
    pub fn new(table: impl Into<String>, name: impl Into<String>, columns: Vec<String>) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            columns,
            name_explicit: false,
        }
    }
}

/// Check constraint entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckConstraint {
    /// Parent table name
    pub table: String,
    /// Constraint name
    pub name: String,
    /// Check expression
    pub value: String,
}

impl CheckConstraint {
    /// Create a new check constraint entity
    pub fn new(
        table: impl Into<String>,
        name: impl Into<String>,
        value: impl Into<String>,
    ) -> Self {
        Self {
            table: table.into(),
            name: name.into(),
            value: value.into(),
        }
    }
}

/// View entity - represents a view definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct View {
    /// View name
    pub name: String,
    /// View definition (AS SELECT ...)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    /// Whether this is an existing view (not managed by drizzle)
    #[serde(default)]
    pub is_existing: bool,
}

impl View {
    /// Create a new view entity
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            definition: None,
            is_existing: false,
        }
    }
}

// =============================================================================
// Unified Entity Enum
// =============================================================================

/// Unified SQLite DDL entity enum for serialization
///
/// Uses internally-tagged enum representation where `entityType` discriminates variants.
/// This replaces the need for `entity_type: String` fields on each DDL struct.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "entityType")]
pub enum SqliteEntity {
    #[serde(rename = "tables")]
    Table(Table),
    #[serde(rename = "columns")]
    Column(Column),
    #[serde(rename = "indexes")]
    Index(Index),
    #[serde(rename = "fks")]
    ForeignKey(ForeignKey),
    #[serde(rename = "pks")]
    PrimaryKey(PrimaryKey),
    #[serde(rename = "uniques")]
    UniqueConstraint(UniqueConstraint),
    #[serde(rename = "checks")]
    CheckConstraint(CheckConstraint),
    #[serde(rename = "views")]
    View(View),
}

impl SqliteEntity {
    /// Get the entity kind for this entity
    pub fn kind(&self) -> EntityKind {
        match self {
            SqliteEntity::Table(_) => EntityKind::Table,
            SqliteEntity::Column(_) => EntityKind::Column,
            SqliteEntity::Index(_) => EntityKind::Index,
            SqliteEntity::ForeignKey(_) => EntityKind::ForeignKey,
            SqliteEntity::PrimaryKey(_) => EntityKind::PrimaryKey,
            SqliteEntity::UniqueConstraint(_) => EntityKind::UniqueConstraint,
            SqliteEntity::CheckConstraint(_) => EntityKind::CheckConstraint,
            SqliteEntity::View(_) => EntityKind::View,
        }
    }
}

// =============================================================================
// Naming Helpers (matching drizzle-kit grammar.ts patterns)
// =============================================================================

/// Generate a default name for a foreign key constraint
pub fn name_for_fk(
    table: &str,
    columns: &[String],
    table_to: &str,
    columns_to: &[String],
) -> String {
    format!(
        "fk_{}_{}_{}_{}_fk",
        table,
        columns.join("_"),
        table_to,
        columns_to.join("_")
    )
}

/// Generate a default name for a unique constraint
pub fn name_for_unique(table: &str, columns: &[String]) -> String {
    format!("{}_{}_unique", table, columns.join("_"))
}

/// Generate a default name for a primary key constraint
pub fn name_for_pk(table: &str) -> String {
    format!("{}_pk", table)
}

/// Generate a default name for an index
pub fn name_for_index(table: &str, columns: &[String]) -> String {
    format!("{}_{}_idx", table, columns.join("_"))
}

/// Generate a default name for a check constraint
pub fn name_for_check(table: &str, index: usize) -> String {
    format!("{}_check_{}", table, index)
}

// =============================================================================
// SQL Type Affinities (matching grammar.ts)
// =============================================================================

/// Integer type affinities
const INT_AFFINITIES: &[&str] = &[
    "int",
    "integer",
    "tinyint",
    "smallint",
    "mediumint",
    "bigint",
    "unsigned big int",
    "int2",
    "int8",
];

/// Real number type affinities
const REAL_AFFINITIES: &[&str] = &["real", "double", "double precision", "float"];

/// Numeric type affinities
const NUMERIC_AFFINITIES: &[&str] = &["numeric", "decimal", "boolean", "date", "datetime"];

/// Text type affinities
const TEXT_AFFINITIES: &[&str] = &[
    "text",
    "character",
    "varchar",
    "varying character",
    "nchar",
    "native character",
    "nvarchar",
    "clob",
];

/// SQL type category
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SqlTypeCategory {
    Integer,
    Real,
    Numeric,
    Text,
    Blob,
}

impl SqlTypeCategory {
    /// Determine the type category for a SQL type string
    pub fn from_sql_type(sql_type: &str) -> Self {
        let lowered = sql_type.to_lowercase();

        // Check integer affinities
        if INT_AFFINITIES
            .iter()
            .any(|a| lowered == *a || lowered.starts_with(&format!("{}(", a)))
        {
            return Self::Integer;
        }

        // Check real affinities
        if REAL_AFFINITIES
            .iter()
            .any(|a| lowered == *a || lowered.starts_with(&format!("{}(", a)))
        {
            return Self::Real;
        }

        // Check numeric affinities
        if NUMERIC_AFFINITIES
            .iter()
            .any(|a| lowered == *a || lowered.starts_with(&format!("{}(", a)))
        {
            return Self::Numeric;
        }

        // Check text affinities
        if TEXT_AFFINITIES
            .iter()
            .any(|a| lowered == *a || lowered.starts_with(&format!("{}(", a)))
        {
            return Self::Text;
        }

        // Check blob
        if lowered == "blob" || lowered.starts_with("blob(") {
            return Self::Blob;
        }

        // Default to numeric for unknown types
        Self::Numeric
    }

    /// Get the drizzle import name for this type
    pub const fn drizzle_import(&self) -> &'static str {
        match self {
            Self::Integer => "integer",
            Self::Real => "real",
            Self::Numeric => "numeric",
            Self::Text => "text",
            Self::Blob => "blob",
        }
    }
}

/// Normalize a SQL type to its canonical form
pub fn normalize_sql_type(sql_type: &str) -> String {
    let lowered = sql_type.to_lowercase();

    // Integer types
    if [
        "int",
        "tinyint",
        "smallint",
        "mediumint",
        "bigint",
        "unsigned big int",
    ]
    .iter()
    .any(|t| lowered.starts_with(t))
    {
        return "integer".to_string();
    }

    // Text types with optional length
    if [
        "character",
        "varchar",
        "varying character",
        "nchar",
        "native character",
        "nvarchar",
    ]
    .iter()
    .any(|t| lowered.starts_with(t))
    {
        // Extract length if present
        if let Some(len) = extract_type_length(&lowered) {
            return format!("text({})", len);
        }
        return "text".to_string();
    }

    if lowered.starts_with("text") || lowered == "clob" {
        return "text".to_string();
    }

    if lowered.starts_with("blob") {
        return "blob".to_string();
    }

    if ["real", "double", "double precision", "float"]
        .iter()
        .any(|t| lowered.starts_with(t))
    {
        return "real".to_string();
    }

    "numeric".to_string()
}

/// Extract numeric length from type like "varchar(255)"
fn extract_type_length(sql_type: &str) -> Option<u32> {
    let start = sql_type.find('(')?;
    let end = sql_type.find(')')?;
    sql_type[start + 1..end].parse().ok()
}

// =============================================================================
// DDL Parsing (matching grammar.ts)
// =============================================================================

/// Parsed check constraint from DDL
#[derive(Debug, Clone)]
pub struct ParsedCheck {
    pub name: Option<String>,
    pub value: String,
}

/// Parsed primary key from DDL
#[derive(Debug, Clone, Default)]
pub struct ParsedPrimaryKey {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// Parsed unique constraint from DDL
#[derive(Debug, Clone)]
pub struct ParsedUnique {
    pub name: Option<String>,
    pub columns: Vec<String>,
}

/// Parsed foreign key from DDL
#[derive(Debug, Clone)]
pub struct ParsedForeignKey {
    pub name: Option<String>,
    pub from_table: String,
    pub from_columns: Vec<String>,
    pub to_table: String,
    pub to_columns: Vec<String>,
}

/// Parsed generated column info
#[derive(Debug, Clone)]
pub struct ParsedGenerated {
    pub expression: String,
    pub gen_type: GeneratedType,
}

/// Result of parsing a CREATE TABLE statement
#[derive(Debug, Clone, Default)]
pub struct ParsedTableDDL {
    pub pk: ParsedPrimaryKey,
    pub uniques: Vec<ParsedUnique>,
    pub checks: Vec<ParsedCheck>,
    /// Is this a STRICT table?
    pub strict: bool,
    /// Is this a WITHOUT ROWID table?
    pub without_rowid: bool,
}

/// Clean an identifier by removing quotes
fn clean_identifier(id: &str) -> String {
    id.trim()
        .trim_start_matches(|c| c == '[' || c == '`' || c == '"')
        .trim_end_matches(|c| c == ']' || c == '`' || c == '"')
        .to_string()
}

/// Parse columns from a comma-separated string
fn parse_columns_str(columns_str: &str) -> Vec<String> {
    columns_str
        .split(',')
        .map(|c| clean_identifier(c))
        .collect()
}

/// Parse a CREATE TABLE DDL statement to extract constraints
#[cfg(feature = "std")]
pub fn parse_table_ddl(sql: &str) -> ParsedTableDDL {
    use regex::Regex;

    let mut result = ParsedTableDDL::default();

    // Normalize whitespace
    let normalized = sql
        .replace(['\r', '\n'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Extract table body
    let body_start = match normalized.find('(') {
        Some(i) => i + 1,
        None => return result,
    };
    let body_end = match normalized.rfind(')') {
        Some(i) => i,
        None => return result,
    };
    let mut table_body = normalized[body_start..body_end].to_string();

    // Check for table options after the closing paren
    // e.g., CREATE TABLE foo (...) WITHOUT ROWID, STRICT
    let table_suffix = &normalized[body_end + 1..].to_uppercase();
    result.strict = table_suffix.contains("STRICT");
    result.without_rowid = table_suffix.contains("WITHOUT ROWID");

    let ident = r#"(?:\[[^\]]+\]|`[^`]+`|"[^"]+"|[\w_]+)"#;

    // Named CHECK constraints
    let named_check_re =
        Regex::new(r#"(?i)CONSTRAINT\s+["'`\[]?(\w+)["'`\]]?\s+CHECK\s*\((.*?)\)"#).unwrap();
    for cap in named_check_re.captures_iter(&table_body.clone()) {
        result.checks.push(ParsedCheck {
            name: Some(cap[1].to_string()),
            value: cap[2].trim().to_string(),
        });
        table_body = table_body.replace(&cap[0], "");
    }

    // Unnamed CHECK constraints
    let unnamed_check_re = Regex::new(r"(?i)CHECK\s*\((.*?)\)").unwrap();
    for cap in unnamed_check_re.captures_iter(&table_body.clone()) {
        let value = cap[1].trim().to_string();
        // Skip if we already have this check value
        if !result.checks.iter().any(|c| c.value == value) {
            result.checks.push(ParsedCheck { name: None, value });
        }
        table_body = table_body.replace(&cap[0], "");
    }

    // Table-level UNIQUE constraints
    let unique_constraint_re = Regex::new(&format!(
        r"(?i)CONSTRAINT\s+({ident})\s+UNIQUE\s*\(([^)]+)\)"
    ))
    .unwrap();
    for cap in unique_constraint_re.captures_iter(&table_body.clone()) {
        result.uniques.push(ParsedUnique {
            name: Some(clean_identifier(&cap[1])),
            columns: parse_columns_str(&cap[2]),
        });
        table_body = table_body.replace(&cap[0], "");
    }

    // Table-level PRIMARY KEY constraint
    let pk_constraint_re = Regex::new(&format!(
        r"(?i)CONSTRAINT\s+({ident})\s+PRIMARY\s+KEY\s*\(([^)]+)\)"
    ))
    .unwrap();
    if let Some(cap) = pk_constraint_re.captures(&table_body.clone()) {
        result.pk = ParsedPrimaryKey {
            name: Some(clean_identifier(&cap[1])),
            columns: parse_columns_str(&cap[2]),
        };
        table_body = table_body.replace(&cap[0], "");
    }

    // Check remaining column definitions for inline constraints
    for def in table_body.split(',') {
        let trimmed = def.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Inline PRIMARY KEY
        let inline_pk_re = Regex::new(&format!(r"(?i)^({ident})\s+.*\bPRIMARY\s+KEY\b")).unwrap();
        if let Some(cap) = inline_pk_re.captures(trimmed) {
            let pk_column = clean_identifier(&cap[1]);

            // Check for constraint name
            let constraint_name_re = Regex::new(&format!(r"(?i)CONSTRAINT\s+({ident})")).unwrap();
            let name = constraint_name_re
                .captures(trimmed)
                .map(|c| clean_identifier(&c[1]));

            if result.pk.columns.is_empty() {
                result.pk = ParsedPrimaryKey {
                    name,
                    columns: vec![pk_column],
                };
            }
        }

        // Inline UNIQUE
        let inline_unique_re = Regex::new(&format!(r"(?i)^({ident})\s+.*\bUNIQUE\b")).unwrap();
        if let Some(cap) = inline_unique_re.captures(trimmed) {
            let uq_column = clean_identifier(&cap[1]);

            // Skip if already exists
            if !result
                .uniques
                .iter()
                .any(|u| u.columns.len() == 1 && u.columns[0] == uq_column)
            {
                let constraint_name_re =
                    Regex::new(&format!(r"(?i)CONSTRAINT\s+({ident})")).unwrap();
                let name = constraint_name_re
                    .captures(trimmed)
                    .map(|c| clean_identifier(&c[1]));

                result.uniques.push(ParsedUnique {
                    name,
                    columns: vec![uq_column],
                });
            }
        }
    }

    result
}

/// Parse foreign keys from a CREATE TABLE DDL statement
#[cfg(feature = "std")]
pub fn parse_foreign_keys(sql: &str, table_name: &str) -> Vec<ParsedForeignKey> {
    use regex::Regex;

    let mut results = Vec::new();

    // Normalize whitespace
    let normalized = sql
        .replace(['\r', '\n'], " ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");

    // Extract table body
    let body_start = match normalized.find('(') {
        Some(i) => i + 1,
        None => return results,
    };
    let body_end = match normalized.rfind(')') {
        Some(i) => i,
        None => return results,
    };
    let mut table_body = normalized[body_start..body_end].to_string();

    let ident = r#"(?:\[[^\]]+\]|`[^`]+`|"[^"]+"|[\w_]+)"#;

    // Table-level FOREIGN KEY constraints
    let table_fk_re = Regex::new(&format!(
        r"(?i)(?:CONSTRAINT\s+({ident})\s+)?FOREIGN\s+KEY\s*\(([^)]+)\)\s+REFERENCES\s+({ident})(?:\s*\(([^)]+)\))?"
    )).unwrap();

    for cap in table_fk_re.captures_iter(&table_body.clone()) {
        results.push(ParsedForeignKey {
            name: cap.get(1).map(|m| clean_identifier(m.as_str())),
            from_table: table_name.to_string(),
            from_columns: parse_columns_str(&cap[2]),
            to_table: clean_identifier(&cap[3]),
            to_columns: cap
                .get(4)
                .map(|m| parse_columns_str(m.as_str()))
                .unwrap_or_default(),
        });
        table_body = table_body.replace(&cap[0], "");
    }

    // Inline REFERENCES
    for def in table_body.split(',') {
        let trimmed = def.trim();

        let inline_fk_re = Regex::new(&format!(
            r"(?i)^({ident}).*?\s+REFERENCES\s+({ident})(?:\s*\(([^)]+)\))?"
        ))
        .unwrap();

        if let Some(cap) = inline_fk_re.captures(trimmed) {
            let from_column = clean_identifier(&cap[1]);
            let to_table = clean_identifier(&cap[2]);
            let to_columns = cap
                .get(3)
                .map(|m| parse_columns_str(m.as_str()))
                .unwrap_or_default();

            let constraint_name_re = Regex::new(&format!(r"(?i)CONSTRAINT\s+({ident})")).unwrap();
            let name = constraint_name_re
                .captures(trimmed)
                .map(|c| clean_identifier(&c[1]));

            results.push(ParsedForeignKey {
                name,
                from_table: table_name.to_string(),
                from_columns: vec![from_column],
                to_table,
                to_columns,
            });
        }
    }

    results
}

/// Extract generated columns from a CREATE TABLE statement
#[cfg(feature = "std")]
pub fn extract_generated_columns(sql: &str) -> std::collections::HashMap<String, ParsedGenerated> {
    use regex::Regex;
    use std::collections::HashMap;

    let mut columns = HashMap::new();

    let re =
        Regex::new(r#"(?i)["'`\[]?(\w+)["'`\]]?\s+\w+\s+GENERATED\s+ALWAYS\s+AS\s*\("#).unwrap();

    for cap in re.captures_iter(sql) {
        let column_name = cap[1].to_string();
        let start_index = cap.get(0).unwrap().end() - 1;

        // Find matching closing parenthesis
        let mut depth = 1;
        let mut end_index = start_index + 1;
        let chars: Vec<char> = sql.chars().collect();

        while end_index < chars.len() && depth > 0 {
            match chars[end_index] {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            end_index += 1;
        }

        let expression = sql[start_index..end_index].to_string();

        // Find STORED/VIRTUAL after the expression
        let after_expr = &sql[end_index..];
        let gen_type = if after_expr.to_uppercase().contains("STORED") {
            GeneratedType::Stored
        } else {
            GeneratedType::Virtual
        };

        columns.insert(
            column_name,
            ParsedGenerated {
                expression,
                gen_type,
            },
        );
    }

    columns
}

/// Parse a VIEW AS statement to extract the definition
#[cfg(feature = "std")]
pub fn parse_view_definition(sql: &str) -> Option<String> {
    use regex::Regex;

    let re = Regex::new(r"(?is)\bAS\b\s+(WITH.+|SELECT.+)$").unwrap();
    re.captures(sql).map(|cap| cap[1].to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_table_serialization() {
        let table = Table::new("users");
        // Wrap in SqliteEntity to get the entityType tag
        let entity = SqliteEntity::Table(table);
        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains(r#""entityType":"tables""#));
        assert!(json.contains(r#""name":"users""#));
    }

    #[test]
    fn test_column_serialization() {
        let col = Column::new("users", "id", "integer")
            .not_null()
            .autoincrement();
        // Wrap in SqliteEntity to get the entityType tag
        let entity = SqliteEntity::Column(col);
        let json = serde_json::to_string(&entity).unwrap();
        assert!(json.contains(r#""entityType":"columns""#));
        assert!(json.contains(r#""table":"users""#));
        assert!(json.contains(r#""name":"id""#));
        assert!(json.contains(r#""notNull":true"#));
        assert!(json.contains(r#""autoincrement":true"#));
    }

    #[test]
    fn test_fk_name_generation() {
        let name = name_for_fk(
            "posts",
            &["author_id".to_string()],
            "users",
            &["id".to_string()],
        );
        assert_eq!(name, "fk_posts_author_id_users_id_fk");
    }

    #[test]
    fn test_unique_name_generation() {
        let name = name_for_unique("users", &["email".to_string()]);
        assert_eq!(name, "users_email_unique");
    }

    #[test]
    fn test_pk_name_generation() {
        let name = name_for_pk("users");
        assert_eq!(name, "users_pk");
    }
}

// =============================================================================
// Entity Trait Implementations
// =============================================================================

impl Entity for Table {
    const KIND: EntityKind = EntityKind::Table;

    fn key(&self) -> EntityKey {
        EntityKey::simple(&self.name)
    }
}

impl Entity for Column {
    const KIND: EntityKind = EntityKind::Column;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.table, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.table))
    }
}

impl Entity for Index {
    const KIND: EntityKind = EntityKind::Index;

    fn key(&self) -> EntityKey {
        EntityKey::simple(&self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.table))
    }
}

impl Entity for ForeignKey {
    const KIND: EntityKind = EntityKind::ForeignKey;

    fn key(&self) -> EntityKey {
        EntityKey::simple(&self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.table))
    }
}

impl Entity for PrimaryKey {
    const KIND: EntityKind = EntityKind::PrimaryKey;

    fn key(&self) -> EntityKey {
        // PK is unique per table
        EntityKey::simple(&self.table)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.table))
    }
}

impl Entity for UniqueConstraint {
    const KIND: EntityKind = EntityKind::UniqueConstraint;

    fn key(&self) -> EntityKey {
        EntityKey::simple(&self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.table))
    }
}

impl Entity for CheckConstraint {
    const KIND: EntityKind = EntityKind::CheckConstraint;

    fn key(&self) -> EntityKey {
        EntityKey::simple(&self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.table))
    }
}

impl Entity for View {
    const KIND: EntityKind = EntityKind::View;

    fn key(&self) -> EntityKey {
        EntityKey::simple(&self.name)
    }
}
