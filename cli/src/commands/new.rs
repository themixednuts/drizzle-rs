//! Interactive schema builder (`drizzle new`)
//!
//! Walks the user through an interactive wizard to define tables, columns,
//! indexes, and foreign keys, then generates Rust schema code using the
//! existing codegen pipeline (the same one `drizzle introspect` uses).

use std::borrow::Cow;
use std::path::PathBuf;

use inquire::validator::Validation;
use inquire::{Confirm, MultiSelect, Select, Text};

use crate::config::{Config, Dialect};
use crate::error::CliError;
use crate::output;

// ── Public API ──────────────────────────────────────────────────────────────

pub struct NewOptions {
    pub dialect: Option<Dialect>,
    pub schema: Option<String>,
}

pub fn run(config: Option<&Config>, options: NewOptions) -> Result<(), CliError> {
    // Phase 1: Setup
    let dialect = resolve_dialect(config, options.dialect)?;
    let casing = prompt_casing()?;
    let output_path = resolve_output_path(config, options.schema)?;
    let schema_name = prompt_schema_name()?;

    // Phase 2: Enums (PostgreSQL only)
    let mut enums: Vec<EnumDef> = Vec::new();
    if dialect == Dialect::Postgresql {
        enums = prompt_enums()?;
    }

    // Phase 3 & 4: Tables + Columns
    let mut tables: Vec<TableDef> = Vec::new();
    loop {
        let table = prompt_table(dialect, &enums)?;
        tables.push(table);
        if !confirm("Add another table?", false)? {
            break;
        }
    }

    // Phase 5: Indexes
    let mut indexes: Vec<IndexDef> = Vec::new();
    if confirm("Add indexes?", false)? {
        indexes = prompt_indexes(&tables)?;
    }

    // Phase 6: Foreign Keys
    let mut fks: Vec<ForeignKeyDef> = Vec::new();
    if tables.len() > 1 && confirm("Add foreign keys?", false)? {
        fks = prompt_foreign_keys(&tables, dialect)?;
    }

    // Phase 7: Generate
    let code = match dialect {
        Dialect::Sqlite | Dialect::Turso => {
            generate_sqlite(&tables, &indexes, &fks, &schema_name, casing)
        }
        Dialect::Postgresql => {
            generate_postgres(&tables, &indexes, &fks, &enums, &schema_name, casing)
        }
    };

    // Write output
    let path = PathBuf::from(&output_path);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CliError::IoError(format!("Failed to create directory: {e}")))?;
    }
    std::fs::write(&path, &code)
        .map_err(|e| CliError::IoError(format!("Failed to write schema file: {e}")))?;

    // Print summary
    println!();
    println!("{}", output::success("Schema generated successfully!"));
    println!();
    println!(
        "  Tables: {}",
        tables
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if !indexes.is_empty() {
        println!(
            "  Indexes: {}",
            indexes
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !fks.is_empty() {
        println!("  Foreign keys: {}", fks.len());
    }
    println!("  Output: {}", output_path);
    println!();
    println!("Next steps:");
    println!(
        "  Run {} to generate your first migration",
        output::heading("drizzle generate")
    );

    Ok(())
}

// ── Intermediate structs ────────────────────────────────────────────────────

struct EnumDef {
    name: String,
    variants: Vec<String>,
}

struct TableDef {
    name: String,
    columns: Vec<ColumnDef>,
    /// SQLite only
    strict: bool,
    /// SQLite only
    without_rowid: bool,
    /// PostgreSQL only
    pg_schema: String,
}

struct ColumnDef {
    name: String,
    /// The SQL type string the codegen expects
    sql_type: String,
    not_null: bool,
    primary_key: bool,
    autoincrement: bool,
    unique: bool,
    default: Option<String>,
    /// For PG identity columns
    identity: bool,
    /// For PG enum columns: the enum name
    enum_name: Option<String>,
}

struct IndexDef {
    name: String,
    table: String,
    columns: Vec<String>,
    unique: bool,
    /// PG schema
    pg_schema: String,
}

struct ForeignKeyDef {
    name: String,
    table: String,
    columns: Vec<String>,
    table_to: String,
    columns_to: Vec<String>,
    on_delete: String,
    on_update: String,
    /// PG schema
    pg_schema: String,
    pg_schema_to: String,
}

// ── Phase 1: Setup prompts ──────────────────────────────────────────────────

fn resolve_dialect(
    config: Option<&Config>,
    cli_dialect: Option<Dialect>,
) -> Result<Dialect, CliError> {
    if let Some(d) = cli_dialect {
        return Ok(d);
    }
    if let Some(c) = config {
        return Ok(c.dialect());
    }
    let options = vec!["SQLite", "PostgreSQL"];
    let answer = Select::new("Select database dialect:", options)
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;
    match answer {
        "SQLite" => Ok(Dialect::Sqlite),
        "PostgreSQL" => Ok(Dialect::Postgresql),
        _ => unreachable!(),
    }
}

fn prompt_casing() -> Result<FieldCasing, CliError> {
    let options = vec!["snake_case (default)", "camelCase"];
    let answer = Select::new("Select field casing:", options)
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;
    match answer {
        s if s.starts_with("snake") => Ok(FieldCasing::Snake),
        s if s.starts_with("camel") => Ok(FieldCasing::Camel),
        _ => Ok(FieldCasing::Snake),
    }
}

fn resolve_output_path(
    config: Option<&Config>,
    cli_schema: Option<String>,
) -> Result<String, CliError> {
    if let Some(s) = cli_schema {
        return Ok(s);
    }
    let default = config
        .map(|c| c.schema_display())
        .unwrap_or_else(|| "src/schema.rs".to_string());
    Text::new("Schema output path:")
        .with_default(&default)
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))
}

