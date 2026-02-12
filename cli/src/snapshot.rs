//! Schema snapshot builder from parsed schema files
//!
//! This module converts `ParseResult` from the schema parser into
//! `Snapshot` types that can be used for migration diffing.

use drizzle_migrations::parser::{ParseResult, ParsedField, ParsedIndex, ParsedTable};
use drizzle_migrations::postgres::PostgresSnapshot;
use drizzle_migrations::schema::Snapshot;
use drizzle_migrations::sqlite::SQLiteSnapshot;
use drizzle_types::Dialect;
use heck::{ToLowerCamelCase, ToSnakeCase};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};

use crate::config::Casing;

/// Convert a `ParseResult` into a `Snapshot` for migration diffing
///
/// Uses the provided `dialect` from config rather than the parser-detected dialect,
/// allowing users to have multi-dialect schema files and select which to use via config.
pub fn parse_result_to_snapshot(
    result: &ParseResult,
    dialect: Dialect,
    casing: Option<Casing>,
) -> Snapshot {
    match dialect {
        Dialect::SQLite => Snapshot::Sqlite(build_sqlite_snapshot(result, casing)),
        Dialect::PostgreSQL => Snapshot::Postgres(build_postgres_snapshot(result, casing)),
        _ => unreachable!("Unsupported dialect for drizzle-cli snapshot generation: {dialect:?}"),
    }
}

fn apply_casing(name: &str, casing: Casing) -> String {
    match casing {
        Casing::SnakeCase => name.to_snake_case(),
        Casing::CamelCase => name.to_lower_camel_case(),
    }
}

fn trim_wrapping_quotes(s: &str) -> String {
    s.trim().trim_matches('"').trim_matches('\'').to_string()
}

fn extract_index_name_attr(attr: &str) -> Option<String> {
    let start = attr.find('(')?;
    let end = attr.rfind(')')?;
    let content = &attr[start + 1..end];

    for part in content.split(',') {
        let part = part.trim();
        if let Some((k, v)) = part.split_once('=')
            && k.trim() == "name"
        {
            return Some(trim_wrapping_quotes(v));
        }
    }

    None
}

fn resolve_table_name(table: &ParsedTable, casing: Casing) -> String {
    table
        .attr_value("name")
        .map(|v| trim_wrapping_quotes(&v))
        .unwrap_or_else(|| apply_casing(&table.name, casing))
}

fn resolve_field_name(field: &ParsedField, casing: Casing) -> String {
    field
        .attr_value("name")
        .map(|v| trim_wrapping_quotes(&v))
        .unwrap_or_else(|| apply_casing(&field.name, casing))
}

