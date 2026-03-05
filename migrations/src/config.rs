use drizzle_types::{Dialect, MigrationTracking};

/// Configuration for migration tracking metadata.
///
/// Pass this to `db.migrate(&migrations, config)` to control:
/// - tracking table name
/// - tracking schema (PostgreSQL)
/// - SQL dialect used for migration metadata queries
///
/// # Example
///
/// ```rust,no_run
/// # use drizzle_migrations::{MigrateConfig, Migration};
/// # fn main() -> Result<(), Box<dyn std::error::Error>> {
/// # struct Db;
/// # impl Db {
/// #     fn migrate(&self, _migrations: &[Migration], _config: MigrateConfig<'_>) -> Result<(), Box<dyn std::error::Error>> {
/// #         Ok(())
/// #     }
/// # }
/// # let db = Db;
/// # let migrations: Vec<Migration> = Vec::new();
///
/// // SQLite defaults
/// db.migrate(&migrations, MigrateConfig::SQLITE)?;
///
/// // PostgreSQL defaults with custom table
/// db.migrate(&migrations, MigrateConfig::POSTGRES.table("custom_migrations"))?;
/// # Ok(())
/// # }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct MigrateConfig<'a> {
    /// Shared migration tracking metadata.
    pub tracking: MigrationTracking<'a>,
    /// Database dialect used for SQL generation.
    dialect: Dialect,
}

impl MigrateConfig<'static> {
    /// Default SQLite migration tracking configuration.
    pub const SQLITE: Self = Self {
        tracking: MigrationTracking::SQLITE,
        dialect: Dialect::SQLite,
    };

    /// Default PostgreSQL migration tracking configuration.
    pub const POSTGRES: Self = Self {
        tracking: MigrationTracking::POSTGRES,
        dialect: Dialect::PostgreSQL,
    };
}

impl<'a> MigrateConfig<'a> {
    /// Build config from explicit tracking metadata.
    pub const fn from_tracking(dialect: Dialect, tracking: MigrationTracking<'a>) -> Self {
        Self { tracking, dialect }
    }

    /// Override migrations tracking table.
    pub const fn table<'b>(self, table: &'b str) -> MigrateConfig<'b>
    where
        'a: 'b,
    {
        MigrateConfig {
            tracking: self.tracking.table(table),
            dialect: self.dialect,
        }
    }

    /// Override migrations tracking schema (PostgreSQL).
    pub const fn schema<'b>(self, schema: &'b str) -> MigrateConfig<'b>
    where
        'a: 'b,
    {
        MigrateConfig {
            tracking: self.tracking.schema(schema),
            dialect: self.dialect,
        }
    }

    #[inline]
    pub(crate) const fn dialect(self) -> Dialect {
        self.dialect
    }
}