fn prompt_schema_name() -> Result<String, CliError> {
    Text::new("Schema struct name:")
        .with_default("AppSchema")
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))
}

// ── Phase 2: Enums (PostgreSQL only) ────────────────────────────────────────

fn prompt_enums() -> Result<Vec<EnumDef>, CliError> {
    let mut enums = Vec::new();
    if !confirm("Define any enums?", false)? {
        return Ok(enums);
    }
    loop {
        let name = Text::new("Enum name:")
            .with_validator(|s: &str| {
                if s.is_empty() {
                    Ok(Validation::Invalid("Name cannot be empty".into()))
                } else if !is_valid_identifier(s) {
                    Ok(Validation::Invalid(
                        "Must be a valid Rust identifier".into(),
                    ))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

        let mut variants = Vec::new();
        loop {
            let variant = Text::new("  Enum variant (empty to finish):")
                .prompt()
                .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;
            if variant.is_empty() {
                break;
            }
            variants.push(variant);
        }
        if variants.is_empty() {
            println!("  Skipping enum with no variants.");
        } else {
            enums.push(EnumDef { name, variants });
        }
        if !confirm("Add another enum?", false)? {
            break;
        }
    }
    Ok(enums)
}

// ── Phase 3 & 4: Tables + Columns ──────────────────────────────────────────

fn prompt_table(dialect: Dialect, enums: &[EnumDef]) -> Result<TableDef, CliError> {
    let name = Text::new("Table name:")
        .with_validator(|s: &str| {
            if s.is_empty() {
                Ok(Validation::Invalid("Name cannot be empty".into()))
            } else if !is_valid_identifier(s) {
                Ok(Validation::Invalid(
                    "Must be a valid Rust identifier (letters, digits, underscores)".into(),
                ))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

    let mut strict = false;
    let mut without_rowid = false;
    let mut pg_schema = "public".to_string();

    match dialect {
        Dialect::Sqlite | Dialect::Turso => {
            let table_opts = vec!["strict", "without_rowid"];
            let selected = MultiSelect::new("Table options (space to toggle):", table_opts)
                .prompt()
                .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;
            strict = selected.contains(&"strict");
            without_rowid = selected.contains(&"without_rowid");
        }
        Dialect::Postgresql => {
            pg_schema = Text::new("PostgreSQL schema:")
                .with_default("public")
                .prompt()
                .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;
        }
    }

    // Columns
    let mut columns = Vec::new();
    println!();
    println!("  Define columns for '{}':", name);
    loop {
        let col = prompt_column(dialect, enums)?;
        columns.push(col);
        if !confirm("  Add another column?", true)? {
            break;
        }
    }

    Ok(TableDef {
        name,
        columns,
        strict,
        without_rowid,
        pg_schema,
    })
}

fn prompt_column(dialect: Dialect, enums: &[EnumDef]) -> Result<ColumnDef, CliError> {
    let col_name = Text::new("  Column name:")
        .with_validator(|s: &str| {
            if s.is_empty() {
                Ok(Validation::Invalid("Name cannot be empty".into()))
            } else if !is_valid_identifier(s) {
                Ok(Validation::Invalid("Must be a valid identifier".into()))
            } else {
                Ok(Validation::Valid)
            }
        })
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

    let (sql_type, enum_name) = prompt_type(dialect, enums)?;

    let nullable = confirm("  Nullable (Option<T>)?", false)?;

    let constraint_opts = match dialect {
        Dialect::Sqlite | Dialect::Turso => {
            vec!["Primary Key", "Autoincrement", "Unique", "Default value"]
        }
        Dialect::Postgresql => {
            vec![
                "Primary Key",
                "Identity (auto-increment)",
                "Unique",
                "Default value",
            ]
        }
    };
    let selected = MultiSelect::new("  Column constraints (space to toggle):", constraint_opts)
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

    let primary_key = selected.iter().any(|s| s.starts_with("Primary"));
    let autoincrement = selected.iter().any(|s| s.starts_with("Autoincrement"));
    let identity = selected.iter().any(|s| s.starts_with("Identity"));
    let unique = selected.iter().any(|s| s.starts_with("Unique"));
    let has_default = selected.iter().any(|s| s.starts_with("Default"));

    let default = if has_default {
        let val = Text::new("  Default value:")
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;
        Some(val)
    } else {
        None
    };

    Ok(ColumnDef {
        name: col_name,
        sql_type,
        not_null: !nullable,
        primary_key,
        autoincrement,
        unique,
        default,
        identity,
        enum_name,
    })
}

fn prompt_type(dialect: Dialect, enums: &[EnumDef]) -> Result<(String, Option<String>), CliError> {
    let mut options: Vec<String> = match dialect {
        Dialect::Sqlite | Dialect::Turso => {
            vec![
                "i32".into(),
                "i64".into(),
                "f64".into(),
                "String".into(),
                "bool".into(),
                "Vec<u8>".into(),
            ]
        }
        Dialect::Postgresql => {
            vec![
                "i16".into(),
                "i32".into(),
                "i64".into(),
                "f32".into(),
                "f64".into(),
                "String".into(),
                "bool".into(),
                "Vec<u8>".into(),
                "uuid::Uuid".into(),
                "chrono::NaiveDate".into(),
                "chrono::NaiveDateTime".into(),
                "chrono::DateTime<chrono::Utc>".into(),
                "serde_json::Value".into(),
            ]
        }
    };

    // Append user-defined enums as type choices
    for e in enums {
        options.push(format!("enum:{}", e.name));
    }

    let refs: Vec<&str> = options.iter().map(|s| s.as_str()).collect();
    let chosen = Select::new("  Rust type:", refs)
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

    // Map user-friendly Rust type → SQL type string for codegen
    if let Some(enum_name) = chosen.strip_prefix("enum:") {
        // For enum columns, the sql_type is the enum name itself
        return Ok((enum_name.to_string(), Some(enum_name.to_string())));
    }

    let sql_type = match dialect {
        Dialect::Sqlite | Dialect::Turso => match chosen {
            "i32" | "i64" => "integer",
            "f64" => "real",
            "String" => "text",
            "bool" => "boolean",
            "Vec<u8>" => "blob",
            _ => "text",
        },
        Dialect::Postgresql => match chosen {
            "i16" => "int2",
            "i32" => "int4",
            "i64" => "int8",
            "f32" => "float4",
            "f64" => "float8",
            "String" => "text",
            "bool" => "bool",
            "Vec<u8>" => "bytea",
            "uuid::Uuid" => "uuid",
            "chrono::NaiveDate" => "date",
            "chrono::NaiveDateTime" => "timestamp",
            "chrono::DateTime<chrono::Utc>" => "timestamptz",
            "serde_json::Value" => "jsonb",
            _ => "text",
        },
    };

    Ok((sql_type.to_string(), None))
}

// ── Phase 5: Indexes ────────────────────────────────────────────────────────

fn prompt_indexes(tables: &[TableDef]) -> Result<Vec<IndexDef>, CliError> {
    let mut indexes = Vec::new();
    loop {
        let table_names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();
        let table_name = Select::new("Index on which table?", table_names)
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

        let table = tables.iter().find(|t| t.name == table_name).unwrap();
        let col_names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();

        if col_names.is_empty() {
            println!("  Table has no columns, skipping.");
            if !confirm("Add another index?", false)? {
                break;
            }
            continue;
        }

        let selected_cols = MultiSelect::new("Select columns for index:", col_names)
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

        if selected_cols.is_empty() {
            println!("  No columns selected, skipping.");
            if !confirm("Add another index?", false)? {
                break;
            }
            continue;
        }

        let is_unique = confirm("  Unique index?", false)?;

        let suggested_name = format!("{}_{}_idx", table_name, selected_cols.join("_"));
        let idx_name = Text::new("  Index name:")
            .with_default(&suggested_name)
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

        indexes.push(IndexDef {
            name: idx_name,
            table: table_name.to_string(),
            columns: selected_cols.into_iter().map(|s| s.to_string()).collect(),
            unique: is_unique,
            pg_schema: table.pg_schema.clone(),
        });

        if !confirm("Add another index?", false)? {
            break;
        }
    }
    Ok(indexes)
}

// ── Phase 6: Foreign Keys ───────────────────────────────────────────────────

fn prompt_foreign_keys(
    tables: &[TableDef],
    dialect: Dialect,
) -> Result<Vec<ForeignKeyDef>, CliError> {
    let mut fks = Vec::new();
    let action_options = vec![
        "No Action",
        "Cascade",
        "Set Null",
        "Set Default",
        "Restrict",
    ];
    loop {
        let table_names: Vec<&str> = tables.iter().map(|t| t.name.as_str()).collect();

        let src_table_name = Select::new("Source table:", table_names.clone())
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;
        let src_table = tables.iter().find(|t| t.name == src_table_name).unwrap();
        let src_col_names: Vec<&str> = src_table.columns.iter().map(|c| c.name.as_str()).collect();

        let src_cols = MultiSelect::new("Source column(s):", src_col_names)
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

        let tgt_table_name = Select::new("Target (referenced) table:", table_names)
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;
        let tgt_table = tables.iter().find(|t| t.name == tgt_table_name).unwrap();
        let tgt_col_names: Vec<&str> = tgt_table.columns.iter().map(|c| c.name.as_str()).collect();

        let tgt_cols = MultiSelect::new("Target column(s):", tgt_col_names)
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

        let on_delete = Select::new("ON DELETE action:", action_options.clone())
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

        let on_update = Select::new("ON UPDATE action:", action_options.clone())
            .prompt()
            .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

        let fk_name = format!("{}_{}_fk", src_table_name, src_cols.join("_"));

        let pg_schema_to = match dialect {
            Dialect::Postgresql => tgt_table.pg_schema.clone(),
            _ => String::new(),
        };

        fks.push(ForeignKeyDef {
            name: fk_name,
            table: src_table_name.to_string(),
            columns: src_cols.into_iter().map(|s| s.to_string()).collect(),
            table_to: tgt_table_name.to_string(),
            columns_to: tgt_cols.into_iter().map(|s| s.to_string()).collect(),
            on_delete: on_delete.to_string(),
            on_update: on_update.to_string(),
            pg_schema: src_table.pg_schema.clone(),
            pg_schema_to,
        });

        if !confirm("Add another foreign key?", false)? {
            break;
        }
    }
    Ok(fks)
}

// ── Phase 7: Code generation ────────────────────────────────────────────────

fn generate_sqlite(
    tables: &[TableDef],
    indexes: &[IndexDef],
    fks: &[ForeignKeyDef],
    schema_name: &str,
    casing: FieldCasing,
) -> String {
    use drizzle_migrations::sqlite::codegen;
    use drizzle_migrations::sqlite::collection::SQLiteDDL;
    use drizzle_types::sqlite::ddl::{
        Column, ForeignKey, Index, IndexColumn, PrimaryKey, Table, UniqueConstraint,
    };

    let mut ddl = SQLiteDDL::new();

    for (table_idx, table) in tables.iter().enumerate() {
        let mut t = Table::new(table.name.clone());
        if table.strict {
            t = t.strict();
        }
        if table.without_rowid {
            t = t.without_rowid();
        }
        ddl.tables.push(t);

        let mut pk_cols: Vec<String> = Vec::new();
        let mut unique_cols: Vec<String> = Vec::new();

        for (col_idx, col) in table.columns.iter().enumerate() {
            let mut column =
                Column::new(table.name.clone(), col.name.clone(), col.sql_type.clone());
            if col.not_null {
                column = column.not_null();
            }
            if col.autoincrement {
                column = column.autoincrement();
            }
            if let Some(ref default) = col.default {
                column = column.default_value(default.clone());
            }
            // Set ordinal position to preserve order
            column.ordinal_position = Some((col_idx as i32) + 1);
            ddl.columns.push(column);

            if col.primary_key {
                pk_cols.push(col.name.clone());
            }
            if col.unique {
                unique_cols.push(col.name.clone());
            }
        }

        if !pk_cols.is_empty() {
            ddl.pks.push(PrimaryKey::from_strings(
                table.name.clone(),
                format!("{}_pk", table.name),
                pk_cols,
            ));
        }
        for uc in unique_cols {
            ddl.uniques.push(UniqueConstraint::from_strings(
                table.name.clone(),
                format!("{}_{}_unique", table.name, uc),
                vec![uc],
            ));
        }

        // Drop the table_idx binding explicitly
        let _ = table_idx;
    }

    // Add indexes
    for idx in indexes {
        let columns: Vec<IndexColumn> = idx
            .columns
            .iter()
            .map(|c| IndexColumn::new(c.clone()))
            .collect();
        let mut index = Index::new(idx.table.clone(), idx.name.clone(), columns);
        if idx.unique {
            index = index.unique();
        }
        ddl.indexes.push(index);
    }

    // Add foreign keys
    for fk in fks {
        let mut foreign_key = ForeignKey::from_strings(
            fk.table.clone(),
            fk.name.clone(),
            fk.columns.clone(),
            fk.table_to.clone(),
            fk.columns_to.clone(),
        );
        if fk.on_delete != "No Action" {
            foreign_key = foreign_key.on_delete(fk.on_delete.to_uppercase());
        }
        if fk.on_update != "No Action" {
            foreign_key = foreign_key.on_update(fk.on_update.to_uppercase());
        }
        ddl.fks.push(foreign_key);
    }

    let field_casing = match casing {
        FieldCasing::Snake => codegen::FieldCasing::Snake,
        FieldCasing::Camel => codegen::FieldCasing::Camel,
    };

    let options = codegen::CodegenOptions {
        module_doc: Some("Generated by `drizzle new`".to_string()),
        include_schema: true,
        schema_name: schema_name.to_string(),
        use_pub: true,
        field_casing,
    };

    codegen::generate_rust_schema(&ddl, &options).code
}

fn generate_postgres(
    tables: &[TableDef],
    indexes: &[IndexDef],
    fks: &[ForeignKeyDef],
    enums: &[EnumDef],
    schema_name: &str,
    casing: FieldCasing,
) -> String {
    use drizzle_migrations::postgres::codegen;
    use drizzle_migrations::postgres::collection::PostgresDDL;
    use drizzle_types::postgres::ddl::{
        Column, Enum, ForeignKey, Index, IndexColumn, PrimaryKey, Table, UniqueConstraint,
    };

    let mut ddl = PostgresDDL::new();

    // Add enums
    for e in enums {
        let values: Vec<Cow<'static, str>> =
            e.variants.iter().map(|v| Cow::Owned(v.clone())).collect();
        ddl.enums.push(Enum::new(
            "public",
            Cow::<str>::Owned(e.name.clone()),
            Cow::<[Cow<'static, str>]>::Owned(values),
        ));
    }

    // Add tables + columns
    for table in tables {
        ddl.tables
            .push(Table::new(table.pg_schema.clone(), table.name.clone()));

        let mut pk_cols: Vec<String> = Vec::new();
        let mut unique_cols: Vec<String> = Vec::new();

        for (col_idx, col) in table.columns.iter().enumerate() {
            let mut column = Column::new(
                table.pg_schema.clone(),
                table.name.clone(),
                col.name.clone(),
                col.sql_type.clone(),
            );
            if col.not_null {
                column = column.not_null();
            }
            if let Some(ref default) = col.default {
                column = column.default_value(default.clone());
            }
            if col.identity {
                use drizzle_types::postgres::ddl::Identity;
                let seq_name = format!("{}_{}_seq", table.name, col.name);
                column.identity = Some(Identity::always(seq_name));
            }
            if col.enum_name.is_some() {
                // Set type_schema so codegen can find it in the enum_map
                column.type_schema = Some(Cow::Owned(table.pg_schema.clone()));
            }
            column.ordinal_position = Some((col_idx as i32) + 1);
            ddl.columns.push(column);

            if col.primary_key {
                pk_cols.push(col.name.clone());
            }
            if col.unique {
                unique_cols.push(col.name.clone());
            }
        }

        if !pk_cols.is_empty() {
            ddl.pks.push(PrimaryKey::from_strings(
                table.pg_schema.clone(),
                table.name.clone(),
                format!("{}_pk", table.name),
                pk_cols,
            ));
        }
        for uc in unique_cols {
            ddl.uniques.push(UniqueConstraint::from_strings(
                table.pg_schema.clone(),
                table.name.clone(),
                format!("{}_{}_unique", table.name, uc),
                vec![uc],
            ));
        }
    }

    // Add indexes
    for idx in indexes {
        let columns: Vec<IndexColumn> = idx
            .columns
            .iter()
            .map(|c| IndexColumn::new(c.clone()))
            .collect();
        let mut index = Index::new(
            idx.pg_schema.clone(),
            idx.table.clone(),
            idx.name.clone(),
            columns,
        );
        if idx.unique {
            index = index.unique();
        }
        ddl.indexes.push(index);
    }

    // Add foreign keys
    for fk in fks {
        let mut foreign_key = ForeignKey::from_strings(
            fk.pg_schema.clone(),
            fk.table.clone(),
            fk.name.clone(),
            fk.columns.clone(),
            fk.pg_schema_to.clone(),
            fk.table_to.clone(),
            fk.columns_to.clone(),
        );
        if fk.on_delete != "No Action" {
            foreign_key = foreign_key.on_delete(fk.on_delete.to_uppercase());
        }
        if fk.on_update != "No Action" {
            foreign_key = foreign_key.on_update(fk.on_update.to_uppercase());
        }
        ddl.fks.push(foreign_key);
    }

    let field_casing = match casing {
        FieldCasing::Snake => codegen::FieldCasing::Snake,
        FieldCasing::Camel => codegen::FieldCasing::Camel,
    };

    let options = codegen::CodegenOptions {
        module_doc: Some("Generated by `drizzle new`".to_string()),
        include_schema: true,
        schema_name: schema_name.to_string(),
        use_pub: true,
        field_casing,
    };

    codegen::generate_rust_schema(&ddl, &options).code
}

// ── Utility helpers ─────────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy)]
enum FieldCasing {
    Snake,
    Camel,
}

fn confirm(message: &str, default: bool) -> Result<bool, CliError> {
    Confirm::new(message)
        .with_default(default)
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))
}

fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }
    let mut chars = s.chars();
    let first = chars.next().unwrap();
    if !first.is_ascii_alphabetic() && first != '_' {
        return false;
    }
    chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}
