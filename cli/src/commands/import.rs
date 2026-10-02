//! `drizzle import`: ports a TypeScript drizzle-orm project by writing the
//! Rust schema from its drizzle-kit migration snapshots.
//!
//! drizzle-kit already serializes the TypeScript schema into each migration's
//! snapshot, so no TypeScript is read. The newest snapshot is upgraded in
//! memory to the current format (the same chain `drizzle up` runs) and
//! written as Rust with the codegen behind `drizzle introspect`.
//!
//! Accepted inputs:
//!
//! * a drizzle-kit `out` folder in the folder layout
//!   (`<tag>/migration.sql` + `<tag>/snapshot.json`): the last folder wins;
//! * a folder in the older journal layout (`NNNN_tag.sql` +
//!   `meta/_journal.json` + `meta/NNNN_snapshot.json`): the last journal
//!   entry's snapshot wins;
//! * a single `snapshot.json` file.
//!
//! The migration files are left where they are unless `--upgrade` is given.

use crate::codegen::{SchemaCode, SchemaCodeOptions, schema_code};
use crate::config::{CONFIG_FILE, Config, Dialect, IntrospectCasing, Schema};
use crate::error::CliError;
use crate::output;
use drizzle_migrations::schema::Snapshot;
use drizzle_migrations::upgrade::upgrade_to_latest;
use drizzle_migrations::version::{is_supported_version, needs_upgrade, snapshot_version};
use serde_json::Value;
use std::fmt::Display;
use std::fs;
use std::path::{Path, PathBuf};

/// Schema path used when neither `--out` nor a config names one.
const DEFAULT_SCHEMA_PATH: &str = "src/schema.rs";

#[derive(clap::Args, Debug, Clone)]
pub struct ImportOptions {
    /// drizzle-kit migrations folder (either layout) or a snapshot.json file
    #[arg(value_name = "PATH")]
    pub path: PathBuf,

    /// File to write the schema to, or `-` for stdout [default: the config's
    /// `schema`, else src/schema.rs]
    #[arg(long, value_name = "FILE")]
    pub out: Option<PathBuf>,

    /// Overwrite the output file if it exists
    #[arg(long)]
    pub force: bool,

    /// Dialect of the snapshot (default: its `dialect` field)
    #[arg(long)]
    pub dialect: Option<Dialect>,

    /// Casing for Rust field names (camel or preserve) [default: the config's
    /// `introspect.casing` when set, else snake_case]
    #[arg(long)]
    pub casing: Option<IntrospectCasing>,

    /// Name of the generated schema struct
    #[arg(long, value_name = "NAME", default_value = "Schema")]
    pub schema_name: String,

    /// Convert a journal-layout folder to the folder layout in place, as
    /// `drizzle up` does
    #[arg(long)]
    pub upgrade: bool,

    /// Write a starter drizzle.config.toml for the imported project when
    /// none exists
    #[arg(long)]
    pub init_config: bool,
}

/// How the migrations next to the imported snapshot are laid out.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Layout {
    /// `<tag>/migration.sql` + `<tag>/snapshot.json` (drizzle-kit 1.x and
    /// drizzle-rs).
    Folders,
    /// `NNNN_tag.sql` + `meta/_journal.json` (drizzle-kit 0.x).
    Journal,
}

/// The snapshot to import and the migrations folder it belongs to.
#[derive(Debug)]
struct Source {
    snapshot_path: PathBuf,
    /// The migrations folder and its layout, when the snapshot sits in one.
    migrations: Option<(PathBuf, Layout)>,
}

/// Where human-readable messages go: stdout, or stderr when the schema
/// itself is printed to stdout.
#[derive(Clone, Copy)]
struct Messages {
    to_stderr: bool,
}

impl Messages {
    fn line(self, text: impl Display) {
        if self.to_stderr {
            eprintln!("{text}");
        } else {
            println!("{text}");
        }
    }
}

