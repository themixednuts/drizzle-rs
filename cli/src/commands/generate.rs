//! Generate command implementation
//!
//! Generates migration files from schema changes.

use std::path::Path;

use colored::Colorize;

use crate::config::DrizzleConfig;
use crate::error::CliError;
use crate::snapshot::parse_result_to_snapshot;

/// Run the generate command
pub fn run(
    config: &DrizzleConfig,
    db_name: Option<&str>,
    name: Option<String>,
    custom: bool,
) -> Result<(), CliError> {
    use drizzle_migrations::journal::Journal;
    use drizzle_migrations::parser::SchemaParser;
    use drizzle_migrations::words::generate_migration_tag;

    let db = config.database(db_name)?;

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{} {}", "Database:".bright_blue(), name);
    }

    println!("{}", "Generating migration...".bright_cyan());

    // Create output directories if they don't exist
    let out_dir = db.migrations_dir();
    let meta_dir = db.meta_dir();
    std::fs::create_dir_all(out_dir).map_err(|e| CliError::IoError(e.to_string()))?;
    std::fs::create_dir_all(&meta_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    // Handle custom migration (empty migration file for manual SQL)
    if custom {
        return generate_custom_migration(db, name);
    }

    // Parse schema files
    let schema_files = db.schema_files()?;
    if schema_files.is_empty() {
        return Err(CliError::NoSchemaFiles(db.schema_display()));
    }

    println!(
        "  {} {} schema file(s)",
        "Parsing".bright_blue(),
        schema_files.len()
    );

    let mut combined_code = String::new();
    for path in &schema_files {
        let code = std::fs::read_to_string(path)
            .map_err(|e| CliError::IoError(format!("Failed to read {}: {}", path.display(), e)))?;
        combined_code.push_str(&code);
        combined_code.push('\n');
    }

    let parse_result = SchemaParser::parse(&combined_code);

    if parse_result.tables.is_empty() && parse_result.indexes.is_empty() {
        println!("{}", "No tables or indexes found in schema files.".yellow());
        return Ok(());
    }

    println!(
        "  {} {} table(s), {} index(es)",
        "Found".bright_blue(),
        parse_result.tables.len(),
        parse_result.indexes.len()
    );

    // Build current snapshot from parsed schema
    let current_snapshot = parse_result_to_snapshot(&parse_result);

    // Load previous snapshot if exists
    let journal_path = db.journal_path();
    let dialect = db.dialect.to_base();
    let prev_snapshot = load_previous_snapshot(out_dir, &journal_path, dialect)?;

    // Generate diff
    let sql_statements = generate_diff(&prev_snapshot, &current_snapshot, db.breakpoints)?;

    if sql_statements.is_empty() {
        println!("{}", "No schema changes detected 😴".yellow());
        return Ok(());
    }

    println!(
        "  {} {} SQL statement(s)",
        "Generated".bright_blue(),
        sql_statements.len()
    );

    // Write migration files
    let next_idx = load_next_migration_index(&journal_path);
    let migration_tag = match name {
        Some(custom_name) => format!("{:04}_{}", next_idx, custom_name),
        None => generate_migration_tag(next_idx),
    };

    // Create migration subdirectory: {out}/{tag}/
    let migration_dir = out_dir.join(&migration_tag);
    std::fs::create_dir_all(&migration_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/migration.sql
    let migration_sql_path = migration_dir.join("migration.sql");
    let sql_content = sql_statements.join("\n\n");
    std::fs::write(&migration_sql_path, &sql_content)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/snapshot.json
    let snapshot_path = migration_dir.join("snapshot.json");
    current_snapshot
        .save(&snapshot_path)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Update journal
    let mut journal = Journal::load_or_create(&journal_path, dialect)
        .map_err(|e| CliError::IoError(e.to_string()))?;
    journal.add_entry(migration_tag.clone(), db.breakpoints);
    journal
        .save(&journal_path)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    println!(
        "{}",
        format!("Migration generated: {}", migration_tag).bright_green()
    );
    println!("   {}", migration_dir.display());

    Ok(())
}

/// Generate an empty custom migration for manual SQL
fn generate_custom_migration(
    db: &crate::config::DatabaseConfig,
    name: Option<String>,
) -> Result<(), CliError> {
    use drizzle_migrations::journal::Journal;

    let out_dir = db.migrations_dir();
    let journal_path = db.journal_path();
    let dialect = db.dialect.to_base();

    let next_idx = load_next_migration_index(&journal_path);
    let migration_name = name.unwrap_or_else(|| "custom".to_string());
    let migration_tag = format!("{:04}_{}", next_idx, migration_name);

    // Create migration subdirectory: {out}/{tag}/
    let migration_dir = out_dir.join(&migration_tag);
    std::fs::create_dir_all(&migration_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/migration.sql with comment
    let migration_sql_path = migration_dir.join("migration.sql");
    let sql_content = "-- Custom SQL migration file, put your code below! --\n\n";
    std::fs::write(&migration_sql_path, sql_content)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Update journal
    let mut journal = Journal::load_or_create(&journal_path, dialect)
        .map_err(|e| CliError::IoError(e.to_string()))?;
    journal.add_entry(migration_tag.clone(), db.breakpoints);
    journal
        .save(&journal_path)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    println!(
        "{}",
        format!("✅ Custom migration created: {}", migration_tag).bright_green()
    );
    println!("   {}", migration_dir.display());
    println!(
        "{}",
        "   Edit the migration file to add your SQL statements.".bright_blue()
    );

    Ok(())
}

/// Load the previous snapshot from the migration directory
fn load_previous_snapshot(
    out_dir: &Path,
    journal_path: &Path,
    dialect: drizzle_types::Dialect,
) -> Result<drizzle_migrations::schema::Snapshot, CliError> {
    use drizzle_migrations::journal::Journal;
    use drizzle_migrations::schema::Snapshot;

    // Try to load journal and get latest snapshot
    if let Ok(journal) = Journal::load(journal_path)
        && let Some(latest) = journal.entries.last()
    {
        // Snapshot is in {out}/{tag}/snapshot.json
        let snapshot_path = out_dir.join(&latest.tag).join("snapshot.json");
        if snapshot_path.exists() {
            return Snapshot::load(&snapshot_path, dialect)
                .map_err(|e| CliError::IoError(e.to_string()));
        }
    }

    // No previous snapshot, return empty
    Ok(Snapshot::empty(dialect))
}

/// Generate diff between two snapshots
fn generate_diff(
    prev: &drizzle_migrations::schema::Snapshot,
    current: &drizzle_migrations::schema::Snapshot,
    breakpoints: bool,
) -> Result<Vec<String>, CliError> {
    use drizzle_migrations::schema::Snapshot;

    match (prev, current) {
        (Snapshot::Sqlite(prev_snap), Snapshot::Sqlite(curr_snap)) => {
            use drizzle_migrations::sqlite::diff_snapshots;
            use drizzle_migrations::sqlite::statements::SqliteGenerator;

            let diff = diff_snapshots(prev_snap, curr_snap);
            let generator = SqliteGenerator::new().with_breakpoints(breakpoints);
            Ok(generator.generate_migration(&diff))
        }
        (Snapshot::Postgres(prev_snap), Snapshot::Postgres(curr_snap)) => {
            use drizzle_migrations::postgres::diff_full_snapshots;
            use drizzle_migrations::postgres::statements::PostgresGenerator;

            let diff = diff_full_snapshots(prev_snap, curr_snap);
            let generator = PostgresGenerator::new().with_breakpoints(breakpoints);
            Ok(generator.generate(&diff.diffs))
        }
        _ => Err(CliError::DialectMismatch),
    }
}

/// Load the next migration index from journal
fn load_next_migration_index(journal_path: &Path) -> u32 {
    use drizzle_migrations::journal::Journal;

    Journal::load(journal_path)
        .map(|j| j.entries.len() as u32)
        .unwrap_or(0)
}
