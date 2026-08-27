//! Driver-neutral PostgreSQL transaction options.

use core::marker::PhantomData;

use crate::common::PostgresTransactionType;

/// PostgreSQL transaction isolation level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IsolationLevel {
    /// `READ UNCOMMITTED`.
    ReadUncommitted,
    /// `READ COMMITTED`.
    ReadCommitted,
    /// `REPEATABLE READ`.
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

/// PostgreSQL transaction access mode.
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

/// Options applied when starting a PostgreSQL transaction.
///
/// The default leaves every choice to the server. Use [`Self::builder`] when
/// choices are known statically; its typestate only exposes `DEFERRABLE` for
/// `SERIALIZABLE READ ONLY` transactions, the combination where PostgreSQL
/// gives the option meaning.
///
/// ```compile_fail
/// use drizzle_postgres::TransactionConfig;
///
/// // DEFERRABLE only has meaning for a serializable, read-only transaction.
/// let _ = TransactionConfig::builder()
///     .serializable()
///     .read_write()
///     .deferrable();
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransactionConfig {
    isolation_level: Option<IsolationLevel>,
    access_mode: Option<AccessMode>,
    deferrable: bool,
    legacy_read_committed: bool,
}

impl TransactionConfig {
    /// Uses server defaults for every option.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            isolation_level: None,
            access_mode: None,
            deferrable: false,
            legacy_read_committed: false,
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
        self.deferrable = false;
        self.legacy_read_committed = false;
        self
    }

    /// Selects an access mode supplied at runtime.
    #[must_use]
    pub const fn access_mode(mut self, mode: AccessMode) -> Self {
        self.access_mode = Some(mode);
        self.deferrable = false;
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

    /// Whether `DEFERRABLE` was requested.
    #[must_use]
    pub const fn is_deferrable(&self) -> bool {
        self.deferrable
    }

    /// Whether wire-protocol adapters should preserve the legacy
    /// server-default behavior of `PostgresTransactionType::ReadCommitted`.
    #[doc(hidden)]
    #[must_use]
    pub const fn uses_server_default_isolation(&self) -> bool {
        self.isolation_level.is_none() || self.legacy_read_committed
    }
}

impl From<PostgresTransactionType> for TransactionConfig {
    fn from(tx_type: PostgresTransactionType) -> Self {
        let isolation_level = match tx_type {
            PostgresTransactionType::ReadCommitted => Some(IsolationLevel::ReadCommitted),
            PostgresTransactionType::ReadUncommitted => Some(IsolationLevel::ReadUncommitted),
            PostgresTransactionType::RepeatableRead => Some(IsolationLevel::RepeatableRead),
            PostgresTransactionType::Serializable => Some(IsolationLevel::Serializable),
        };
        Self {
            isolation_level,
            legacy_read_committed: matches!(tx_type, PostgresTransactionType::ReadCommitted),
            ..Self::new()
        }
    }
}

/// Typestated builder for [`TransactionConfig`].
///
/// State parameters are inferred and do not need to be named by callers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[must_use]
pub struct ConfigBuilder<Isolation = state::ServerDefault, Access = state::ServerDefault> {
    config: TransactionConfig,
    state: PhantomData<(Isolation, Access)>,
}

impl ConfigBuilder {
    const fn new() -> Self {
        Self {
            config: TransactionConfig::new(),
            state: PhantomData,
        }
    }
}

impl<Isolation, Access> ConfigBuilder<Isolation, Access> {
    const fn isolation<Next>(mut self, level: IsolationLevel) -> ConfigBuilder<Next, Access> {
        self.config.isolation_level = Some(level);
        self.config.deferrable = false;
        self.config.legacy_read_committed = false;
        ConfigBuilder {
            config: self.config,
            state: PhantomData,
        }
    }

    const fn access<Next>(mut self, mode: AccessMode) -> ConfigBuilder<Isolation, Next> {
        self.config.access_mode = Some(mode);
        self.config.deferrable = false;
        ConfigBuilder {
            config: self.config,
            state: PhantomData,
        }
    }

