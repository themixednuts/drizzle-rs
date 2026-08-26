//! Driver-neutral MySQL transaction options.

/// MySQL transaction isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MySQLIsolationLevel {
    /// `READ UNCOMMITTED`.
    ReadUncommitted,
    /// `READ COMMITTED`.
    ReadCommitted,
    /// `REPEATABLE READ` (the InnoDB default).
    RepeatableRead,
    /// `SERIALIZABLE`.
    Serializable,
}

impl core::fmt::Display for MySQLIsolationLevel {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::ReadUncommitted => "READ UNCOMMITTED",
            Self::ReadCommitted => "READ COMMITTED",
            Self::RepeatableRead => "REPEATABLE READ",
            Self::Serializable => "SERIALIZABLE",
        })
    }
}

/// MySQL transaction access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MySQLAccessMode {
    /// Reject writes in the transaction.
    ReadOnly,
    /// Permit reads and writes.
    ReadWrite,
}

impl core::fmt::Display for MySQLAccessMode {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::ReadOnly => "READ ONLY",
            Self::ReadWrite => "READ WRITE",
        })
    }
}

/// Options applied when starting a MySQL transaction.
///
/// Concrete adapters translate this value into their client's transaction
/// options and remain responsible for server/engine capability errors. Nested
/// transactions are implemented with savepoints by the adapter, not by
/// starting another wire-driver transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct MySQLTransactionConfig {
    isolation_level: Option<MySQLIsolationLevel>,
    access_mode: Option<MySQLAccessMode>,
    consistent_snapshot: bool,
}

impl MySQLTransactionConfig {
    /// Uses server defaults for every option.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            isolation_level: None,
            access_mode: None,
            consistent_snapshot: false,
        }
    }

    /// Selects an isolation level.
    #[must_use]
    pub const fn isolation_level(mut self, level: MySQLIsolationLevel) -> Self {
        self.isolation_level = Some(level);
        self
    }

    /// Selects read-only or read-write access.
    #[must_use]
    pub const fn access_mode(mut self, mode: MySQLAccessMode) -> Self {
        self.access_mode = Some(mode);
        self
    }

    /// Requests `WITH CONSISTENT SNAPSHOT`.
    #[must_use]
    pub const fn with_consistent_snapshot(mut self) -> Self {
        self.consistent_snapshot = true;
        self
    }

    /// Configured isolation level, or `None` to use the server default.
    #[must_use]
    pub const fn isolation(&self) -> Option<MySQLIsolationLevel> {
        self.isolation_level
    }

    /// Configured access mode, or `None` to use the server default.
    #[must_use]
    pub const fn access(&self) -> Option<MySQLAccessMode> {
        self.access_mode
    }

    /// Whether a consistent snapshot was requested.
    #[must_use]
    pub const fn consistent_snapshot(&self) -> bool {
        self.consistent_snapshot
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_options_are_independent_and_default_to_server_policy() {
        assert_eq!(
            MySQLTransactionConfig::new(),
            MySQLTransactionConfig::default()
        );

        let config = MySQLTransactionConfig::new()
            .isolation_level(MySQLIsolationLevel::Serializable)
            .access_mode(MySQLAccessMode::ReadOnly)
            .with_consistent_snapshot();
        assert_eq!(config.isolation(), Some(MySQLIsolationLevel::Serializable));
        assert_eq!(config.access(), Some(MySQLAccessMode::ReadOnly));
        assert!(config.consistent_snapshot());
    }
}
