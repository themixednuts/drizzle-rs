//! Drop command - remove a migration
//!
//! This command allows you to drop the last migration, removing:
//! 1. The SQL file
//! 2. The snapshot
//! 3. The journal entry

use crate::error::CliError;
use colored::Colorize;
use drizzle_migrations::Journal;
use std::io::{self, Write};
use std::path::Path;

pub struct DropOptions {
    pub out_dir: String,
}

pub fn run(opts: DropOptions) -> anyhow::Result<()> {
    let migrations_dir = Path::new(&opts.out_dir).join("migrations");
    let meta_dir = migrations_dir.join("meta");

    if !migrations_dir.exists() {
        return Err(CliError::NoMigrations(migrations_dir.display().to_string()).into());
    }

    // Load journal
    let journal_path = meta_dir.join("_journal.json");
    if !journal_path.exists() {
        return Err(CliError::JournalNotFound(journal_path.display().to_string()).into());
    }

    let journal_content = std::fs::read_to_string(&journal_path)?;
    let mut journal: Journal = serde_json::from_str(&journal_content)?;

    if journal.entries.is_empty() {
        println!("{}", "No migrations to drop".yellow());
        return Ok(());
    }

    // Show available migrations
    println!("Available migrations:\n");
    for (i, entry) in journal.entries.iter().enumerate().rev() {
        println!("  [{}] {}", i, entry.tag);
    }
    println!();

    // Prompt for which migration to drop
    print!("Enter migration number to drop (or 'last' for the last one): ");
    io::stdout().flush()?;

    let mut input = String::new();
    io::stdin().read_line(&mut input)?;
    let input = input.trim();

    let idx = if input == "last" {
        journal.entries.len() - 1
    } else {
        input.parse::<usize>().map_err(|_| anyhow::anyhow!("Invalid input"))?
    };

    if idx >= journal.entries.len() {
        anyhow::bail!("Invalid migration index");
    }

    let entry = &journal.entries[idx];
    let sql_path = migrations_dir.join(format!("{}.sql", entry.tag));
    let snapshot_path = meta_dir.join(format!("{:04}_snapshot.json", entry.idx));

    // Confirm
    println!();
    println!("This will delete:");
    println!("  - {}", sql_path.display());
    println!("  - {}", snapshot_path.display());
    println!();

    print!("Are you sure? [y/N] ");
    io::stdout().flush()?;

    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm)?;

    if confirm.trim().to_lowercase() != "y" {
        println!("Cancelled");
        return Ok(());
    }

    // Remove files
    if sql_path.exists() {
        std::fs::remove_file(&sql_path)?;
        println!("  {} Removed {}", "✓".green(), sql_path.display());
    }

    if snapshot_path.exists() {
        std::fs::remove_file(&snapshot_path)?;
        println!("  {} Removed {}", "✓".green(), snapshot_path.display());
    }

    // Update journal
    let removed_tag = journal.entries.remove(idx).tag;

    // Re-index remaining entries
    for (i, entry) in journal.entries.iter_mut().enumerate() {
        entry.idx = i as u32;
    }

    let journal_content = serde_json::to_string_pretty(&journal)?;
    std::fs::write(&journal_path, journal_content)?;

    println!();
    println!(
        "{} Dropped migration: {}",
        "✓".green().bold(),
        removed_tag.cyan()
    );

    Ok(())
}

