//! Driver-neutral MySQL transaction options.

use core::marker::PhantomData;

/// MySQL transaction isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// `READ UNCOMMITTED`.
    ReadUncommitted,
    /// `READ COMMITTED`.
    ReadCommitted,
    /// `REPEATABLE READ` (the InnoDB default).
    RepeatableRead,
    /// `SERIALIZABLE`.
    Serializable,
}

impl core::fmt::Display for IsolationLevel {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::ReadUncommitted => "READ UNCOMMITTED",
            Self::ReadCommitted => "READ COMMITTED",
            Self::RepeatableRead => "REPEATABLE READ",
            Self::Serializable => "SERIALIZABLE",
        })
    }
}

/// Backwards-compatible name for [`IsolationLevel`].
pub type MySQLIsolationLevel = IsolationLevel;

/// MySQL transaction access mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccessMode {
    /// Reject writes in the transaction.
    ReadOnly,
    /// Permit reads and writes.
    ReadWrite,
}

impl core::fmt::Display for AccessMode {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::ReadOnly => "READ ONLY",
            Self::ReadWrite => "READ WRITE",
        })
    }
}

/// Backwards-compatible name for [`AccessMode`].
pub type MySQLAccessMode = AccessMode;

/// Options applied when starting a MySQL transaction.
///
/// Use [`TransactionConfig::builder`] when the choices are known statically.
/// Its typestate only exposes [`ConfigBuilder::snapshot`] after
/// [`ConfigBuilder::repeatable_read`], matching MySQL's requirement for a
/// consistent snapshot. The direct setters remain useful when values come
/// from runtime configuration.
///
/// ```compile_fail
/// use drizzle_mysql::TransactionConfig;
///
/// // MySQL cannot provide this snapshot under SERIALIZABLE isolation.
/// let _ = TransactionConfig::builder()
///     .serializable()
///     .snapshot();
/// ```
///
/// Concrete adapters translate this value into their client's transaction
/// options and remain responsible for server/engine capability errors. Nested
/// transactions are implemented with savepoints by the adapter, not by
/// starting another wire-driver transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransactionConfig {
    isolation_level: Option<IsolationLevel>,
    access_mode: Option<AccessMode>,
    consistent_snapshot: bool,
}

impl TransactionConfig {
    /// Uses server defaults for every option.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            isolation_level: None,
            access_mode: None,
            consistent_snapshot: false,
        }
    }

    /// Starts a typestated transaction configuration.
    pub const fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Selects an isolation level supplied at runtime.
    #[must_use]
    pub const fn isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = Some(level);
        if !matches!(level, IsolationLevel::RepeatableRead) {
            self.consistent_snapshot = false;
        }
        self
    }

    /// Selects read-only or read-write access supplied at runtime.
    #[must_use]
    pub const fn access_mode(mut self, mode: AccessMode) -> Self {
        self.access_mode = Some(mode);
        self
    }

    /// Requests `WITH CONSISTENT SNAPSHOT` for runtime-derived configuration.
    ///
    /// This selects `REPEATABLE READ`, the isolation level where MySQL gives
    /// the option its documented meaning. Prefer the typestated builder when
    /// the isolation level is known in code.
    #[must_use]
    pub const fn with_consistent_snapshot(mut self) -> Self {
        self.isolation_level = Some(IsolationLevel::RepeatableRead);
        self.consistent_snapshot = true;
        self
    }

    /// Configured isolation level, or `None` to use the server default.
    #[must_use]
    pub const fn isolation(&self) -> Option<IsolationLevel> {
        self.isolation_level
    }

    /// Configured access mode, or `None` to use the server default.
    #[must_use]
    pub const fn access(&self) -> Option<AccessMode> {
        self.access_mode
    }

    /// Whether a consistent snapshot was requested.
    #[must_use]
    pub const fn consistent_snapshot(&self) -> bool {
        self.consistent_snapshot
    }
}

/// Backwards-compatible name for [`TransactionConfig`].
pub type MySQLTransactionConfig = TransactionConfig;

/// Typestated builder for [`TransactionConfig`].
///
/// The state parameter is inferred and does not need to be named by callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct ConfigBuilder<Isolation = state::ServerDefault> {
    config: TransactionConfig,
    state: PhantomData<Isolation>,
}

impl ConfigBuilder {
    const fn new() -> Self {
        Self {
            config: TransactionConfig::new(),
            state: PhantomData,
        }
    }
}

