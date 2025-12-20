//! Schema Parser for Generated Rust Code
//!
//! This module provides utilities for parsing generated Rust schema code
//! into structured data that can be used for validation, analysis, or tooling.
//! Supports SQLite, PostgreSQL, and MySQL (future) generated code.
//!
//! # Example
//!
//! ```rust,ignore
//! use drizzle_migrations::parser::SchemaParser;
//! use drizzle_types::Dialect;
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
//! let users = result.table("Users", Dialect::SQLite).unwrap();
//! assert!(users.field("id").unwrap().has_attr("primary"));
//! ```

mod combinators;
mod types;

pub use types::*;

use std::collections::HashMap;

use drizzle_types::Dialect;

use combinators::{parse_index_struct, parse_schema_struct, parse_table_struct};

// =============================================================================
// Dialect Extensions
// =============================================================================

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
// Schema Parser
// =============================================================================

/// Parser for generated Rust schema code
pub struct SchemaParser;

impl SchemaParser {
    /// Parse generated Rust schema code into structured data
    /// Automatically detects SQLite or PostgreSQL dialect
    pub fn parse(code: &str) -> ParseResult {
        let mut result = ParseResult::default();

        // Parse all table structs
        for (dialect, table) in Self::parse_tables(code) {
            if result.dialect == Dialect::default() {
                result.dialect = dialect;
            }
            let key = format!("{}:{}", dialect_key(dialect), table.name);
            result.tables.insert(key, table);
        }

        // Parse all index structs
        for (dialect, index) in Self::parse_indexes(code) {
            let key = format!("{}:{}", dialect_key(dialect), index.name);
            result.indexes.insert(key, index);
        }

        // Parse schema struct (if present)
        if let Some((dialect, schema)) = Self::parse_schema(code) {
            result.dialect = dialect;
            result.schema = Some(schema);
        }

        result
    }

    /// Parse all table structs from code
    fn parse_tables(code: &str) -> Vec<(Dialect, ParsedTable)> {
        let mut tables = Vec::new();

        // Try each dialect
        for (prefix, dialect) in [
            ("#[SQLiteTable", Dialect::SQLite),
            ("#[PostgresTable", Dialect::PostgreSQL),
            ("#[MySQLTable", Dialect::MySQL),
        ] {
            let mut remaining = code;
            while let Some(start) = remaining.find(prefix) {
                let slice = &remaining[start..];
                if let Ok((_, mut table)) = parse_table_struct(slice) {
                    table.dialect = dialect;
                    tables.push((dialect, table));
                }
                remaining = &remaining[start + prefix.len()..];
            }
        }

        tables
    }

    /// Parse all index structs from code
    fn parse_indexes(code: &str) -> Vec<(Dialect, ParsedIndex)> {
        let mut indexes = Vec::new();

        for (prefix, dialect) in [
            ("#[SQLiteIndex", Dialect::SQLite),
            ("#[PostgresIndex", Dialect::PostgreSQL),
            ("#[MySQLIndex", Dialect::MySQL),
        ] {
            let mut remaining = code;
            while let Some(start) = remaining.find(prefix) {
                let slice = &remaining[start..];
                if let Ok((_, mut index)) = parse_index_struct(slice) {
                    index.dialect = dialect;
                    indexes.push((dialect, index));
                }
                remaining = &remaining[start + prefix.len()..];
            }
        }

        indexes
    }

    /// Parse schema struct from code
    fn parse_schema(code: &str) -> Option<(Dialect, ParsedSchema)> {
        for (derive, dialect) in [
            ("#[derive(SQLiteSchema)]", Dialect::SQLite),
            ("#[derive(PostgresSchema)]", Dialect::PostgreSQL),
            ("#[derive(MySQLSchema)]", Dialect::MySQL),
        ] {
            if let Some(start) = code.find(derive) {
                let slice = &code[start..];
                if let Ok((_, mut schema)) = parse_schema_struct(slice) {
                    schema.dialect = dialect;
                    return Some((dialect, schema));
                }
            }
        }
        None
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
        assert_eq!(result.dialect, Dialect::SQLite);

        let users = result.table("Users", Dialect::SQLite).unwrap();
        assert_eq!(users.name, "Users");
        assert_eq!(users.fields.len(), 3);

        let id_field = users.field("id").unwrap();
        assert!(id_field.has_attr("primary"));
        assert!(id_field.has_attr("autoincrement"));
        assert!(id_field.is_primary_key());

        let email_field = users.field("email").unwrap();
        assert!(email_field.is_nullable());
    }

