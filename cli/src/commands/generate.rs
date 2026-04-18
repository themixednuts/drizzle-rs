//! Generate command implementation
//!
//! Generates migration files from schema changes.

use std::path::Path;

use crate::commands::overrides;
use crate::config::{Casing, Config, Dialect, Driver, MigrationPrefix};
use crate::error::CliError;
use crate::output;
use crate::snapshot::parse_result_to_snapshot;

#[derive(Debug, Clone)]
pub struct GenerateOptions {
    pub name: Option<String>,
    pub custom: bool,
    pub casing: Option<Casing>,
    pub dialect: Option<Dialect>,
    pub driver: Option<Driver>,
    pub schema: Option<Vec<String>>,
    pub out: Option<std::path::PathBuf>,
    pub breakpoints: Option<bool>,
}

/// Run the generate command
pub fn run(config: &Config, db_name: Option<&str>, opts: GenerateOptions) -> Result<(), CliError> {
    use drizzle_migrations::parser::SchemaParser;
    use drizzle_migrations::words::{PrefixMode, generate_migration_tag_with_mode};

    let db = config.database(db_name)?;

    // CLI flag overrides config
    let effective_casing = opts.casing.or(db.casing);
    let effective_dialect = overrides::resolve_dialect(db, opts.dialect);
    let effective_driver = overrides::resolve_driver(db, effective_dialect, opts.driver)?;
    let effective_breakpoints = opts.breakpoints.unwrap_or(db.breakpoints);
    let out_dir = opts
        .out
        .clone()
        .unwrap_or_else(|| db.migrations_dir().to_path_buf());

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{}: {}", output::label("Database"), name);
    }

    println!("{}", output::heading("Generating migration..."));

    println!(
        "  {}: {}",
        output::label("Dialect"),
        effective_dialect.as_str()
    );
    if let Some(driver) = effective_driver {
        println!("  {}: {}", output::label("Driver"), driver);
    }

    // Create output directories if they don't exist
    std::fs::create_dir_all(&out_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    let legacy_journal_path = out_dir.join("meta").join("_journal.json");
    if legacy_journal_path.exists() {
        return Err(CliError::Other(
            "Detected old drizzle-kit migration folders. Upgrade them before generating new migrations."
                .to_string(),
        ));
    }

    // Handle custom migration (empty migration file for manual SQL)
    if opts.custom {
        let bundle = db.bundle_enabled();
        return generate_custom_migration(
            &out_dir,
            effective_breakpoints,
            db.migrations.as_ref().and_then(|m| m.prefix),
            opts.name,
            bundle,
        );
    }

    // Parse schema files
    let schema_files = overrides::resolve_schema_files(db, opts.schema.as_deref())?;
    if schema_files.is_empty() {
        return Err(CliError::NoSchemaFiles(overrides::resolve_schema_display(
            db,
            opts.schema.as_deref(),
        )));
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
    let dialect = effective_dialect.to_base();

    // Build current snapshot from parsed schema (use config dialect, not parser-detected)
    let current_snapshot = parse_result_to_snapshot(&parse_result, dialect, effective_casing);

    // Load previous snapshot if exists
    let prev_snapshot = load_previous_snapshot(&out_dir, dialect)?;

    // Generate diff
    let generated = generate_diff(&prev_snapshot, &current_snapshot)?;

    if generated.is_empty() {
        println!("{}", output::warning("No schema changes detected 😴"));
        return Ok(());
    }

    println!(
        "  {} {} SQL statement(s)",
        output::label("Generated"),
        generated.statements.len()
    );

    let prefix_mode = db
        .migrations
        .as_ref()
        .and_then(|m| m.prefix)
        .map(map_prefix_mode)
        .unwrap_or(PrefixMode::Timestamp);

    let next_idx = next_migration_index(&out_dir)?;
    let migration_tag =
        generate_migration_tag_with_mode(prefix_mode, next_idx, opts.name.as_deref());

    // Create migration subdirectory: {out}/{tag}/
    let migration_dir = out_dir.join(&migration_tag);
    std::fs::create_dir_all(&migration_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/migration.sql
    let migration_sql_path = migration_dir.join("migration.sql");
    let sql_content = if effective_breakpoints {
        generated.statements.join("\n--> statement-breakpoint\n")
    } else {
        generated.statements.join("\n\n")
    };
    std::fs::write(&migration_sql_path, &sql_content)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/snapshot.json
    let snapshot_path = migration_dir.join("snapshot.json");
    generated
        .snapshot
        .save(&snapshot_path)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Regenerate {out_dir}/migrations.js bundle index when enabled.
    // Auto-enabled for driver = durable-sqlite (see `DatabaseConfig::bundle_enabled`).
    if db.bundle_enabled() {
        write_migrations_js(&out_dir)?;
    }

    println!(
        "{}",
        output::success(&format!("Migration generated: {}", migration_tag))
    );
    println!("   {}", migration_dir.display());

    Ok(())
}

/// Generate an empty custom migration for manual SQL
fn generate_custom_migration(
    out_dir: &Path,
    _breakpoints: bool,
    prefix: Option<MigrationPrefix>,
    name: Option<String>,
    bundle: bool,
) -> Result<(), CliError> {
    use drizzle_migrations::words::{PrefixMode, generate_migration_tag_with_mode};

    let custom_name = name.unwrap_or_else(|| "custom".to_string());

    let prefix_mode = prefix.map(map_prefix_mode).unwrap_or(PrefixMode::Timestamp);

    let migration_tag = generate_migration_tag_with_mode(
        prefix_mode,
        next_migration_index(out_dir)?,
        Some(&custom_name),
    );

    // Create migration subdirectory: {out}/{tag}/
    let migration_dir = out_dir.join(&migration_tag);
    std::fs::create_dir_all(&migration_dir).map_err(|e| CliError::IoError(e.to_string()))?;

    // Write {tag}/migration.sql with comment
    let migration_sql_path = migration_dir.join("migration.sql");
    let sql_content = "-- Custom SQL migration file, put your code below! --\n\n";
    std::fs::write(&migration_sql_path, sql_content)
        .map_err(|e| CliError::IoError(e.to_string()))?;

    // Regenerate {out_dir}/migrations.js bundle index when enabled.
    if bundle {
        write_migrations_js(out_dir)?;
    }

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

fn map_prefix_mode(p: MigrationPrefix) -> drizzle_migrations::PrefixMode {
    match p {
        MigrationPrefix::Index => drizzle_migrations::PrefixMode::Index,
        MigrationPrefix::Timestamp => drizzle_migrations::PrefixMode::Timestamp,
        MigrationPrefix::Supabase => drizzle_migrations::PrefixMode::Supabase,
        MigrationPrefix::Unix => drizzle_migrations::PrefixMode::Unix,
        MigrationPrefix::None => drizzle_migrations::PrefixMode::None,
    }
}

/// Load the previous snapshot from the migration directory
fn load_previous_snapshot(
    out_dir: &Path,
    dialect: drizzle_types::Dialect,
) -> Result<drizzle_migrations::schema::Snapshot, CliError> {
    use drizzle_migrations::schema::Snapshot;

    if let Some(snapshot_path) = latest_v3_snapshot_path(out_dir)? {
        return Snapshot::load(&snapshot_path, dialect)
            .map_err(|e| CliError::IoError(e.to_string()));
    }

    // No previous snapshot, return empty
    Ok(Snapshot::empty(dialect))
}

fn next_migration_index(out_dir: &Path) -> Result<u32, CliError> {
    let entries = collect_v3_migration_tags(out_dir)?;
    let mut max_index: Option<u32> = None;

    for tag in &entries {
        let Some(prefix) = tag.split('_').next() else {
            continue;
        };

        if prefix.len() > 10 || !prefix.chars().all(|c| c.is_ascii_digit()) {
            continue;
        }

        if let Ok(idx) = prefix.parse::<u32>() {
            max_index = Some(max_index.map_or(idx, |curr| curr.max(idx)));
        }
    }

    Ok(max_index
        .map(|idx| idx.saturating_add(1))
        .unwrap_or(entries.len() as u32))
}

fn collect_v3_migration_tags(out_dir: &Path) -> Result<Vec<String>, CliError> {
    if !out_dir.exists() {
        return Ok(Vec::new());
    }

    let mut tags = Vec::new();
    for entry in std::fs::read_dir(out_dir).map_err(|e| CliError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        if !entry
            .file_type()
            .map_err(|e| CliError::IoError(e.to_string()))?
            .is_dir()
        {
            continue;
        }

        let tag = entry.file_name().to_string_lossy().to_string();
        if tag == "meta" {
            continue;
        }

        if entry.path().join("migration.sql").exists() {
            tags.push(tag);
        }
    }

    tags.sort();
    Ok(tags)
}

fn latest_v3_snapshot_path(out_dir: &Path) -> Result<Option<std::path::PathBuf>, CliError> {
    if !out_dir.exists() {
        return Ok(None);
    }

    let mut tags = Vec::new();
    for entry in std::fs::read_dir(out_dir).map_err(|e| CliError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        if !entry
            .file_type()
            .map_err(|e| CliError::IoError(e.to_string()))?
            .is_dir()
        {
            continue;
        }

        let tag = entry.file_name().to_string_lossy().to_string();
        if tag == "meta" {
            continue;
        }

        let snapshot_path = entry.path().join("snapshot.json");
        if snapshot_path.exists() {
            tags.push((tag, snapshot_path));
        }
    }

    tags.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(tags.pop().map(|(_, path)| path))
}

/// Write a `migrations.js` bundle index at the root of the migrations output
/// folder.
///
/// Mirrors `drizzle-kit`'s `bundle: true` output. JS bundlers (Metro for
/// Expo/React Native, Cloudflare Workers' bundler for Durable Objects SQLite)
/// require static `import` statements to embed SQL text into the final JS
/// bundle; this file is the entry point.
///
/// Rust-only consumers can ignore it — our [`drizzle_migrations::MigrationDir`]
/// loader reads the `migration.sql` files directly.
pub fn write_migrations_js(out_dir: &Path) -> Result<(), CliError> {
    let tags = collect_v3_migration_tags(out_dir)?;

    let mut content = String::new();
    for (idx, tag) in tags.iter().enumerate() {
        let import_name = format!("m{:04}", idx);
        // Forward slashes work in JS import specifiers on every platform,
        // including Windows — they are URL-style paths, not filesystem paths.
        content.push_str(&format!(
            "import {} from './{}/migration.sql';\n",
            import_name, tag
        ));
    }

    content.push_str("\nexport default {\n  migrations: {\n");
    for (idx, tag) in tags.iter().enumerate() {
        content.push_str(&format!("    \"{}\": m{:04},\n", tag, idx));
    }
    content.push_str("  }\n};\n");

    let migrations_js_path = out_dir.join("migrations.js");
    std::fs::write(&migrations_js_path, content).map_err(|e| CliError::IoError(e.to_string()))?;

    Ok(())
}

/// Generate diff between two snapshots
fn generate_diff(
    prev: &drizzle_migrations::schema::Snapshot,
    current: &drizzle_migrations::schema::Snapshot,
) -> Result<drizzle_migrations::Plan, CliError> {
    drizzle_migrations::diff(prev, current).map_err(|error| match error {
        drizzle_migrations::MigrationError::DialectMismatch => CliError::DialectMismatch,
        drizzle_migrations::MigrationError::NoChanges => {
            CliError::Other("No schema changes detected".to_string())
        }
        drizzle_migrations::MigrationError::ConfigError(_)
        | drizzle_migrations::MigrationError::IoError(_)
        | drizzle_migrations::MigrationError::SnapshotError(_) => {
            CliError::MigrationError(error.to_string())
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn touch_migration(out_dir: &Path, tag: &str) {
        let dir = out_dir.join(tag);
        std::fs::create_dir_all(&dir).expect("mkdir migration folder");
        std::fs::write(dir.join("migration.sql"), "-- stub\n").expect("write migration.sql");
    }

    #[test]
    fn migrations_js_contains_import_and_export_map_in_tag_order() {
        let tmp = tempdir().expect("tempdir");
        let out_dir = tmp.path();

        touch_migration(out_dir, "20230331141203_first");
        touch_migration(out_dir, "20230401091530_second");
        touch_migration(out_dir, "20230501111111_third");

        write_migrations_js(out_dir).expect("write migrations.js");

        let contents =
            std::fs::read_to_string(out_dir.join("migrations.js")).expect("read migrations.js");

        assert!(
            contents.contains("import m0000 from './20230331141203_first/migration.sql';"),
            "first import present"
        );
        assert!(
            contents.contains("import m0001 from './20230401091530_second/migration.sql';"),
            "second import present"
        );
        assert!(
            contents.contains("import m0002 from './20230501111111_third/migration.sql';"),
            "third import present"
        );
        assert!(
            contents.contains("\"20230331141203_first\": m0000,"),
            "first map entry"
        );
        assert!(
            contents.contains("\"20230401091530_second\": m0001,"),
            "second map entry"
        );
        assert!(
            contents.contains("\"20230501111111_third\": m0002,"),
            "third map entry"
        );
        assert!(
            contents.contains("export default {"),
            "export default present"
        );
    }

    #[test]
    fn migrations_js_is_empty_shell_when_no_migrations_exist() {
        let tmp = tempdir().expect("tempdir");
        let out_dir = tmp.path();

        write_migrations_js(out_dir).expect("write migrations.js");

        let contents =
            std::fs::read_to_string(out_dir.join("migrations.js")).expect("read migrations.js");

        assert!(!contents.contains("import "), "no imports when empty");
        assert!(
            contents.contains("export default {"),
            "export default still present"
        );
        assert!(
            contents.contains("migrations: {"),
            "migrations map still present"
        );
    }

    #[test]
    fn migrations_js_uses_forward_slashes_in_import_paths() {
        // JS import specifiers use URL-style paths (always forward slashes),
        // regardless of host filesystem separator. This guards against a
        // Windows-specific regression that upstream shipped in beta.22.
        let tmp = tempdir().expect("tempdir");
        let out_dir = tmp.path();

        touch_migration(out_dir, "20230331141203_first");

        write_migrations_js(out_dir).expect("write migrations.js");

        let contents =
            std::fs::read_to_string(out_dir.join("migrations.js")).expect("read migrations.js");

        assert!(
            !contents.contains('\\'),
            "import paths must use forward slashes even on Windows"
        );
        assert!(contents.contains("'./20230331141203_first/migration.sql'"));
    }
}
