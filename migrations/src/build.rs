//! Build-time migration generation helpers.
//!
//! This module is intended for `build.rs` flows where users do not want to use
//! the CLI. It parses Rust schema files, computes diffs against the latest
//! snapshot in `./drizzle`, and writes a new migration folder when needed.
//!
//! # Recommended flow
//!
//! ```rust,no_run
//! use drizzle_migrations::build::{Config, Output, run};
//! use drizzle_types::Dialect;
//!
//! fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let cfg = Config::new(Dialect::SQLite)
//!         .file("src/schema.rs")
//!         .out("./drizzle");
//!
//!     // Tell Cargo to rerun build.rs when schema files change.
//!     cfg.watch();
//!
//!     match run(&cfg)? {
//!         Output::NoChanges => {}
//!         Output::Generated { tag, path, .. } => {
//!             println!("cargo:warning=generated migration {tag} at {}", path.display());
//!         }
//!     }
//!
//!     Ok(())
//! }
//! ```

use crate::generate::diff;
use crate::journal::Journal;
use crate::parser::SchemaParser;
use crate::schema::Snapshot;
use crate::snapshot_builder::parse_result_to_snapshot;
use crate::words::{PrefixMode, generate_migration_tag_with_mode};
pub use drizzle_types::Casing;
use drizzle_types::Dialect;
use std::path::{Path, PathBuf};

/// Build-time migration generation configuration.
#[derive(Debug, Clone)]
pub struct Config {
    files: Vec<PathBuf>,
    out_dir: PathBuf,
    dialect: Dialect,
    casing: Option<Casing>,
    breakpoints: bool,
    prefix_mode: PrefixMode,
    custom_name: Option<String>,
}

impl Config {
    /// Create a new configuration.
    ///
    /// `out_dir` defaults to `./drizzle`, breakpoints are enabled by default,
    /// and migration tag prefixes default to timestamp mode.
    #[must_use]
    pub fn new(dialect: Dialect) -> Self {
        Self {
            files: Vec::new(),
            out_dir: PathBuf::from("./drizzle"),
            dialect,
            casing: None,
            breakpoints: true,
            prefix_mode: PrefixMode::Timestamp,
            custom_name: None,
        }
    }

    /// Add one Rust source file to the build input set.
    #[must_use]
    pub fn file(self, path: impl Into<PathBuf>) -> Self {
        let mut this = self;
        this.files.push(path.into());
        this
    }

    /// Set the output migrations directory.
    #[must_use]
    pub fn out(mut self, out_dir: impl Into<PathBuf>) -> Self {
        self.out_dir = out_dir.into();
        self
    }

    /// Set the inferred naming casing strategy.
    #[must_use]
    pub fn casing(mut self, casing: Casing) -> Self {
        self.casing = Some(casing);
        self
    }

    /// Enable or disable statement breakpoints in written SQL.
    #[must_use]
    pub fn breakpoints(mut self, enabled: bool) -> Self {
        self.breakpoints = enabled;
        self
    }

    /// Set migration tag prefix mode.
    #[must_use]
    pub fn prefix_mode(mut self, mode: PrefixMode) -> Self {
        self.prefix_mode = mode;
        self
    }

    /// Set a custom suffix for the generated migration tag.
    #[must_use]
    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.custom_name = Some(name.into());
        self
    }

    /// Emit `cargo:rerun-if-changed=...` directives for each configured file.
    pub fn watch(&self) {
        for path in &self.files {
            println!("cargo:rerun-if-changed={}", path.display());
        }
    }
}

/// Result of a build-time migration generation run.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Output {
    /// No schema changes were detected.
    NoChanges,
    /// A new migration folder was written.
    Generated {
        /// Generated migration tag (folder name).
        tag: String,
        /// Absolute/relative path to the written migration directory.
        path: PathBuf,
        /// Number of SQL statements emitted.
        statement_count: usize,
    },
}

impl Output {
    #[must_use]
    pub fn is_generated(&self) -> bool {
        matches!(self, Self::Generated { .. })
    }
}

/// Errors that can occur while generating migrations in `build.rs`.
#[derive(Debug, thiserror::Error)]
pub enum BuildError {
    #[error("unsupported dialect for build generation: {0:?}")]
    UnsupportedDialect(Dialect),