/// Build an SQLite snapshot from parsed schema
fn build_sqlite_snapshot(result: &ParseResult, casing: Option<Casing>) -> SQLiteSnapshot {
    use drizzle_migrations::sqlite::{PrimaryKey, SqliteEntity, Table, UniqueConstraint};

    let mut snapshot = SQLiteSnapshot::new();
    let name_casing = casing.unwrap_or(Casing::SnakeCase);

    let sqlite_tables: Vec<_> = result
        .tables
        .values()
        .filter(|t| t.dialect == Dialect::SQLite)
        .collect();

    let mut table_name_map: HashMap<String, String> = HashMap::new();
    let mut field_name_map: HashMap<(String, String), String> = HashMap::new();
    for table in &sqlite_tables {
        let table_name = resolve_table_name(table, name_casing);
        table_name_map.insert(table.name.clone(), table_name);
        for field in &table.fields {
            field_name_map.insert(
                (table.name.clone(), field.name.clone()),
                resolve_field_name(field, name_casing),
            );
        }
    }

    // Process tables (only those matching SQLite dialect)
    for table in sqlite_tables {
        let table_name = table_name_map
            .get(&table.name)
            .cloned()
            .unwrap_or_else(|| resolve_table_name(table, name_casing));

        // Add table entity
        let mut sqlite_table = Table::new(table_name.clone());
        sqlite_table.strict = table.is_strict();
        sqlite_table.without_rowid = table.is_without_rowid();
        snapshot.add_entity(SqliteEntity::Table(sqlite_table));

        // Process columns
        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col_name = field_name_map
                .get(&(table.name.clone(), field.name.clone()))
                .cloned()
                .unwrap_or_else(|| resolve_field_name(field, name_casing));
            let col = build_sqlite_column(&table_name, field, &col_name);
            snapshot.add_entity(SqliteEntity::Column(col));

            // Track primary key columns
            if field.is_primary_key() {
                pk_columns.push(col_name.clone());
            }

            // Add unique constraint if column is unique (not primary)
            if field.is_unique() && !field.is_primary_key() {
                let constraint_name = format!("{}_{}_unique", table_name, col_name);
                snapshot.add_entity(SqliteEntity::UniqueConstraint(
                    UniqueConstraint::from_strings(
                        table_name.clone(),
                        constraint_name,
                        vec![col_name.clone()],
                    ),
                ));
            }

            // Add foreign key if references exist
            if let Some(ref_target) = field.references()
                && let Some(fk) = build_sqlite_foreign_key(
                    &table_name,
                    &col_name,
                    field,
                    &ref_target,
                    &table_name_map,
                    &field_name_map,
                    name_casing,
                )
            {
                snapshot.add_entity(SqliteEntity::ForeignKey(fk));
            }
        }

        // Add primary key entity
        if !pk_columns.is_empty() {
            let pk_name = format!("{}_pkey", table_name);
            snapshot.add_entity(SqliteEntity::PrimaryKey(PrimaryKey::from_strings(
                table_name, pk_name, pk_columns,
            )));
        }
    }

    // Process indexes (only those matching SQLite dialect)
    for index in result
        .indexes
        .values()
        .filter(|i| i.dialect == Dialect::SQLite)
    {
        let idx = build_sqlite_index(index, &table_name_map, &field_name_map, name_casing);
        snapshot.add_entity(SqliteEntity::Index(idx));
    }

    snapshot
}

