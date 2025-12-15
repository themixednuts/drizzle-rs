//! Rusqlite driver implementation for SQLite databases
//!
//! This module provides CLI commands, introspection, and push functionality
//! for rusqlite (file-based SQLite) connections.

use std::path::PathBuf;

use clap::Parser;

use crate::config::Config;
use crate::config::cli::{CliArgs, CliCommand};
use crate::config::credentials::SqliteCredentials;
use crate::config::error::ConfigError;
use crate::config::markers::{RusqliteConnection, SqliteDialect};
use crate::schema::Schema;

impl<S: Schema> Config<S, SqliteDialect, RusqliteConnection, SqliteCredentials> {
    /// Run the CLI with command line arguments for rusqlite connections.
    pub fn run_cli(self) {
        let args = CliArgs::parse();

        let result = match args.command {
            CliCommand::Generate { name, custom } => self.cmd_generate(name, custom),
            CliCommand::Status => self.cmd_status(),
            CliCommand::Push => self.cmd_push(),
            CliCommand::Introspect { output } => self.cmd_introspect(output),
        };

        if let Err(e) = result {
            eprintln!("Error: {}", e);
            std::process::exit(1);
        }
    }

    /// Introspect the SQLite database and generate a snapshot
    fn cmd_introspect(&self, output: Option<PathBuf>) -> Result<(), ConfigError> {
        use crate::schema::Snapshot;
        use crate::sqlite::ddl::{Table, View, extract_generated_columns};
        use crate::sqlite::introspect::{
            IntrospectionResult, RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo,
            process_columns, process_foreign_keys, process_indexes, queries,
        };

        let output_dir = output.unwrap_or_else(|| self.out.clone());

        // Ensure output directory exists
        std::fs::create_dir_all(&output_dir).map_err(|e| ConfigError::IoError(e.to_string()))?;

        println!("🔍 Introspecting SQLite database...");
        println!("  Path:   {}", self.credentials.path);
        println!("  Output: {}", output_dir.display());

        // Connect to the database
        let conn = rusqlite::Connection::open(&self.credentials.path)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        let mut result = IntrospectionResult::default();

        // Get tables
        let mut stmt = conn
            .prepare(queries::TABLES_QUERY)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let tables: Vec<(String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let mut table_sql_map = std::collections::HashMap::new();
        for (name, sql) in &tables {
            if let Some(sql) = sql {
                table_sql_map.insert(name.clone(), sql.clone());
            }
            result.tables.push(Table::new(name.clone()));
        }

        // Get columns
        let mut stmt = conn
            .prepare(queries::COLUMNS_QUERY)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_columns: Vec<RawColumnInfo> = stmt
            .query_map([], |row| {
                Ok(RawColumnInfo {
                    table: row.get(0)?,
                    name: row.get(1)?,
                    column_type: row.get(2)?,
                    not_null: row.get(3)?,
                    default_value: row.get(4)?,
                    pk: row.get(5)?,
                    hidden: row.get(6)?,
                    sql: row.get(7)?,
                })
            })
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        // Parse generated columns from table SQL
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

        // Get indexes and foreign keys per table
        for table in &result.tables {
            // Indexes
            let index_query = queries::indexes_query(&table.name);
            if let Ok(mut stmt) = conn.prepare(&index_query) {
                let raw_indexes: Vec<RawIndexInfo> = stmt
                    .query_map([], |row| {
                        Ok(RawIndexInfo {
                            table: table.name.clone(),
                            name: row.get(1)?,
                            unique: row.get(2)?,
                            origin: row.get(3)?,
                            partial: row.get(4)?,
                        })
                    })
                    .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect();

                // Get index columns
                let mut index_columns = Vec::new();
                for idx in &raw_indexes {
                    let info_query = queries::index_info_query(&idx.name);
                    if let Ok(mut stmt) = conn.prepare(&info_query) {
                        let cols: Vec<RawIndexColumn> = stmt
                            .query_map([], |row| {
                                Ok(RawIndexColumn {
                                    index_name: idx.name.clone(),
                                    seqno: row.get(0)?,
                                    cid: row.get(1)?,
                                    name: row.get(2)?,
                                    desc: row.get(3)?,
                                    coll: row.get(4)?,
                                    key: row.get(5)?,
                                })
                            })
                            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                            .filter_map(|r| r.ok())
                            .collect();
                        index_columns.extend(cols);
                    }
                }

                let indexes = process_indexes(&raw_indexes, &index_columns, &table_sql_map);
                result.indexes.extend(indexes);
            }

            // Foreign keys
            let fk_query = queries::foreign_keys_query(&table.name);
            if let Ok(mut stmt) = conn.prepare(&fk_query) {
                let raw_fks: Vec<RawForeignKey> = stmt
                    .query_map([], |row| {
                        Ok(RawForeignKey {
                            table: table.name.clone(),
                            id: row.get(0)?,
                            seq: row.get(1)?,
                            to_table: row.get(2)?,
                            from_column: row.get(3)?,
                            to_column: row.get(4)?,
                            on_update: row.get(5)?,
                            on_delete: row.get(6)?,
                            r#match: row.get(7)?,
                        })
                    })
                    .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect();

                let fks = process_foreign_keys(&raw_fks);
                result.foreign_keys.extend(fks);
            }
        }

        // Get views
        if let Ok(mut stmt) = conn.prepare(queries::VIEWS_QUERY) {
            let views: Vec<View> = stmt
                .query_map([], |row| {
                    let name: String = row.get(0)?;
                    let sql: Option<String> = row.get(1)?;
                    let definition =
                        sql.and_then(|s| crate::sqlite::introspect::parse_view_sql(&s));
                    Ok(View {
                        name,
                        definition,
                        is_existing: false,
                    })
                })
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
            result.views = views;
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
        println!("  Views:        {}", result.views.len());
        println!("  Snapshot:     {}", snapshot_path.display());

        Ok(())
    }

    /// Push schema changes directly to the database without creating migration files
    fn cmd_push(&self) -> Result<(), ConfigError> {
        use crate::schema::Snapshot;
        use crate::sqlite::ddl::{Table, View, extract_generated_columns};
        use crate::sqlite::introspect::{
            IntrospectionResult, RawColumnInfo, RawForeignKey, RawIndexColumn, RawIndexInfo,
            parse_view_sql, process_columns, process_foreign_keys, process_indexes, queries,
        };

        println!("🚀 Pushing schema changes to SQLite database...");
        println!("  Path: {}", self.credentials.path);

        // Connect to the database
        let conn = rusqlite::Connection::open(&self.credentials.path)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;

        // Introspect current database state
        let mut result = IntrospectionResult::default();

        // Get tables
        let mut stmt = conn
            .prepare(queries::TABLES_QUERY)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let tables: Vec<(String, Option<String>)> = stmt
            .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        let mut table_sql_map = std::collections::HashMap::new();
        for (name, sql) in &tables {
            if let Some(sql) = sql {
                table_sql_map.insert(name.clone(), sql.clone());
            }
            result.tables.push(Table::new(name.clone()));
        }

        // Get columns
        let mut stmt = conn
            .prepare(queries::COLUMNS_QUERY)
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?;
        let raw_columns: Vec<RawColumnInfo> = stmt
            .query_map([], |row| {
                Ok(RawColumnInfo {
                    table: row.get(0)?,
                    name: row.get(1)?,
                    column_type: row.get(2)?,
                    not_null: row.get(3)?,
                    default_value: row.get(4)?,
                    pk: row.get(5)?,
                    hidden: row.get(6)?,
                    sql: row.get(7)?,
                })
            })
            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
            .filter_map(|r| r.ok())
            .collect();

        // Parse generated columns from table SQL
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

        // Get indexes and foreign keys per table
        for table in &result.tables {
            // Indexes
            let index_query = queries::indexes_query(&table.name);
            if let Ok(mut stmt) = conn.prepare(&index_query) {
                let raw_indexes: Vec<RawIndexInfo> = stmt
                    .query_map([], |row| {
                        Ok(RawIndexInfo {
                            table: table.name.clone(),
                            name: row.get(1)?,
                            unique: row.get(2)?,
                            origin: row.get(3)?,
                            partial: row.get(4)?,
                        })
                    })
                    .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect();

                // Get index columns
                let mut index_columns = Vec::new();
                for idx in &raw_indexes {
                    let info_query = queries::index_info_query(&idx.name);
                    if let Ok(mut stmt) = conn.prepare(&info_query) {
                        let cols: Vec<RawIndexColumn> = stmt
                            .query_map([], |row| {
                                Ok(RawIndexColumn {
                                    index_name: idx.name.clone(),
                                    seqno: row.get(0)?,
                                    cid: row.get(1)?,
                                    name: row.get(2)?,
                                    desc: row.get(3)?,
                                    coll: row.get(4)?,
                                    key: row.get(5)?,
                                })
                            })
                            .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                            .filter_map(|r| r.ok())
                            .collect();
                        index_columns.extend(cols);
                    }
                }

                let indexes = process_indexes(&raw_indexes, &index_columns, &table_sql_map);
                result.indexes.extend(indexes);
            }

            // Foreign keys
            let fk_query = queries::foreign_keys_query(&table.name);
            if let Ok(mut stmt) = conn.prepare(&fk_query) {
                let raw_fks: Vec<RawForeignKey> = stmt
                    .query_map([], |row| {
                        Ok(RawForeignKey {
                            table: table.name.clone(),
                            id: row.get(0)?,
                            seq: row.get(1)?,
                            to_table: row.get(2)?,
                            from_column: row.get(3)?,
                            to_column: row.get(4)?,
                            on_update: row.get(5)?,
                            on_delete: row.get(6)?,
                            r#match: row.get(7)?,
                        })
                    })
                    .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                    .filter_map(|r| r.ok())
                    .collect();

                let fks = process_foreign_keys(&raw_fks);
                result.foreign_keys.extend(fks);
            }
        }

        // Get views
        if let Ok(mut stmt) = conn.prepare(queries::VIEWS_QUERY) {
            let views: Vec<View> = stmt
                .query_map([], |row| {
                    let name: String = row.get(0)?;
                    let sql: Option<String> = row.get(1)?;
                    let definition = sql.and_then(|s| parse_view_sql(&s));
                    Ok(View {
                        name,
                        definition,
                        is_existing: false,
                    })
                })
                .map_err(|e| ConfigError::ConnectionError(e.to_string()))?
                .filter_map(|r| r.ok())
                .collect();
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
            conn.execute_batch(stmt)
                .map_err(|e| ConfigError::ConnectionError(format!("Failed to execute: {}", e)))?;
        }

        println!(
            "✓ Schema pushed successfully! ({} statements)",
            sql_statements.len()
        );

        Ok(())
    }
}
