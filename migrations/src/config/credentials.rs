//! Driver-specific credentials
//!
//! This module defines credential types for each supported database driver.

/// Credentials for rusqlite (file-based SQLite)
#[derive(Clone, Debug, Default)]
pub struct SqliteCredentials {
    /// Path to the database file (e.g., "./dev.db", ":memory:")
    pub path: String,
}

impl SqliteCredentials {
    pub fn new(path: impl Into<String>) -> Self {
        Self { path: path.into() }
    }

    pub fn in_memory() -> Self {
        Self {
            path: ":memory:".to_string(),
        }
    }
}

/// Credentials for Turso/LibSQL remote connections
#[derive(Clone, Debug, Default)]
pub struct TursoCredentials {
    /// Remote database URL (e.g., "libsql://mydb-myorg.turso.io")
    pub url: String,
    /// Auth token for authentication
    pub auth_token: String,
}

impl TursoCredentials {
    pub fn new(url: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            auth_token: auth_token.into(),
        }
    }
}

/// Credentials for LibSQL embedded replica  
#[derive(Clone, Debug, Default)]
pub struct LibsqlCredentials {
    /// Path to local database file
    pub path: String,
    /// Optional sync URL for embedded replica
    pub sync_url: Option<String>,
    /// Optional auth token for sync
    pub auth_token: Option<String>,
}

impl LibsqlCredentials {
    pub fn local(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            sync_url: None,
            auth_token: None,
        }
    }

    pub fn with_sync(
        path: impl Into<String>,
        sync_url: impl Into<String>,
        auth_token: impl Into<String>,
    ) -> Self {
        Self {
            path: path.into(),
            sync_url: Some(sync_url.into()),
            auth_token: Some(auth_token.into()),
        }
    }
}

/// Credentials for PostgreSQL connections
#[derive(Clone, Debug, Default)]
pub struct PostgresCredentials {
    /// Host address
    pub host: String,
    /// Port number (default: 5432)
    pub port: u16,
    /// Username
    pub username: String,
    /// Password
    pub password: String,
    /// Database name
    pub database: String,
    /// SSL mode (optional)
    pub ssl_mode: Option<String>,
}

impl PostgresCredentials {
    pub fn new(
        host: impl Into<String>,
        port: u16,
        username: impl Into<String>,
        password: impl Into<String>,
        database: impl Into<String>,
    ) -> Self {
        Self {
            host: host.into(),
            port,
            username: username.into(),
            password: password.into(),
            database: database.into(),
            ssl_mode: None,
        }
    }

    /// Create from a connection URL
    pub fn from_url(url: impl Into<String>) -> Self {
        // Simple URL parsing - a real implementation would parse properly
        Self {
            host: url.into(), // Just store the full URL
            port: 5432,
            username: String::new(),
            password: String::new(),
            database: String::new(),
            ssl_mode: None,
        }
    }

    pub fn ssl_mode(mut self, mode: impl Into<String>) -> Self {
        self.ssl_mode = Some(mode.into());
        self
    }

    /// Build connection string
    pub fn connection_string(&self) -> String {
        let mut s = format!(
            "host={} port={} user={} password={} dbname={}",
            self.host, self.port, self.username, self.password, self.database
        );
        if let Some(ref ssl) = self.ssl_mode {
            s.push_str(&format!(" sslmode={}", ssl));
        }
        s
    }
}

/// Marker for no credentials (CLI-only mode)
#[derive(Clone, Debug, Default)]
pub struct NoCredentials;
