//! Cloudflare D1 HTTP client for the CLI.
//!
//! Talks to the D1 REST API
//! (`https://api.cloudflare.com/client/v4/accounts/{account}/d1/database/{db}/query`)
//! using `reqwest`. Mirrors the upstream `drizzle-kit` `d1-http` driver — same
//! endpoints, same request/response shape, same batching-via-`;` strategy for
//! atomic migration application.
//!
//! # Why HTTP and not a binding?
//!
//! Inside a Worker you use the `D1Database` binding directly (the main
//! `drizzle` crate's `d1` feature wraps that). The CLI doesn't run inside a
//! Worker — it runs on a developer's machine — so it has to go through the
//! public REST API with an account-scoped API token.
//!
//! # Scope
//!
//! This module only implements the operations the CLI actually needs:
//!
//! - `ensure_tracking_table` – create the drizzle migrations table
//! - `query_applied_names` / `query_applied_records` – read migration history
//! - `execute_statements` – run arbitrary SQL (used by `drizzle push`)
//! - `run_migrations` – apply pending migrations via the batch endpoint
//! - `init_metadata` – seed the first migration row without running its SQL
//!
//! We do **not** port the v0 → v1 tracking-table upgrade (`ALTER TABLE …
//! ADD COLUMN name …`) here — users coming fresh to D1 start at v1, and
//! anyone with a legacy tracking table can run the upgrade SQL via
//! `wrangler d1 execute` one time and then use this CLI.

use serde::{Deserialize, Serialize};

use crate::error::CliError;
use drizzle_migrations::Migrations;

use super::{AppliedMigrationRecord, MigrationResult};

// ---------------------------------------------------------------------------
// Wire types — match the upstream D1 REST shape exactly.
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct Request<'a> {
    sql: &'a str,
    #[serde(skip_serializing_if = "<[serde_json::Value]>::is_empty")]
    params: &'a [serde_json::Value],
}

/// Top-level D1 REST response.
///
/// Upstream TypeScript:
/// ```ts
/// | { success: true;  result: { results: any[] | { columns: string[]; rows: any[][] } }[] }
/// | { success: false; errors: { code: number; message: string }[] }
/// ```
///
/// We use a single struct with optional fields rather than an untagged enum —
/// serde untagged with a `success: bool` discriminator is fragile, and the
/// absent-vs-empty-array distinction doesn't matter for our consumer.
#[derive(Deserialize, Debug)]
struct Response {
    success: bool,
    #[serde(default)]
    result: Vec<ResultEntry>,
    #[serde(default)]
    errors: Vec<ApiError>,
}

#[derive(Deserialize, Debug)]
struct ResultEntry {
    #[serde(default)]
    results: Option<Rows>,
}

/// `results` is either an array of row-objects (default `/query` shape) or a
/// `{columns, rows}` pair (the `/raw` shape). We handle both because D1 has
/// been known to return either when the query produces no rows.
#[derive(Deserialize, Debug)]
#[serde(untagged)]
enum Rows {
    /// `/query` endpoint: `[{col: val, ...}, ...]`
    Objects(Vec<serde_json::Map<String, serde_json::Value>>),
    /// `/raw` endpoint: `{columns: [...], rows: [[...], ...]}`
    Values {
        #[allow(dead_code)]
        columns: Vec<String>,
        rows: Vec<Vec<serde_json::Value>>,
    },
}

#[derive(Deserialize, Debug)]
struct ApiError {
    code: i64,
    message: String,
}

// ---------------------------------------------------------------------------
// Client
// ---------------------------------------------------------------------------

/// Low-level HTTP client bound to a single D1 database.
pub(super) struct D1HttpClient {
    http: reqwest::Client,
    /// `https://api.cloudflare.com/client/v4/accounts/{account}/d1/database/{db}`
    base_url: String,
    /// Pre-formatted `Bearer <token>` header value.
    auth_header: String,
}

impl D1HttpClient {
    pub fn new(account_id: &str, database_id: &str, token: &str) -> Result<Self, CliError> {
        let http = reqwest::Client::builder()
            .user_agent(concat!("drizzle-cli/", env!("CARGO_PKG_VERSION")))
            .build()
            .map_err(|e| {
                CliError::ConnectionError(format!("Failed to build reqwest client: {e}"))
            })?;

        Ok(Self {
            http,
            base_url: format!(
                "https://api.cloudflare.com/client/v4/accounts/{account_id}/d1/database/{database_id}"
            ),
            auth_header: format!("Bearer {token}"),
        })
    }

