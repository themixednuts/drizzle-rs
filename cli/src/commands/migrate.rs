//! Migrate command implementation
//!
//! Runs pending migrations against the database.

use crate::config::Config;
use crate::error::CliError;
use crate::output;

#[derive(Debug, Clone, Copy, Default)]
pub struct MigrateOptions {
    pub verify: bool,
    pub plan: bool,
    pub safe: bool,
}

/// Run the migrate command
pub fn run(config: &Config, db_name: Option<&str>, opts: MigrateOptions) -> Result<(), CliError> {
    if opts.safe && opts.verify {
        return Err(CliError::Other(
            "--safe can't be combined with --verify".to_string(),
        ));
    }
    if opts.safe && opts.plan {
        return Err(CliError::Other(
            "--safe can't be combined with --plan".to_string(),
        ));
    }

    let db = config.database(db_name)?;

    if !config.is_single_database() {
        let name = db_name.unwrap_or("(default)");
        println!("{}: {}", output::label("Database"), name);
    }

    let heading = if opts.verify {
        "Verifying migrations..."
    } else if opts.plan {
        "Planning migrations..."
    } else if opts.safe {
        "Running safe migration flow..."
    } else {
        "Running migrations..."
    };
    println!("{}", output::heading(heading));
    println!();

    let out_dir = db.migrations_dir();

    // Check if migrations directory exists
    if !out_dir.exists() {
        println!("  {}", output::warning("No migrations directory found."));
        println!("  Run 'drizzle generate' to create your first migration.");
        return Ok(());
    }

    // Get credentials
    let credentials = db.credentials()?;

    let credentials = match credentials {
        Some(c) => c,
        None => {
            println!("{}", output::warning("No database credentials configured."));
            println!();
            println!("Add credentials to your drizzle.config.toml:");
            println!();
            println!("  {}", output::muted("[dbCredentials]"));
            println!("  {}", output::muted("url = \"./dev.db\""));
            println!();
            println!("Or use an environment variable:");
            println!();
            println!("  {}", output::muted("[dbCredentials]"));
            println!("  {}", output::muted("url = { env = \"DATABASE_URL\" }"));
            return Ok(());
        }
    };

    let plan = if opts.verify || opts.plan || opts.safe {
        Some(crate::db::verify_migrations(
            &credentials,
            db.dialect,
            out_dir,
            db.migrations_table(),
            db.migrations_schema(),
        )?)
    } else {
        None
    };

    if let Some(plan) = &plan {
        println!(
            "  {} {}",
            output::label("Applied migrations:"),
            plan.applied_count
        );
        println!(
            "  {} {} ({} statement(s))",
            output::label("Pending migrations:"),
            plan.pending_count,
            plan.pending_statements
        );

        if !plan.pending_migrations.is_empty() {
            println!("  {}", output::label("Pending tags:"));
            for tag in &plan.pending_migrations {
                println!("    {} {}", output::label("->"), tag);
            }
        }
        println!();

        if opts.verify {
            println!("{}", output::success("Migration verification passed."));
            return Ok(());
        }

        if opts.plan {
            println!("{}", output::success("Migration plan complete."));
            return Ok(());
        }

        if opts.safe && plan.pending_count == 0 {
            println!("  {}", output::success("No pending migrations."));
            println!();
            println!("{}", output::success("Safe migration complete!"));
            return Ok(());
        }
    }

    // Run migrations
    let result = crate::db::run_migrations(
        &credentials,
        db.dialect,
        out_dir,
        db.migrations_table(),
        db.migrations_schema(),
    )?;

    if result.applied_count == 0 {
        println!("  {}", output::success("No pending migrations."));
    } else {
        println!(
            "  {} {} migration(s):",
            output::success("Applied"),
            result.applied_count
        );
        for hash in &result.applied_migrations {
            println!("    {} {}", output::label("->"), hash);
        }
    }

    println!();
    if opts.safe {
        println!("{}", output::success("Safe migration complete!"));
    } else {
        println!("{}", output::success("Migrations complete!"));
    }

    Ok(())
}