/// Build a PostgreSQL snapshot from parsed schema
fn build_postgres_snapshot(result: &ParseResult, casing: Option<Casing>) -> PostgresSnapshot {
    use drizzle_migrations::postgres::{
        PostgresEntity, PrimaryKey, Schema as PgSchema, Table, UniqueConstraint,
    };

    let mut snapshot = PostgresSnapshot::new();
    let name_casing = casing.unwrap_or(Casing::SnakeCase);

    let pg_tables: Vec<_> = result
        .tables
        .values()
        .filter(|t| t.dialect == Dialect::PostgreSQL)
        .collect();

    let mut table_name_map: HashMap<String, String> = HashMap::new();
    let mut field_name_map: HashMap<(String, String), String> = HashMap::new();
    for table in &pg_tables {
        table_name_map.insert(table.name.clone(), resolve_table_name(table, name_casing));
        for field in &table.fields {
            field_name_map.insert(
                (table.name.clone(), field.name.clone()),
                resolve_field_name(field, name_casing),
            );
        }
    }

    // Map parsed struct name -> schema for cross-entity resolution (FKs/indexes)
    let mut table_schemas: HashMap<String, String> = HashMap::new();
    let mut schemas: HashSet<String> = HashSet::new();
    for table in &pg_tables {
        let schema_name = table.schema_name().unwrap_or_else(|| "public".to_string());
        table_schemas.insert(table.name.clone(), schema_name.clone());
        schemas.insert(schema_name);
    }
    if schemas.is_empty() {
        schemas.insert("public".to_string());
    }

    // Add all discovered schemas in deterministic order
    let mut schema_list: Vec<String> = schemas.into_iter().collect();
    schema_list.sort();
    for schema in schema_list {
        snapshot.add_entity(PostgresEntity::Schema(PgSchema::new(schema)));
    }

    // Process tables (only those matching PostgreSQL dialect)
    for table in pg_tables {
        let table_name = table_name_map
            .get(&table.name)
            .cloned()
            .unwrap_or_else(|| resolve_table_name(table, name_casing));
        let schema_name = table.schema_name().unwrap_or_else(|| "public".to_string());

        // Add table entity
        snapshot.add_entity(PostgresEntity::Table(Table {
            schema: schema_name.clone().into(),
            name: table_name.clone().into(),
            is_rls_enabled: None,
        }));

        // Process columns
        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col_name = field_name_map
                .get(&(table.name.clone(), field.name.clone()))
                .cloned()
                .unwrap_or_else(|| resolve_field_name(field, name_casing));
            let col = build_postgres_column(&schema_name, &table_name, field, &col_name);
            snapshot.add_entity(PostgresEntity::Column(col));

            // Track primary key columns
            if field.is_primary_key() {
                pk_columns.push(col_name.clone());
            }

            // Add unique constraint if column is unique (not primary)
            if field.is_unique() && !field.is_primary_key() {
                snapshot.add_entity(PostgresEntity::UniqueConstraint(
                    UniqueConstraint::from_strings(
                        schema_name.clone(),
                        table_name.clone(),
                        format!("{}_{}_key", table_name, col_name),
                        vec![col_name.clone()],
                    ),
                ));
            }

            // Add foreign key if references exist
            if let Some(ref_target) = field.references()
                && let Some(fk) = build_postgres_foreign_key(
                    &schema_name,
                    &table_name,
                    &col_name,
                    field,
                    &ref_target,
                    &table_name_map,
                    &field_name_map,
                    &table_schemas,
                    name_casing,
                )
            {
                snapshot.add_entity(PostgresEntity::ForeignKey(fk));
            }
        }

        // Add primary key entity
        if !pk_columns.is_empty() {
            snapshot.add_entity(PostgresEntity::PrimaryKey(PrimaryKey::from_strings(
                schema_name,
                table_name.clone(),
                format!("{}_pkey", table_name),
                pk_columns,
            )));
        }
    }

    // Process indexes (only those matching PostgreSQL dialect)
    for index in result
        .indexes
        .values()
        .filter(|i| i.dialect == Dialect::PostgreSQL)
    {
        let idx = build_postgres_index(
            index,
            &table_name_map,
            &field_name_map,
            &table_schemas,
            name_casing,
        );
        snapshot.add_entity(PostgresEntity::Index(idx));
    }

    snapshot
}

/// Build an SQLite column from a parsed field
fn build_sqlite_column(
    table_name: &str,
    field: &ParsedField,
    col_name: &str,
) -> drizzle_migrations::sqlite::Column {
    use drizzle_migrations::sqlite::Column;

    let col_type = infer_sqlite_type(&field.ty);

    let mut col = Column::new(table_name.to_string(), col_name.to_string(), col_type);

    if !field.is_nullable() {
        col = col.not_null();
    }

    if field.is_autoincrement() {
        col = col.autoincrement();
    }

    if let Some(default) = field.default_value() {
        col = col.default_value(default);
    }

    col
}

/// Build a PostgreSQL column from a parsed field
fn build_postgres_column(
    schema_name: &str,
    table_name: &str,
    field: &ParsedField,
    col_name: &str,
) -> drizzle_migrations::postgres::Column {
    use drizzle_migrations::postgres::ddl::IdentityType;
    use drizzle_migrations::postgres::{Column, Identity};

    let col_type = infer_postgres_type(&field.ty);
    let is_serial = field.has_attr("serial") || field.has_attr("bigserial");
    let is_identity = field.has_attr("generated") || field.has_attr("identity");

    Column {
        schema: schema_name.to_string().into(),
        table: table_name.to_string().into(),
        name: col_name.to_string().into(),
        sql_type: col_type.into(),
        type_schema: None,
        not_null: !field.is_nullable(),
        default: field.default_value().map(Cow::Owned),
        generated: None,
        identity: if is_serial || is_identity {
            Some(Identity {
                name: format!("{}_{}_seq", table_name, col_name).into(),
                schema: Some(schema_name.to_string().into()),
                type_: if is_identity {
                    IdentityType::Always
                } else {
                    IdentityType::ByDefault
                },
                increment: None,
                min_value: None,
                max_value: None,
                start_with: None,
                cache: None,
                cycle: None,
            })
        } else {
            None
        },
        dimensions: None,
        ordinal_position: None,
    }
}

