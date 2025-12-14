//! Schema Parser for Generated Rust Code
//!
//! This module provides utilities for parsing generated Rust schema code
//! into structured data that can be used for validation, analysis, or tooling.
//! Supports both SQLite and PostgreSQL generated code.
//!
//! # Example
//!
//! ```rust,ignore
//! use drizzle_migrations::parser::SchemaParser;
//!
//! let code = r#"
//! #[SQLiteTable]
//! struct Users {
//!     #[column(primary, autoincrement)]
//!     id: i64,
//!     name: String,
//! }
//! "#;
//!
//! let result = SchemaParser::parse(code);
//! let users = result.table("Users").unwrap();
//! assert!(users.field("id").unwrap().has_attr("primary"));
//! ```

use std::collections::HashMap;

// =============================================================================
// Dialect (re-exported from drizzle-types)
// =============================================================================

/// Re-export the shared Dialect enum
pub use drizzle_types::Dialect;

/// Extension trait for parser-specific dialect functionality
pub trait DialectParserExt {
    /// Get the table attribute prefix for this dialect
    fn table_prefix(&self) -> &'static str;
    /// Get the index attribute prefix for this dialect  
    fn index_prefix(&self) -> &'static str;
    /// Get the schema derive attribute for this dialect
    fn schema_derive(&self) -> &'static str;
}

impl DialectParserExt for Dialect {
    fn table_prefix(&self) -> &'static str {
        match self {
            Dialect::SQLite => "#[SQLiteTable",
            Dialect::PostgreSQL => "#[PostgresTable",
            Dialect::MySQL => "#[MySQLTable",
        }
    }

    fn index_prefix(&self) -> &'static str {
        match self {
            Dialect::SQLite => "#[SQLiteIndex",
            Dialect::PostgreSQL => "#[PostgresIndex",
            Dialect::MySQL => "#[MySQLIndex",
        }
    }

    fn schema_derive(&self) -> &'static str {
        match self {
            Dialect::SQLite => "#[derive(SQLiteSchema)]",
            Dialect::PostgreSQL => "#[derive(PostgresSchema)]",
            Dialect::MySQL => "#[derive(MySQLSchema)]",
        }
    }
}

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

    /// Get the table name from the first column reference
    pub fn table_name(&self) -> Option<&str> {
        self.columns.first().and_then(|c| c.split("::").next())
    }
}

impl ParsedField {
    /// Check if field has a specific attribute
    pub fn has_attr(&self, attr: &str) -> bool {
        self.attrs.iter().any(|a| a.contains(attr))
    }

    /// Get the value of an attribute assignment (e.g., `default = 42` -> Some("42"))
    ///
    /// # Example
    /// ```rust,ignore
    /// // For #[column(default = 42, references = Users::id)]
    /// field.attr_value("default") // -> Some("42")
    /// field.attr_value("references") // -> Some("Users::id")
    /// field.attr_value("primary") // -> None (not an assignment)
    /// ```
    pub fn attr_value(&self, key: &str) -> Option<String> {
        for attr in &self.attrs {
            if let Some(value) = Self::extract_attr_value(attr, key) {
                return Some(value);
            }
        }
        None
    }