/// Runs `drizzle import` (see the [module docs](self)).
///
/// `config` is the loaded `drizzle.config.toml`, if there is one; it only
/// supplies the default output path and casing.
///
/// # Errors
///
/// Returns [`CliError`] if `db_name` does not match the config, no snapshot
/// is found at the path, the snapshot is not valid JSON, its dialect is
/// unknown or unsupported, its version is too old or too new, it does not
/// convert to the current format, the output file exists without
/// `--force`, or a file cannot be written.
pub fn run(
    config: Option<&Config>,
    db_name: Option<&str>,
    opts: &ImportOptions,
) -> Result<(), CliError> {
    let db = config.map(|c| c.database(db_name)).transpose()?;
    let to_stdout = opts.out.as_deref() == Some(Path::new("-"));
    let say = Messages {
        to_stderr: to_stdout,
    };
    if to_stdout && opts.upgrade {
        return Err(CliError::Other(
            "--upgrade cannot be combined with --out - (it prints to stdout)".into(),
        ));
    }

    let source = find_snapshot(&opts.path)?;
    let contents = fs::read_to_string(&source.snapshot_path)
        .map_err(|e| CliError::IoError(format!("{}: {e}", source.snapshot_path.display())))?;
    let json: Value = serde_json::from_str(&contents).map_err(|e| {
        CliError::Other(format!(
            "Invalid JSON in {}: {e}",
            source.snapshot_path.display()
        ))
    })?;

    let detected = snapshot_dialect(&json, &source.snapshot_path);
    let dialect = match (opts.dialect, detected) {
        // `--dialect turso` on a `sqlite` snapshot, or a snapshot without a
        // usable `dialect` field.
        (Some(wanted), Ok(found)) if wanted.to_base() != found.to_base() => {
            return Err(CliError::Other(format!(
                "{} is a {} snapshot; --dialect {} does not match it",
                source.snapshot_path.display(),
                found.as_str(),
                wanted.as_str()
            )));
        }
        (Some(wanted), _) => wanted,
        (None, detected) => detected?,
    };
    let snapshot = load_snapshot(json, dialect, &source.snapshot_path)?;

    let casing = opts.casing.or_else(|| {
        db.and_then(|db| db.introspect.as_ref())
            .map(|introspect| introspect.casing)
    });
    let module_doc = format!(
        "Imported from the drizzle-kit snapshot {}",
        forward_slashes(&source.snapshot_path)
    );
    let generated = schema_code(
        &snapshot,
        &SchemaCodeOptions {
            casing,
            module_doc: &module_doc,
            schema_name: &opts.schema_name,
        },
    )?;

    let out_path = if to_stdout {
        None
    } else {
        Some(resolve_out_path(opts.out.as_deref(), db)?)
    };

    say.line(output::heading("Importing drizzle-kit snapshot..."));
    say.line("");
    say.line(format!(
        "  {}: {}",
        output::label("Snapshot"),
        source.snapshot_path.display()
    ));
    say.line(format!(
        "  {}: {}",
        output::label("Dialect"),
        dialect.as_str()
    ));
    say.line(format!(
        "  {}: {}",
        output::label("Output"),
        out_path
            .as_deref()
            .map_or_else(|| "stdout".to_string(), |p| p.display().to_string())
    ));

    match &out_path {
        None => print!("{}", generated.code),
        Some(path) => write_schema(path, &generated.code, opts.force)?,
    }

    print_summary(say, &generated);

    if opts.upgrade {
        match &source.migrations {
            Some((dir, Layout::Journal)) => {
                say.line("");
                let converted =
                    crate::commands::upgrade::convert_legacy_layout(dir, dialect.to_base())?;
                say.line(output::success(&format!(
                    "Converted {converted} migration(s) in {} to the folder layout",
                    dir.display()
                )));
            }
            _ => say.line(output::warning(
                "--upgrade: the snapshot is not in a journal-layout folder; nothing to convert",
            )),
        }
    }

    let migrations = source.migrations.as_ref().map(|(dir, layout)| {
        let layout = if opts.upgrade {
            Layout::Folders
        } else {
            *layout
        };
        (dir.as_path(), layout)
    });

    let wrote_config = if opts.init_config {
        init_config(say, config, dialect, out_path.as_deref(), migrations)?
    } else {
        false
    };

    print_next_steps(
        say,
        &NextSteps {
            config,
            db_out: db.map(crate::config::DatabaseConfig::migrations_dir),
            wrote_config,
            dialect,
            schema_path: out_path.as_deref(),
            migrations,
        },
    );
    print_manual_port_notes(say);

    Ok(())
}

