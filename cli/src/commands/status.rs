//! Status command - show current migration status

use colored::Colorize;
use drizzle_migrations::Journal;
use std::path::Path;

pub fn run(out_dir: &str) -> anyhow::Result<()> {
    let migrations_dir = Path::new(out_dir).join("migrations");
    let meta_dir = migrations_dir.join("meta");

    println!("Migration Status");
    println!("{}", "─".repeat(50));
    println!();

    if !migrations_dir.exists() {
        println!("  {} No migrations directory", "!".yellow());
        println!(
            "  Run {} to create your first migration",
            "drizzle generate".cyan()
        );
        return Ok(());
    }

    // Load journal
    let journal_path = meta_dir.join("_journal.json");
    if !journal_path.exists() {
        println!("  {} No journal file", "!".yellow());
        println!(
            "  Run {} to create your first migration",
            "drizzle generate".cyan()
        );
        return Ok(());
    }

    let journal_content = std::fs::read_to_string(&journal_path)?;
    let journal: Journal = serde_json::from_str(&journal_content)?;

    println!("  Dialect: {}", journal.dialect.cyan());
    println!("  Version: {}", journal.version);
    println!("  Directory: {}", migrations_dir.display());
    println!();

    if journal.entries.is_empty() {
        println!("  {} No migrations yet", "!".yellow());
        println!(
            "  Run {} to create your first migration",
            "drizzle generate".cyan()
        );
        return Ok(());
    }

    println!("  Migrations ({}):", journal.entries.len());
    for entry in &journal.entries {
        let sql_path = migrations_dir.join(format!("{}.sql", entry.tag));
        let exists = sql_path.exists();

        let status = if exists { "✓".green() } else { "✗".red() };

        let timestamp = format_timestamp(entry.when);

        println!(
            "    {} {:04} {} ({})",
            status,
            entry.idx,
            entry.tag,
            timestamp.dimmed()
        );
    }

    println!();

    Ok(())
}

fn format_timestamp(ms: u64) -> String {
    // Simple timestamp formatting
    if ms == 0 {
        return "unknown".to_string();
    }

    let secs = ms / 1000;
    let datetime = std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs as u64);

    // Format as ISO date
    format!("{:?}", datetime)
        .replace("SystemTime", "")
        .replace("{ tv_sec:", "")
        .replace(", tv_nsec:", "")
        .replace("}", "")
        .trim()
        .to_string()
}
