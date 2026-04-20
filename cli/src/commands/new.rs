//! Interactive schema builder (`drizzle new`)
//!
//! Walks the user through an interactive wizard to define tables, columns,
//! indexes, and foreign keys, then generates Rust schema code using the
//! existing codegen pipeline (the same one `drizzle introspect` uses).
//!
//! Supports JSON import/export for CI-friendly, reproducible schema generation:
//! - `drizzle new --json` reads a JSON schema definition from stdin
//! - `drizzle new --json --from file.json` reads from a file
//! - `drizzle new --export-json out.json` exports the schema as JSON
//! - `drizzle new --schema-help` prints the expected JSON shape

use std::borrow::Cow;
use std::collections::HashSet;
use std::path::PathBuf;

use inquire::validator::Validation;
use inquire::{Confirm, MultiSelect, Select, Text};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::config::{Config, Dialect};
use crate::error::CliError;
use crate::output;

// ── Public API ──────────────────────────────────────────────────────────────

pub struct NewOptions {
    pub dialect: Option<Dialect>,
    pub schema: Option<String>,
    pub json: bool,
    pub from: Option<PathBuf>,
    pub export_json: Option<PathBuf>,
    pub schema_help: bool,
}

/// Run the `new` command to scaffold a schema definition.
///
/// # Errors
///
/// Returns [`CliError`] if loading or validating the JSON schema definition
/// fails, if interactive prompts are cancelled, or if writing the generated
/// Rust schema files fails.
pub fn run(config: Option<&Config>, options: &NewOptions) -> Result<(), CliError> {
    // --schema-help: print annotated example and exit
    if options.schema_help {
        print_json_schema();
        return Ok(());
    }

    // Build the schema definition from either JSON input or interactive prompts
    let def = if options.json {
        load_json(options.from.as_deref())?
    } else {
        collect_interactively(config, options)?
    };

    // Validate the schema definition
    validate_schema(&def)?;

    // Export JSON if requested
    if let Some(ref export_path) = options.export_json {
        export_to_json(&def, export_path)?;
    }

    // Determine output path (JSON definition's output_path, or CLI override)
    let output_path = if let Some(ref s) = options.schema {
        s.clone()
    } else {
        def.output_path.clone()
    };

    // Generate code
    let code = match def.dialect {
        Dialect::Sqlite | Dialect::Turso => generate_sqlite(
            &def.tables,
            &def.indexes,
            &def.foreign_keys,
            &def.schema_name,
            def.casing,
        ),
        Dialect::Postgresql => generate_postgres(
            &def.tables,
            &def.indexes,
            &def.foreign_keys,
            &def.enums,
            &def.schema_name,
            def.casing,
        ),
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
        def.tables
            .iter()
            .map(|t| t.name.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    );
    if !def.indexes.is_empty() {
        println!(
            "  Indexes: {}",
            def.indexes
                .iter()
                .map(|i| i.name.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    if !def.foreign_keys.is_empty() {
        println!("  Foreign keys: {}", def.foreign_keys.len());
    }
    println!("  Output: {output_path}");
    if let Some(ref export_path) = options.export_json {
        println!("  JSON export: {}", export_path.display());
    }
    println!();
    println!("Next steps:");
    println!(
        "  Run {} to generate your first migration",
        output::heading("drizzle generate")
    );

    Ok(())
}

// ── Schema definition (top-level JSON document) ─────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct SchemaDefinition {
    pub dialect: Dialect,
    #[serde(default = "default_casing")]
    pub casing: FieldCasing,
    #[serde(default = "default_schema_name")]
    pub schema_name: String,
    #[serde(default = "default_output_path")]
    pub output_path: String,
    #[serde(default)]
    pub enums: Vec<EnumDef>,
    pub tables: Vec<TableDef>,
    #[serde(default)]
    pub indexes: Vec<IndexDef>,
    #[serde(default)]
    pub foreign_keys: Vec<ForeignKeyDef>,
}

const fn default_casing() -> FieldCasing {
    FieldCasing::Snake
}

fn default_schema_name() -> String {
    "AppSchema".to_string()
}

fn default_output_path() -> String {
    "src/schema.rs".to_string()
}

fn default_fk_action() -> String {
    "No Action".to_string()
}

// ── Intermediate structs ────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
    /// `SQLite` only
    #[serde(default)]
    pub strict: bool,
    /// `SQLite` only
    #[serde(default)]
    pub without_rowid: bool,
    /// `PostgreSQL` only
    #[serde(default = "default_pg_schema")]
    pub pg_schema: String,
}

fn default_pg_schema() -> String {
    "public".to_string()
}

/// Auto-generation strategy for a column value.
///
/// `autoincrement` is `SQLite`-specific (`INTEGER PRIMARY KEY AUTOINCREMENT`) and
/// `identity` is `PostgreSQL`-specific (`GENERATED ALWAYS AS IDENTITY`). They are
/// mutually exclusive dialect variants, so they live in a single optional enum
/// rather than two parallel booleans.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum AutoGenKind {
    /// `SQLite` `INTEGER PRIMARY KEY AUTOINCREMENT`.
    Autoincrement,
    /// `PostgreSQL` `GENERATED ALWAYS AS IDENTITY`.
    Identity,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ColumnDef {
    pub name: String,
    /// The SQL type string the codegen expects
    pub sql_type: String,
    #[serde(default)]
    pub not_null: bool,
    #[serde(default)]
    pub primary_key: bool,
    #[serde(default)]
    pub unique: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    /// Auto-generation strategy (`SQLite` autoincrement / `PostgreSQL` identity).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auto_gen: Option<AutoGenKind>,
    /// For PG enum columns: the enum name
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enum_name: Option<String>,
}

impl ColumnDef {
    #[must_use]
    pub const fn is_autoincrement(&self) -> bool {
        matches!(self.auto_gen, Some(AutoGenKind::Autoincrement))
    }

    #[must_use]
    pub const fn is_identity(&self) -> bool {
        matches!(self.auto_gen, Some(AutoGenKind::Identity))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct IndexDef {
    pub name: String,
    pub table: String,
    pub columns: Vec<String>,
    #[serde(default)]
    pub unique: bool,
    /// PG schema
    #[serde(default)]
    pub pg_schema: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct ForeignKeyDef {
    pub name: String,
    pub table: String,
    pub columns: Vec<String>,
    pub table_to: String,
    pub columns_to: Vec<String>,
    #[serde(default = "default_fk_action")]
    pub on_delete: String,
    #[serde(default = "default_fk_action")]
    pub on_update: String,
    /// PG schema
    #[serde(default)]
    pub pg_schema: String,
    #[serde(default)]
    pub pg_schema_to: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema)]
pub enum FieldCasing {
    #[serde(rename = "snake_case")]
    Snake,
    #[serde(rename = "camelCase")]
    Camel,
}

// ── JSON import/export ──────────────────────────────────────────────────────

fn load_json(from: Option<&std::path::Path>) -> Result<SchemaDefinition, CliError> {
    let content = if let Some(path) = from {
        std::fs::read_to_string(path)
            .map_err(|e| CliError::IoError(format!("Failed to read {}: {e}", path.display())))?
    } else {
        use std::io::Read;
        let mut buf = String::new();
        std::io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| CliError::IoError(format!("Failed to read stdin: {e}")))?;
        buf
    };
    serde_json::from_str(&content)
        .map_err(|e| CliError::Other(format!("Invalid JSON schema definition: {e}")))
}

fn export_to_json(def: &SchemaDefinition, path: &std::path::Path) -> Result<(), CliError> {
    let json = serde_json::to_string_pretty(def)
        .map_err(|e| CliError::Other(format!("Failed to serialize schema: {e}")))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| CliError::IoError(format!("Failed to create directory: {e}")))?;
    }
    std::fs::write(path, json)
        .map_err(|e| CliError::IoError(format!("Failed to write JSON: {e}")))?;
    Ok(())
}

fn print_json_schema() {
    let schema = schemars::schema_for!(SchemaDefinition);
    let json = serde_json::to_string_pretty(&schema).expect("schema serialization cannot fail");
    println!("{json}");
    println!();
    println!(
        "Valid on_delete/on_update actions: \"No Action\", \"Cascade\", \"Set Null\", \"Set Default\", \"Restrict\""
    );
    println!();
    println!("Tip: Run `drizzle new --export-json schema.json` to export an interactive");
    println!(
        "session as valid JSON, then edit and replay with `drizzle new --json --from schema.json`."
    );
}

// ── Validation ──────────────────────────────────────────────────────────────

const VALID_FK_ACTIONS: &[&str] = &[
    "No Action",
    "Cascade",
    "Set Null",
    "Set Default",
    "Restrict",
];

fn validate_schema(def: &SchemaDefinition) -> Result<(), CliError> {
    // Must have at least one table
    if def.tables.is_empty() {
        return Err(CliError::Other(
            "Schema must have at least one table".into(),
        ));
    }

    // Check table names are valid and unique, and per-table column/dialect rules
    let mut table_names = HashSet::new();
    for table in &def.tables {
        if !is_valid_identifier(&table.name) {
            return Err(CliError::Other(format!(
                "Invalid table name: '{}'",
                table.name
            )));
        }
        if !table_names.insert(&table.name) {
            return Err(CliError::Other(format!(
                "Duplicate table name: '{}'",
                table.name
            )));
        }
        validate_table(table, def.dialect)?;
    }

    let enum_names = validate_enums(def)?;

    // Validate enum references in columns
    for table in &def.tables {
        for col in &table.columns {
            if let Some(ref en) = col.enum_name
                && !enum_names.contains(en.as_str())
            {
                return Err(CliError::Other(format!(
                    "Column '{}.{}' references unknown enum '{}'",
                    table.name, col.name, en
                )));
            }
        }
    }

    validate_indexes(def)?;
    validate_foreign_keys(def)?;

    Ok(())
}

/// Validate a single table: column names, dialect-specific column rules, and
/// that the table itself has at least one column.
fn validate_table(table: &TableDef, dialect: Dialect) -> Result<(), CliError> {
    if table.columns.is_empty() {
        return Err(CliError::Other(format!(
            "Table '{}' must have at least one column",
            table.name
        )));
    }

    let mut col_names = HashSet::new();
    for col in &table.columns {
        if !is_valid_identifier(&col.name) {
            return Err(CliError::Other(format!(
                "Invalid column name '{}' in table '{}'",
                col.name, table.name
            )));
        }
        if !col_names.insert(&col.name) {
            return Err(CliError::Other(format!(
                "Duplicate column name '{}' in table '{}'",
                col.name, table.name
            )));
        }
    }

    match dialect {
        Dialect::Sqlite | Dialect::Turso => {
            for col in &table.columns {
                if col.is_identity() {
                    return Err(CliError::Other(format!(
                        "Column '{}.{}': 'identity' is only supported for PostgreSQL",
                        table.name, col.name
                    )));
                }
                if col.enum_name.is_some() {
                    return Err(CliError::Other(format!(
                        "Column '{}.{}': 'enum_name' is only supported for PostgreSQL",
                        table.name, col.name
                    )));
                }
            }
        }
        Dialect::Postgresql => {
            if table.strict {
                return Err(CliError::Other(format!(
                    "Table '{}': 'strict' is only supported for SQLite",
                    table.name
                )));
            }
            if table.without_rowid {
                return Err(CliError::Other(format!(
                    "Table '{}': 'without_rowid' is only supported for SQLite",
                    table.name
                )));
            }
            for col in &table.columns {
                if col.is_autoincrement() {
                    return Err(CliError::Other(format!(
                        "Column '{}.{}': 'autoincrement' is only supported for SQLite (use 'identity' for PostgreSQL)",
                        table.name, col.name
                    )));
                }
            }
        }
    }

    Ok(())
}

/// Validate enum definitions and return the set of declared enum names.
fn validate_enums(def: &SchemaDefinition) -> Result<HashSet<&str>, CliError> {
    if def.dialect != Dialect::Postgresql && !def.enums.is_empty() {
        return Err(CliError::Other(
            "Enums are only supported for PostgreSQL".into(),
        ));
    }
    let mut enum_names = HashSet::new();
    for e in &def.enums {
        if !is_valid_identifier(&e.name) {
            return Err(CliError::Other(format!("Invalid enum name: '{}'", e.name)));
        }
        if !enum_names.insert(e.name.as_str()) {
            return Err(CliError::Other(format!(
                "Duplicate enum name: '{}'",
                e.name
            )));
        }
        if e.variants.is_empty() {
            return Err(CliError::Other(format!(
                "Enum '{}' must have at least one variant",
                e.name
            )));
        }
    }
    Ok(enum_names)
}

/// Validate that each index references a real table and real columns.
fn validate_indexes(def: &SchemaDefinition) -> Result<(), CliError> {
    for idx in &def.indexes {
        let table = def.tables.iter().find(|t| t.name == idx.table);
        let Some(table) = table else {
            return Err(CliError::Other(format!(
                "Index '{}' references unknown table '{}'",
                idx.name, idx.table
            )));
        };
        for col_name in &idx.columns {
            if !table.columns.iter().any(|c| &c.name == col_name) {
                return Err(CliError::Other(format!(
                    "Index '{}' references unknown column '{}.{}'",
                    idx.name, idx.table, col_name
                )));
            }
        }
    }
    Ok(())
}

/// Validate that each foreign key references real tables/columns and uses a
/// recognized on-delete / on-update action.
fn validate_foreign_keys(def: &SchemaDefinition) -> Result<(), CliError> {
    for fk in &def.foreign_keys {
        // Source table
        let src = def.tables.iter().find(|t| t.name == fk.table);
        let Some(src) = src else {
            return Err(CliError::Other(format!(
                "Foreign key '{}' references unknown source table '{}'",
                fk.name, fk.table
            )));
        };
        for col_name in &fk.columns {
            if !src.columns.iter().any(|c| &c.name == col_name) {
                return Err(CliError::Other(format!(
                    "Foreign key '{}' references unknown source column '{}.{}'",
                    fk.name, fk.table, col_name
                )));
            }
        }

        // Target table
        let tgt = def.tables.iter().find(|t| t.name == fk.table_to);
        let Some(tgt) = tgt else {
            return Err(CliError::Other(format!(
                "Foreign key '{}' references unknown target table '{}'",
                fk.name, fk.table_to
            )));
        };
        for col_name in &fk.columns_to {
            if !tgt.columns.iter().any(|c| &c.name == col_name) {
                return Err(CliError::Other(format!(
                    "Foreign key '{}' references unknown target column '{}.{}'",
                    fk.name, fk.table_to, col_name
                )));
            }
        }

        // Validate FK actions
        if !VALID_FK_ACTIONS.contains(&fk.on_delete.as_str()) {
            return Err(CliError::Other(format!(
                "Foreign key '{}': invalid on_delete action '{}'. Valid: {}",
                fk.name,
                fk.on_delete,
                VALID_FK_ACTIONS.join(", ")
            )));
        }
        if !VALID_FK_ACTIONS.contains(&fk.on_update.as_str()) {
            return Err(CliError::Other(format!(
                "Foreign key '{}': invalid on_update action '{}'. Valid: {}",
                fk.name,
                fk.on_update,
                VALID_FK_ACTIONS.join(", ")
            )));
        }
    }
    Ok(())
}

// ── Interactive collection ──────────────────────────────────────────────────

fn collect_interactively(
    config: Option<&Config>,
    options: &NewOptions,
) -> Result<SchemaDefinition, CliError> {
    // Phase 1: Setup
    let dialect = resolve_dialect(config, options.dialect)?;
    let casing = prompt_casing()?;
    let output_path = resolve_output_path(config, options.schema.clone())?;
    let schema_name = prompt_schema_name()?;

    // Phase 2: Enums (PostgreSQL only)
    let enums: Vec<EnumDef> = if dialect == Dialect::Postgresql {
        prompt_enums()?
    } else {
        Vec::new()
    };

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
    let indexes: Vec<IndexDef> = if confirm("Add indexes?", false)? {
        prompt_indexes(&tables)?
    } else {
        Vec::new()
    };

    // Phase 6: Foreign Keys
    let foreign_keys: Vec<ForeignKeyDef> =
        if tables.len() > 1 && confirm("Add foreign keys?", false)? {
            prompt_foreign_keys(&tables, dialect)?
        } else {
            Vec::new()
        };

    Ok(SchemaDefinition {
        dialect,
        casing,
        schema_name,
        output_path,
        enums,
        tables,
        indexes,
        foreign_keys,
    })
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
    let default = config.map_or_else(
        || "src/schema.rs".to_string(),
        super::super::config::Config::schema_display,
    );
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
    println!("  Define columns for '{name}':");
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

    let auto_gen = if autoincrement {
        Some(AutoGenKind::Autoincrement)
    } else if identity {
        Some(AutoGenKind::Identity)
    } else {
        None
    };

    Ok(ColumnDef {
        name: col_name,
        sql_type,
        not_null: !nullable,
        primary_key,
        unique,
        default,
        auto_gen,
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

    let refs: Vec<&str> = options.iter().map(std::string::String::as_str).collect();
    let chosen = Select::new("  Rust type:", refs)
        .prompt()
        .map_err(|e| CliError::Other(format!("Prompt cancelled: {e}")))?;

    // Map user-friendly Rust type -> SQL type string for codegen
    if let Some(enum_name) = chosen.strip_prefix("enum:") {
        // For enum columns, the sql_type is the enum name itself
        return Ok((enum_name.to_string(), Some(enum_name.to_string())));
    }

    let sql_type = match dialect {
        Dialect::Sqlite | Dialect::Turso => match chosen {
            "i32" | "i64" => "integer",
            "f64" => "real",
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
            columns: selected_cols
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
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
            columns: src_cols
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
            table_to: tgt_table_name.to_string(),
            columns_to: tgt_cols
                .into_iter()
                .map(std::string::ToString::to_string)
                .collect(),
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
            if col.is_autoincrement() {
                column = column.autoincrement();
            }
            if let Some(ref default) = col.default {
                column = column.default_value(default.clone());
            }
            // Set ordinal position to preserve order
            column.ordinal_position = Some(
                i32::try_from(col_idx)
                    .ok()
                    .and_then(|i| i.checked_add(1))
                    .unwrap_or(i32::MAX),
            );
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
    use drizzle_types::postgres::ddl::{Enum, Table};

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
        add_postgres_table_columns(&mut ddl, table);
    }

    add_postgres_indexes(&mut ddl, indexes);
    add_postgres_foreign_keys(&mut ddl, fks);

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

/// Populate `ddl.columns`, `ddl.pks`, `ddl.uniques` for a single postgres table.
fn add_postgres_table_columns(
    ddl: &mut drizzle_migrations::postgres::collection::PostgresDDL,
    table: &TableDef,
) {
    use drizzle_types::postgres::ddl::{Column, PrimaryKey, UniqueConstraint};

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
        if col.is_identity() {
            use drizzle_types::postgres::ddl::Identity;
            let seq_name = format!("{}_{}_seq", table.name, col.name);
            column.identity = Some(Identity::always(seq_name));
        }
        if col.enum_name.is_some() {
            // Set type_schema so codegen can find it in the enum_map
            column.type_schema = Some(Cow::Owned(table.pg_schema.clone()));
        }
        column.ordinal_position = Some(
            i32::try_from(col_idx)
                .ok()
                .and_then(|i| i.checked_add(1))
                .unwrap_or(i32::MAX),
        );
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

/// Append index definitions to the postgres DDL collection.
fn add_postgres_indexes(
    ddl: &mut drizzle_migrations::postgres::collection::PostgresDDL,
    indexes: &[IndexDef],
) {
    use drizzle_types::postgres::ddl::{Index, IndexColumn};
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
}

/// Append foreign-key definitions to the postgres DDL collection.
fn add_postgres_foreign_keys(
    ddl: &mut drizzle_migrations::postgres::collection::PostgresDDL,
    fks: &[ForeignKeyDef],
) {
    use drizzle_types::postgres::ddl::ForeignKey;
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
}

// ── Utility helpers ─────────────────────────────────────────────────────────

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

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal_sqlite_def() -> SchemaDefinition {
        SchemaDefinition {
            dialect: Dialect::Sqlite,
            casing: FieldCasing::Snake,
            schema_name: "TestSchema".into(),
            output_path: "src/schema.rs".into(),
            enums: vec![],
            tables: vec![TableDef {
                name: "users".into(),
                columns: vec![ColumnDef {
                    name: "id".into(),
                    sql_type: "integer".into(),
                    not_null: true,
                    primary_key: true,
                    unique: false,
                    default: None,
                    auto_gen: None,
                    enum_name: None,
                }],
                strict: false,
                without_rowid: false,
                pg_schema: String::new(),
            }],
            indexes: vec![],
            foreign_keys: vec![],
        }
    }

    #[test]
    fn validate_minimal_schema() {
        let def = minimal_sqlite_def();
        assert!(validate_schema(&def).is_ok());
    }

    #[test]
    fn validate_rejects_empty_tables() {
        let mut def = minimal_sqlite_def();
        def.tables.clear();
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("at least one table"));
    }

    #[test]
    fn validate_rejects_duplicate_table_names() {
        let mut def = minimal_sqlite_def();
        def.tables.push(def.tables[0].clone());
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("Duplicate table name"));
    }

    #[test]
    fn validate_rejects_empty_columns() {
        let mut def = minimal_sqlite_def();
        def.tables[0].columns.clear();
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("at least one column"));
    }

    #[test]
    fn validate_rejects_duplicate_column_names() {
        let mut def = minimal_sqlite_def();
        let dup = def.tables[0].columns[0].clone();
        def.tables[0].columns.push(dup);
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("Duplicate column name"));
    }

    #[test]
    fn validate_rejects_identity_on_sqlite() {
        let mut def = minimal_sqlite_def();
        def.tables[0].columns[0].auto_gen = Some(AutoGenKind::Identity);
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("identity"));
        assert!(err.to_string().contains("PostgreSQL"));
    }

    #[test]
    fn validate_rejects_autoincrement_on_postgres() {
        let mut def = minimal_sqlite_def();
        def.dialect = Dialect::Postgresql;
        def.tables[0].columns[0].auto_gen = Some(AutoGenKind::Autoincrement);
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("autoincrement"));
        assert!(err.to_string().contains("SQLite"));
    }

    #[test]
    fn validate_rejects_strict_on_postgres() {
        let mut def = minimal_sqlite_def();
        def.dialect = Dialect::Postgresql;
        def.tables[0].strict = true;
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("strict"));
        assert!(err.to_string().contains("SQLite"));
    }

    #[test]
    fn validate_rejects_enums_on_sqlite() {
        let mut def = minimal_sqlite_def();
        def.enums.push(EnumDef {
            name: "status".into(),
            variants: vec!["active".into()],
        });
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("Enums"));
        assert!(err.to_string().contains("PostgreSQL"));
    }

    #[test]
    fn validate_rejects_unknown_enum_reference() {
        let mut def = minimal_sqlite_def();
        def.dialect = Dialect::Postgresql;
        def.tables[0].columns[0].enum_name = Some("nonexistent".into());
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("unknown enum"));
    }

    #[test]
    fn validate_rejects_bad_fk_table_ref() {
        let mut def = minimal_sqlite_def();
        def.foreign_keys.push(ForeignKeyDef {
            name: "test_fk".into(),
            table: "nonexistent".into(),
            columns: vec!["id".into()],
            table_to: "users".into(),
            columns_to: vec!["id".into()],
            on_delete: "No Action".into(),
            on_update: "No Action".into(),
            pg_schema: String::new(),
            pg_schema_to: String::new(),
        });
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("unknown source table"));
    }

    #[test]
    fn validate_rejects_bad_fk_action() {
        let mut def = minimal_sqlite_def();
        def.tables.push(TableDef {
            name: "posts".into(),
            columns: vec![ColumnDef {
                name: "user_id".into(),
                sql_type: "integer".into(),
                not_null: true,
                primary_key: false,
                unique: false,
                default: None,
                auto_gen: None,
                enum_name: None,
            }],
            strict: false,
            without_rowid: false,
            pg_schema: String::new(),
        });
        def.foreign_keys.push(ForeignKeyDef {
            name: "posts_user_id_fk".into(),
            table: "posts".into(),
            columns: vec!["user_id".into()],
            table_to: "users".into(),
            columns_to: vec!["id".into()],
            on_delete: "INVALID".into(),
            on_update: "No Action".into(),
            pg_schema: String::new(),
            pg_schema_to: String::new(),
        });
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("invalid on_delete"));
    }

    #[test]
    fn validate_rejects_bad_index_column_ref() {
        let mut def = minimal_sqlite_def();
        def.indexes.push(IndexDef {
            name: "test_idx".into(),
            table: "users".into(),
            columns: vec!["nonexistent".into()],
            unique: false,
            pg_schema: String::new(),
        });
        let err = validate_schema(&def).unwrap_err();
        assert!(err.to_string().contains("unknown column"));
    }

    #[test]
    fn json_round_trip() {
        let def = minimal_sqlite_def();
        let json = serde_json::to_string_pretty(&def).unwrap();
        let parsed: SchemaDefinition = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.dialect, def.dialect);
        assert_eq!(parsed.tables.len(), 1);
        assert_eq!(parsed.tables[0].name, "users");
        assert_eq!(parsed.tables[0].columns[0].name, "id");
    }

    #[test]
    fn json_defaults_applied() {
        let json = r#"{
            "dialect": "sqlite",
            "tables": [{
                "name": "items",
                "columns": [{"name": "id", "sql_type": "integer"}]
            }]
        }"#;
        let def: SchemaDefinition = serde_json::from_str(json).unwrap();
        assert_eq!(def.schema_name, "AppSchema");
        assert_eq!(def.output_path, "src/schema.rs");
        assert!(def.enums.is_empty());
        assert!(def.indexes.is_empty());
        assert!(def.foreign_keys.is_empty());
        assert!(!def.tables[0].columns[0].not_null);
        assert!(!def.tables[0].columns[0].primary_key);
    }

    #[test]
    fn json_fk_action_defaults() {
        let json = r#"{
            "name": "test_fk",
            "table": "a",
            "columns": ["x"],
            "table_to": "b",
            "columns_to": ["y"]
        }"#;
        let fk: ForeignKeyDef = serde_json::from_str(json).unwrap();
        assert_eq!(fk.on_delete, "No Action");
        assert_eq!(fk.on_update, "No Action");
    }
}