// =============================================================================
// Finding the snapshot
// =============================================================================

fn find_snapshot(path: &Path) -> Result<Source, CliError> {
    if path.is_file() {
        return Ok(Source {
            snapshot_path: path.to_path_buf(),
            migrations: migrations_around_file(path),
        });
    }
    if !path.is_dir() {
        return Err(CliError::Other(format!(
            "{} does not exist. Pass a drizzle-kit migrations folder (the `out` of drizzle.config.ts) or a snapshot.json file.",
            path.display()
        )));
    }

    let journal_path = path.join("meta").join("_journal.json");
    if journal_path.is_file() {
        return Ok(Source {
            snapshot_path: last_journal_snapshot(path, &journal_path)?,
            migrations: Some((path.to_path_buf(), Layout::Journal)),
        });
    }

    let mut folders = Vec::new();
    for entry in fs::read_dir(path).map_err(|e| CliError::IoError(e.to_string()))? {
        let entry = entry.map_err(|e| CliError::IoError(e.to_string()))?;
        let candidate = entry.path().join("snapshot.json");
        if candidate.is_file() {
            folders.push(entry.file_name());
        }
    }
    // Folder names start with their creation timestamp, so the last name
    // is the newest migration (the order drizzle-kit and `migrate` use).
    folders.sort();
    let Some(newest) = folders.last() else {
        return Err(CliError::Other(format!(
            "No drizzle-kit snapshots found in {}: expected <tag>/snapshot.json folders or meta/_journal.json. Pass the `out` folder of drizzle.config.ts, or a snapshot.json file.",
            path.display()
        )));
    };
    Ok(Source {
        snapshot_path: path.join(newest).join("snapshot.json"),
        migrations: Some((path.to_path_buf(), Layout::Folders)),
    })
}

/// The snapshot of the last entry in a drizzle-kit journal.
fn last_journal_snapshot(dir: &Path, journal_path: &Path) -> Result<PathBuf, CliError> {
    let text = fs::read_to_string(journal_path).map_err(|e| CliError::IoError(e.to_string()))?;
    let journal: Value = serde_json::from_str(&text)
        .map_err(|e| CliError::Other(format!("Invalid JSON in {}: {e}", journal_path.display())))?;
    let last = journal
        .get("entries")
        .and_then(Value::as_array)
        .and_then(|entries| entries.last())
        .ok_or_else(|| {
            CliError::Other(format!(
                "{} lists no migrations, so there is no snapshot to import",
                journal_path.display()
            ))
        })?;
    let idx = last.get("idx").and_then(Value::as_u64).ok_or_else(|| {
        CliError::Other(format!(
            "The last entry of {} has no 'idx'",
            journal_path.display()
        ))
    })?;
    let snapshot_path = dir.join("meta").join(format!("{idx:04}_snapshot.json"));
    if !snapshot_path.is_file() {
        return Err(CliError::Other(format!(
            "{} is missing (named by the last entry of {})",
            snapshot_path.display(),
            journal_path.display()
        )));
    }
    Ok(snapshot_path)
}

