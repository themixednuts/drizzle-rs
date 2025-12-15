//! Tokio-postgres driver implementation for async PostgreSQL databases
//!
//! This module provides async introspection and push functionality
//! for tokio-postgres connections.

use std::path::PathBuf;

use clap::Parser;

use crate::config::Config;
use crate::config::cli::{CliArgs, CliCommand};
use crate::config::credentials::PostgresCredentials;
use crate::config::error::ConfigError;
use crate::config::markers::{PostgresDialect, TokioPostgresConnection};
use crate::schema::Schema;

impl<S: Schema> Config<S, PostgresDialect, TokioPostgresConnection, PostgresCredentials> {
    /// Run the CLI with command line arguments for tokio-postgres connections.
    ///
    /// This is an async function - the caller must provide an async runtime.
    ///
    /// # Example
    ///
    /// ```ignore
    /// #[tokio::main]
    /// async fn main() {
    ///     TokioPostgresConfigBuilder::new("localhost", 5432, "user", "pass", "mydb")
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
    ///
    /// This reads the journal to find pending migrations, executes them in order,
    /// and tracks which migrations have been applied in the `__drizzle_migrations` table.
    pub async fn migrate(&self) -> Result<(), ConfigError> {
        use crate::journal::Journal;

        println!("🚀 Running migrations on PostgreSQL database...");
        println!(
            "  Host: {}:{}",
            self.credentials.host, self.credentials.port
        );
        println!("  DB:   {}", self.credentials.database);

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
        let (client, connection) = self.connect().await?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        // Create migrations tracking table if it doesn't exist
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS __drizzle_migrations (
                    id SERIAL PRIMARY KEY,
                    hash TEXT NOT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                )",
                &[],
            )
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        // Get already applied migrations
        let rows = client
            .query("SELECT hash FROM __drizzle_migrations", &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let applied: std::collections::HashSet<String> =
            rows.iter().map(|row| row.get(0)).collect();

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
            client
                .batch_execute(&sql)
                .await
                .map_err(|e| ConfigError::ConnectionError(format!("Migration failed: {}", e)))?;

            // Record the migration
            client
                .execute(
                    "INSERT INTO __drizzle_migrations (hash) VALUES ($1)",
                    &[&entry.tag],
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

    /// Connect to the PostgreSQL database and return the client.
    ///
    /// The caller is responsible for spawning the connection task.
    /// Returns a tuple of (client, connection) where the connection should be spawned.
    async fn connect(
        &self,
    ) -> Result<
        (
            tokio_postgres::Client,
            tokio_postgres::Connection<tokio_postgres::Socket, tokio_postgres::tls::NoTlsStream>,
        ),
        ConfigError,
    > {
        let conn_str = self.credentials.connection_string();
        tokio_postgres::connect(&conn_str, tokio_postgres::NoTls)
            .await
            .map_err(|e| {
                // Mask password in connection string for error message
                let masked_conn_str = format!(
                    "host={} port={} user={} password=*** dbname={}",
                    self.credentials.host,
                    self.credentials.port,
                    self.credentials.username,
                    self.credentials.database
                );
                ConfigError::ConnectionError(format!(
                    "Failed to connect to PostgreSQL: {}\n  Connection: {}",
                    e, masked_conn_str
                ))
            })
    }

    /// Introspect the PostgreSQL database via tokio-postgres and generate a snapshot.
    ///
    /// This is an async function - the caller must provide an async runtime.
    pub async fn introspect(&self, output: Option<PathBuf>) -> Result<(), ConfigError> {
        use crate::postgres::ddl::Schema as DbSchema;
        use crate::postgres::introspect::{
            IntrospectionResult, RawColumnInfo, RawEnumInfo, RawSequenceInfo, RawTableInfo,
            RawViewInfo, process_columns, process_enums, process_sequences, process_tables,
            process_views, queries,
        };
        use crate::schema::Snapshot;

        let output_dir = output.unwrap_or_else(|| self.out.clone());
        std::fs::create_dir_all(&output_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("🔍 Introspecting PostgreSQL database via tokio-postgres...");
        println!(
            "  Host:   {}:{}",
            self.credentials.host, self.credentials.port
        );
        println!("  DB:     {}", self.credentials.database);
        println!("  Output: {}", output_dir.display());

        // Connect to the database
        let (client, connection) = self.connect().await?;

        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        let mut result = IntrospectionResult::default();

        // Get schemas
        let schema_rows = client
            .query(queries::SCHEMAS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        for row in &schema_rows {
            let name: String = row.get(0);
            result.schemas.push(DbSchema { name });
        }

        // Get tables
        let table_rows = client
            .query(queries::TABLES_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_tables: Vec<RawTableInfo> = table_rows
            .iter()
            .map(|row| RawTableInfo {
                schema: row.get(0),
                name: row.get(1),
                is_rls_enabled: row.get(2),
            })
            .collect();
        result.tables = process_tables(&raw_tables);

        // Get columns
        let column_rows = client
            .query(queries::COLUMNS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_columns: Vec<RawColumnInfo> = column_rows
            .iter()
            .map(|row| RawColumnInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                column_type: row.get(3),
                type_schema: row.get(4),
                not_null: row.get(5),
                default_value: row.get(6),
                is_identity: row.get(7),
                identity_type: row.get(8),
                is_generated: row.get(9),
                generated_expression: row.get(10),
                ordinal_position: row.get(11),
            })
            .collect();
        result.columns = process_columns(&raw_columns);

        // Get enums
        let enum_rows = client
            .query(queries::ENUMS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_enums: Vec<RawEnumInfo> = enum_rows
            .iter()
            .map(|row| RawEnumInfo {
                schema: row.get(0),
                name: row.get(1),
                values: row.get(2),
            })
            .collect();
        result.enums = process_enums(&raw_enums);

        // Get sequences
        let seq_rows = client
            .query(queries::SEQUENCES_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_seqs: Vec<RawSequenceInfo> = seq_rows
            .iter()
            .map(|row| RawSequenceInfo {
                schema: row.get(0),
                name: row.get(1),
                data_type: row.get(2),
                start_value: row.get(3),
                min_value: row.get(4),
                max_value: row.get(5),
                increment: row.get(6),
                cycle: row.get(7),
                cache_value: row.get(8),
            })
            .collect();
        result.sequences = process_sequences(&raw_seqs);

        // Get views
        let view_rows = client
            .query(queries::VIEWS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_views: Vec<RawViewInfo> = view_rows
            .iter()
            .map(|row| RawViewInfo {
                schema: row.get(0),
                name: row.get(1),
                definition: row.get(2),
                is_materialized: row.get(3),
            })
            .collect();
        result.views = process_views(&raw_views);

        // Convert to snapshot and save
        let snapshot = result.to_snapshot();
        let snapshot_path = output_dir.join("snapshot.json");

        Snapshot::Postgres(snapshot)
            .save(&snapshot_path)
            .map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("✓ Introspection complete!");
        println!("  Schemas:      {}", result.schemas.len());
        println!("  Tables:       {}", result.tables.len());
        println!("  Columns:      {}", result.columns.len());
        println!("  Enums:        {}", result.enums.len());
        println!("  Sequences:    {}", result.sequences.len());
        println!("  Views:        {}", result.views.len());
        println!("  Snapshot:     {}", snapshot_path.display());

        Ok(())
    }

    /// Push schema changes directly to the database without creating migration files.
    ///
    /// This is an async function - the caller must provide an async runtime.
    pub async fn push(&self) -> Result<(), ConfigError> {
        use crate::postgres::ddl::Schema as DbSchema;
        use crate::postgres::introspect::{
            IntrospectionResult, RawColumnInfo, RawEnumInfo, RawSequenceInfo, RawTableInfo,
            RawViewInfo, process_columns, process_enums, process_sequences, process_tables,
            process_views, queries,
        };
        use crate::schema::Snapshot;

        println!("🚀 Pushing schema changes to PostgreSQL database via tokio-postgres...");
        println!(
            "  Host: {}:{}",
            self.credentials.host, self.credentials.port
        );
        println!("  DB:   {}", self.credentials.database);

        // Connect to the database
        let (client, connection) = self.connect().await?;

        // Spawn the connection task
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("Connection error: {}", e);
            }
        });

        let mut result = IntrospectionResult::default();

        // Get schemas
        let schema_rows = client
            .query(queries::SCHEMAS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        for row in &schema_rows {
            let name: String = row.get(0);
            result.schemas.push(DbSchema { name });
        }

        // Get tables
        let table_rows = client
            .query(queries::TABLES_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_tables: Vec<RawTableInfo> = table_rows
            .iter()
            .map(|row| RawTableInfo {
                schema: row.get(0),
                name: row.get(1),
                is_rls_enabled: row.get(2),
            })
            .collect();
        result.tables = process_tables(&raw_tables);

        // Get columns
        let column_rows = client
            .query(queries::COLUMNS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_columns: Vec<RawColumnInfo> = column_rows
            .iter()
            .map(|row| RawColumnInfo {
                schema: row.get(0),
                table: row.get(1),
                name: row.get(2),
                column_type: row.get(3),
                type_schema: row.get(4),
                not_null: row.get(5),
                default_value: row.get(6),
                is_identity: row.get(7),
                identity_type: row.get(8),
                is_generated: row.get(9),
                generated_expression: row.get(10),
                ordinal_position: row.get(11),
            })
            .collect();
        result.columns = process_columns(&raw_columns);

        // Get enums
        let enum_rows = client
            .query(queries::ENUMS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_enums: Vec<RawEnumInfo> = enum_rows
            .iter()
            .map(|row| RawEnumInfo {
                schema: row.get(0),
                name: row.get(1),
                values: row.get(2),
            })
            .collect();
        result.enums = process_enums(&raw_enums);

        // Get sequences
        let seq_rows = client
            .query(queries::SEQUENCES_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_seqs: Vec<RawSequenceInfo> = seq_rows
            .iter()
            .map(|row| RawSequenceInfo {
                schema: row.get(0),
                name: row.get(1),
                data_type: row.get(2),
                start_value: row.get(3),
                min_value: row.get(4),
                max_value: row.get(5),
                increment: row.get(6),
                cycle: row.get(7),
                cache_value: row.get(8),
            })
            .collect();
        result.sequences = process_sequences(&raw_seqs);

        // Get views
        let view_rows = client
            .query(queries::VIEWS_QUERY, &[])
            .await
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_views: Vec<RawViewInfo> = view_rows
            .iter()
            .map(|row| RawViewInfo {
                schema: row.get(0),
                name: row.get(1),
                definition: row.get(2),
                is_materialized: row.get(3),
            })
            .collect();
        result.views = process_views(&raw_views);

        // Convert introspection to snapshot (database state)
        let db_snapshot = result.to_snapshot();

        // Get desired schema snapshot (code state)
        let code_snapshot = self.to_snapshot();

        // Generate diff (database -> code = what changes need to be made)
        let sql_statements = match (&Snapshot::Postgres(db_snapshot), &code_snapshot) {
            (Snapshot::Postgres(prev_snap), Snapshot::Postgres(curr_snap)) => {
                use crate::postgres::{diff_snapshots, statements::PostgresGenerator};

                let diff = diff_snapshots(&prev_snap.ddl, &curr_snap.ddl);
                if !diff.has_changes() {
                    println!("No schema changes detected 😴");
                    return Ok(());
                }

                let generator = PostgresGenerator::new().with_breakpoints(false);
                generator.generate(&diff.diffs)
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
            client
                .batch_execute(stmt)
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
