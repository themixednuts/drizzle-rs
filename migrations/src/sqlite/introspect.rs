//! SQLite database introspection
//!
//! This module provides functionality to introspect an existing SQLite database
//! and extract its schema as DDL entities, matching drizzle-kit introspect.ts

use super::ddl::{
    Column, ForeignKey, Index, IndexColumn, IndexOrigin, PrimaryKey, SqliteEntity, Table,
    UniqueConstraint, View,
};
use super::snapshot::SQLiteSnapshot;

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
    let columns: Vec<Column> = raw_columns
        .iter()
        .filter(|c| c.hidden != 2 && c.hidden != 3) // Filter out hidden columns
        .map(|c| {
            let key = format!("{}:{}", c.table, c.name);
            let generated = generated_columns.get(&key).map(|g| super::ddl::Generated {
                expression: g.expression.clone().into(),
                gen_type: g.gen_type,
            });

            let is_autoincrement = is_auto_increment(&c.sql, &c.name);

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

/// Check if a column is autoincrement based on the CREATE TABLE SQL
fn is_auto_increment(sql: &Option<String>, column_name: &str) -> bool {
    if let Some(sql) = sql {
        // Check if the column is marked as INTEGER PRIMARY KEY AUTOINCREMENT
        let pattern = format!(
            r#"(?i)["'`\[]?{}["'`\]]?\s+INTEGER\s+PRIMARY\s+KEY\s+AUTOINCREMENT"#,
            regex::escape(column_name)
        );
        if let Ok(re) = regex::Regex::new(&pattern) {
            return re.is_match(sql);
        }
    }
    false
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
            let columns_cow: Vec<Cow<'static, str>> =
                fks.iter().map(|f| Cow::Owned(f.from_column.clone())).collect();
            let columns_to_cow: Vec<Cow<'static, str>> =
                fks.iter().map(|f| Cow::Owned(f.to_column.clone())).collect();

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
    let columns_cow: Vec<Cow<'static, str>> =
        pk_columns.into_iter().map(Cow::Owned).collect();

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

    let columns_cow: Vec<Cow<'static, str>> =
        columns.into_iter().map(Cow::Owned).collect();

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
            let columns_cow: Vec<Cow<'static, str>> =
                parsed.columns.iter().map(|c| Cow::Owned(c.clone())).collect();

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
    fn test_is_auto_increment() {
        let sql = Some(
            "CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT)".to_string(),
        );
        assert!(is_auto_increment(&sql, "id"));
        assert!(!is_auto_increment(&sql, "name"));
    }
}