/// The migrations folder a snapshot file sits in, if any:
/// `<out>/<tag>/snapshot.json` or `<out>/meta/NNNN_snapshot.json`.
fn migrations_around_file(file: &Path) -> Option<(PathBuf, Layout)> {
    let parent = file.parent()?;
    let grandparent = parent.parent()?;
    if parent.file_name().is_some_and(|name| name == "meta")
        && parent.join("_journal.json").is_file()
    {
        return Some((grandparent.to_path_buf(), Layout::Journal));
    }
    if file.file_name().is_some_and(|name| name == "snapshot.json")
        && parent.join("migration.sql").is_file()
    {
        return Some((grandparent.to_path_buf(), Layout::Folders));
    }
    None
}

// =============================================================================
// Reading the snapshot
// =============================================================================

/// The dialect named by a snapshot's `dialect` field.
fn snapshot_dialect(json: &Value, path: &Path) -> Result<Dialect, CliError> {
    let Some(name) = json.get("dialect").and_then(Value::as_str) else {
        return Err(CliError::Other(format!(
            "{} has no \"dialect\" field; pass --dialect",
            path.display()
        )));
    };
    match name {
        "sqlite" => Ok(Dialect::Sqlite),
        "turso" => Ok(Dialect::Turso),
        "postgresql" | "postgres" | "pg" => Ok(Dialect::Postgresql),
        "mysql" => Ok(Dialect::Mysql),
        other => Err(CliError::Other(format!(
            "{} is a \"{other}\" snapshot; drizzle-rs supports sqlite, turso, postgresql and mysql",
            path.display()
        ))),
    }
}

/// Checks the version, upgrades the document to the current format, and
/// loads it.
fn load_snapshot(json: Value, dialect: Dialect, path: &Path) -> Result<Snapshot, CliError> {
    let base = dialect.to_base();
    let version = match json.get("version") {
        Some(Value::String(version)) => version.clone(),
        Some(Value::Number(version)) => version.to_string(),
        _ => {
            return Err(CliError::Other(format!(
                "{} has no snapshot \"version\"; is it a drizzle-kit snapshot?",
                path.display()
            )));
        }
    };
    if !is_supported_version(base, &version) {
        use drizzle_migrations::version::{
            MYSQL_MIN_SUPPORTED_VERSION, POSTGRES_MIN_SUPPORTED_VERSION,
            SQLITE_MIN_SUPPORTED_VERSION,
        };
        let min = match base {
            drizzle_types::Dialect::SQLite => SQLITE_MIN_SUPPORTED_VERSION,
            drizzle_types::Dialect::PostgreSQL => POSTGRES_MIN_SUPPORTED_VERSION,
            drizzle_types::Dialect::MySQL => MYSQL_MIN_SUPPORTED_VERSION,
        };
        let latest = snapshot_version(base);
        let hint = if needs_upgrade(base, &version) {
            "Run `npx drizzle-kit up` in the TypeScript project to upgrade its snapshots, then import again."
        } else {
            "It is newer than this drizzle-cli understands; update drizzle-cli."
        };
        return Err(CliError::Other(format!(
            "{} is a {} snapshot at version {version}; drizzle import reads versions {min} to {latest}. {hint}",
            path.display(),
            dialect.as_str(),
        )));
    }

    let upgraded = upgrade_to_latest(json, base);
    let invalid = |e: &dyn Display| {
        CliError::Other(format!(
            "{} did not convert to a {} snapshot: {e}",
            path.display(),
            dialect.as_str()
        ))
    };
    let snapshot = match base {
        drizzle_types::Dialect::SQLite => {
            Snapshot::Sqlite(serde_json::from_value(upgraded).map_err(|e| invalid(&e))?)
        }
        drizzle_types::Dialect::PostgreSQL => {
            Snapshot::Postgres(serde_json::from_value(upgraded).map_err(|e| invalid(&e))?)
        }
        drizzle_types::Dialect::MySQL => {
            let snapshot: drizzle_migrations::mysql::MySQLSnapshot =
                serde_json::from_value(upgraded).map_err(|e| invalid(&e))?;
            drizzle_migrations::mysql::MySQLDDL::try_from_entities(snapshot.ddl.clone())
                .map_err(|e| invalid(&e))?;
            Snapshot::MySQL(snapshot)
        }
    };
    Ok(snapshot)
}

