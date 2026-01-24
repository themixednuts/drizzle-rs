//! SQLite database introspection
//!
//! This module provides functionality to introspect an existing SQLite database
//! and extract its schema as DDL entities, matching drizzle-kit introspect.ts

use super::ddl::{
    Column, ForeignKey, Index, IndexColumn, IndexOrigin, PrimaryKey, SqliteEntity, Table,
    UniqueConstraint, View,
};
use super::ddl::{GeneratedType, ParsedGenerated};
use super::snapshot::SQLiteSnapshot;
use std::collections::HashMap;

/// Error type for introspection operations
#[derive(Debug, Clone)]
pub struct IntrospectError {
    pub message: String,
    pub table: Option<String>,
}

impl std::fmt::Display for IntrospectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(table) = &self.table {
            write!(f, "Introspection error for '{}': {}", table, self.message)
        } else {
            write!(f, "Introspection error: {}", self.message)
        }
    }
}

impl std::error::Error for IntrospectError {}

/// Result type for introspection
pub type IntrospectResult<T> = Result<T, IntrospectError>;

/// Raw column info from pragma_table_xinfo
#[derive(Debug, Clone)]
pub struct RawColumnInfo {
    pub table: String,
    pub cid: i32,
    pub name: String,
    pub column_type: String,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub pk: i32,
    pub hidden: i32,
    pub sql: Option<String>,
}

/// Raw index info from pragma_index_list
#[derive(Debug, Clone)]
pub struct RawIndexInfo {
    pub table: String,
    pub name: String,
    pub unique: bool,
    pub origin: String, // 'c' for CREATE INDEX, 'u' for UNIQUE, 'pk' for PRIMARY KEY
    pub partial: bool,
}

/// Raw index column from pragma_index_xinfo
#[derive(Debug, Clone)]
pub struct RawIndexColumn {
    pub index_name: String,
    pub seqno: i32,
    pub cid: i32,
    pub name: Option<String>,
    pub desc: bool,
    pub coll: String,
    pub key: bool,
}

/// Raw foreign key info from pragma_foreign_key_list
#[derive(Debug, Clone)]
pub struct RawForeignKey {
    pub table: String,
    pub id: i32,
    pub seq: i32,
    pub to_table: String,
    pub from_column: String,
    pub to_column: String,
    pub on_update: String,
    pub on_delete: String,
    pub r#match: String,
}

/// Raw view info
#[derive(Debug, Clone)]
pub struct RawViewInfo {
    pub name: String,
    pub sql: String,
}

/// Entity filter function type
pub type EntityFilter = Box<dyn Fn(&str, &str) -> bool>;

/// Default entity filter that allows everything
pub fn default_filter() -> EntityFilter {
    Box::new(|_entity_type, _name| true)
}

/// System table filter - excludes SQLite system tables and drizzle migrations
pub fn system_table_filter(name: &str) -> bool {
    !name.starts_with("sqlite_")
        && !name.starts_with("_cf_")
        && !name.starts_with("_litestream_")
        && !name.starts_with("libsql_")
        && !name.starts_with("d1_")
        && name != "__drizzle_migrations"
}

/// Introspection result containing all extracted entities
#[derive(Debug, Clone, Default)]
pub struct IntrospectionResult {
    pub tables: Vec<Table>,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
    pub primary_keys: Vec<PrimaryKey>,
    pub unique_constraints: Vec<UniqueConstraint>,
    pub views: Vec<View>,
    pub errors: Vec<IntrospectError>,
}

impl IntrospectionResult {
    /// Convert to a snapshot
    pub fn to_snapshot(&self) -> SQLiteSnapshot {
        let mut snapshot = SQLiteSnapshot::new();

        for table in &self.tables {
            snapshot.add_entity(SqliteEntity::Table(table.clone()));
        }
        for column in &self.columns {
            snapshot.add_entity(SqliteEntity::Column(column.clone()));
        }
        for index in &self.indexes {
            snapshot.add_entity(SqliteEntity::Index(index.clone()));
        }
        for fk in &self.foreign_keys {
            snapshot.add_entity(SqliteEntity::ForeignKey(fk.clone()));
        }
        for pk in &self.primary_keys {
            snapshot.add_entity(SqliteEntity::PrimaryKey(pk.clone()));
        }
        for unique in &self.unique_constraints {
            snapshot.add_entity(SqliteEntity::UniqueConstraint(unique.clone()));
        }
        for view in &self.views {
            snapshot.add_entity(SqliteEntity::View(view.clone()));
        }

        snapshot
    }

