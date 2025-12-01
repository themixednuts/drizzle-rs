//! Generate migration command
//!
//! This command generates a new migration by:
//! 1. Loading the current schema from a JSON file (exported by build.rs or the app)
//! 2. Loading the last snapshot from the migrations directory
//! 3. Diffing the two to find changes
//! 4. Generating SQL statements for the changes
//! 5. Writing the migration file and new snapshot

use crate::schema::Schema;
use crate::snapshot;
use colored::Colorize;
use drizzle_migrations::Journal;
use drizzle_migrations::sqlgen::sqlite::SqliteGenerator;
use drizzle_migrations::sqlite::SQLiteSnapshot;
use std::path::Path;

pub struct GenerateOptions {
    pub schema_path: Option<String>,
    pub out_dir: String,
    pub dialect: String,
    pub name: Option<String>,
    pub custom: bool,
    pub prefix: String,
    pub breakpoints: bool,
}

pub fn run(opts: GenerateOptions) -> anyhow::Result<()> {
    let migrations_dir = Path::new(&opts.out_dir).join("migrations");

    // Handle custom (empty) migration
    if opts.custom {
        return create_custom_migration(
            &migrations_dir,
            opts.name.as_deref(),
            &opts.dialect,
            opts.breakpoints,
        );
    }

    // Load the current schema
    let schema_path = opts.schema_path.ok_or_else(|| {
        anyhow::anyhow!(
            "Schema path required. Use --schema or set 'schema' in drizzle.toml.\n\
             You can export schema using: Schema::default().to_snapshot_json()"
        )
    })?;

    let current_schema = Schema::load(Path::new(&schema_path), &opts.dialect)?;

    // Get the current snapshot from schema
    let current_snapshot = match current_schema {
        Schema::Sqlite(s) => s,
    };

    // Load the previous snapshot (or create empty)
    let prev_snapshot = snapshot::load_latest_snapshot(&migrations_dir, &opts.dialect)?
        .unwrap_or_else(|| snapshot::empty_snapshot(&opts.dialect).unwrap());

    // Diff the snapshots
    let diff = drizzle_migrations::sqlite::diff_snapshots(&prev_snapshot, &current_snapshot);

    if !diff.has_changes() {
        println!("{}", "No schema changes detected".yellow());
        return Ok(());
    }

    // Generate SQL statements
    let generator = SqliteGenerator::new().with_breakpoints(opts.breakpoints);
    let sql_statements = generator.generate_migration(&diff);

    if sql_statements.is_empty() {
        println!("{}", "No schema changes detected".yellow());
        return Ok(());
    }

    // Create migration file
    let (tag, _idx) = write_migration(
        &migrations_dir,
        &sql_statements,
        &current_snapshot,
        &opts.dialect,
        opts.name.as_deref(),
        &opts.prefix,
        opts.breakpoints,
    )?;

    println!("{} Created migration: {}", "✓".green().bold(), tag.cyan());

    // Print summary
    println!("\n{}", "Changes:".bold());
    print_diff_summary(&diff);

    Ok(())
}

fn create_custom_migration(
    migrations_dir: &Path,
    name: Option<&str>,
    dialect: &str,
    breakpoints: bool,
) -> anyhow::Result<()> {
    std::fs::create_dir_all(migrations_dir.join("meta"))?;

    // Load or create journal
    let journal_path = migrations_dir.join("meta").join("_journal.json");
    let mut journal = Journal::load_or_create(&journal_path, dialect)
        .map_err(|e| anyhow::anyhow!("Failed to load journal: {}", e))?;

    // Load the previous snapshot BEFORE modifying the journal
    // For custom migrations, we copy the previous snapshot or create an empty one
    let snapshot_to_save = snapshot::load_latest_snapshot(migrations_dir, dialect)?
        .unwrap_or_else(|| snapshot::empty_snapshot(dialect).unwrap());

    // Generate migration tag
    let idx = journal.next_idx();
    let tag = generate_tag(idx, name);

    // Create empty SQL file
    let sql_content = "-- Custom migration\n-- Add your SQL statements here\n";

    let sql_path = migrations_dir.join(format!("{}.sql", tag));
    std::fs::write(&sql_path, sql_content)?;

    // Save snapshot for this migration
    snapshot::save_snapshot(migrations_dir, &snapshot_to_save, idx)?;

    // Add entry to journal and save
    journal.add_entry(tag.clone(), breakpoints);
    journal
        .save(&journal_path)
        .map_err(|e| anyhow::anyhow!("Failed to save journal: {}", e))?;

    println!(
        "{} Created custom migration: {}",
        "✓".green().bold(),
        tag.cyan()
    );
    println!("  Edit: {}", sql_path.display());

    Ok(())
}