// =============================================================================
// Writing the schema
// =============================================================================

/// `--out`, else the config's single `schema` file, else `src/schema.rs`.
fn resolve_out_path(
    out: Option<&Path>,
    db: Option<&crate::config::DatabaseConfig>,
) -> Result<PathBuf, CliError> {
    if let Some(out) = out {
        return Ok(out.to_path_buf());
    }
    let Some(db) = db else {
        return Ok(PathBuf::from(DEFAULT_SCHEMA_PATH));
    };
    match &db.schema {
        Schema::One(path) if !path.contains(['*', '?', '[', '{']) => Ok(PathBuf::from(path)),
        _ => Err(CliError::Other(format!(
            "The config's schema ({}) is not a single file; pass --out <FILE>",
            db.schema_display()
        ))),
    }
}

fn write_schema(path: &Path, code: &str, force: bool) -> Result<(), CliError> {
    if path.exists() && !force {
        return Err(CliError::Other(format!(
            "{} already exists. Pass --force to overwrite it, or --out <FILE> to write elsewhere.",
            path.display()
        )));
    }
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| {
            CliError::Other(format!(
                "Failed to create output directory '{}': {e}",
                parent.display()
            ))
        })?;
    }
    fs::write(path, code).map_err(|e| {
        CliError::Other(format!(
            "Failed to write schema file '{}': {e}",
            path.display()
        ))
    })
}

fn print_summary(say: Messages, generated: &SchemaCode) {
    say.line("");
    for line in crate::commands::introspect::summary_lines(
        generated.table_count,
        generated.index_count,
        generated.view_count,
        &generated.warnings,
    ) {
        say.line(line);
    }
}

// =============================================================================
// Config and next steps
// =============================================================================

/// Writes a starter config for the imported project. Returns whether it
/// wrote one.
fn init_config(
    say: Messages,
    config: Option<&Config>,
    dialect: Dialect,
    schema_path: Option<&Path>,
    migrations: Option<(&Path, Layout)>,
) -> Result<bool, CliError> {
    let config_path = Path::new(CONFIG_FILE);
    if config.is_some() || config_path.exists() {
        say.line(output::warning(&format!(
            "--init-config: a config already exists, so {CONFIG_FILE} was not written"
        )));
        return Ok(false);
    }
    let schema = schema_path.map_or_else(|| DEFAULT_SCHEMA_PATH.to_string(), forward_slashes);
    let out = migrations.map_or_else(
        || crate::commands::init::DEFAULT_OUT.to_string(),
        |(dir, _)| forward_slashes(dir),
    );
    let contents = crate::commands::init::config_contents(dialect.as_str(), None, &schema, &out)?;
    fs::write(config_path, contents).map_err(|e| CliError::IoError(e.to_string()))?;
    say.line("");
    say.line(output::success(&format!(
        "Created {CONFIG_FILE} (schema = \"{schema}\", out = \"{out}\")"
    )));
    Ok(true)
}

