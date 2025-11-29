//! Migration embedding macro implementation

use proc_macro2::TokenStream;
use quote::quote;
use std::path::PathBuf;
use syn::{parse::Parse, parse::ParseStream, LitStr, Token};

/// Input for include_migrations! macro
pub struct IncludeMigrationsInput {
    /// Path to drizzle directory (e.g., "./drizzle")
    pub path: LitStr,
    /// Optional dialect override
    pub dialect: Option<Dialect>,
}

#[derive(Clone, Copy)]
pub enum Dialect {
    Sqlite,
    Postgresql,
    Mysql,
}

impl Parse for IncludeMigrationsInput {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: LitStr = input.parse()?;

        // Check for optional dialect argument
        let dialect = if input.peek(Token![,]) {
            let _: Token![,] = input.parse()?;
            let dialect_str: LitStr = input.parse()?;
            match dialect_str.value().to_lowercase().as_str() {
                "sqlite" => Some(Dialect::Sqlite),
                "postgresql" | "postgres" => Some(Dialect::Postgresql),
                "mysql" => Some(Dialect::Mysql),
                other => {
                    return Err(syn::Error::new(
                        dialect_str.span(),
                        format!("Unknown dialect '{}'. Expected 'sqlite', 'postgresql', or 'mysql'", other),
                    ));
                }
            }
        } else {
            None
        };

        Ok(Self { path, dialect })
    }
}

/// Journal entry from _journal.json
#[derive(serde::Deserialize)]
struct JournalEntry {
    idx: u32,
    #[serde(default)]
    when: i64,
    tag: String,
    #[serde(default)]
    breakpoints: bool,
}

/// Journal file structure
#[derive(serde::Deserialize)]
struct Journal {
    #[serde(default)]
    version: String,
    dialect: String,
    entries: Vec<JournalEntry>,
}

/// Drizzle config structure (minimal for dialect detection)
#[derive(serde::Deserialize)]
struct DrizzleConfig {
    dialect: String,
    #[serde(default = "default_out")]
    out: PathBuf,
}

fn default_out() -> PathBuf {
    PathBuf::from("./drizzle")
}

pub fn include_migrations_impl(input: IncludeMigrationsInput) -> Result<TokenStream, syn::Error> {
    let path_str = input.path.value();
    let span = input.path.span();

    // Get the directory where Cargo.toml is located (CARGO_MANIFEST_DIR)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        syn::Error::new(span, "CARGO_MANIFEST_DIR not set - are you running from cargo?")
    })?;

    let base_path = PathBuf::from(&manifest_dir).join(&path_str);

    // Try to detect dialect from config or journal
    let (migrations_dir, dialect) = if base_path.join("drizzle.toml").exists() {
        // Config file exists, parse it
        let config_path = base_path.join("drizzle.toml");
        let config_content = std::fs::read_to_string(&config_path).map_err(|e| {
            syn::Error::new(span, format!("Failed to read {}: {}", config_path.display(), e))
        })?;

        let config: DrizzleConfig = toml::from_str(&config_content).map_err(|e| {
            syn::Error::new(span, format!("Failed to parse {}: {}", config_path.display(), e))
        })?;

        let dialect = input.dialect.unwrap_or_else(|| match config.dialect.as_str() {
            "postgresql" => Dialect::Postgresql,
            "mysql" => Dialect::Mysql,
            _ => Dialect::Sqlite,
        });

        (base_path.join(&config.out).join("migrations"), dialect)
    } else if base_path.join("migrations").exists() {
        // Direct migrations directory
        let journal_path = base_path.join("migrations").join("meta").join("_journal.json");
        let dialect = if journal_path.exists() {
            let journal_content = std::fs::read_to_string(&journal_path).map_err(|e| {
                syn::Error::new(span, format!("Failed to read {}: {}", journal_path.display(), e))
            })?;
            let journal: Journal = serde_json::from_str(&journal_content).map_err(|e| {
                syn::Error::new(span, format!("Failed to parse {}: {}", journal_path.display(), e))
            })?;
            input.dialect.unwrap_or_else(|| match journal.dialect.as_str() {
                "postgresql" => Dialect::Postgresql,
                "mysql" => Dialect::Mysql,
                _ => Dialect::Sqlite,
            })
        } else {
            input.dialect.unwrap_or(Dialect::Sqlite)
        };
        (base_path.join("migrations"), dialect)
    } else {
        // Assume the path IS the migrations directory
        (base_path.clone(), input.dialect.unwrap_or(Dialect::Sqlite))
    };

    // Read journal
    let journal_path = migrations_dir.join("meta").join("_journal.json");
    if !journal_path.exists() {
        return Err(syn::Error::new(
            span,
            format!(
                "No migrations found. Expected journal at: {}\n\
                 Make sure you have generated migrations first.",
                journal_path.display()
            ),
        ));
    }

    let journal_content = std::fs::read_to_string(&journal_path).map_err(|e| {
        syn::Error::new(span, format!("Failed to read {}: {}", journal_path.display(), e))
    })?;

    let journal: Journal = serde_json::from_str(&journal_content).map_err(|e| {
        syn::Error::new(span, format!("Failed to parse {}: {}", journal_path.display(), e))
    })?;

    // Validate all migration files exist and generate entries
    let mut migration_entries = Vec::new();

    for entry in &journal.entries {
        let sql_filename = format!("{}.sql", entry.tag);
        let sql_path = migrations_dir.join(&sql_filename);

        if !sql_path.exists() {
            return Err(syn::Error::new(
                span,
                format!(
                    "Migration file not found: {}\n\
                     The journal references this migration but the file is missing.",
                    sql_path.display()
                ),
            ));
        }

        // Create a relative path from manifest dir for include_str!
        let relative_path = sql_path
            .strip_prefix(&manifest_dir)
            .unwrap_or(&sql_path)
            .to_string_lossy()
            .replace('\\', "/"); // Normalize for include_str!

        let tag = &entry.tag;
        let idx = entry.idx;

        migration_entries.push(quote! {
            ::drizzle_migrations::EmbeddedMigration::new(
                #tag,
                include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #relative_path)),
                #idx,
            )
        });
    }

    // Generate dialect token
    let dialect_token = match dialect {
        Dialect::Sqlite => quote! { ::drizzle_migrations::Dialect::Sqlite },
        Dialect::Postgresql => quote! { ::drizzle_migrations::Dialect::Postgresql },
        Dialect::Mysql => quote! { ::drizzle_migrations::Dialect::Mysql },
    };

    // Generate rerun-if-changed directives for build script
    // (These are printed but don't affect proc macros directly - users should
    // add them to build.rs if they want rebuild on migration changes)

    // Generate the final output
    let num_migrations = migration_entries.len();
    let output = quote! {
        {
            // Static array of embedded migrations
            static ENTRIES: &[::drizzle_migrations::EmbeddedMigration] = &[
                #(#migration_entries),*
            ];

            // Compile-time assertion that we have migrations
            const _: () = {
                if ENTRIES.len() == 0 {
                    // This is fine - empty migrations is valid
                }
            };

            ::drizzle_migrations::EmbeddedMigrations::new(ENTRIES, #dialect_token)
        }
    };

    // Print helpful info about what was embedded
    if std::env::var("DRIZZLE_DEBUG").is_ok() {
        eprintln!(
            "[drizzle] Embedded {} migrations from {}",
            num_migrations,
            migrations_dir.display()
        );
    }

    Ok(output)
}