/// Build an SQLite foreign key from a parsed field
fn build_sqlite_foreign_key(
    table_name: &str,
    col_name: &str,
    field: &ParsedField,
    ref_target: &str,
    table_name_map: &HashMap<String, String>,
    field_name_map: &HashMap<(String, String), String>,
    casing: Casing,
) -> Option<drizzle_migrations::sqlite::ForeignKey> {
    use drizzle_migrations::sqlite::ForeignKey;

    // Parse "Table::column" reference
    let parts: Vec<&str> = ref_target.split("::").collect();
    if parts.len() != 2 {
        return None;
    }

    let ref_table = table_name_map
        .get(parts[0])
        .cloned()
        .unwrap_or_else(|| apply_casing(parts[0], casing));
    let ref_column = field_name_map
        .get(&(parts[0].to_string(), parts[1].to_string()))
        .cloned()
        .unwrap_or_else(|| apply_casing(parts[1], casing));
    let fk_name = format!(
        "{}_{}_{}_{}_fk",
        table_name, col_name, ref_table, ref_column
    );

    let mut fk = ForeignKey::from_strings(
        table_name.to_string(),
        fk_name,
        vec![col_name.to_string()],
        ref_table,
        vec![ref_column],
    );

    fk.on_delete = field.on_delete().map(Cow::Owned);
    fk.on_update = field.on_update().map(Cow::Owned);

    Some(fk)
}

/// Build a PostgreSQL foreign key from a parsed field
fn build_postgres_foreign_key(
    schema_name: &str,
    table_name: &str,
    col_name: &str,
    field: &ParsedField,
    ref_target: &str,
    table_name_map: &HashMap<String, String>,
    field_name_map: &HashMap<(String, String), String>,
    table_schemas: &HashMap<String, String>,
    casing: Casing,
) -> Option<drizzle_migrations::postgres::ForeignKey> {
    use drizzle_migrations::postgres::ForeignKey;

    // Parse "Table::column" reference
    let parts: Vec<&str> = ref_target.split("::").collect();
    if parts.len() != 2 {
        return None;
    }

    let ref_table_struct = parts[0];
    let ref_table = table_name_map
        .get(ref_table_struct)
        .cloned()
        .unwrap_or_else(|| apply_casing(ref_table_struct, casing));
    let ref_column = field_name_map
        .get(&(ref_table_struct.to_string(), parts[1].to_string()))
        .cloned()
        .unwrap_or_else(|| apply_casing(parts[1], casing));
    let ref_schema = table_schemas
        .get(ref_table_struct)
        .cloned()
        .unwrap_or_else(|| "public".to_string());
    let fk_name = format!(
        "{}_{}_{}_{}_fk",
        table_name, col_name, ref_table, ref_column
    );

    Some(ForeignKey {
        schema: schema_name.to_string().into(),
        table: table_name.to_string().into(),
        name: fk_name.into(),
        name_explicit: false,
        columns: Cow::Owned(vec![Cow::Owned(col_name.to_string())]),
        schema_to: ref_schema.into(),
        table_to: ref_table.into(),
        columns_to: Cow::Owned(vec![Cow::Owned(ref_column)]),
        on_update: field.on_update().map(Cow::Owned),
        on_delete: field.on_delete().map(Cow::Owned),
    })
}

