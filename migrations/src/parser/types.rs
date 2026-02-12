//! Parser types - shared data structures for all dialects

use std::collections::HashMap;

use drizzle_types::Dialect;

// =============================================================================
// Parsed Types
// =============================================================================

/// Parsed table struct from generated code
#[derive(Debug, Clone, Default)]
pub struct ParsedTable {
    /// Struct name (PascalCase)
    pub name: String,
    /// Full table attribute (e.g., "#[SQLiteTable(strict)]" or "#[PostgresTable]")
    pub attr: String,
    /// Parsed fields
    pub fields: Vec<ParsedField>,
    /// Detected dialect
    pub dialect: Dialect,
}

/// Parsed index struct from generated code
#[derive(Debug, Clone, Default)]
pub struct ParsedIndex {
    /// Struct name (PascalCase)
    pub name: String,
    /// Full index attribute (e.g., "#[SQLiteIndex(unique)]" or "#[PostgresIndex(unique)]")
    pub attr: String,
    /// Column references (e.g., ["Users::id", "Users::name"])
    pub columns: Vec<String>,
    /// Detected dialect
    pub dialect: Dialect,
}

/// Parsed schema struct from generated code
#[derive(Debug, Clone, Default)]
pub struct ParsedSchema {
    /// Struct name (e.g., "AppSchema")
    pub name: String,
    /// Schema members (field_name -> type_name)
    pub members: HashMap<String, String>,
    /// Detected dialect
    pub dialect: Dialect,
}

/// Parsed field from generated struct
#[derive(Debug, Clone, Default)]
pub struct ParsedField {
    /// Field name (snake_case)
    pub name: String,
    /// Rust type (e.g., "i64", "Option<String>")
    pub ty: String,
    /// Column attributes (e.g., "#[column(primary, autoincrement)]")
    pub attrs: Vec<String>,
}

/// Result of parsing schema code
#[derive(Debug, Clone, Default)]
pub struct ParseResult {
    /// Parsed table structs
    pub tables: HashMap<String, ParsedTable>,
    /// Parsed index structs
    pub indexes: HashMap<String, ParsedIndex>,
    /// Parsed schema struct (if present)
    pub schema: Option<ParsedSchema>,
    /// Detected dialect (based on first found table/schema)
    pub dialect: Dialect,
}

// =============================================================================
// Implementations
// =============================================================================

impl ParsedTable {
    /// Get a field by name
    pub fn field(&self, name: &str) -> Option<&ParsedField> {
        self.fields.iter().find(|f| f.name == name)
    }

    /// Check if table has a specific attribute in its #[SQLiteTable(...)]
    pub fn has_table_attr(&self, attr: &str) -> bool {
        self.attr.contains(attr)
    }

    /// Get the value of a table attribute assignment (e.g. `schema = "auth"`).
    pub fn attr_value(&self, key: &str) -> Option<String> {
        extract_attr_value_from_attr(&self.attr, key)
    }

    /// Get the PostgreSQL schema name if set on `#[PostgresTable(...)]`.
    pub fn schema_name(&self) -> Option<String> {
        self.attr_value("schema")
            .map(|v| trim_wrapping_quotes(v.trim()).to_string())
    }

    /// Check if table is marked as SQLite STRICT.
    pub fn is_strict(&self) -> bool {
        self.has_table_attr("strict")
    }

    /// Check if table is marked as SQLite WITHOUT ROWID.
    pub fn is_without_rowid(&self) -> bool {
        self.has_table_attr("without_rowid")
    }

    /// Get all field names
    pub fn field_names(&self) -> Vec<&str> {
        self.fields.iter().map(|f| f.name.as_str()).collect()
    }
}

impl ParsedIndex {
    /// Check if index is unique
    pub fn is_unique(&self) -> bool {
        self.attr.contains("unique")
    }

    /// Check if PostgreSQL index is created concurrently.
    pub fn is_concurrent(&self) -> bool {
        self.attr.contains("concurrent")
    }

    /// Get PostgreSQL index method (e.g. `btree`, `gin`) if explicitly set.
    pub fn method(&self) -> Option<String> {
        self.attr_value("method")
            .map(|v| trim_wrapping_quotes(v.trim()).to_string())
    }

    /// Get PostgreSQL partial-index WHERE clause if explicitly set.
    pub fn where_clause(&self) -> Option<String> {
        self.attr_value("where")
            .map(|v| trim_wrapping_quotes(v.trim()).to_string())
    }

    /// Get the table name from the first column reference
    pub fn table_name(&self) -> Option<&str> {
        self.columns.first().and_then(|c| c.split("::").next())
    }

    /// Get the value of an index attribute assignment.
    fn attr_value(&self, key: &str) -> Option<String> {
        extract_attr_value_from_attr(&self.attr, key)
    }
}

impl ParsedField {
    /// Check if field has a specific attribute
    pub fn has_attr(&self, attr: &str) -> bool {
        self.attrs.iter().any(|a| a.contains(attr))
    }

    /// Get the value of an attribute assignment (e.g., `default = 42` -> Some("42"))
    pub fn attr_value(&self, key: &str) -> Option<String> {
        for attr in &self.attrs {
            if let Some(value) = Self::extract_attr_value(attr, key) {
                return Some(value);
            }
        }
        None
    }