    #[test]
    fn test_parse_table_with_options() {
        let code = r#"
#[SQLiteTable(strict, without_rowid)]
struct Products {
    #[column(primary)]
    id: String,
    name: String,
}
"#;
        let result = SchemaParser::parse(code);
        let products = result.table("Products", Dialect::SQLite).unwrap();
        assert!(products.has_table_attr("strict"));
        assert!(products.has_table_attr("without_rowid"));
    }

    #[test]
    fn test_parse_index() {
        let code = r#"
#[SQLiteIndex(unique)]
struct IdxUsersEmail(Users::email);
"#;
        let result = SchemaParser::parse(code);
        let idx = result.index("IdxUsersEmail", Dialect::SQLite).unwrap();
        assert!(idx.is_unique());
        assert_eq!(idx.columns, vec!["Users::email"]);
    }

    #[test]
    fn test_parse_schema() {
        let code = r#"
#[SQLiteTable]
struct Users {
    id: i64,
}

#[derive(SQLiteSchema)]
struct AppSchema {
    users: Users,
}
"#;
        let result = SchemaParser::parse(code);
        assert!(result.schema.is_some());
        let schema = result.schema.unwrap();
        assert_eq!(schema.name, "AppSchema");
        assert!(schema.members.contains_key("users"));
    }

    #[test]
    fn test_parse_postgres_table() {
        let code = r#"
#[PostgresTable]
struct Users {
    #[column(primary, identity)]
    id: i32,
    name: String,
}
"#;
        let result = SchemaParser::parse(code);
        assert_eq!(result.dialect, Dialect::PostgreSQL);

        let users = result.table("Users", Dialect::PostgreSQL).unwrap();
        assert_eq!(users.dialect, Dialect::PostgreSQL);
    }

    #[test]
    fn test_parse_field_with_references() {
        let code = r#"
#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i64,
    #[column(references = Users::id, on_delete = Cascade)]
    user_id: i64,
}
"#;
        let result = SchemaParser::parse(code);
        let posts = result.table("Posts", Dialect::SQLite).unwrap();
        let user_id = posts.field("user_id").unwrap();

        assert_eq!(user_id.references(), Some("Users::id".to_string()));
        assert_eq!(user_id.on_delete(), Some("Cascade".to_string()));
    }

    #[test]
    fn test_multi_dialect_schema() {
        let code = r#"
#[SQLiteTable]
struct SqliteUsers {
    id: i64,
}

#[PostgresTable]
struct PostgresUsers {
    id: i32,
}
"#;
        let result = SchemaParser::parse(code);
        assert!(result.table("SqliteUsers", Dialect::SQLite).is_some());
        assert!(result.table("PostgresUsers", Dialect::PostgreSQL).is_some());
    }

    #[test]
    fn test_attr_values_with_defaults() {
        let code = r#"
#[SQLiteTable]
struct Config {
    #[column(default = 42)]
    count: i64,
    #[column(default = "hello")]
    message: String,
}
"#;
        let result = SchemaParser::parse(code);
        let config = result.table("Config", Dialect::SQLite).unwrap();

        let count = config.field("count").unwrap();
        assert_eq!(count.default_value(), Some("42".to_string()));

        let message = config.field("message").unwrap();
        assert_eq!(message.default_value(), Some("\"hello\"".to_string()));
    }

    #[test]
    fn test_parse_postgres_index() {
        let code = r#"
#[PostgresIndex]
struct IdxUsersName(Users::name);
"#;
        let result = SchemaParser::parse(code);
        let idx = result.index("IdxUsersName", Dialect::PostgreSQL).unwrap();
        assert_eq!(idx.dialect, Dialect::PostgreSQL);
        assert!(!idx.is_unique());
    }

    #[test]
    fn test_parse_postgres_schema() {
        let code = r#"
#[derive(PostgresSchema)]
struct DbSchema {
    users: Users,
    posts: Posts,
}
"#;
        let result = SchemaParser::parse(code);
        assert!(result.schema.is_some());
        let schema = result.schema.unwrap();
        assert_eq!(schema.dialect, Dialect::PostgreSQL);
        assert_eq!(schema.members.len(), 2);
    }

    #[test]
    fn test_dialect_detection() {
        let sqlite_code = r#"
#[SQLiteTable]
struct T { id: i64 }
"#;
        let pg_code = r#"
#[PostgresTable]
struct T { id: i32 }
"#;
        assert_eq!(SchemaParser::parse(sqlite_code).dialect, Dialect::SQLite);
        assert_eq!(SchemaParser::parse(pg_code).dialect, Dialect::PostgreSQL);
    }
}