/// Build an SQLite index from a parsed index
fn build_sqlite_index(
    index: &ParsedIndex,
    table_name_map: &HashMap<String, String>,
    field_name_map: &HashMap<(String, String), String>,
    casing: Casing,
) -> drizzle_migrations::sqlite::Index {
    use drizzle_migrations::sqlite::{Index, IndexColumn, IndexOrigin};

    let table_struct = index.table_name().unwrap_or_default();
    let table_name = table_name_map
        .get(table_struct)
        .cloned()
        .unwrap_or_else(|| apply_casing(table_struct, casing));
    let index_name =
        extract_index_name_attr(&index.attr).unwrap_or_else(|| apply_casing(&index.name, casing));

    let columns: Vec<IndexColumn> = index
        .columns
        .iter()
        .filter_map(|c| {
            // Parse "Table::column" and extract just the column
            let mut parts = c.split("::");
            let table = parts.next()?;
            let field = parts.next()?;
            let col_name = field_name_map
                .get(&(table.to_string(), field.to_string()))
                .cloned()
                .unwrap_or_else(|| apply_casing(field, casing));
            Some(IndexColumn::new(col_name))
        })
        .collect();

    Index {
        table: table_name.into(),
        name: index_name.into(),
        columns,
        is_unique: index.is_unique(),
        where_clause: None,
        origin: IndexOrigin::Manual,
    }
}

/// Build a PostgreSQL index from a parsed index
fn build_postgres_index(
    index: &ParsedIndex,
    table_name_map: &HashMap<String, String>,
    field_name_map: &HashMap<(String, String), String>,
    table_schemas: &HashMap<String, String>,
    casing: Casing,
) -> drizzle_migrations::postgres::Index {
    use drizzle_migrations::postgres::{Index, IndexColumn};

    let table_struct = index.table_name().unwrap_or_default();
    let table_name = table_name_map
        .get(table_struct)
        .cloned()
        .unwrap_or_else(|| apply_casing(table_struct, casing));
    let schema_name = table_schemas
        .get(table_struct)
        .cloned()
        .unwrap_or_else(|| "public".to_string());
    let index_name =
        extract_index_name_attr(&index.attr).unwrap_or_else(|| apply_casing(&index.name, casing));

    let columns: Vec<IndexColumn> = index
        .columns
        .iter()
        .filter_map(|c| {
            let mut parts = c.split("::");
            let table = parts.next()?;
            let field = parts.next()?;
            let col_name = field_name_map
                .get(&(table.to_string(), field.to_string()))
                .cloned()
                .unwrap_or_else(|| apply_casing(field, casing));
            Some(IndexColumn::new(col_name))
        })
        .collect();

    Index {
        schema: schema_name.into(),
        table: table_name.into(),
        name: index_name.into(),
        name_explicit: false,
        columns,
        is_unique: index.is_unique(),
        where_clause: index.where_clause().map(Cow::Owned),
        method: index.method().map(Cow::Owned),
        with: None,
        concurrently: index.is_concurrent(),
    }
}

/// Infer SQLite type from Rust type string
fn infer_sqlite_type(rust_type: &str) -> String {
    let base_type = rust_type
        .trim()
        .strip_prefix("Option<")
        .and_then(|s| s.strip_suffix(">"))
        .unwrap_or(rust_type)
        .trim();

    match base_type {
        "i8" | "i16" | "i32" | "i64" | "u8" | "u16" | "u32" | "u64" | "isize" | "usize"
        | "bool" => "integer".to_string(),
        "f32" | "f64" => "real".to_string(),
        "String" | "&str" | "str" => "text".to_string(),
        "Vec<u8>" | "[u8]" => "blob".to_string(),
        _ if base_type.contains("Uuid") => "text".to_string(),
        _ if base_type.contains("DateTime") => "text".to_string(),
        _ if base_type.contains("NaiveDate") => "text".to_string(),
        _ => "any".to_string(),
    }
}

