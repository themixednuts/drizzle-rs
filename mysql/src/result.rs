//! Driver-independent results for MySQL mutation statements.

/// The normalized result of executing a MySQL `INSERT`, `UPDATE`, or
/// `DELETE` statement.
///
/// Concrete adapters translate their client's OK packet into this value so
/// application code never depends on `mysql` or `mysql_async` result types.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MySQLMutationResult {
    affected_rows: u64,
    last_insert_id: Option<u64>,
}

impl MySQLMutationResult {
    /// Creates a normalized mutation result.
    #[must_use]
    pub const fn new(affected_rows: u64, last_insert_id: Option<u64>) -> Self {
        Self {
            affected_rows,
            last_insert_id,
        }
    }

    /// Number of rows affected according to MySQL's OK packet.
    #[must_use]
    pub const fn affected_rows(self) -> u64 {
        self.affected_rows
    }

    /// First generated `AUTO_INCREMENT` value, when the server reported one.
    ///
    /// This is execution metadata, not SQL `RETURNING`. It does not imply
    /// that every inserted row received a contiguous generated identifier.
    #[must_use]
    pub const fn last_insert_id(self) -> Option<u64> {
        self.last_insert_id
    }
}