    /// Get all attribute key-value pairs as a HashMap
    pub fn attr_values(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for attr in &self.attrs {
            // Extract content inside #[column(...)]
            if let Some(start) = attr.find('(')
                && let Some(end) = attr.rfind(')')
            {
                let content = &attr[start + 1..end];
                // Parse key = value pairs
                for part in split_attr_parts(content) {
                    if let Some(eq_pos) = part.find('=') {
                        let key = part[..eq_pos].trim();
                        let value = part[eq_pos + 1..].trim();
                        result.insert(key.to_string(), value.to_string());
                    }
                }
            }
        }
        result
    }

    /// Get the combined column attribute string
    pub fn column_attr(&self) -> String {
        self.attrs.join(", ")
    }

    /// Check if field is nullable (Option<T>)
    pub fn is_nullable(&self) -> bool {
        self.ty.starts_with("Option<")
    }

    /// Check if field is a primary key
    pub fn is_primary_key(&self) -> bool {
        self.has_attr("primary")
    }

    /// Check if field has autoincrement
    pub fn is_autoincrement(&self) -> bool {
        self.has_attr("autoincrement")
    }

    /// Check if field is unique
    pub fn is_unique(&self) -> bool {
        self.has_attr("unique")
    }

    /// Get the default value if present
    pub fn default_value(&self) -> Option<String> {
        self.attr_value("default")
    }

    /// Get the references target if present (e.g., "Users::id")
    pub fn references(&self) -> Option<String> {
        self.attr_value("references")
    }

    /// Get the on_delete action if present
    pub fn on_delete(&self) -> Option<String> {
        self.attr_value("on_delete")
    }

    /// Get the on_update action if present
    pub fn on_update(&self) -> Option<String> {
        self.attr_value("on_update")
    }

    /// Extract a value for a key from an attribute string
    fn extract_attr_value(attr: &str, key: &str) -> Option<String> {
        extract_attr_value_from_attr(attr, key)
    }
}

/// Extract a value for `key` from an attribute string like `#[name(k = v, ...)]`.
fn extract_attr_value_from_attr(attr: &str, key: &str) -> Option<String> {
    let start = attr.find('(')?;
    let end = attr.rfind(')')?;
    let content = &attr[start + 1..end];

    for part in split_attr_parts(content) {
        let part = part.trim();
        if let Some(eq_pos) = part.find('=') {
            let k = part[..eq_pos].trim();
            if k == key {
                return Some(part[eq_pos + 1..].trim().to_string());
            }
        }
    }

    None
}

/// Split attribute content by commas, respecting nested structures and quoted strings.
fn split_attr_parts(content: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut depth = 0usize;
    let mut start = 0;
    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (i, c) in content.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }

        match c {
            '\\' if in_single || in_double => {
                escaped = true;
            }
            '\'' if !in_double => {
                in_single = !in_single;
            }
            '"' if !in_single => {
                in_double = !in_double;
            }
            '(' | '<' | '[' | '{' if !in_single && !in_double => {
                depth += 1;
            }
            ')' | '>' | ']' | '}' if !in_single && !in_double => {
                depth = depth.saturating_sub(1);
            }
            ',' if depth == 0 && !in_single && !in_double => {
                parts.push(content[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }

    if start < content.len() {
        parts.push(content[start..].trim());
    }

    parts
}

fn trim_wrapping_quotes(value: &str) -> &str {
    let bytes = value.as_bytes();
    if bytes.len() >= 2
        && ((bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"')
            || (bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\''))
    {
        &value[1..value.len() - 1]
    } else {
        value
    }
}

impl ParseResult {
    /// Get a table by name and dialect
    pub fn table(&self, name: &str, dialect: Dialect) -> Option<&ParsedTable> {
        let key = format!("{}:{}", dialect_key(dialect), name);
        self.tables.get(&key)
    }

    /// Get an index by name and dialect
    pub fn index(&self, name: &str, dialect: Dialect) -> Option<&ParsedIndex> {
        let key = format!("{}:{}", dialect_key(dialect), name);
        self.indexes.get(&key)
    }

    /// Get all tables for a specific dialect
    pub fn tables_for_dialect(&self, dialect: Dialect) -> impl Iterator<Item = &ParsedTable> {
        let prefix = format!("{}:", dialect_key(dialect));
        self.tables
            .iter()
            .filter(move |(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v)
    }

    /// Get all indexes for a specific dialect
    pub fn indexes_for_dialect(&self, dialect: Dialect) -> impl Iterator<Item = &ParsedIndex> {
        let prefix = format!("{}:", dialect_key(dialect));
        self.indexes
            .iter()
            .filter(move |(k, _)| k.starts_with(&prefix))
            .map(|(_, v)| v)
    }

    /// Get all table names (without dialect prefix)
    pub fn table_names(&self) -> Vec<&str> {
        self.tables
            .keys()
            .filter_map(|s| s.split(':').nth(1))
            .collect()
    }

    /// Get all index names (without dialect prefix)
    pub fn index_names(&self) -> Vec<&str> {
        self.indexes
            .keys()
            .filter_map(|s| s.split(':').nth(1))
            .collect()
    }
}

/// Get the key prefix for a dialect
fn dialect_key(dialect: Dialect) -> &'static str {
    match dialect {
        Dialect::SQLite => "sqlite",
        Dialect::PostgreSQL => "postgres",
        Dialect::MySQL => "mysql",
    }
}