/// Infer PostgreSQL type from Rust type string
fn infer_postgres_type(rust_type: &str) -> String {
    let base_type = rust_type
        .trim()
        .strip_prefix("Option<")
        .and_then(|s| s.strip_suffix(">"))
        .unwrap_or(rust_type)
        .trim();

    match base_type {
        "i16" => "smallint".to_string(),
        "i32" => "integer".to_string(),
        "i64" => "bigint".to_string(),
        "u8" | "u16" | "u32" => "integer".to_string(),
        "u64" => "bigint".to_string(),
        "f32" => "real".to_string(),
        "f64" => "double precision".to_string(),
        "bool" => "boolean".to_string(),
        "String" | "&str" | "str" => "text".to_string(),
        "Vec<u8>" | "[u8]" => "bytea".to_string(),
        _ if base_type.contains("Uuid") => "uuid".to_string(),
        _ if base_type.contains("DateTime") => "timestamptz".to_string(),
        _ if base_type.contains("NaiveDateTime") => "timestamp".to_string(),
        _ if base_type.contains("NaiveDate") => "date".to_string(),
        _ if base_type.contains("NaiveTime") => "time".to_string(),
        _ if base_type.contains("IpAddr") => "inet".to_string(),
        _ if base_type.contains("MacAddr") => "macaddr".to_string(),
        _ if base_type.contains("Point") => "point".to_string(),
        _ if base_type.contains("Decimal") => "numeric".to_string(),
        _ => "text".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_sqlite_type() {
        assert_eq!(infer_sqlite_type("i32"), "integer");
        assert_eq!(infer_sqlite_type("i64"), "integer");
        assert_eq!(infer_sqlite_type("f64"), "real");
        assert_eq!(infer_sqlite_type("String"), "text");
        assert_eq!(infer_sqlite_type("Option<String>"), "text");
        assert_eq!(infer_sqlite_type("Vec<u8>"), "blob");
    }

    #[test]
    fn test_infer_postgres_type() {
        assert_eq!(infer_postgres_type("i32"), "integer");
        assert_eq!(infer_postgres_type("i64"), "bigint");
        assert_eq!(infer_postgres_type("bool"), "boolean");
        assert_eq!(infer_postgres_type("String"), "text");
        assert_eq!(infer_postgres_type("Vec<u8>"), "bytea");
        assert_eq!(infer_postgres_type("Uuid"), "uuid");
    }

    /// Test that changing a column from Option<String> to String generates table recreation
    #[test]
    fn test_nullable_to_not_null_generates_migration() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::sqlite::collection::SQLiteDDL;
        use drizzle_migrations::sqlite::diff::compute_migration;

        // Previous schema: email is nullable (Option<String>)
        let prev_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
}
"#;

        // Current schema: email is NOT nullable (String)
        let cur_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub name: String,
    pub email: String,
}
"#;

        let prev_result = SchemaParser::parse(prev_code);
        let cur_result = SchemaParser::parse(cur_code);

        let prev_snapshot = parse_result_to_snapshot(&prev_result, Dialect::SQLite, None);
        let cur_snapshot = parse_result_to_snapshot(&cur_result, Dialect::SQLite, None);

        // Extract DDL from snapshots
        let (prev_ddl, cur_ddl) = match (&prev_snapshot, &cur_snapshot) {
            (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => (
                SQLiteDDL::from_entities(p.ddl.clone()),
                SQLiteDDL::from_entities(c.ddl.clone()),
            ),
            _ => panic!("Expected SQLite snapshots"),
        };

        // Check that previous email column is nullable and current is not
        let prev_email = prev_ddl
            .columns
            .one("user", "email")
            .expect("email column in prev");
        let cur_email = cur_ddl
            .columns
            .one("user", "email")
            .expect("email column in cur");
        assert!(!prev_email.not_null, "Previous email should be nullable");
        assert!(cur_email.not_null, "Current email should be NOT NULL");

        // Compute migration
        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have SQL statements for table recreation
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate migration SQL for nullable change"
        );

        let combined = migration.sql_statements.join("\n");
        assert!(
            combined.contains("PRAGMA foreign_keys=OFF"),
            "Should contain PRAGMA foreign_keys=OFF for table recreation"
        );
        assert!(
            combined.contains("__new_user"),
            "Should create temporary table __new_user"
        );
        assert!(
            combined.contains("NOT NULL"),
            "New table should have NOT NULL on email column"
        );
        assert!(combined.contains("DROP TABLE"), "Should drop old table");
        assert!(
            combined.contains("RENAME TO"),
            "Should rename temp table to original"
        );
    }

    /// Test that changing a column from String to Option<String> generates table recreation
    #[test]
    fn test_not_null_to_nullable_generates_migration() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::sqlite::collection::SQLiteDDL;
        use drizzle_migrations::sqlite::diff::compute_migration;

        // Previous schema: email is NOT nullable (String)
        let prev_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub email: String,
}
"#;

        // Current schema: email is nullable (Option<String>)
        let cur_code = r#"
