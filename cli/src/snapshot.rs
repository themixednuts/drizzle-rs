//! Schema snapshot builder from parsed schema files
//!
//! This module converts `ParseResult` from the schema parser into
//! `Snapshot` types that can be used for migration diffing.

use drizzle_migrations::parser::{ParseResult, ParsedField, ParsedIndex};
use drizzle_migrations::postgres::PostgresSnapshot;
use drizzle_migrations::schema::Snapshot;
use drizzle_migrations::sqlite::SQLiteSnapshot;
use drizzle_types::Dialect;

/// Convert a `ParseResult` into a `Snapshot` for migration diffing
pub fn parse_result_to_snapshot(result: &ParseResult) -> Snapshot {
    match result.dialect {
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

    // Process tables
    for table in result.tables.values() {
        let table_name = to_snake_case(&table.name);

        // Add table entity
        snapshot.add_entity(SqliteEntity::Table(Table::new(&table_name)));

        // Process columns
        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col = build_sqlite_column(&table_name, field);
            snapshot.add_entity(SqliteEntity::Column(col));

            // Track primary key columns
            if field.is_primary_key() {
                pk_columns.push(to_snake_case(&field.name));
            }

            // Add unique constraint if column is unique (not primary)
            if field.is_unique() && !field.is_primary_key() {
                let col_name = to_snake_case(&field.name);
                let constraint_name = format!("{}_{}_unique", table_name, col_name);
                snapshot.add_entity(SqliteEntity::UniqueConstraint(UniqueConstraint::new(
                    &table_name,
                    &constraint_name,
                    vec![col_name],
                )));
            }

            // Add foreign key if references exist
            if let Some(ref_target) = field.references() {
                if let Some(fk) = build_sqlite_foreign_key(&table_name, field, &ref_target) {
                    snapshot.add_entity(SqliteEntity::ForeignKey(fk));
                }
            }
        }

        // Add primary key entity
        if !pk_columns.is_empty() {
            let pk_name = format!("{}_pk", table_name);
            snapshot.add_entity(SqliteEntity::PrimaryKey(PrimaryKey::new(
                &table_name,
                &pk_name,
                pk_columns,
            )));
        }
    }

    // Process indexes
    for index in result.indexes.values() {
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
    snapshot.add_entity(PostgresEntity::Schema(PgSchema {
        name: "public".to_string(),
    }));

    // Process tables
    for table in result.tables.values() {
        let table_name = to_snake_case(&table.name);

        // Add table entity
        snapshot.add_entity(PostgresEntity::Table(Table {
            schema: "public".to_string(),
            name: table_name.clone(),
            is_rls_enabled: None,
        }));

        // Process columns
        let mut pk_columns = Vec::new();

        for field in &table.fields {
            let col = build_postgres_column(&table_name, field);
            snapshot.add_entity(PostgresEntity::Column(col));

            // Track primary key columns
            if field.is_primary_key() {
                pk_columns.push(to_snake_case(&field.name));
            }

            // Add unique constraint if column is unique (not primary)
            if field.is_unique() && !field.is_primary_key() {
                let col_name = to_snake_case(&field.name);
                snapshot.add_entity(PostgresEntity::UniqueConstraint(UniqueConstraint {
                    schema: "public".to_string(),
                    table: table_name.clone(),
                    name: format!("{}_{}_key", table_name, col_name),
                    name_explicit: false,
                    columns: vec![col_name],
                    nulls_not_distinct: false,
                }));
            }

            // Add foreign key if references exist
            if let Some(ref_target) = field.references() {
                if let Some(fk) = build_postgres_foreign_key(&table_name, field, &ref_target) {
                    snapshot.add_entity(PostgresEntity::ForeignKey(fk));
                }
            }
        }

        // Add primary key entity
        if !pk_columns.is_empty() {
            snapshot.add_entity(PostgresEntity::PrimaryKey(PrimaryKey {
                schema: "public".to_string(),
                table: table_name.clone(),
                name: format!("{}_pkey", table_name),
                name_explicit: false,
                columns: pk_columns,
            }));
        }
    }

    // Process indexes
    for index in result.indexes.values() {
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

    let col_name = to_snake_case(&field.name);
    let col_type = infer_sqlite_type(&field.ty);

    let mut col = Column::new(table_name, &col_name, col_type);

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
    use drizzle_migrations::postgres::{Column, Identity};

    let col_name = to_snake_case(&field.name);
    let col_type = infer_postgres_type(&field.ty);
    let is_serial = field.has_attr("serial") || field.has_attr("bigserial");
    let is_identity = field.has_attr("generated") || field.has_attr("identity");

    Column {
        schema: "public".to_string(),
        table: table_name.to_string(),
        name: col_name.clone(),
        sql_type: col_type,
        type_schema: None,
        not_null: !field.is_nullable(),
        default: field.default_value(),
        generated: None,
        identity: if is_serial || is_identity {
            Some(Identity {
                name: format!("{}_{}_seq", table_name, col_name),
                schema: Some("public".to_string()),
                type_: if is_identity {
                    "always".to_string()
                } else {
                    "byDefault".to_string()
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

    let ref_table = to_snake_case(parts[0]);
    let ref_column = to_snake_case(parts[1]);
    let col_name = to_snake_case(&field.name);
    let fk_name = format!(
        "{}_{}_{}_{}_fk",
        table_name, col_name, ref_table, ref_column
    );

    let mut fk = ForeignKey::new(
        table_name,
        &fk_name,
        vec![col_name],
        ref_table,
        vec![ref_column],
    );

    fk.on_delete = field.on_delete();
    fk.on_update = field.on_update();

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

    let ref_table = to_snake_case(parts[0]);
    let ref_column = to_snake_case(parts[1]);
    let col_name = to_snake_case(&field.name);
    let fk_name = format!(
        "{}_{}_{}_{}_fk",
        table_name, col_name, ref_table, ref_column
    );

    Some(ForeignKey {
        schema: "public".to_string(),
        table: table_name.to_string(),
        name: fk_name,
        name_explicit: false,
        columns: vec![col_name],
        schema_to: "public".to_string(),
        table_to: ref_table,
        columns_to: vec![ref_column],
        on_update: field.on_update(),
        on_delete: field.on_delete(),
    })
}

/// Build an SQLite index from a parsed index
fn build_sqlite_index(index: &ParsedIndex) -> drizzle_migrations::sqlite::Index {
    use drizzle_migrations::sqlite::{Index, IndexColumn, IndexOrigin};

    let table_name = index
        .table_name()
        .map(|s| to_snake_case(s))
        .unwrap_or_default();
    let index_name = to_snake_case(&index.name);

    let columns: Vec<IndexColumn> = index
        .columns
        .iter()
        .filter_map(|c| {
            // Parse "Table::column" and extract just the column
            c.split("::").last().map(|s| IndexColumn {
                value: to_snake_case(s),
                is_expression: false,
            })
        })
        .collect();

    Index {
        table: table_name,
        name: index_name,
        columns,
        is_unique: index.is_unique(),
        r#where: None,
        origin: IndexOrigin::Manual,
    }
}

/// Build a PostgreSQL index from a parsed index
fn build_postgres_index(index: &ParsedIndex) -> drizzle_migrations::postgres::Index {
    use drizzle_migrations::postgres::{Index, IndexColumn};

    let table_name = index
        .table_name()
        .map(|s| to_snake_case(s))
        .unwrap_or_default();
    let index_name = to_snake_case(&index.name);

    let columns: Vec<IndexColumn> = index
        .columns
        .iter()
        .filter_map(|c| {
            c.split("::").last().map(|s| IndexColumn {
                value: to_snake_case(s),
                is_expression: false,
                asc: true,
                nulls_first: false,
                opclass: None,
            })
        })
        .collect();

    Index {
        schema: "public".to_string(),
        table: table_name,
        name: index_name,
        columns,
        is_unique: index.is_unique(),
        r#where: None,
        method: None,
        concurrently: false,
        r#with: None,
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

/// Convert PascalCase or camelCase to snake_case
fn to_snake_case(s: &str) -> String {
    let mut result = String::with_capacity(s.len() + 4);
    let mut prev_was_upper = false;
    let mut prev_was_underscore = true;

    for c in s.chars() {
        if c.is_uppercase() {
            if !prev_was_underscore && !prev_was_upper {
                result.push('_');
            }
            result.push(c.to_lowercase().next().unwrap_or(c));
            prev_was_upper = true;
            prev_was_underscore = false;
        } else if c == '_' {
            result.push(c);
            prev_was_underscore = true;
            prev_was_upper = false;
        } else {
            result.push(c);
            prev_was_upper = false;
            prev_was_underscore = false;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_to_snake_case() {
        assert_eq!(to_snake_case("Users"), "users");
        assert_eq!(to_snake_case("UserPosts"), "user_posts");
        assert_eq!(to_snake_case("userId"), "user_id");
        assert_eq!(to_snake_case("user_id"), "user_id");
        assert_eq!(to_snake_case("HTTPRequest"), "h_t_t_p_request"); // Edge case
    }

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
}