/// `path` with forward slashes (relative paths kept), for the config file
/// and the generated module doc.
fn forward_slashes(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

struct NextSteps<'a> {
    config: Option<&'a Config>,
    db_out: Option<&'a Path>,
    wrote_config: bool,
    dialect: Dialect,
    schema_path: Option<&'a Path>,
    migrations: Option<(&'a Path, Layout)>,
}

fn print_next_steps(say: Messages, steps: &NextSteps<'_>) {
    say.line("");
    say.line(output::heading("Next steps:"));
    let mut n = 0;
    let mut step = |text: String| {
        n += 1;
        say.line(format!("  {n}. {text}"));
    };

    let out_display = steps
        .migrations
        .map_or_else(|| "./drizzle".to_string(), |(dir, _)| forward_slashes(dir));

    if steps.wrote_config {
        step(format!(
            "Fill in [dbCredentials] in {CONFIG_FILE} (it points `out` at {out_display})."
        ));
    } else if steps.config.is_none() {
        let schema = steps
            .schema_path
            .map_or_else(|| DEFAULT_SCHEMA_PATH.to_string(), forward_slashes);
        step(format!(
            "Create {CONFIG_FILE} with the existing migrations folder as `out` (or rerun with --init-config):"
        ));
        say.line(output::muted(&format!(
            "       dialect = \"{}\"",
            steps.dialect.as_str()
        )));
        say.line(output::muted(&format!("       schema = \"{schema}\"")));
        say.line(output::muted(&format!("       out = \"{out_display}\"")));
    } else if let Some((dir, _)) = steps.migrations
        && steps.db_out.is_none_or(|out| !same_dir(out, dir))
    {
        step(format!(
            "Set `out = \"{out_display}\"` in {CONFIG_FILE} so drizzle-rs uses the existing migrations."
        ));
    }

    if let Some((_, Layout::Journal)) = steps.migrations {
        step(format!(
            "Convert the migrations folder to the folder layout drizzle-rs reads (moves the files in place; or rerun with --upgrade):\n       {}",
            output::muted(&format!("drizzle up --out {out_display}"))
        ));
    }

    step(format!(
        "{} sees the migrations drizzle-orm already applied in its tracking table (`__drizzle_migrations`; schema `drizzle` on PostgreSQL) and runs only new ones. If drizzle.config.ts set `migrations.table` or `migrations.schema`, set the same under [migrations].",
        output::heading("drizzle migrate")
    ));
    step(format!(
        "Run {}: for an unchanged schema it reports no changes.",
        output::heading("drizzle generate")
    ));
}

fn same_dir(a: &Path, b: &Path) -> bool {
    match (fs::canonicalize(a), fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}

fn print_manual_port_notes(say: Messages) {
    say.line("");
    say.line(output::heading(
        "Snapshots do not record these; port them by hand:",
    ));
    for note in [
        "relations(): declare foreign keys with #[column(references = Table::column)]; relation accessors come from those",
        "$type<T>() and column modes ({ mode: 'json' | 'timestamp' | 'bigint' }): fields get the column's storage type; switch to your Rust type (e.g. #[column(json)] with a serde type, or a #[derive(...Enum)])",
        "$default / $defaultFn / $onUpdate / $onUpdateFn: values computed in JS; use #[column(default_fn = path)] or set them in code",
        "customType(): the column keeps its SQL type; map it to a Rust type yourself",
    ] {
        say.line(format!("  {} {note}", output::warning("-")));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn dialect_spellings() {
        let path = Path::new("snapshot.json");
        for (name, dialect) in [
            ("sqlite", Dialect::Sqlite),
            ("turso", Dialect::Turso),
            ("postgresql", Dialect::Postgresql),
            ("postgres", Dialect::Postgresql),
            ("mysql", Dialect::Mysql),
        ] {
            assert_eq!(
                snapshot_dialect(&json!({ "dialect": name }), path).unwrap(),
                dialect
            );
        }
        assert!(snapshot_dialect(&json!({ "dialect": "singlestore" }), path).is_err());
        assert!(snapshot_dialect(&json!({}), path).is_err());
    }

    #[test]
    fn old_and_future_versions_are_refused() {
        let path = Path::new("snapshot.json");
        let old = load_snapshot(
            json!({ "version": "4", "dialect": "sqlite" }),
            Dialect::Sqlite,
            path,
        )
        .unwrap_err()
        .to_string();
        assert!(old.contains("version 4"), "{old}");
        assert!(old.contains("drizzle-kit up"), "{old}");

        let future = load_snapshot(
            json!({ "version": "99", "dialect": "postgresql" }),
            Dialect::Postgresql,
            path,
        )
        .unwrap_err()
        .to_string();
        assert!(future.contains("update drizzle-cli"), "{future}");
    }
}