impl<Isolation> ConfigBuilder<Isolation> {
    const fn isolation<Next>(mut self, level: IsolationLevel) -> ConfigBuilder<Next> {
        self.config.isolation_level = Some(level);
        if !matches!(level, IsolationLevel::RepeatableRead) {
            self.config.consistent_snapshot = false;
        }
        ConfigBuilder {
            config: self.config,
            state: PhantomData,
        }
    }

    /// Selects an isolation level supplied at runtime.
    pub const fn isolation_level(self, level: IsolationLevel) -> ConfigBuilder<state::Dynamic> {
        self.isolation(level)
    }

    /// Uses `READ UNCOMMITTED` isolation.
    pub const fn read_uncommitted(self) -> ConfigBuilder<state::ReadUncommitted> {
        self.isolation(IsolationLevel::ReadUncommitted)
    }

    /// Uses `READ COMMITTED` isolation.
    pub const fn read_committed(self) -> ConfigBuilder<state::ReadCommitted> {
        self.isolation(IsolationLevel::ReadCommitted)
    }

    /// Uses `REPEATABLE READ` isolation.
    pub const fn repeatable_read(self) -> ConfigBuilder<state::RepeatableRead> {
        self.isolation(IsolationLevel::RepeatableRead)
    }

    /// Uses `SERIALIZABLE` isolation.
    pub const fn serializable(self) -> ConfigBuilder<state::Serializable> {
        self.isolation(IsolationLevel::Serializable)
    }

    /// Rejects writes in the transaction.
    pub const fn read_only(mut self) -> Self {
        self.config.access_mode = Some(AccessMode::ReadOnly);
        self
    }

    /// Permits reads and writes in the transaction.
    pub const fn read_write(mut self) -> Self {
        self.config.access_mode = Some(AccessMode::ReadWrite);
        self
    }

    /// Finishes the configuration.
    #[must_use]
    pub const fn build(self) -> TransactionConfig {
        self.config
    }
}

impl ConfigBuilder<state::RepeatableRead> {
    /// Requests a consistent snapshot.
    pub const fn snapshot(mut self) -> Self {
        self.config.consistent_snapshot = true;
        self
    }
}

/// Typestate markers used by [`ConfigBuilder`].
#[doc(hidden)]
pub mod state {
    /// Server-selected isolation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ServerDefault;
    /// Isolation supplied at runtime.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Dynamic;
    /// `READ UNCOMMITTED` isolation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReadUncommitted;
    /// `READ COMMITTED` isolation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReadCommitted;
    /// `REPEATABLE READ` isolation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RepeatableRead;
    /// `SERIALIZABLE` isolation.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Serializable;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transaction_options_default_to_server_policy() {
        assert_eq!(TransactionConfig::new(), TransactionConfig::default());
    }

    #[test]
    fn builder_preserves_selected_options() {
        let config = TransactionConfig::builder()
            .repeatable_read()
            .read_only()
            .snapshot()
            .build();

        assert_eq!(config.isolation(), Some(IsolationLevel::RepeatableRead));
        assert_eq!(config.access(), Some(AccessMode::ReadOnly));
        assert!(config.consistent_snapshot());
    }

    #[test]
    fn runtime_options_remain_available() {
        let config = TransactionConfig::new()
            .isolation_level(IsolationLevel::Serializable)
            .access_mode(AccessMode::ReadWrite)
            .with_consistent_snapshot();

        assert_eq!(config.isolation(), Some(IsolationLevel::RepeatableRead));
        assert_eq!(config.access(), Some(AccessMode::ReadWrite));
        assert!(config.consistent_snapshot());
    }

    #[test]
    fn runtime_isolation_change_clears_snapshot() {
        let config = TransactionConfig::new()
            .with_consistent_snapshot()
            .isolation_level(IsolationLevel::Serializable);

        assert_eq!(config.isolation(), Some(IsolationLevel::Serializable));
        assert!(!config.consistent_snapshot());
    }

    #[test]
    fn repeated_runtime_isolation_preserves_snapshot() {
        let config = TransactionConfig::new()
            .with_consistent_snapshot()
            .isolation_level(IsolationLevel::RepeatableRead);

        assert!(config.consistent_snapshot());
    }

    #[test]
    fn changing_isolation_clears_snapshot() {
        let config = TransactionConfig::builder()
            .repeatable_read()
            .snapshot()
            .serializable()
            .build();

        assert_eq!(config.isolation(), Some(IsolationLevel::Serializable));
        assert!(!config.consistent_snapshot());
    }

    #[test]
    fn repeated_builder_isolation_preserves_snapshot() {
        let config = TransactionConfig::builder()
            .repeatable_read()
            .snapshot()
            .repeatable_read()
            .build();

        assert!(config.consistent_snapshot());
    }
}