    /// POST `{sql, params}` to `{base_url}{path}` and return the parsed body.
    /// Errors out if `success: false`.
    async fn post(
        &self,
        path: &str,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<Response, CliError> {
        let url = format!("{}{}", self.base_url, path);
        let resp = self
            .http
            .post(&url)
            .header(reqwest::header::AUTHORIZATION, &self.auth_header)
            .header(reqwest::header::CONTENT_TYPE, "application/json")
            .json(&Request { sql, params })
            .send()
            .await
            .map_err(|e| CliError::ConnectionError(format!("D1 HTTP request failed: {e}")))?;

        let status = resp.status();
        // D1 returns JSON even on 4xx/5xx. Read body regardless and surface the
        // API-level error if present; fall back to the HTTP status otherwise.
        let text = resp
            .text()
            .await
            .map_err(|e| CliError::Other(format!("D1 response read failed ({status}): {e}")))?;

        let body: Response = serde_json::from_str(&text).map_err(|e| {
            CliError::Other(format!(
                "D1 response parse failed ({status}): {e}\nbody: {text}"
            ))
        })?;

        if !body.success {
            let msg = if body.errors.is_empty() {
                format!("HTTP {status}")
            } else {
                body.errors
                    .iter()
                    .map(|e| format!("{}: {}", e.code, e.message))
                    .collect::<Vec<_>>()
                    .join("\n")
            };
            return Err(CliError::MigrationError(format!("D1 API error: {msg}")));
        }

        Ok(body)
    }

    /// Execute a single SQL statement with parameters. Returns the raw rows as
    /// row-objects (`/query` endpoint, upstream `all`/`run` mode).
    pub async fn query(
        &self,
        sql: &str,
        params: &[serde_json::Value],
    ) -> Result<Vec<serde_json::Map<String, serde_json::Value>>, CliError> {
        let body = self.post("/query", sql, params).await?;
        Ok(extract_row_objects(body))
    }

    /// Execute a single SQL statement and discard the result. Used for DDL /
    /// DML where the caller doesn't care about the rows.
    pub async fn run(&self, sql: &str) -> Result<(), CliError> {
        self.post("/query", sql, &[]).await?;
        Ok(())
    }

    /// Upstream `remoteBatchCallback`: join statements with `"; "` and POST
    /// once. D1 runs a multi-statement body atomically, which is the closest
    /// thing to a transaction available over the REST API.
    pub async fn batch(&self, statements: &[&str]) -> Result<(), CliError> {
        let joined = statements
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("; ");
        if joined.is_empty() {
            return Ok(());
        }
        self.post("/query", &joined, &[]).await?;
        Ok(())
    }
}

/// Flatten a `Response` into a single list of row-objects, collapsing across
/// batched results. Empty / missing `results` fields become no rows.
fn extract_row_objects(body: Response) -> Vec<serde_json::Map<String, serde_json::Value>> {
    let mut out = Vec::new();
    for entry in body.result {
        match entry.results {
            Some(Rows::Objects(rows)) => out.extend(rows),
            Some(Rows::Values { rows, columns }) => {
                // Tolerate /raw-shaped results even on /query — zip columns+values.
                for row in rows {
                    let mut map = serde_json::Map::new();
                    for (col, val) in columns.iter().zip(row) {
                        map.insert(col.clone(), val);
                    }
                    out.push(map);
                }
            }
            None => {}
        }
    }
    out
}

// ---------------------------------------------------------------------------
// High-level ops used by db/mod.rs dispatch
// ---------------------------------------------------------------------------

fn rt() -> Result<tokio::runtime::Runtime, CliError> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|e| CliError::Other(format!("Failed to create async runtime: {e}")))
}

fn client(account_id: &str, database_id: &str, token: &str) -> Result<D1HttpClient, CliError> {
    D1HttpClient::new(account_id, database_id, token)
}

