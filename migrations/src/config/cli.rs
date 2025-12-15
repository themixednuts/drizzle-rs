//! CLI arguments for drizzle migrations
//!
//! This module defines the command-line interface for the drizzle migration tool.

use clap::{Parser, Subcommand};

/// Drizzle CLI arguments
#[derive(Parser, Debug)]
#[command(name = "drizzle")]
#[command(about = "Drizzle migration CLI", long_about = None)]
pub struct CliArgs {
    #[command(subcommand)]
    pub command: CliCommand,
}

/// CLI subcommands
#[derive(Subcommand, Debug)]
pub enum CliCommand {
    /// Generate a new migration from schema changes
    Generate {
        /// Migration name (optional, auto-generated if not provided)
        #[arg(short, long)]
        name: Option<String>,

        /// Create a custom (empty) migration file
        #[arg(long)]
        custom: bool,
    },
    /// Run pending migrations from the migrations folder
    Migrate,
    /// Show migration status
    Status,
    /// Push schema changes directly to the database (no migration file)
    Push,
    /// Introspect an existing database and generate a snapshot
    Introspect {
        /// Output directory for the generated snapshot (defaults to out directory)
        #[arg(short, long)]
        output: Option<std::path::PathBuf>,
    },
}