    /// Get all attribute key-value pairs as a HashMap
    ///
    /// # Example
    /// ```rust,ignore
    /// // For #[column(default = 42, references = Users::id, primary)]
    /// field.attr_values() // -> {"default": "42", "references": "Users::id"}
    /// ```
    pub fn attr_values(&self) -> HashMap<String, String> {
        let mut result = HashMap::new();
        for attr in &self.attrs {
            // Extract content inside #[column(...)]
            if let Some(start) = attr.find('(') {
                if let Some(end) = attr.rfind(')') {
                    let content = &attr[start + 1..end];
                    // Parse key = value pairs
                    for part in Self::split_attr_parts(content) {
                        if let Some(eq_pos) = part.find('=') {
                            let key = part[..eq_pos].trim();
                            let value = part[eq_pos + 1..].trim();
                            result.insert(key.to_string(), value.to_string());
                        }
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
        // Extract content inside #[column(...)]
        let start = attr.find('(')?;
        let end = attr.rfind(')')?;
        let content = &attr[start + 1..end];

        // Find the key and extract its value
        for part in Self::split_attr_parts(content) {
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

    /// Split attribute content by commas, respecting nested structures
    fn split_attr_parts(content: &str) -> Vec<&str> {
        let mut parts = Vec::new();
        let mut depth: i32 = 0;
        let mut start = 0;

        for (i, c) in content.char_indices() {
            match c {
                '(' | '<' | '[' | '{' => depth += 1,
                ')' | '>' | ']' | '}' => depth = depth.saturating_sub(1),
                ',' if depth == 0 => {
                    parts.push(content[start..i].trim());
                    start = i + 1;
                }
                _ => {}
            }
        }

        // Add the last part
        if start < content.len() {
            parts.push(content[start..].trim());
        }

        parts
    }
}

impl ParseResult {
    /// Get a table by name
    pub fn table(&self, name: &str) -> Option<&ParsedTable> {
        self.tables.get(name)
    }

    /// Get an index by name
    pub fn index(&self, name: &str) -> Option<&ParsedIndex> {
        self.indexes.get(name)
    }

    /// Get all table names
    pub fn table_names(&self) -> Vec<&str> {
        self.tables.keys().map(|s| s.as_str()).collect()
    }

    /// Get all index names
    pub fn index_names(&self) -> Vec<&str> {
        self.indexes.keys().map(|s| s.as_str()).collect()
    }
}

// =============================================================================
// Schema Parser
// =============================================================================

/// Parser for generated Rust schema code
pub struct SchemaParser;

impl SchemaParser {
    /// Parse generated Rust schema code into structured data
    /// Automatically detects SQLite or PostgreSQL dialect
    pub fn parse(code: &str) -> ParseResult {
        let mut result = ParseResult::default();
        let lines: Vec<&str> = code.lines().collect();

        let mut i = 0;
        while i < lines.len() {
            let line = lines[i].trim();

            // Parse #[SQLiteTable...] structs
            if line.starts_with("#[SQLiteTable") {
                if result.dialect == Dialect::default() {
                    result.dialect = Dialect::SQLite;
                }
                if let Some(mut table) = Self::parse_table(&lines, &mut i) {
                    table.dialect = Dialect::SQLite;
                    result.tables.insert(table.name.clone(), table);
                }
                continue;
            }

            // Parse #[PostgresTable...] structs
            if line.starts_with("#[PostgresTable") {
                if result.dialect == Dialect::default() {
                    result.dialect = Dialect::PostgreSQL;
                }
                if let Some(mut table) = Self::parse_table(&lines, &mut i) {
                    table.dialect = Dialect::PostgreSQL;
                    result.tables.insert(table.name.clone(), table);
                }
                continue;
            }

            // Parse #[SQLiteIndex...] structs
            if line.starts_with("#[SQLiteIndex") {
                if let Some(mut index) = Self::parse_index(&lines, &mut i) {
                    index.dialect = Dialect::SQLite;
                    result.indexes.insert(index.name.clone(), index);
                }
                continue;
            }

            // Parse #[PostgresIndex...] structs
            if line.starts_with("#[PostgresIndex") {
                if let Some(mut index) = Self::parse_index(&lines, &mut i) {
                    index.dialect = Dialect::PostgreSQL;
                    result.indexes.insert(index.name.clone(), index);
                }
                continue;
            }

            // Parse #[derive(SQLiteSchema)] structs
            if line.contains("#[derive(SQLiteSchema)]") {
                if let Some(mut schema) = Self::parse_schema(&lines, &mut i) {
                    schema.dialect = Dialect::SQLite;
                    result.dialect = Dialect::SQLite;
                    result.schema = Some(schema);
                }
                continue;
            }

            // Parse #[derive(PostgresSchema)] structs
            if line.contains("#[derive(PostgresSchema)]") {
                if let Some(mut schema) = Self::parse_schema(&lines, &mut i) {
                    schema.dialect = Dialect::PostgreSQL;
                    result.dialect = Dialect::PostgreSQL;
                    result.schema = Some(schema);
                }
                continue;
            }

            i += 1;
        }

        result
    }

    /// Parse a table struct
    fn parse_table(lines: &[&str], i: &mut usize) -> Option<ParsedTable> {
        let table_attr = lines[*i].trim().to_string();
        *i += 1;

        if *i >= lines.len() {
            return None;
        }

        let struct_line = lines[*i].trim();
        let name = Self::parse_struct_name(struct_line)?;

        let mut table = ParsedTable {
            name,
            attr: table_attr,
            fields: Vec::new(),
            dialect: Dialect::default(),
        };

        // Parse fields until closing brace
        *i += 1;
        let mut pending_attrs: Vec<String> = Vec::new();

        while *i < lines.len() {
            let field_line = lines[*i].trim();

            if field_line == "}" || field_line.starts_with('}') {
                break;
            }

            // Collect #[column(...)] attributes
            if field_line.starts_with("#[column(") {
                pending_attrs.push(field_line.to_string());
            } else if let Some((field_name, field_type)) = Self::parse_field_line(field_line) {
                let field = ParsedField {
                    name: field_name,
                    ty: field_type,
                    attrs: std::mem::take(&mut pending_attrs),
                };
                table.fields.push(field);
            }

            *i += 1;
        }

        Some(table)
    }

    /// Parse an index struct
    fn parse_index(lines: &[&str], i: &mut usize) -> Option<ParsedIndex> {
        let index_attr = lines[*i].trim().to_string();
        *i += 1;

        if *i >= lines.len() {
            return None;
        }

        let struct_line = lines[*i].trim();

        // Parse tuple struct: struct IdxName(Table::col1, Table::col2);
        let name = Self::parse_struct_name(struct_line)?;

        // Extract columns from tuple struct
        let columns = if let Some(start) = struct_line.find('(') {
            if let Some(end) = struct_line.rfind(')') {
                let cols_str = &struct_line[start + 1..end];
                cols_str
                    .split(',')
                    .map(|c| c.trim().trim_end_matches(';').to_string())
                    .filter(|c| !c.is_empty())
                    .collect()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        Some(ParsedIndex {
            name,
            attr: index_attr,
            columns,
            dialect: Dialect::default(),
        })
    }

    /// Parse a schema struct
    fn parse_schema(lines: &[&str], i: &mut usize) -> Option<ParsedSchema> {
        // Skip the derive line
        *i += 1;

        if *i >= lines.len() {
            return None;
        }

        let struct_line = lines[*i].trim();
        let name = Self::parse_struct_name(struct_line)?;

        let mut schema = ParsedSchema {
            name,
            members: HashMap::new(),
            dialect: Dialect::default(),
        };

        // Parse members until closing brace
        *i += 1;
        while *i < lines.len() {
            let member_line = lines[*i].trim();

            if member_line == "}" || member_line.starts_with('}') {
                break;
            }

            if let Some((field_name, field_type)) = Self::parse_field_line(member_line) {
                schema.members.insert(field_name, field_type);
            }

            *i += 1;
        }

        Some(schema)
    }

    /// Parse struct name from a struct line
    fn parse_struct_name(line: &str) -> Option<String> {
        // Handles: "struct Foo {", "struct Foo(Bar);", "pub struct Foo {"
        if !line.contains("struct ") {
            return None;
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        for (idx, part) in parts.iter().enumerate() {
            if *part == "struct" && idx + 1 < parts.len() {
                let full_name = parts[idx + 1];
                // For tuple structs like "IdxPostsTitle(Posts::title);",
                // we need to split at '(' to get just the struct name
                let name = full_name
                    .split(|c| c == '{' || c == '(' || c == ';')
                    .next()
                    .unwrap_or(full_name)
                    .trim();
                return Some(name.to_string());
            }
        }
        None
    }

    /// Parse field name and type from a field line
    fn parse_field_line(line: &str) -> Option<(String, String)> {
        // Handles: "pub field_name: FieldType," or "field_name: FieldType,"
        let line = line.trim().trim_end_matches(',');
        if !line.contains(':') || line.starts_with('#') || line.starts_with("//") {
            return None;
        }

        let parts: Vec<&str> = line.splitn(2, ':').collect();
        if parts.len() != 2 {
            return None;
        }

        let name_part = parts[0].trim();
        let type_part = parts[1].trim();

        // Handle "pub field_name"
        let name = if name_part.starts_with("pub ") {
            name_part.strip_prefix("pub ").unwrap().trim()
        } else {
            name_part
        };

        Some((name.to_string(), type_part.to_string()))
    }
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_table() {
        let code = r#"
#[SQLiteTable]
struct Users {
    #[column(primary, autoincrement)]
    id: i64,
    name: String,
    email: Option<String>,
}
"#;

        let result = SchemaParser::parse(code);
        let users = result.table("Users").unwrap();

        assert_eq!(users.name, "Users");
        assert_eq!(users.attr, "#[SQLiteTable]");
        assert_eq!(users.fields.len(), 3);

        let id = users.field("id").unwrap();
        assert_eq!(id.ty, "i64");
        assert!(id.has_attr("primary"));
        assert!(id.has_attr("autoincrement"));
        assert!(id.is_primary_key());
        assert!(id.is_autoincrement());
        assert!(!id.is_nullable());

        let email = users.field("email").unwrap();
        assert!(email.is_nullable());
    }

    #[test]
    fn test_parse_table_with_options() {
        let code = r#"
#[SQLiteTable(strict, without_rowid)]
struct Settings {
    id: i64,
}
"#;

        let result = SchemaParser::parse(code);
        let settings = result.table("Settings").unwrap();

        assert!(settings.has_table_attr("strict"));
        assert!(settings.has_table_attr("without_rowid"));
    }

    #[test]
    fn test_parse_index() {
        let code = r#"
#[SQLiteIndex(unique)]
struct IdxUsersEmail(Users::email);

#[SQLiteIndex]
struct IdxUsersNameAge(Users::name, Users::age);
"#;

        let result = SchemaParser::parse(code);

        let email_idx = result.index("IdxUsersEmail").unwrap();
        assert!(email_idx.is_unique());
        assert_eq!(email_idx.columns, vec!["Users::email"]);
        assert_eq!(email_idx.table_name(), Some("Users"));

        let name_age_idx = result.index("IdxUsersNameAge").unwrap();
        assert!(!name_age_idx.is_unique());
        assert_eq!(name_age_idx.columns, vec!["Users::name", "Users::age"]);
    }

    #[test]
    fn test_parse_schema() {
        let code = r#"
#[derive(SQLiteSchema)]
pub struct AppSchema {
    pub users: Users,
    pub posts: Posts,
}
"#;

        let result = SchemaParser::parse(code);
        let schema = result.schema.unwrap();

        assert_eq!(schema.name, "AppSchema");
        assert_eq!(schema.members.get("users"), Some(&"Users".to_string()));
        assert_eq!(schema.members.get("posts"), Some(&"Posts".to_string()));
    }

    #[test]
    fn test_parse_field_with_references() {
        let code = r#"
#[SQLiteTable]
struct Posts {
    #[column(references = Users::id, on_delete = cascade)]
    author_id: i64,
}
"#;

        let result = SchemaParser::parse(code);
        let posts = result.table("Posts").unwrap();
        let author_id = posts.field("author_id").unwrap();

        // Test has_attr (substring matching)
        assert!(author_id.has_attr("references = Users::id"));
        assert!(author_id.has_attr("on_delete = cascade"));

        // Test attr_value (extract specific values)
        assert_eq!(
            author_id.attr_value("references"),
            Some("Users::id".to_string())
        );
        assert_eq!(
            author_id.attr_value("on_delete"),
            Some("cascade".to_string())
        );
        assert_eq!(author_id.attr_value("on_update"), None);

        // Test convenience methods
        assert_eq!(author_id.references(), Some("Users::id".to_string()));
        assert_eq!(author_id.on_delete(), Some("cascade".to_string()));
        assert_eq!(author_id.on_update(), None);
    }

    #[test]
    fn test_attr_values_with_defaults() {
        let code = r#"
#[SQLiteTable]
struct Settings {
    #[column(primary)]
    id: i64,
    #[column(default = 42)]
    count: i64,
    #[column(default = "hello world")]
    message: String,
    #[column(default = 3.14, unique)]
    ratio: f64,
}
"#;

        let result = SchemaParser::parse(code);
        let settings = result.table("Settings").unwrap();

        // id has primary but no default
        let id = settings.field("id").unwrap();
        assert!(id.is_primary_key());
        assert_eq!(id.default_value(), None);

        // count has numeric default
        let count = settings.field("count").unwrap();
        assert_eq!(count.default_value(), Some("42".to_string()));
        assert_eq!(count.attr_value("default"), Some("42".to_string()));

        // message has string default with spaces
        let message = settings.field("message").unwrap();
        assert_eq!(message.default_value(), Some("\"hello world\"".to_string()));

        // ratio has both default and unique
        let ratio = settings.field("ratio").unwrap();
        assert_eq!(ratio.default_value(), Some("3.14".to_string()));
        assert!(ratio.is_unique());

        // Test attr_values to get all key-value pairs
        let ratio_values = ratio.attr_values();
        assert_eq!(ratio_values.get("default"), Some(&"3.14".to_string()));
        assert!(!ratio_values.contains_key("unique")); // unique is not a key=value pair
    }

    #[test]
    fn test_parse_postgres_table() {
        let code = r#"
#[PostgresTable]
struct Users {
    #[column(primary)]
    id: i32,
    #[column(unique)]
    email: String,
    bio: Option<String>,
}
"#;

        let result = SchemaParser::parse(code);
        assert_eq!(result.dialect, Dialect::PostgreSQL);

        let users = result.table("Users").unwrap();
        assert_eq!(users.name, "Users");
        assert_eq!(users.attr, "#[PostgresTable]");
        assert_eq!(users.dialect, Dialect::PostgreSQL);
        assert_eq!(users.fields.len(), 3);

        let id = users.field("id").unwrap();
        assert_eq!(id.ty, "i32");
        assert!(id.is_primary_key());

        let email = users.field("email").unwrap();
        assert!(email.is_unique());
        assert!(!email.is_nullable());

        let bio = users.field("bio").unwrap();
        assert!(bio.is_nullable());
    }

    #[test]
    fn test_parse_postgres_schema() {
        let code = r#"
#[derive(PostgresSchema)]
pub struct AppSchema {
    pub users: Users,
    pub orders: Orders,
}
"#;

        let result = SchemaParser::parse(code);
        assert_eq!(result.dialect, Dialect::PostgreSQL);

        let schema = result.schema.unwrap();
        assert_eq!(schema.name, "AppSchema");
        assert_eq!(schema.dialect, Dialect::PostgreSQL);
        assert_eq!(schema.members.get("users"), Some(&"Users".to_string()));
        assert_eq!(schema.members.get("orders"), Some(&"Orders".to_string()));
    }

    #[test]
    fn test_parse_postgres_index() {
        let code = r#"
#[PostgresIndex(unique)]
struct IdxUsersEmail(Users::email);
"#;

        let result = SchemaParser::parse(code);

        let idx = result.index("IdxUsersEmail").unwrap();
        assert_eq!(idx.dialect, Dialect::PostgreSQL);
        assert!(idx.is_unique());
        assert_eq!(idx.columns, vec!["Users::email"]);
    }

    #[test]
    fn test_dialect_detection() {
        // SQLite code
        let sqlite_code = r#"
#[SQLiteTable]
struct Users { id: i64, }
"#;
        let sqlite_result = SchemaParser::parse(sqlite_code);
        assert_eq!(sqlite_result.dialect, Dialect::SQLite);

        // Postgres code
        let postgres_code = r#"
#[PostgresTable]
struct Users { id: i32, }
"#;
        let postgres_result = SchemaParser::parse(postgres_code);
        assert_eq!(postgres_result.dialect, Dialect::PostgreSQL);
    }
}