pub(super) fn inspect_migrations(
    set: &Migrations,
    account_id: &str,
    database_id: &str,
    token: &str,
) -> Result<super::MigrationPlan, CliError> {
    let rt = rt()?;
    rt.block_on(async {
        let c = client(account_id, database_id, token)?;
        ensure_tracking_table(&c, set).await?;
        let applied = query_applied_records(&c, set).await?;
        super::build_migration_plan(set, &applied)
    })
}

pub(super) fn run_migrations(
    set: &Migrations,
    account_id: &str,
    database_id: &str,
    token: &str,
) -> Result<MigrationResult, CliError> {
    let rt = rt()?;
    rt.block_on(async {
        let c = client(account_id, database_id, token)?;
        ensure_tracking_table(&c, set).await?;

        let applied_names = query_applied_names(&c, set).await?;
        let pending: Vec<_> = set.pending(&applied_names).collect();
        if pending.is_empty() {
            return Ok(MigrationResult {
                applied_count: 0,
                applied_migrations: vec![],
            });
        }

        // Build one big SQL blob per migration: statements + the record_migration
        // INSERT. Each migration is sent as its own batch so a failure in a later
        // migration doesn't erase the success of the earlier ones.
        let mut applied_hashes = Vec::new();
        for migration in &pending {
            let mut stmts: Vec<&str> = migration
                .statements()
                .iter()
                .map(std::string::String::as_str)
                .filter(|s| !s.trim().is_empty())
                .collect();
            let record_sql = set.record_migration_sql(migration);
            stmts.push(&record_sql);

            c.batch(&stmts).await.map_err(|e| match e {
                CliError::MigrationError(inner) => CliError::MigrationError(format!(
                    "Migration '{}' failed: {}",
                    migration.hash(),
                    inner
                )),
                other => other,
            })?;

            applied_hashes.push(migration.hash().to_string());
        }

        Ok(MigrationResult {
            applied_count: applied_hashes.len(),
            applied_migrations: applied_hashes,
        })
    })
}

pub(super) fn execute_statements(
    account_id: &str,
    database_id: &str,
    token: &str,
    statements: &[String],
) -> Result<(), CliError> {
    let rt = rt()?;
    rt.block_on(async {
        let c = client(account_id, database_id, token)?;
        let refs: Vec<&str> = statements.iter().map(String::as_str).collect();
        c.batch(&refs).await
    })
}

pub(super) fn init_metadata(
    set: &Migrations,
    account_id: &str,
    database_id: &str,
    token: &str,
) -> Result<(), CliError> {
    let rt = rt()?;
    rt.block_on(async {
        let c = client(account_id, database_id, token)?;
        ensure_tracking_table(&c, set).await?;

        let applied_names = query_applied_names(&c, set).await?;
        super::validate_init_metadata(&applied_names, set)?;

        let Some(first) = set.all().first() else {
            return Ok(());
        };

        c.run(&set.record_migration_sql(first)).await?;
        Ok(())
    })
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

async fn ensure_tracking_table(c: &D1HttpClient, set: &Migrations) -> Result<(), CliError> {
    c.run(&set.create_table_sql()).await
}

async fn query_applied_names(c: &D1HttpClient, set: &Migrations) -> Result<Vec<String>, CliError> {
    // D1 returns an API error if the table doesn't exist; we just ensured it,
    // so surface errors normally here.
    let rows = c.query(&set.applied_names_sql(), &[]).await?;
    Ok(rows
        .into_iter()
        .filter_map(|mut row| {
            row.remove("name")
                .and_then(|v| v.as_str().map(String::from))
        })
        .collect())
}

async fn query_applied_records(
    c: &D1HttpClient,
    set: &Migrations,
) -> Result<Vec<AppliedMigrationRecord>, CliError> {
    let sql = format!(
        r#"SELECT "hash", "name" FROM {} WHERE "name" IS NOT NULL ORDER BY id;"#,
        set.table_ident_sql()
    );
    let rows = c.query(&sql, &[]).await?;
    Ok(rows
        .into_iter()
        .filter_map(|mut row| {
            let hash = row.remove("hash")?.as_str()?.to_string();
            let name = row.remove("name")?.as_str()?.to_string();
            Some(AppliedMigrationRecord { hash, name })
        })
        .collect())
}
