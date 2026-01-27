//! Generate command implementation
//!
//! Generates migration files from schema changes.

use std::path::Path;

use crate::config::{Casing, Config};
use crate::error::CliError;
use crate::output;
use crate::snapshot::parse_result_to_snapshot;

/// Run the generate command
pub fn run(
    config: &Config,
    db_name: Option<&str>,
    name: Option<String>,
    custom: bool,
    casing: Option<Casing>,
) -> Result<(), CliError> {
    use drizzle_migrations::journal::Journal;
    use drizzle_migrations::parser::SchemaParser;
    use drizzle_migrations::words::{PrefixMode, generate_migration_tag_with_mode};

    let db = config.database(db_name)?;

    // CLI flag overrides config, config default is camelCase
    let _effective_casing = casing.unwrap_or_else(|| db.effective_casing());

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{}: {}", output::label("Database"), name);
    }

    println!("{}", output::heading("Generating migration..."));

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
        output::label("Parsing"),
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
        println!(
            "{}",
            output::warning("No tables or indexes found in schema files.")
        );
        return Ok(());
    }

    println!(
        "  {} {} table(s), {} index(es)",
        output::label("Found"),
        parse_result.tables.len(),
        parse_result.indexes.len()
    );

    // Get dialect from config
    let dialect = db.dialect.to_base();

    // Build current snapshot from parsed schema (use config dialect, not parser-detected)
    let current_snapshot = parse_result_to_snapshot(&parse_result, dialect);

    // Load previous snapshot if exists
    let journal_path = db.journal_path();
    let prev_snapshot = load_previous_snapshot(out_dir, &journal_path, dialect)?;

    // Generate diff
    let sql_statements = generate_diff(&prev_snapshot, &current_snapshot, db.breakpoints)?;

    if sql_statements.is_empty() {
        println!("{}", output::warning("No schema changes detected 😴"));
        return Ok(());
    }

    println!(
        "  {} {} SQL statement(s)",
        output::label("Generated"),
        sql_statements.len()
    );

    // Load or create journal (needed for index-based prefixes)
    let mut journal = Journal::load_or_create(&journal_path, dialect)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    let prefix_mode = db
        .migrations
        .as_ref()
        .and_then(|m| m.prefix)
        .map(map_prefix_mode)
        .unwrap_or(PrefixMode::Timestamp);

    let migration_tag =
        generate_migration_tag_with_mode(prefix_mode, journal.next_idx(), name.as_deref());

    // Create migration subdirectory: {out}/{tag}/
    let migration_dir = out_dir.join(&migration_tag);
    std::fs::create_dir_all(&migration_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/migration.sql
    let migration_sql_path = migration_dir.join("migration.sql");
    let sql_content = if db.breakpoints {
        sql_statements.join("\n--> statement-breakpoint\n")
    } else {
        sql_statements.join("\n\n")
    };
    std::fs::write(&migration_sql_path, &sql_content)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/snapshot.json
    let snapshot_path = migration_dir.join("snapshot.json");
    current_snapshot
        .save(&snapshot_path)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Update journal
    journal.add_entry(migration_tag.clone(), db.breakpoints);
    journal
        .save(&journal_path)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    println!(
        "{}",
        output::success(&format!("Migration generated: {}", migration_tag))
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
    use drizzle_migrations::words::{PrefixMode, generate_migration_tag_with_mode};

    let out_dir = db.migrations_dir();
    let journal_path = db.journal_path();
    let dialect = db.dialect.to_base();

    let custom_name = name.unwrap_or_else(|| "custom".to_string());
    let mut journal = Journal::load_or_create(&journal_path, dialect)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    let prefix_mode = db
        .migrations
        .as_ref()
        .and_then(|m| m.prefix)
        .map(map_prefix_mode)
        .unwrap_or(PrefixMode::Timestamp);

    let migration_tag =
        generate_migration_tag_with_mode(prefix_mode, journal.next_idx(), Some(&custom_name));

    // Create migration subdirectory: {out}/{tag}/
    let migration_dir = out_dir.join(&migration_tag);
    std::fs::create_dir_all(&migration_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/migration.sql with comment
    let migration_sql_path = migration_dir.join("migration.sql");
    let sql_content = "-- Custom SQL migration file, put your code below! --\n\n";
    std::fs::write(&migration_sql_path, sql_content)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Update journal
    journal.add_entry(migration_tag.clone(), db.breakpoints);
    journal
        .save(&journal_path)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    println!(
        "{}",
        output::success(&format!("Custom migration created: {}", migration_tag))
    );
    println!("   {}", migration_dir.display());
    println!(
        "{}",
        output::label("   Edit the migration file to add your SQL statements.")
    );

    Ok(())
}

fn map_prefix_mode(p: crate::config::MigrationPrefix) -> drizzle_migrations::PrefixMode {
    match p {
        crate::config::MigrationPrefix::Index => drizzle_migrations::PrefixMode::Index,
        crate::config::MigrationPrefix::Timestamp => drizzle_migrations::PrefixMode::Timestamp,
        crate::config::MigrationPrefix::Supabase => drizzle_migrations::PrefixMode::Supabase,
        crate::config::MigrationPrefix::Unix => drizzle_migrations::PrefixMode::Unix,
        crate::config::MigrationPrefix::None => drizzle_migrations::PrefixMode::None,
    }
}

/// Load the previous snapshot from the migration directory
fn load_previous_snapshot(
    out_dir: &Path,
    journal_path: &Path,
    dialect: drizzle_types::Dialect,
) -> Result<drizzle_migrations::schema::Snapshot, CliError> {
    use drizzle_migrations::journal::Journal;
    use drizzle_migrations::schema::Snapshot;

    // If a journal exists, it must be readable. Silently ignoring parse errors can
    // lead to generating incorrect diffs and destructive migrations.
    if journal_path.exists() {
        let journal = Journal::load(journal_path).map_err(|e| CliError::IoError(e.to_string()))?;
        if let Some(latest) = journal.entries.last() {
            // Snapshot is in {out}/{tag}/snapshot.json
            let snapshot_path = out_dir.join(&latest.tag).join("snapshot.json");
            if snapshot_path.exists() {
                return Snapshot::load(&snapshot_path, dialect)
                    .map_err(|e| CliError::IoError(e.to_string()));
            }
        }
    }

    // No previous snapshot, return empty
    Ok(Snapshot::empty(dialect))
}

/// Generate diff between two snapshots
fn generate_diff(
    prev: &drizzle_migrations::schema::Snapshot,
    current: &drizzle_migrations::schema::Snapshot,
    _breakpoints: bool,
) -> Result<Vec<String>, CliError> {
    use drizzle_migrations::schema::Snapshot;

    match (prev, current) {
        (Snapshot::Sqlite(prev_snap), Snapshot::Sqlite(curr_snap)) => {
            use drizzle_migrations::sqlite::collection::SQLiteDDL;
            use drizzle_migrations::sqlite::diff::compute_migration;

            // Convert snapshots to DDL collections
            let prev_ddl = SQLiteDDL::from_entities(prev_snap.ddl.clone());
            let cur_ddl = SQLiteDDL::from_entities(curr_snap.ddl.clone());

            // Use compute_migration which properly handles column alterations
            // via table recreation (SQLite doesn't support ALTER COLUMN)
            let migration = compute_migration(&prev_ddl, &cur_ddl);
            Ok(migration.sql_statements)
        }
        (Snapshot::Postgres(prev_snap), Snapshot::Postgres(curr_snap)) => {
            use drizzle_migrations::postgres::diff_full_snapshots;
            use drizzle_migrations::postgres::statements::PostgresGenerator;

            let diff = diff_full_snapshots(prev_snap, curr_snap);
            let generator = PostgresGenerator::new().with_breakpoints(_breakpoints);
            Ok(generator.generate(&diff.diffs))
        }
        _ => Err(CliError::DialectMismatch),
    }
}