    /// Check if introspection had any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
}

/// Process raw column info into Column entities
pub fn process_columns(
    raw_columns: &[RawColumnInfo],
    generated_columns: &std::collections::HashMap<String, super::ddl::ParsedGenerated>,
    _pk_columns: &std::collections::HashSet<(String, String)>, // (table, column) - reserved for future use
) -> (Vec<Column>, Vec<PrimaryKey>) {
    // Precompute AUTOINCREMENT columns once per table (avoids per-column regex compilation).
    let mut autoinc_by_table: HashMap<String, std::collections::HashSet<String>> = HashMap::new();
    for c in raw_columns {
        if autoinc_by_table.contains_key(&c.table) {
            continue;
        }
        let Some(sql) = c.sql.as_deref() else {
            continue;
        };
        autoinc_by_table.insert(
            c.table.clone(),
            parse_autoincrement_columns_from_table_sql(sql),
        );
    }

    let columns: Vec<Column> = raw_columns
        .iter()
        .filter(|c| c.hidden != 2 && c.hidden != 3) // Filter out hidden columns
        .map(|c| {
            let key = format!("{}:{}", c.table, c.name);
            let generated = generated_columns.get(&key).map(|g| super::ddl::Generated {
                expression: g.expression.clone().into(),
                gen_type: g.gen_type,
            });

            let is_autoincrement = autoinc_by_table
                .get(&c.table)
                .is_some_and(|set| set.contains(&c.name));

            Column {
                table: c.table.clone().into(),
                name: c.name.clone().into(),
                sql_type: normalize_sql_type(&c.column_type).into(),
                not_null: c.not_null,
                autoincrement: if is_autoincrement { Some(true) } else { None },
                primary_key: None, // Handled via PrimaryKey entity
                unique: None,      // Handled via UniqueConstraint entity
                default: c.default_value.clone().map(|s| s.into()),
                generated,
                ordinal_position: Some(c.cid),
            }
        })
        .collect();

    // Extract primary keys from raw columns
    let mut pk_map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();

    for c in raw_columns.iter().filter(|c| c.pk > 0) {
        pk_map
            .entry(c.table.clone())
            .or_default()
            .push(c.name.clone());
    }

    let primary_keys: Vec<PrimaryKey> = pk_map
        .into_iter()
        .map(|(table, cols)| {
            let name = super::ddl::name_for_pk(&table);
            PrimaryKey {
                table: table.into(),
                name: name.into(),
                name_explicit: false,
                columns: cols.into_iter().map(|c| c.into()).collect(),
            }
        })
        .collect();

    (columns, primary_keys)
}

/// Normalize a SQL type to lowercase canonical form
fn normalize_sql_type(sql_type: &str) -> String {
    sql_type.to_lowercase()
}

