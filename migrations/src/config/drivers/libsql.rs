//! LibSQL driver implementation for SQLite databases with embedded replica
//!
//! This module provides async introspection and push functionality
//! for libsql connections (local or with sync).

use std::path::PathBuf;

use clap::Parser;

use crate::config::Config;
use crate::config::cli::{CliArgs, CliCommand};
use crate::config::credentials::LibsqlCredentials;
use crate::config::error::ConfigError;
use crate::config::markers::{LibsqlConnection, SqliteDialect};
use crate::schema::Schema;

impl<S: Schema> Config<S, SqliteDialect, LibsqlConnection, LibsqlCredentials> {
    /// Run the CLI with command line arguments for libsql connections.
    ///
    /// This is an async function - the caller must provide an async runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[tokio::main]
    /// async fn main() {
    ///     LibsqlConfigBuilder::new("./local.db")
    ///         .schema::<AppSchema>()
    ///         .build()
    ///         .run_cli()
    ///         .await;
    /// }
    /// ```
    pub async fn run_cli(self) {
        let args = CliArgs::parse();

        let result = match args.command {
            CliCommand::Generate { name, custom } => self.cmd_generate(name, custom),
            CliCommand::Migrate => self.migrate().await,
            CliCommand::Status => self.cmd_status(),
            CliCommand::Push => self.push().await,
            CliCommand::Introspect { output } => self.introspect(output).await,
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    /// Run pending migrations from the migrations folder.
    pub async fn migrate(&self) -> Result<(), ConfigError> {
        use crate::journal::Journal;

        println!("🚀 Running migrations on LibSQL database...");
        println!("  Path: {}", self.credentials.path);

        let journal_path = self.journal_path();
        if !journal_path.exists() {
            println!("No migrations found.");
            return Ok(());
        }

        let journal =
            Journal::load(&journal_path).map_err(|e| ConfigError::IoError(e.to_string()))?;

        if journal.entries.is_empty() {
            println!("No migrations found.");
            return Ok(());
        }

        // Connect to the database
        let db = libsql::Builder::new_local(&self.credentials.path)
            .build()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let conn = db
            .connect()
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        // Create migrations tracking table if it doesn't exist
        conn.execute(
            "CREATE TABLE IF NOT EXISTS __drizzle_migrations (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                hash TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s', 'now'))
            )",
            (),
        )
        .await
        .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        // Get already applied migrations
        let mut rows = conn
            .query("SELECT hash FROM __drizzle_migrations", ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let mut applied = std::collections::HashSet::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            let hash: String = row
                .get(0)
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
            applied.insert(hash);
        }

        let mut applied_count = 0;
        for entry in &journal.entries {
            if applied.contains(&entry.tag) {
                continue;
            }

            let migration_path = self.migrations_dir().join(&entry.tag).join("migration.sql");
            if !migration_path.exists() {
                return Err(ConfigError::IoError(format!(
                    "Migration file not found: {}",
                    migration_path.display()
                )));
            }

            let sql = std::fs::read_to_string(&migration_path)
                .map_err(|e| ConfigError::IoError(e.to_string()))?;

            println!("  Applying: {}", entry.tag);

            // Execute the migration
            conn.execute_batch(&sql)
                .await
                .map_err(|e| ConfigError::ConnectionError(format!("Migration failed: {}", e)))?;

            // Record the migration
            conn.execute(
                "INSERT INTO __drizzle_migrations (hash) VALUES (?)",
                libsql::params![entry.tag.clone()],
            )
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

            applied_count += 1;
        }

        if applied_count == 0 {
            println!("✓ All migrations already applied.");
        } else {
            println!("✓ Applied {} migration(s).", applied_count);
        }

        Ok(())
    }

    /// Introspect the SQLite database via libsql and generate a snapshot.
    ///
    /// This is an async function - the caller must provide an async runtime.
    pub async fn introspect(&self, output: Option<PathBuf>) -> Result<(), ConfigError> {
        use crate::schema::Snapshot;
        use crate::sqlite::ddl::{Table, extract_generated_columns};
        use crate::sqlite::introspect::{
            IntrospectionResult, RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo,
            process_columns, process_foreign_keys, process_indexes, queries,
        };

        let output_dir = output.unwrap_or_else(|| self.out.clone());
        std::fs::create_dir_all(&output_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("🔍 Introspecting SQLite database via libsql...");
        println!("  Path:   {}", self.credentials.path);
        println!("  Output: {}", output_dir.display());

        // Connect to libsql
        let db = if let (Some(sync_url), Some(auth_token)) =
            (&self.credentials.sync_url, &self.credentials.auth_token)
        {
            libsql::Builder::new_remote_replica(
                &self.credentials.path,
                sync_url.clone(),
                auth_token.clone(),
            )
            .build()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        } else {
            libsql::Builder::new_local(&self.credentials.path)
                .build()
                .await
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        };

        let conn = db
            .connect()
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut result = IntrospectionResult::default();

        // Get tables
        let mut rows = conn
            .query(queries::TABLES_QUERY, ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut table_sql_map = std::collections::HashMap::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            let name: String = row
                .get(0)
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
            let sql: Option<String> = row.get(1).ok();
            if let Some(ref sql) = sql {
                table_sql_map.insert(name.clone(), sql.clone());
            }
            result.tables.push(Table::new(name));
        }

        // Get columns
        let mut column_rows = conn
            .query(queries::COLUMNS_QUERY, ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut raw_columns = Vec::new();
        while let Some(row) = column_rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            raw_columns.push(RawColumnInfo {
                table: row.get(0).unwrap_or_default(),
                name: row.get(1).unwrap_or_default(),
                column_type: row.get(2).unwrap_or_default(),
                not_null: row.get(3).unwrap_or(false),
                default_value: row.get(4).ok(),
                pk: row.get(5).unwrap_or(0),
                hidden: row.get(6).unwrap_or(0),
                sql: row.get(7).ok(),
            });
        }

        // Parse generated columns
        let mut generated_columns = std::collections::HashMap::new();
        for (table_name, sql) in &table_sql_map {
            let table_gens = extract_generated_columns(sql);
            for (col_name, generated_col) in table_gens {
                let key = format!("{}:{}", table_name, col_name);
                generated_columns.insert(key, generated_col);
            }
        }

        let pk_columns = std::collections::HashSet::new();
        let (columns, primary_keys) =
            process_columns(&raw_columns, &generated_columns, &pk_columns);
        result.columns = columns;
        result.primary_keys = primary_keys;

        // Get indexes per table
        for table in &result.tables {
            let index_query = queries::indexes_query(&table.name);
            if let Ok(mut stmt) = conn.query(&index_query, ()).await {
                let mut raw_indexes = Vec::new();
                while let Ok(Some(row)) = stmt.next().await {
                    raw_indexes.push(RawIndexInfo {
                        table: table.name.clone(),
                        name: row.get(1).unwrap_or_default(),
                        unique: row.get(2).unwrap_or(false),
                        origin: row.get(3).unwrap_or_default(),
                        partial: row.get(4).unwrap_or(false),
                    });
                }

                let mut index_columns = Vec::new();
                for idx in &raw_indexes {
                    let info_query = queries::index_info_query(&idx.name);
                    if let Ok(mut cols_stmt) = conn.query(&info_query, ()).await {
                        while let Ok(Some(row)) = cols_stmt.next().await {
                            index_columns.push(RawIndexColumn {
                                index_name: idx.name.clone(),
                                seqno: row.get(0).unwrap_or(0),
                                cid: row.get(1).unwrap_or(0),
                                name: row.get(2).ok(),
                                desc: row.get(3).unwrap_or(false),
                                coll: row.get(4).unwrap_or_default(),
                                key: row.get(5).unwrap_or(false),
                            });
                        }
                    }
                }

                let indexes = process_indexes(&raw_indexes, &index_columns, &table_sql_map);
                result.indexes.extend(indexes);
            }

            // Foreign keys
            let fk_query = queries::foreign_keys_query(&table.name);
            if let Ok(mut fk_stmt) = conn.query(&fk_query, ()).await {
                let mut raw_fks = Vec::new();
                while let Ok(Some(row)) = fk_stmt.next().await {
                    raw_fks.push(RawForeignKey {
                        table: table.name.clone(),
                        id: row.get(0).unwrap_or(0),
                        seq: row.get(1).unwrap_or(0),
                        to_table: row.get(2).unwrap_or_default(),
                        from_column: row.get(3).unwrap_or_default(),
                        to_column: row.get(4).unwrap_or_default(),
                        on_update: row.get(5).unwrap_or_default(),
                        on_delete: row.get(6).unwrap_or_default(),
                        r#match: row.get(7).unwrap_or_default(),
                    });
                }
                let fks = process_foreign_keys(&raw_fks);
                result.foreign_keys.extend(fks);
            }
        }

        // Convert to snapshot and save
        let snapshot = result.to_snapshot();
        let snapshot_path = output_dir.join("snapshot.json");

        Snapshot::Sqlite(snapshot)
            .save(&snapshot_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("✓ Introspection complete!");
        println!("  Tables:       {}", result.tables.len());
        println!("  Columns:      {}", result.columns.len());
        println!("  Indexes:      {}", result.indexes.len());
        println!("  Foreign keys: {}", result.foreign_keys.len());
        println!("  Snapshot:     {}", snapshot_path.display());

        Ok(())
    }

    /// Push schema changes directly to the database without creating migration files.
    ///
    /// This is an async function - the caller must provide an async runtime.
    pub async fn push(&self) -> Result<(), ConfigError> {
        use crate::schema::Snapshot;
        use crate::sqlite::ddl::{Table, View, extract_generated_columns};
        use crate::sqlite::introspect::{
            IntrospectionResult, RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo,
            parse_view_sql, process_columns, process_foreign_keys, process_indexes, queries,
        };

        println!("🚀 Pushing schema changes to SQLite database via libsql...");
        println!("  Path: {}", self.credentials.path);

        // Connect to libsql
        let db = if let (Some(sync_url), Some(auth_token)) =
            (&self.credentials.sync_url, &self.credentials.auth_token)
        {
            libsql::Builder::new_remote_replica(
                &self.credentials.path,
                sync_url.clone(),
                auth_token.clone(),
            )
            .build()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        } else {
            libsql::Builder::new_local(&self.credentials.path)
                .build()
                .await
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        };

        let conn = db
            .connect()
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut result = IntrospectionResult::default();

        // Get tables
        let mut rows = conn
            .query(queries::TABLES_QUERY, ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut table_sql_map = std::collections::HashMap::new();
        while let Some(row) = rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            let name: String = row
                .get(0)
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
            let sql: Option<String> = row.get(1).ok();
            if let Some(ref sql) = sql {
                table_sql_map.insert(name.clone(), sql.clone());
            }
            result.tables.push(Table::new(name));
        }

        // Get columns
        let mut column_rows = conn
            .query(queries::COLUMNS_QUERY, ())
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut raw_columns = Vec::new();
        while let Some(row) = column_rows
            .next()
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
        {
            raw_columns.push(RawColumnInfo {
                table: row.get(0).unwrap_or_default(),
                name: row.get(1).unwrap_or_default(),
                column_type: row.get(2).unwrap_or_default(),
                not_null: row.get(3).unwrap_or(false),
                default_value: row.get(4).ok(),
                pk: row.get(5).unwrap_or(0),
                hidden: row.get(6).unwrap_or(0),
                sql: row.get(7).ok(),
            });
        }

        // Parse generated columns
        let mut generated_columns = std::collections::HashMap::new();
        for (table_name, sql) in &table_sql_map {
            let table_gens = extract_generated_columns(sql);
            for (col_name, generated_col) in table_gens {
                let key = format!("{}:{}", table_name, col_name);
                generated_columns.insert(key, generated_col);
            }
        }

        let pk_columns = std::collections::HashSet::new();
        let (columns, primary_keys) =
            process_columns(&raw_columns, &generated_columns, &pk_columns);
        result.columns = columns;
        result.primary_keys = primary_keys;

        // Get indexes per table
        for table in &result.tables {
            let index_query = queries::indexes_query(&table.name);
            if let Ok(mut stmt) = conn.query(&index_query, ()).await {
                let mut raw_indexes = Vec::new();
                while let Ok(Some(row)) = stmt.next().await {
                    raw_indexes.push(RawIndexInfo {
                        table: table.name.clone(),
                        name: row.get(1).unwrap_or_default(),
                        unique: row.get(2).unwrap_or(false),
                        origin: row.get(3).unwrap_or_default(),
                        partial: row.get(4).unwrap_or(false),
                    });
                }

                let mut index_columns = Vec::new();
                for idx in &raw_indexes {
                    let info_query = queries::index_info_query(&idx.name);
                    if let Ok(mut cols_stmt) = conn.query(&info_query, ()).await {
                        while let Ok(Some(row)) = cols_stmt.next().await {
                            index_columns.push(RawIndexColumn {
                                index_name: idx.name.clone(),
                                seqno: row.get(0).unwrap_or(0),
                                cid: row.get(1).unwrap_or(0),
                                name: row.get(2).ok(),
                                desc: row.get(3).unwrap_or(false),
                                coll: row.get(4).unwrap_or_default(),
                                key: row.get(5).unwrap_or(false),
                            });
                        }
                    }
                }

                let indexes = process_indexes(&raw_indexes, &index_columns, &table_sql_map);
                result.indexes.extend(indexes);
            }

            // Foreign keys
            let fk_query = queries::foreign_keys_query(&table.name);
            if let Ok(mut fk_stmt) = conn.query(&fk_query, ()).await {
                let mut raw_fks = Vec::new();
                while let Ok(Some(row)) = fk_stmt.next().await {
                    raw_fks.push(RawForeignKey {
                        table: table.name.clone(),
                        id: row.get(0).unwrap_or(0),
                        seq: row.get(1).unwrap_or(0),
                        to_table: row.get(2).unwrap_or_default(),
                        from_column: row.get(3).unwrap_or_default(),
                        to_column: row.get(4).unwrap_or_default(),
                        on_update: row.get(5).unwrap_or_default(),
                        on_delete: row.get(6).unwrap_or_default(),
                        r#match: row.get(7).unwrap_or_default(),
                    });
                }
                let fks = process_foreign_keys(&raw_fks);
                result.foreign_keys.extend(fks);
            }
        }

        // Get views
        if let Ok(mut view_rows) = conn.query(queries::VIEWS_QUERY, ()).await {
            let mut views = Vec::new();
            while let Ok(Some(row)) = view_rows.next().await {
                let name: String = row.get(0).unwrap_or_default();
                let sql: Option<String> = row.get(1).ok();
                let definition = sql.and_then(|s| parse_view_sql(&s));
                views.push(View {
                    name,
                    definition,
                    is_existing: false,
                });
            }
            result.views = views;
        }

        // Convert introspection to snapshot (database state)
        let db_snapshot = result.to_snapshot();

        // Get desired schema snapshot (code state)
        let code_snapshot = self.to_snapshot();

        // Generate diff (database -> code = what changes need to be made)
        let sql_statements = match (&Snapshot::Sqlite(db_snapshot), &code_snapshot) {
            (Snapshot::Sqlite(prev_snap), Snapshot::Sqlite(curr_snap)) => {
                use crate::sqlite::{diff_snapshots, statements::SqliteGenerator};

                let diff = diff_snapshots(prev_snap, curr_snap);
                if !diff.has_changes() {
                    println!("No schema changes detected 😴");
                    return Ok(());
                }

                let generator = SqliteGenerator::new().with_breakpoints(false);
                generator.generate_migration(&diff)
            }
            _ => {
                return Err(ConfigError::GenerationError(
                    "Mismatched snapshot dialects".into(),
                ));
            }
        };

        if sql_statements.is_empty() {
            println!("No schema changes detected 😴");
            return Ok(());
        }

        println!("\n📋 Changes to apply:");
        for stmt in &sql_statements {
            println!("  {}", stmt.lines().next().unwrap_or(stmt));
        }
        println!();

        // Execute each statement
        for stmt in &sql_statements {
            conn.execute(stmt, ())
                .await
                .map_err(|e| ConfigError::ConnectionError(format!("Failed to execute: {}", e)))?;
        }

        println!(
            "✓ Schema pushed successfully! ({} statements)",
            sql_statements.len()
        );

        Ok(())
    }

    /// Generate a new migration (async-compatible wrapper).
    ///
    /// This calls the sync implementation since it only involves file I/O.
    pub async fn generate(&self, name: Option<String>, custom: bool) -> Result<(), ConfigError> {
        self.cmd_generate(name, custom)
    }

    /// Show migration status (async-compatible wrapper).
    ///
    /// This calls the sync implementation since it only involves file I/O.
    pub async fn status(&self) -> Result<(), ConfigError> {
        self.cmd_status()
    }
}
