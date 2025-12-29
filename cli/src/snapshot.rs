//! Schema snapshot builder from parsed schema files
//!
//! This module converts `ParseResult` from the schema parser into
//! `Snapshot` types that can be used for migration diffing.

use drizzle_migrations::parser::{ParseResult, ParsedField, ParsedIndex};
use drizzle_migrations::postgres::PostgresSnapshot;
use drizzle_migrations::schema::Snapshot;
use drizzle_migrations::sqlite::SQLiteSnapshot;
use drizzle_types::Dialect;
use heck::ToSnakeCase;
use std::borrow::Cow;

/// Convert a `ParseResult` into a `Snapshot` for migration diffing
///
/// Uses the provided `dialect` from config rather than the parser-detected dialect,
/// allowing users to have multi-dialect schema files and select which to use via config.
pub fn parse_result_to_snapshot(result: &ParseResult, dialect: Dialect) -> Snapshot {
    match dialect {
        Dialect::SQLite => Snapshot::Sqlite(build_sqlite_snapshot(result)),
        Dialect::PostgreSQL => Snapshot::Postgres(build_postgres_snapshot(result)),
        Dialect::MySQL => {
            // MySQL not yet fully supported
            panic!("MySQL snapshot generation not yet implemented")
        }
    }
}

/// Build an SQLite snapshot from parsed schema
fn build_sqlite_snapshot(result: &ParseResult) -> SQLiteSnapshot {
    use drizzle_migrations::sqlite::{PrimaryKey, SqliteEntity, Table, UniqueConstraint};

    let mut snapshot = SQLiteSnapshot::new();

    // Process tables (only those matching SQLite dialect)
    for table in result
        .tables
        .values()
        .filter(|t| t.dialect == Dialect::SQLite)
    {
        let table_name = table.name.to_snake_case();

        // Add table entity
        snapshot.add_entity(SqliteEntity::Table(Table::new(table_name.clone())));

        // Process columns
        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col = build_sqlite_column(&table_name, field);
            snapshot.add_entity(SqliteEntity::Column(col));

            // Track primary key columns
            if field.is_primary_key() {
                pk_columns.push(field.name.to_snake_case());
            }

            // Add unique constraint if column is unique (not primary)
            if field.is_unique() && !field.is_primary_key() {
                let col_name = field.name.to_snake_case();
                let constraint_name = format!("{}_{}_unique", table_name, col_name);
                snapshot.add_entity(SqliteEntity::UniqueConstraint(
                    UniqueConstraint::from_strings(
                        table_name.clone(),
                        constraint_name,
                        vec![col_name],
                    ),
                ));
            }

            // Add foreign key if references exist
            if let Some(ref_target) = field.references()
                && let Some(fk) = build_sqlite_foreign_key(&table_name, field, &ref_target)
            {
                snapshot.add_entity(SqliteEntity::ForeignKey(fk));
            }
        }

        // Add primary key entity
        if !pk_columns.is_empty() {
            let pk_name = format!("{}_pk", table_name);
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
        let idx = build_sqlite_index(index);
        snapshot.add_entity(SqliteEntity::Index(idx));
    }

    snapshot
}