#[SQLiteTable]
pub struct User {
    #[column(primary)]
    pub id: i64,
    pub email: Option<String>,
}
"#;

        let prev_result = SchemaParser::parse(prev_code);
        let cur_result = SchemaParser::parse(cur_code);

        let prev_snapshot = parse_result_to_snapshot(&prev_result, Dialect::SQLite, None);
        let cur_snapshot = parse_result_to_snapshot(&cur_result, Dialect::SQLite, None);

        // Extract DDL from snapshots
        let (prev_ddl, cur_ddl) = match (&prev_snapshot, &cur_snapshot) {
            (Snapshot::Sqlite(p), Snapshot::Sqlite(c)) => (
                SQLiteDDL::from_entities(p.ddl.clone()),
                SQLiteDDL::from_entities(c.ddl.clone()),
            ),
            _ => panic!("Expected SQLite snapshots"),
        };

        // Compute migration
        let migration = compute_migration(&prev_ddl, &cur_ddl);

        // Should have SQL statements for table recreation
        assert!(
            !migration.sql_statements.is_empty(),
            "Should generate migration SQL for nullable change"
        );

        let combined = migration.sql_statements.join("\n");
        assert!(
            combined.contains("__new_user"),
            "Should create temporary table for recreation"
        );
    }

    #[test]
    fn test_postgres_schema_and_index_options_are_preserved() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::postgres::ddl::PostgresEntity;

        let code = r#"
#[PostgresTable(schema = "auth")]
pub struct Users {
    #[column(primary)]
    pub id: i32,
}

#[PostgresTable(schema = "app")]
pub struct Sessions {
    #[column(primary)]
    pub id: i32,
    #[column(references = Users::id)]
    pub user_id: i32,
}

#[PostgresIndex(concurrent, method = "gin", where = "user_id > 0")]
pub struct SessionsUserIdx(Sessions::user_id);
"#;

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::PostgreSQL, None);

        let snap = match snapshot {
            Snapshot::Postgres(s) => s,
            _ => panic!("Expected Postgres snapshot"),
        };

        let has_auth_schema = snap
            .ddl
            .iter()
            .any(|e| matches!(e, PostgresEntity::Schema(s) if s.name.as_ref() == "auth"));
        let has_app_schema = snap
            .ddl
            .iter()
            .any(|e| matches!(e, PostgresEntity::Schema(s) if s.name.as_ref() == "app"));
        assert!(has_auth_schema, "missing auth schema entity");
        assert!(has_app_schema, "missing app schema entity");

        let fk = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::ForeignKey(fk) = e {
                Some(fk)
            } else {
                None
            }
        });
        let fk = fk.expect("expected foreign key");
        assert_eq!(fk.schema.as_ref(), "app");
        assert_eq!(fk.schema_to.as_ref(), "auth");

        let idx = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let idx = idx.expect("expected index");
        assert!(idx.concurrently);
        assert_eq!(idx.method.as_deref(), Some("gin"));
        assert_eq!(idx.where_clause.as_deref(), Some("user_id > 0"));
        assert_eq!(idx.schema.as_ref(), "app");
    }

    #[test]
    fn test_sqlite_table_options_and_pk_name_are_preserved() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable(strict, without_rowid)]