fn write_migration(
    migrations_dir: &Path,
    statements: &[String],
    snapshot: &SQLiteSnapshot,
    dialect: &str,
    name: Option<&str>,
    prefix: &str,
    breakpoints: bool,
) -> anyhow::Result<(String, u32)> {
    std::fs::create_dir_all(migrations_dir.join("meta"))?;

    // Load or create journal
    let journal_path = migrations_dir.join("meta").join("_journal.json");
    let mut journal = Journal::load_or_create(&journal_path, dialect)
        .map_err(|e| anyhow::anyhow!("Failed to load journal: {}", e))?;

    // Generate migration tag
    let idx = journal.next_idx();
    let tag = match prefix {
        "timestamp" => {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs();
            if let Some(n) = name {
                format!("{}_{}", ts, n)
            } else {
                format!("{}_{}", ts, random_name())
            }
        }
        "none" => name.map(|n| n.to_string()).unwrap_or_else(random_name),
        _ => {
            // Default: index prefix
            if let Some(n) = name {
                format!("{:04}_{}", idx, n)
            } else {
                format!("{:04}_{}", idx, random_name())
            }
        }
    };

    // Write SQL file
    let sql_content = if breakpoints {
        statements.join("\n--> statement-breakpoint\n")
    } else {
        statements.join("\n")
    };

    let sql_path = migrations_dir.join(format!("{}.sql", tag));
    std::fs::write(&sql_path, &sql_content)?;

    // Add entry to journal
    journal.add_entry(tag.clone(), breakpoints);
    journal
        .save(&journal_path)
        .map_err(|e| anyhow::anyhow!("Failed to save journal: {}", e))?;

    // Save snapshot
    snapshot::save_snapshot(migrations_dir, snapshot, idx)?;

    Ok((tag, idx))
}

fn generate_tag(idx: u32, name: Option<&str>) -> String {
    if let Some(n) = name {
        format!("{:04}_{}", idx, n)
    } else {
        format!("{:04}_{}", idx, random_name())
    }
}

fn random_name() -> String {
    use drizzle_migrations::words::{ADJECTIVES, NOUNS};

    let adj_idx = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as usize
        % ADJECTIVES.len();
    let noun_idx = (adj_idx * 7) % NOUNS.len();

    format!("{}_{}", ADJECTIVES[adj_idx], NOUNS[noun_idx])
}

fn print_diff_summary(diff: &drizzle_migrations::sqlite::SchemaDiff) {
    // Tables
    if !diff.tables.created.is_empty() {
        for table in &diff.tables.created {
            println!("  {} table {}", "+".green(), table.name.green());
        }
    }
    if !diff.tables.deleted.is_empty() {
        for name in &diff.tables.deleted {
            println!("  {} table {}", "-".red(), name.red());
        }
    }

    // Altered tables
    for altered in &diff.tables.altered {
        println!("  {} table {}", "~".yellow(), altered.name.yellow());

        // Columns
        for col in &altered.columns.added {
            println!("    {} column {}", "+".green(), col.name.green());
        }
        for name in &altered.columns.deleted {
            println!("    {} column {}", "-".red(), name.red());
        }
        for ac in &altered.columns.altered {
            println!("    {} column {}", "~".yellow(), ac.name.yellow());
        }

        // Indexes
        for idx in &altered.indexes.added {
            println!("    {} index {}", "+".green(), idx.name.green());
        }
        for name in &altered.indexes.deleted {
            println!("    {} index {}", "-".red(), name.red());
        }

        // Foreign keys
        for fk in &altered.foreign_keys.added {
            println!("    {} foreign key {}", "+".green(), fk.name.green());
        }
        for name in &altered.foreign_keys.deleted {
            println!("    {} foreign key {}", "-".red(), name.red());
        }
    }
}