    #[error("no schema files configured")]
    MissingSchemaFiles,

    #[error("failed to read schema file `{path:?}`: {source}")]
    ReadSchema {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },

    #[error("failed to parse or write migration metadata: {0}")]
    Io(#[from] std::io::Error),

    #[error("failed to generate migration diff: {0}")]
    Migration(#[from] crate::writer::MigrationError),
}

/// Generate and write a migration folder when schema changes are detected.
///
/// This is the high-level API that handles:
/// - diffing against the latest local snapshot
/// - tag generation
/// - writing `migration.sql` and `snapshot.json` in `./drizzle/<tag>/`
///
/// # Example
///
/// ```rust,no_run
/// use drizzle_migrations::build::{Config, Output, run};
/// use drizzle_types::Dialect;
///
/// let cfg = Config::new(Dialect::SQLite)
///     .file("src/schema.rs")
///     .out("./drizzle");
///
/// let outcome = run(&cfg)?;
/// if let Output::Generated { tag, .. } = outcome {
///     println!("generated {tag}");
/// }
/// # Ok::<(), drizzle_migrations::BuildError>(())
/// ```
pub fn run(config: &Config) -> Result<Output, BuildError> {
    if config.files.is_empty() {
        return Err(BuildError::MissingSchemaFiles);
    }

    if !matches!(config.dialect, Dialect::SQLite | Dialect::PostgreSQL) {
        return Err(BuildError::UnsupportedDialect(config.dialect));
    }

    let parse_result = parse_files(&config.files)?;
    if parse_result.tables.is_empty() && parse_result.indexes.is_empty() {
        return Ok(Output::NoChanges);
    }

    let current_snapshot = parse_result_to_snapshot(&parse_result, config.dialect, config.casing);
    let journal_path = config.out_dir.join("meta").join("_journal.json");
    let previous_snapshot = load_previous_snapshot(&config.out_dir, &journal_path, config.dialect)?;
    let generated = diff(&previous_snapshot, &current_snapshot)?;

    if generated.is_empty() {
        return Ok(Output::NoChanges);
    }

    std::fs::create_dir_all(&config.out_dir)?;
    let next_idx = next_migration_index(&config.out_dir)?;
    let tag = generate_migration_tag_with_mode(
        config.prefix_mode,
        next_idx,
        config.custom_name.as_deref(),
    );

    let migration_dir = config.out_dir.join(&tag);
    std::fs::create_dir_all(&migration_dir)?;

    let sql = if config.breakpoints {
        generated.to_sql()
    } else {
        generated.statements.join("\n\n")
    };
    std::fs::write(migration_dir.join("migration.sql"), sql)?;

    generated
        .snapshot
        .save(&migration_dir.join("snapshot.json"))?;

    Ok(Output::Generated {
        tag,
        path: migration_dir,
        statement_count: generated.statements.len(),
    })
}

fn parse_files(files: &[PathBuf]) -> Result<crate::parser::ParseResult, BuildError> {
    let mut combined = String::new();
    for path in files {
        let code = std::fs::read_to_string(path).map_err(|source| BuildError::ReadSchema {
            path: path.clone(),
            source,
        })?;
        combined.push_str(&code);
        combined.push('\n');
    }
    Ok(SchemaParser::parse(&combined))
}

fn load_previous_snapshot(
    out_dir: &Path,
    journal_path: &Path,
    dialect: Dialect,
) -> Result<Snapshot, BuildError> {
    let v3_entries = collect_v3_migration_dirs(out_dir)?;
    if let Some((_, migration_dir)) = v3_entries.last() {
        let snapshot_path = migration_dir.join("snapshot.json");
        if snapshot_path.exists() {
            return Snapshot::load(&snapshot_path, dialect).map_err(BuildError::from);
        }
    }

    // Legacy fallback for pre-V3 folder formats.
    if journal_path.exists() {
        let journal = Journal::load(journal_path)?;
        if let Some(latest) = journal.entries.last() {
            let snapshot_path = out_dir.join(&latest.tag).join("snapshot.json");
            if snapshot_path.exists() {
                return Snapshot::load(&snapshot_path, dialect).map_err(BuildError::from);
            }
        }
    }

    Ok(Snapshot::empty(dialect))
}

fn next_migration_index(out_dir: &Path) -> Result<u32, BuildError> {
    let entries = collect_v3_migration_dirs(out_dir)?;
    let mut max_index: Option<u32> = None;

    for (tag, _) in &entries {
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

fn collect_v3_migration_dirs(out_dir: &Path) -> Result<Vec<(String, PathBuf)>, BuildError> {
    if !out_dir.exists() {
        return Ok(Vec::new());
    }

    let mut entries = Vec::new();
    for entry in std::fs::read_dir(out_dir)? {
        let entry = entry?;
        if !entry.file_type()?.is_dir() {
            continue;
        }

        let tag = entry.file_name().to_string_lossy().to_string();
        if tag == "meta" {
            continue;
        }

        let path = entry.path();
        if path.join("migration.sql").exists() {
            entries.push((tag, path));
        }
    }

    entries.sort_by(|a, b| a.0.cmp(&b.0));
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_creates_then_stabilizes() {
        let dir = tempfile::tempdir().expect("tempdir");
        let schema_path = dir.path().join("schema.rs");
        let out_dir = dir.path().join("drizzle");

        std::fs::write(
            &schema_path,
            r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
}
"#,
        )
        .expect("write schema");

        let cfg = Config::new(Dialect::SQLite)
            .file(&schema_path)
            .out(&out_dir);

        let first = run(&cfg).expect("first generation should succeed");
        assert!(matches!(first, Output::Generated { .. }));
        assert!(
            !out_dir.join("meta").join("_journal.json").exists(),
            "v3 generation should not create legacy journal metadata"
        );

        let second = run(&cfg).expect("second generation should succeed");
        assert_eq!(second, Output::NoChanges);
    }

    #[test]
    fn run_accepts_multiple_files() {
        let dir = tempfile::tempdir().expect("tempdir");
        let users_path = dir.path().join("users.rs");
        let posts_path = dir.path().join("posts.rs");
        let schema_path = dir.path().join("schema.rs");
        let out_dir = dir.path().join("drizzle");

        std::fs::write(
            &users_path,
            r#"
#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    pub name: String,
}
"#,
        )
        .expect("write users schema");

        std::fs::write(
            &posts_path,
            r#"
#[SQLiteTable]
pub struct Posts {
    #[column(primary)]
    pub id: i64,
    #[column(references = Users::id)]
    pub author_id: i64,
}
"#,
        )
        .expect("write posts schema");

        std::fs::write(
            &schema_path,
            r#"
#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
}
"#,
        )
        .expect("write root schema");

        let cfg = Config::new(Dialect::SQLite)
            .file(&users_path)
            .file(&posts_path)
            .file(&schema_path)
            .out(&out_dir);

        let outcome = run(&cfg).expect("generation should succeed");
        let Output::Generated { path, .. } = outcome else {
            panic!("expected a migration to be generated");
        };

        let migration_sql_path = path.join("migration.sql");
        assert!(migration_sql_path.exists(), "migration.sql should exist");
        assert!(
            path.join("snapshot.json").exists(),
            "snapshot.json should exist"
        );

        let migration_sql =
            std::fs::read_to_string(&migration_sql_path).expect("read generated migration.sql");
        let mut statements: Vec<_> = migration_sql
            .split("\n--> statement-breakpoint\n")
            .map(str::to_string)
            .collect();
        statements.sort();

        let mut expected = vec![
            "CREATE TABLE `posts` (\n\t`id` INTEGER PRIMARY KEY,\n\t`author_id` INTEGER NOT NULL,\n\tCONSTRAINT `posts_author_id_users_id_fk` FOREIGN KEY (`author_id`) REFERENCES `users`(`id`)\n);\n".to_string(),
            "CREATE TABLE `users` (\n\t`id` INTEGER PRIMARY KEY,\n\t`name` TEXT NOT NULL\n);\n".to_string(),
        ];
        expected.sort();

        assert_eq!(statements, expected, "unexpected generated migration SQL");
    }
}