pub struct Accounts {
    #[column(primary)]
    pub id: i64,
}
"#;

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::SQLite, None);
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            _ => panic!("Expected SQLite snapshot"),
        };

        let table = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected sqlite table");
        assert!(table.strict, "strict should be preserved");
        assert!(table.without_rowid, "without_rowid should be preserved");

        let pk = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::PrimaryKey(pk) = e {
                Some(pk)
            } else {
                None
            }
        });
        let pk = pk.expect("expected sqlite primary key");
        assert_eq!(pk.name.as_ref(), "accounts_pkey");
    }

    #[test]
    fn test_sqlite_casing_preserves_explicit_names() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::sqlite::SqliteEntity;

        let code = r#"
#[SQLiteTable(name = "users_tbl")]
pub struct UsersTable {
    #[column(name = "user_id", primary)]
    pub userId: i64,
    pub emailAddress: String,
}

#[SQLiteIndex(name = "users_tbl_email_idx")]
pub struct UsersEmailIdx(UsersTable::emailAddress);
"#;

        let result = SchemaParser::parse(code);
        let snapshot = parse_result_to_snapshot(&result, Dialect::SQLite, Some(Casing::SnakeCase));
        let snap = match snapshot {
            Snapshot::Sqlite(s) => s,
            _ => panic!("Expected SQLite snapshot"),
        };

        let table = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected sqlite table");
        assert_eq!(table.name.as_ref(), "users_tbl");

        let user_id = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Column(c) = e
                && c.name.as_ref() == "user_id"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(user_id.is_some(), "expected explicit column name user_id");

        let email_col = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Column(c) = e
                && c.name.as_ref() == "email_address"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(
            email_col.is_some(),
            "expected inferred snake_case column name"
        );

        let index = snap.ddl.iter().find_map(|e| {
            if let SqliteEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let index = index.expect("expected sqlite index");
        assert_eq!(index.name.as_ref(), "users_tbl_email_idx");
    }

    #[test]
    fn test_postgres_casing_preserves_explicit_names() {
        use drizzle_migrations::parser::SchemaParser;
        use drizzle_migrations::postgres::ddl::PostgresEntity;

        let code = r#"
#[PostgresTable(schema = "auth", name = "users_tbl")]
pub struct UsersTable {
    #[column(name = "user_id", primary)]
    pub userId: i32,
    pub createdAt: String,
}

#[PostgresIndex(name = "users_tbl_created_idx")]
pub struct UsersCreatedIdx(UsersTable::createdAt);
"#;

        let result = SchemaParser::parse(code);
        let snapshot =
            parse_result_to_snapshot(&result, Dialect::PostgreSQL, Some(Casing::SnakeCase));
        let snap = match snapshot {
            Snapshot::Postgres(s) => s,
            _ => panic!("Expected Postgres snapshot"),
        };

        let table = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Table(t) = e {
                Some(t)
            } else {
                None
            }
        });
        let table = table.expect("expected postgres table");
        assert_eq!(table.schema.as_ref(), "auth");
        assert_eq!(table.name.as_ref(), "users_tbl");

        let user_id = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Column(c) = e
                && c.name.as_ref() == "user_id"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(user_id.is_some(), "expected explicit column name user_id");

        let created_at = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Column(c) = e
                && c.name.as_ref() == "created_at"
            {
                Some(c)
            } else {
                None
            }
        });
        assert!(
            created_at.is_some(),
            "expected inferred snake_case column name created_at"
        );

        let index = snap.ddl.iter().find_map(|e| {
            if let PostgresEntity::Index(i) = e {
                Some(i)
            } else {
                None
            }
        });
        let index = index.expect("expected postgres index");
        assert_eq!(index.name.as_ref(), "users_tbl_created_idx");
        assert_eq!(index.schema.as_ref(), "auth");
    }
}