    /// Selects an isolation level supplied at runtime.
    pub const fn isolation_level(
        self,
        level: IsolationLevel,
    ) -> ConfigBuilder<state::Dynamic, Access> {
        self.isolation(level)
    }

    /// Uses `READ UNCOMMITTED` isolation.
    pub const fn read_uncommitted(self) -> ConfigBuilder<state::ReadUncommitted, Access> {
        self.isolation(IsolationLevel::ReadUncommitted)
    }

    /// Uses `READ COMMITTED` isolation.
    pub const fn read_committed(self) -> ConfigBuilder<state::ReadCommitted, Access> {
        self.isolation(IsolationLevel::ReadCommitted)
    }

    /// Uses `REPEATABLE READ` isolation.
    pub const fn repeatable_read(self) -> ConfigBuilder<state::RepeatableRead, Access> {
        self.isolation(IsolationLevel::RepeatableRead)
    }

    /// Uses `SERIALIZABLE` isolation.
    pub const fn serializable(self) -> ConfigBuilder<state::Serializable, Access> {
        self.isolation(IsolationLevel::Serializable)
    }

    /// Selects an access mode supplied at runtime.
    pub const fn access_mode(self, mode: AccessMode) -> ConfigBuilder<Isolation, state::Dynamic> {
        self.access(mode)
    }

    /// Rejects writes in the transaction.
    pub const fn read_only(self) -> ConfigBuilder<Isolation, state::ReadOnly> {
        self.access(AccessMode::ReadOnly)
    }

    /// Permits reads and writes in the transaction.
    pub const fn read_write(self) -> ConfigBuilder<Isolation, state::ReadWrite> {
        self.access(AccessMode::ReadWrite)
    }

    /// Finishes the configuration.
    #[must_use]
    pub const fn build(self) -> TransactionConfig {
        self.config
    }
}

impl ConfigBuilder<state::Serializable, state::ReadOnly> {
    /// Defers the initial serializable snapshot until it can run without risk
    /// of a serialization failure.
    pub const fn deferrable(mut self) -> Self {
        self.config.deferrable = true;
        self
    }
}

/// Typestate markers used by [`ConfigBuilder`].
#[doc(hidden)]
pub mod state {
    /// Server-selected option.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ServerDefault;
    /// Option supplied at runtime.
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
    /// Read-only access.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReadOnly;
    /// Read-write access.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ReadWrite;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_uses_server_policy() {
        assert_eq!(TransactionConfig::new(), TransactionConfig::default());
    }

    #[test]
    fn builder_preserves_selected_options() {
        let config = TransactionConfig::builder()
            .serializable()
            .read_only()
            .deferrable()
            .build();

        assert_eq!(config.isolation(), Some(IsolationLevel::Serializable));
        assert_eq!(config.access(), Some(AccessMode::ReadOnly));
        assert!(config.is_deferrable());
    }

    #[test]
    fn legacy_read_committed_preserves_adapter_behavior() {
        let config = TransactionConfig::from(PostgresTransactionType::ReadCommitted);
        assert_eq!(config.isolation(), Some(IsolationLevel::ReadCommitted));
        assert!(config.uses_server_default_isolation());
    }

    #[test]
    fn changing_access_clears_deferrable() {
        let config = TransactionConfig::builder()
            .serializable()
            .read_only()
            .deferrable()
            .read_write()
            .build();

        assert_eq!(config.access(), Some(AccessMode::ReadWrite));
        assert!(!config.is_deferrable());
    }

    #[test]
    fn runtime_setters_clear_invalid_deferrable_state() {
        let config = TransactionConfig::builder()
            .serializable()
            .read_only()
            .deferrable()
            .build()
            .access_mode(AccessMode::ReadWrite)
            .isolation_level(IsolationLevel::ReadCommitted);

        assert!(!config.is_deferrable());
        assert!(!config.uses_server_default_isolation());
    }
}