/// Build a PostgreSQL snapshot from parsed schema
fn build_postgres_snapshot(result: &ParseResult) -> PostgresSnapshot {
    use drizzle_migrations::postgres::{
        PostgresEntity, PrimaryKey, Schema as PgSchema, Table, UniqueConstraint,
    };

    let mut snapshot = PostgresSnapshot::new();

    // Add public schema
    snapshot.add_entity(PostgresEntity::Schema(PgSchema::new("public")));

    // Process tables (only those matching PostgreSQL dialect)
    for table in result
        .tables
        .values()
        .filter(|t| t.dialect == Dialect::PostgreSQL)
    {
        let table_name = table.name.to_snake_case();

        // Add table entity
        snapshot.add_entity(PostgresEntity::Table(Table {
            schema: "public".into(),
            name: table_name.clone().into(),
            is_rls_enabled: None,
        }));

        // Process columns
        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col = build_postgres_column(&table_name, field);
            snapshot.add_entity(PostgresEntity::Column(col));

            // Track primary key columns
            if field.is_primary_key() {
                pk_columns.push(field.name.to_snake_case());
            }

            // Add unique constraint if column is unique (not primary)
            if field.is_unique() && !field.is_primary_key() {
                let col_name = field.name.to_snake_case();
                snapshot.add_entity(PostgresEntity::UniqueConstraint(
                    UniqueConstraint::from_strings(
                        "public".to_string(),
                        table_name.clone(),
                        format!("{}_{}_key", table_name, col_name),
                        vec![col_name],
                    ),
                ));
            }

            // Add foreign key if references exist
            if let Some(ref_target) = field.references()
                && let Some(fk) = build_postgres_foreign_key(&table_name, field, &ref_target)
            {
                snapshot.add_entity(PostgresEntity::ForeignKey(fk));
            }
        }

        // Add primary key entity
        if !pk_columns.is_empty() {
            snapshot.add_entity(PostgresEntity::PrimaryKey(PrimaryKey::from_strings(
                "public".to_string(),
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
        let idx = build_postgres_index(index);
        snapshot.add_entity(PostgresEntity::Index(idx));
    }

    snapshot
}

/// Build an SQLite column from a parsed field
fn build_sqlite_column(
    table_name: &str,
    field: &ParsedField,
) -> drizzle_migrations::sqlite::Column {
    use drizzle_migrations::sqlite::Column;

    let col_name = field.name.to_snake_case();
    let col_type = infer_sqlite_type(&field.ty);

    let mut col = Column::new(table_name.to_string(), col_name, col_type);

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
    table_name: &str,
    field: &ParsedField,
) -> drizzle_migrations::postgres::Column {
    use drizzle_migrations::postgres::ddl::IdentityType;
    use drizzle_migrations::postgres::{Column, Identity};

    let col_name = field.name.to_snake_case();
    let col_type = infer_postgres_type(&field.ty);
    let is_serial = field.has_attr("serial") || field.has_attr("bigserial");
    let is_identity = field.has_attr("generated") || field.has_attr("identity");

    Column {
        schema: "public".into(),
        table: table_name.to_string().into(),
        name: col_name.clone().into(),
        sql_type: col_type.into(),
        type_schema: None,
        not_null: !field.is_nullable(),
        default: field.default_value().map(Cow::Owned),
        generated: None,
        identity: if is_serial || is_identity {
            Some(Identity {
                name: format!("{}_{}_seq", table_name, col_name).into(),
                schema: Some("public".into()),
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
    }
}

/// Build an SQLite foreign key from a parsed field
fn build_sqlite_foreign_key(
    table_name: &str,
    field: &ParsedField,
    ref_target: &str,
) -> Option<drizzle_migrations::sqlite::ForeignKey> {
    use drizzle_migrations::sqlite::ForeignKey;

    // Parse "Table::column" reference
    let parts: Vec<&str> = ref_target.split("::").collect();
    if parts.len() != 2 {
        return None;
    }

    let ref_table = parts[0].to_snake_case();
    let ref_column = parts[1].to_snake_case();
    let col_name = field.name.to_snake_case();
    let fk_name = format!(
        "{}_{}_{}_{}_fk",
        table_name, col_name, ref_table, ref_column
    );

    let mut fk = ForeignKey::from_strings(
        table_name.to_string(),
        fk_name,
        vec![col_name],
        ref_table,
        vec![ref_column],
    );

    fk.on_delete = field.on_delete().map(Cow::Owned);
    fk.on_update = field.on_update().map(Cow::Owned);

    Some(fk)
}

/// Build a PostgreSQL foreign key from a parsed field
fn build_postgres_foreign_key(
    table_name: &str,
    field: &ParsedField,
    ref_target: &str,
) -> Option<drizzle_migrations::postgres::ForeignKey> {
    use drizzle_migrations::postgres::ForeignKey;

    // Parse "Table::column" reference
    let parts: Vec<&str> = ref_target.split("::").collect();
    if parts.len() != 2 {
        return None;
    }

    let ref_table = parts[0].to_snake_case();
    let ref_column = parts[1].to_snake_case();
    let col_name = field.name.to_snake_case();
    let fk_name = format!(
        "{}_{}_{}_{}_fk",
        table_name, col_name, ref_table, ref_column
    );

    Some(ForeignKey {
        schema: "public".into(),
        table: table_name.to_string().into(),
        name: fk_name.into(),
        name_explicit: false,
        columns: Cow::Owned(vec![Cow::Owned(col_name)]),
        schema_to: "public".into(),
        table_to: ref_table.into(),
        columns_to: Cow::Owned(vec![Cow::Owned(ref_column)]),
        on_update: field.on_update().map(Cow::Owned),
        on_delete: field.on_delete().map(Cow::Owned),
    })
}

/// Build an SQLite index from a parsed index
fn build_sqlite_index(index: &ParsedIndex) -> drizzle_migrations::sqlite::Index {
    use drizzle_migrations::sqlite::{Index, IndexColumn, IndexOrigin};

    let table_name = index
        .table_name()
        .map(str::to_snake_case)
        .unwrap_or_default();
    let index_name = index.name.to_snake_case();

    let columns: Vec<IndexColumn> = index
        .columns
        .iter()
        .filter_map(|c| {
            // Parse "Table::column" and extract just the column
            c.split("::")
                .last()
                .map(|s| IndexColumn::new(s.to_snake_case()))
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
fn build_postgres_index(index: &ParsedIndex) -> drizzle_migrations::postgres::Index {
    use drizzle_migrations::postgres::{Index, IndexColumn};

    let table_name = index
        .table_name()
        .map(str::to_snake_case)
        .unwrap_or_default();
    let index_name = index.name.to_snake_case();

    let columns: Vec<IndexColumn> = index
        .columns
        .iter()
        .filter_map(|c| {
            c.split("::")
                .last()
                .map(|s| IndexColumn::new(s.to_snake_case()))
        })
        .collect();

    Index {
        schema: "public".into(),
        table: table_name.into(),
        name: index_name.into(),
        name_explicit: false,
        columns,
        is_unique: index.is_unique(),
        where_clause: None,
        method: None,
        with: None,
        concurrently: false,
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

        let prev_snapshot = parse_result_to_snapshot(&prev_result, Dialect::SQLite);
        let cur_snapshot = parse_result_to_snapshot(&cur_result, Dialect::SQLite);

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

        let prev_snapshot = parse_result_to_snapshot(&prev_result, Dialect::SQLite);
        let cur_snapshot = parse_result_to_snapshot(&cur_result, Dialect::SQLite);

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
}