/// Parse AUTOINCREMENT columns from a CREATE TABLE SQL statement.
///
/// This avoids regex compilation in hot paths and is tolerant of common quoting styles.
fn parse_autoincrement_columns_from_table_sql(sql: &str) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();

    let sql = sql.trim();
    let Some(start) = sql.find('(') else {
        return out;
    };

    let mut depth = 0i32;
    let mut end: Option<usize> = None;
    for (i, ch) in sql.char_indices().skip(start) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(end) = end else {
        return out;
    };

    let body = &sql[start + 1..end];

    // Split on top-level commas (ignore commas inside parentheses).
    let mut parts: Vec<&str> = Vec::new();
    let mut part_start = 0usize;
    let mut p_depth = 0i32;
    for (i, ch) in body.char_indices() {
        match ch {
            '(' => p_depth += 1,
            ')' => p_depth -= 1,
            ',' if p_depth == 0 => {
                parts.push(body[part_start..i].trim());
                part_start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(body[part_start..].trim());

    for item in parts {
        if item.is_empty() {
            continue;
        }

        let upper = item.to_uppercase();
        // Skip table-level constraints.
        if upper.starts_with("CONSTRAINT ")
            || upper.starts_with("PRIMARY ")
            || upper.starts_with("UNIQUE ")
            || upper.starts_with("CHECK ")
            || upper.starts_with("FOREIGN ")
        {
            continue;
        }

        if !upper.contains("AUTOINCREMENT") {
            continue;
        }
        if !(upper.contains("INTEGER") && upper.contains("PRIMARY") && upper.contains("KEY")) {
            continue;
        }

        // Parse column name (first token, handling quotes/backticks/brackets).
        let rest = item.trim();
        let col_name: String;
        if let Some(r) = rest.strip_prefix('"') {
            let Some(endq) = r.find('"') else { continue };
            col_name = r[..endq].to_string();
        } else if let Some(r) = rest.strip_prefix('`') {
            let Some(endq) = r.find('`') else { continue };
            col_name = r[..endq].to_string();
        } else if let Some(r) = rest.strip_prefix('[') {
            let Some(endq) = r.find(']') else { continue };
            col_name = r[..endq].to_string();
        } else {
            let mut it = rest.split_whitespace();
            let Some(name) = it.next() else { continue };
            col_name = name.to_string();
        }

        out.insert(col_name);
    }

    out
}

/// Process raw index info into Index entities
pub fn process_indexes(
    raw_indexes: &[RawIndexInfo],
    index_columns: &[RawIndexColumn],
    _table_sql_map: &std::collections::HashMap<String, String>,
) -> Vec<Index> {
    raw_indexes
        .iter()
        .filter(|idx| idx.origin == "c") // Only CREATE INDEX indexes
        .map(|idx| {
            let columns: Vec<IndexColumn> = index_columns
                .iter()
                .filter(|c| c.index_name == idx.name && c.key)
                .filter_map(|c| {
                    c.name.clone().map(|name| IndexColumn {
                        value: name.into(),
                        is_expression: false,
                    })
                })
                .collect();

            Index {
                table: idx.table.clone().into(),
                name: idx.name.clone().into(),
                columns,
                is_unique: idx.unique,
                where_clause: None,
                origin: IndexOrigin::Manual,
            }
        })
        .collect()
}

/// Extract unique constraints from pragma index list + index_xinfo.
///
/// SQLite reports UNIQUE constraints (including inline column UNIQUE and table-level UNIQUE)
/// as indexes with `origin == "u"`. These should be represented as `UniqueConstraint` entities
/// so codegen can emit `#[column(unique)]` for single-column uniques.
pub fn process_unique_constraints_from_indexes(
    raw_indexes: &[RawIndexInfo],
    index_columns: &[RawIndexColumn],
) -> Vec<UniqueConstraint> {
    use std::borrow::Cow;

    raw_indexes
        .iter()
        .filter(|idx| idx.origin == "u")
        .filter_map(|idx| {
            let mut cols: Vec<(i32, Cow<'static, str>)> = index_columns
                .iter()
                .filter(|c| c.index_name == idx.name && c.key)
                .filter_map(|c| {
                    c.name
                        .as_ref()
                        .map(|name| (c.seqno, Cow::Owned(name.clone())))
                })
                .collect();

            cols.sort_by_key(|(seq, _)| *seq);
            let columns: Vec<Cow<'static, str>> = cols.into_iter().map(|(_, c)| c).collect();
            if columns.is_empty() {
                return None;
            }

            let columns_refs: Vec<&str> = columns.iter().map(|c| c.as_ref()).collect();
            let name = super::ddl::name_for_unique(&idx.table, &columns_refs);

            Some(UniqueConstraint {
                table: Cow::Owned(idx.table.clone()),
                name: Cow::Owned(name),
                name_explicit: false,
                columns: Cow::Owned(columns),
            })
        })
        .collect()
}

/// Process raw foreign key info into ForeignKey entities
pub fn process_foreign_keys(raw_fks: &[RawForeignKey]) -> Vec<ForeignKey> {
    use std::borrow::Cow;

    // Group by table and id
    let mut grouped: std::collections::HashMap<(String, i32), Vec<&RawForeignKey>> =
        std::collections::HashMap::new();

    for fk in raw_fks {
        grouped
            .entry((fk.table.clone(), fk.id))
            .or_default()
            .push(fk);
    }

    grouped
        .into_iter()
        .map(|((table, _id), fks)| {
            let mut fks = fks;
            fks.sort_by_key(|f| f.seq);

            let columns: Vec<&str> = fks.iter().map(|f| f.from_column.as_str()).collect();
            let columns_to: Vec<&str> = fks.iter().map(|f| f.to_column.as_str()).collect();
            let first = fks.first().unwrap();

            let name = super::ddl::name_for_fk(&table, &columns, &first.to_table, &columns_to);

            // Convert columns to Cow
            let columns_cow: Vec<Cow<'static, str>> = fks
                .iter()
                .map(|f| Cow::Owned(f.from_column.clone()))
                .collect();
            let columns_to_cow: Vec<Cow<'static, str>> = fks
                .iter()
                .map(|f| Cow::Owned(f.to_column.clone()))
                .collect();

            ForeignKey {
                table: table.into(),
                name: name.into(),
                name_explicit: false,
                columns: Cow::Owned(columns_cow),
                table_to: first.to_table.clone().into(),
                columns_to: Cow::Owned(columns_to_cow),
                on_update: Some(first.on_update.clone().into()),
                on_delete: Some(first.on_delete.clone().into()),
            }
        })
        .collect()
}

/// Create primary key constraint from column info
///
/// Note: Primary keys are now extracted directly in process_columns() along with columns
/// since the raw column info contains pk field
pub fn create_primary_key(table: &str, pk_columns: Vec<String>) -> PrimaryKey {
    use std::borrow::Cow;

    let name = super::ddl::name_for_pk(table);
    let columns_cow: Vec<Cow<'static, str>> = pk_columns.into_iter().map(Cow::Owned).collect();

    PrimaryKey {
        table: table.to_string().into(),
        name: name.into(),
        name_explicit: false,
        columns: Cow::Owned(columns_cow),
    }
}

/// Create a unique constraint from parsed info
pub fn create_unique_constraint(
    table: &str,
    name: &str,
    columns: Vec<String>,
    name_explicit: bool,
) -> UniqueConstraint {
    use std::borrow::Cow;

    let columns_cow: Vec<Cow<'static, str>> = columns.into_iter().map(Cow::Owned).collect();

    UniqueConstraint {
        table: Cow::Owned(table.to_string()),
        name: Cow::Owned(name.to_string()),
        name_explicit,
        columns: Cow::Owned(columns_cow),
    }
}

/// Extract unique constraints from parsed table info
pub fn process_unique_constraints_from_parsed(
    table: &str,
    parsed_uniques: &[super::ddl::ParsedUnique],
) -> Vec<UniqueConstraint> {
    use std::borrow::Cow;

    parsed_uniques
        .iter()
        .map(|parsed| {
            let columns_refs: Vec<&str> = parsed.columns.iter().map(|s| s.as_str()).collect();
            let (name, name_explicit) = match &parsed.name {
                Some(n) => (n.clone(), true),
                None => (super::ddl::name_for_unique(table, &columns_refs), false),
            };
            let columns_cow: Vec<Cow<'static, str>> = parsed
                .columns
                .iter()
                .map(|c| Cow::Owned(c.clone()))
                .collect();

            UniqueConstraint {
                table: table.to_string().into(),
                name: name.into(),
                name_explicit,
                columns: Cow::Owned(columns_cow),
            }
        })
        .collect()
}

/// SQL queries for SQLite introspection
pub mod queries {
    /// Query to get all tables
    pub const TABLES_QUERY: &str = r#"
        SELECT name, sql
        FROM sqlite_master
        WHERE type = 'table'
          AND name != '__drizzle_migrations'
          AND name NOT LIKE '\_cf\_%' ESCAPE '\'
          AND name NOT LIKE '\_litestream\_%' ESCAPE '\'
          AND name NOT LIKE 'libsql\_%' ESCAPE '\'
          AND name NOT LIKE 'sqlite\_%' ESCAPE '\'
          AND name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY name COLLATE NOCASE
    "#;

    /// Query to get all columns for a table using pragma_table_xinfo
    pub const COLUMNS_QUERY: &str = r#"
        SELECT 
            m.name as "table", 
            p.cid as "cid",
            p.name as "name", 
            p.type as "columnType",
            p."notnull" as "notNull", 
            p.dflt_value as "defaultValue",
            p.pk as pk,
            p.hidden as hidden,
            m.sql
        FROM sqlite_master AS m 
            JOIN pragma_table_xinfo(m.name) AS p
        WHERE 
            m.type = 'table'
            AND m.tbl_name != '__drizzle_migrations' 
            AND m.tbl_name NOT LIKE '\_cf\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE '\_litestream\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'libsql\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'sqlite\_%' ESCAPE '\'
            AND m.tbl_name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY p.cid
    "#;

    /// Query to get all views
    pub const VIEWS_QUERY: &str = r#"
        SELECT name, sql
        FROM sqlite_master
        WHERE type = 'view'
          AND name != '__drizzle_migrations'
          AND name NOT LIKE '\_cf\_%' ESCAPE '\'
          AND name NOT LIKE '\_litestream\_%' ESCAPE '\'
          AND name NOT LIKE 'libsql\_%' ESCAPE '\'
          AND name NOT LIKE 'sqlite\_%' ESCAPE '\'
          AND name NOT LIKE 'd1\_%' ESCAPE '\'
        ORDER BY name COLLATE NOCASE
    "#;

    /// Query template to get indexes for a table
    pub fn indexes_query(table_name: &str) -> String {
        format!("PRAGMA index_list(\"{}\")", table_name)
    }

    /// Query template to get index columns
    pub fn index_info_query(index_name: &str) -> String {
        format!("PRAGMA index_xinfo(\"{}\")", index_name)
    }

    /// Query template to get foreign keys for a table
    pub fn foreign_keys_query(table_name: &str) -> String {
        format!("PRAGMA foreign_key_list(\"{}\")", table_name)
    }
}

/// Parse a view SQL to extract the definition
pub fn parse_view_sql(sql: &str) -> Option<String> {
    // Extract the SELECT part from CREATE VIEW statement
    let upper = sql.to_uppercase();
    if let Some(as_pos) = upper.find(" AS ") {
        let definition = sql[as_pos + 4..].trim();
        // Remove trailing semicolon if present
        let definition = definition.trim_end_matches(';').trim();
        Some(definition.to_string())
    } else {
        None
    }
}

/// Parse generated columns from a CREATE TABLE SQL statement.
///
/// Returns a map keyed by `"table:column"` matching the key format used by `process_columns`.
///
/// This is intentionally a small, tolerant parser (not a full SQL parser). It handles common
/// SQLite syntax for generated columns:
/// - `col TYPE GENERATED ALWAYS AS (expr) STORED`
/// - `col TYPE GENERATED ALWAYS AS (expr) VIRTUAL`
pub fn parse_generated_columns_from_table_sql(
    table: &str,
    sql: &str,
) -> HashMap<String, ParsedGenerated> {
    let mut out: HashMap<String, ParsedGenerated> = HashMap::new();

    let sql = sql.trim();
    // Find the first '(' after CREATE TABLE and extract the table body until the matching ')'.
    let Some(start) = sql.find('(') else {
        return out;
    };

    let mut depth = 0i32;
    let mut end: Option<usize> = None;
    for (i, ch) in sql.char_indices().skip(start) {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(i);
                    break;
                }
            }
            _ => {}
        }
    }
    let Some(end) = end else {
        return out;
    };

    let body = &sql[start + 1..end];

    // Split on top-level commas (ignore commas inside parentheses).
    let mut parts: Vec<&str> = Vec::new();
    let mut part_start = 0usize;
    let mut p_depth = 0i32;
    for (i, ch) in body.char_indices() {
        match ch {
            '(' => p_depth += 1,
            ')' => p_depth -= 1,
            ',' if p_depth == 0 => {
                parts.push(body[part_start..i].trim());
                part_start = i + 1;
            }
            _ => {}
        }
    }
    parts.push(body[part_start..].trim());

    for item in parts {
        if item.is_empty() {
            continue;
        }
        let upper = item.to_uppercase();
        if !upper.contains("GENERATED") {
            continue;
        }
        // Skip table-level constraints.
        if upper.starts_with("CONSTRAINT ")
            || upper.starts_with("PRIMARY ")
            || upper.starts_with("UNIQUE ")
            || upper.starts_with("CHECK ")
            || upper.starts_with("FOREIGN ")
        {
            continue;
        }

        // Parse column name (first token, handling quotes/backticks/brackets).
        let mut rest = item.trim();
        let col_name: String;
        if let Some(r) = rest.strip_prefix('"') {
            if let Some(endq) = r.find('"') {
                col_name = r[..endq].to_string();
                rest = r[endq + 1..].trim_start();
            } else {
                continue;
            }
        } else if let Some(r) = rest.strip_prefix('`') {
            if let Some(endq) = r.find('`') {
                col_name = r[..endq].to_string();
                rest = r[endq + 1..].trim_start();
            } else {
                continue;
            }
        } else if let Some(r) = rest.strip_prefix('[') {
            if let Some(endq) = r.find(']') {
                col_name = r[..endq].to_string();
                rest = r[endq + 1..].trim_start();
            } else {
                continue;
            }
        } else {
            let mut it = rest.split_whitespace();
            let Some(name) = it.next() else { continue };
            col_name = name.to_string();
        }

        // Find " AS (" and extract expression with balanced parentheses.
        let upper_rest = rest.to_uppercase();
        let Some(as_pos) = upper_rest.find(" AS ") else {
            continue;
        };
        let after_as = &rest[as_pos + 4..];
        let Some(expr_start_rel) = after_as.find('(') else {
            continue;
        };
        let expr_start = as_pos + 4 + expr_start_rel;

        let mut expr_depth = 0i32;
        let mut expr_end: Option<usize> = None;
        for (i, ch) in rest.char_indices().skip(expr_start) {
            match ch {
                '(' => expr_depth += 1,
                ')' => {
                    expr_depth -= 1;
                    if expr_depth == 0 {
                        expr_end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(expr_end) = expr_end else {
            continue;
        };

        let expression = rest[expr_start + 1..expr_end].trim().to_string();
        let after_expr = rest[expr_end + 1..].to_uppercase();
        let gen_type = if after_expr.contains("STORED") {
            GeneratedType::Stored
        } else {
            GeneratedType::Virtual
        };

        out.insert(
            format!("{}:{}", table, col_name),
            ParsedGenerated {
                expression,
                gen_type,
            },
        );
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_table_filter() {
        assert!(!system_table_filter("sqlite_master"));
        assert!(!system_table_filter("__drizzle_migrations"));
        assert!(!system_table_filter("_cf_something"));
        assert!(system_table_filter("users"));
        assert!(system_table_filter("posts"));
    }

    #[test]
    fn test_parse_view_sql() {
        let sql = "CREATE VIEW active_users AS SELECT * FROM users WHERE active = 1";
        let definition = parse_view_sql(sql);
        assert_eq!(
            definition,
            Some("SELECT * FROM users WHERE active = 1".to_string())
        );
    }

    #[test]
    fn test_parse_autoincrement_columns_from_table_sql() {
        let sql = "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)";
        let cols = parse_autoincrement_columns_from_table_sql(sql);
        assert!(cols.contains("id"));
        assert!(!cols.contains("name"));
    }

    #[test]
    fn test_parse_generated_columns_from_table_sql() {
        let sql = r#"
CREATE TABLE users (
  id INTEGER PRIMARY KEY,
  first TEXT,
  last TEXT,
  full TEXT GENERATED ALWAYS AS (first || ' ' || last) VIRTUAL,
  total INT GENERATED ALWAYS AS ((id + 1) * 2) STORED
);
"#;
        let map = parse_generated_columns_from_table_sql("users", sql);
        let full = map.get("users:full").expect("full generated");
        assert_eq!(full.gen_type, GeneratedType::Virtual);
        assert!(full.expression.contains("first"));

        let total = map.get("users:total").expect("total generated");
        assert_eq!(total.gen_type, GeneratedType::Stored);
        assert!(total.expression.contains("id"));
    }
}
