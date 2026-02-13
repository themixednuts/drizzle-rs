//! Batch size computation for INSERT statements.
//!
//! Different databases have different limits on the number of parameters
//! in a single statement. We split inserts into batches to stay within limits.

/// Maximum parameter count for SQLite (SQLITE_MAX_VARIABLE_NUMBER default).
pub const SQLITE_MAX_PARAMS: usize = 32766;

/// Maximum parameter count for PostgreSQL.
pub const POSTGRES_MAX_PARAMS: usize = 65535;

/// Dialect hint for selecting the right parameter limit.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dialect {
    Sqlite,
    Postgres,
}

impl Dialect {
    /// Maximum parameters for this dialect.
    pub fn max_params(self) -> usize {
        match self {
            Dialect::Sqlite => SQLITE_MAX_PARAMS,
            Dialect::Postgres => POSTGRES_MAX_PARAMS,
        }
    }

    /// Maximum rows per INSERT batch for a table with `num_columns` columns.
    pub fn max_batch_rows(self, num_columns: usize) -> usize {
        if num_columns == 0 {
            return 0;
        }
        self.max_params() / num_columns
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sqlite_batch_sizing() {
        assert_eq!(Dialect::Sqlite.max_params(), 32766);
        // 10-column table: 32766 / 10 = 3276 rows per batch
        assert_eq!(Dialect::Sqlite.max_batch_rows(10), 3276);
        // Single column: all params available
        assert_eq!(Dialect::Sqlite.max_batch_rows(1), 32766);
    }

    #[test]
    fn postgres_batch_sizing() {
        assert_eq!(Dialect::Postgres.max_params(), 65535);
        assert_eq!(Dialect::Postgres.max_batch_rows(10), 6553);
        assert_eq!(Dialect::Postgres.max_batch_rows(1), 65535);
    }

    #[test]
    fn zero_columns_returns_zero() {
        assert_eq!(Dialect::Sqlite.max_batch_rows(0), 0);
        assert_eq!(Dialect::Postgres.max_batch_rows(0), 0);
    }

    #[test]
    fn many_columns_gives_small_batches() {
        // 100-column table
        assert_eq!(Dialect::Sqlite.max_batch_rows(100), 327);
        assert_eq!(Dialect::Postgres.max_batch_rows(100), 655);
    }
}
