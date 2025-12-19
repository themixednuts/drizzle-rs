//! SQLite DDL types - re-exports from drizzle_types plus parsing types

// Re-export everything from drizzle_types::sqlite::ddl
pub use drizzle_types::sqlite::ddl::*;

// =============================================================================
// Parsing Types - Used during introspection to parse CREATE TABLE statements
// =============================================================================

/// Parsed table options from CREATE TABLE SQL
#[derive(Debug, Clone, Default)]
pub struct ParsedTable {
    /// Whether the table has STRICT mode enabled
    pub strict: bool,
    /// Whether the table is WITHOUT ROWID
    pub without_rowid: bool,
    /// Unique constraints parsed from the DDL
    pub uniques: Vec<ParsedUnique>,
}

/// Parse table options from CREATE TABLE SQL
///
/// This is a simple parser that extracts basic table options.
/// For full constraint parsing, use the more complete introspection methods.
pub fn parse_table_ddl(sql: &str) -> ParsedTable {
    let sql_upper = sql.to_uppercase();
    ParsedTable {
        strict: sql_upper.contains(" STRICT"),
        without_rowid: sql_upper.contains("WITHOUT ROWID"),
        uniques: Vec::new(), // Unique constraints are parsed separately via pragma
    }
}

/// Parsed generated column information from CREATE TABLE SQL
#[derive(Debug, Clone)]
pub struct ParsedGenerated {
    /// SQL expression for generation (e.g., "first_name || ' ' || last_name")
    pub expression: String,
    /// Generation type: stored or virtual
    pub gen_type: GeneratedType,
}

/// Parsed unique constraint from CREATE TABLE SQL
#[derive(Debug, Clone)]
pub struct ParsedUnique {
    /// Constraint name (if explicitly named)
    pub name: Option<String>,
    /// Column names in the unique constraint
    pub columns: Vec<String>,
}

/// Parsed foreign key constraint from CREATE TABLE SQL
#[derive(Debug, Clone)]
pub struct ParsedForeignKey {
    /// Constraint name (if explicitly named)
    pub name: Option<String>,
    /// Source columns
    pub columns: Vec<String>,
    /// Referenced table
    pub table_to: String,
    /// Referenced columns
    pub columns_to: Vec<String>,
    /// ON DELETE action
    pub on_delete: Option<String>,
    /// ON UPDATE action
    pub on_update: Option<String>,
}

/// Parsed primary key information from CREATE TABLE SQL
#[derive(Debug, Clone)]
pub struct ParsedPrimaryKey {
    /// Column names in the primary key
    pub columns: Vec<String>,
    /// Whether any column is autoincrement
    pub autoincrement: bool,
}

/// Parsed check constraint from CREATE TABLE SQL
#[derive(Debug, Clone)]
pub struct ParsedCheck {
    /// Constraint name (if explicitly named)
    pub name: Option<String>,
    /// Check expression
    pub expression: String,
}
