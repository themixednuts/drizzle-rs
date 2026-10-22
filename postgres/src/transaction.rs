//! Driver-neutral `PostgreSQL` transaction options ([`TransactionConfig`]).

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

/// A snapshot exported by a running transaction (`pg_export_snapshot()`),
/// for another transaction to read the same data with
/// [`ConfigBuilder::snapshot`].
///
/// Get one from a transaction's `export_snapshot()`, or parse the text form
/// with [`SnapshotId::new`]. It stays importable only while the exporting
/// transaction is open; after that `PostgreSQL` rejects it. The id is a
/// small `Copy` value, so it can be handed to other threads and tasks.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct SnapshotId {
    bytes: [u8; SnapshotId::CAPACITY],
    len: u8,
}

impl SnapshotId {
    /// The longest id accepted. `PostgreSQL` ids are up to about 30
    /// characters (`00000003-0000001B-1`).
    const CAPACITY: usize = 64;

    /// Parses a snapshot id: hex digits and `-`, as `pg_export_snapshot()`
    /// returns it. Returns `None` for anything else, so an id is always safe
    /// to write into `SET TRANSACTION SNAPSHOT`.
    #[must_use]
    pub const fn new(id: &str) -> Option<Self> {
        let source = id.as_bytes();
        if source.is_empty() || source.len() > Self::CAPACITY {
            return None;
        }
        let mut bytes = [0u8; Self::CAPACITY];
        let mut i = 0;
        while i < source.len() {
            let byte = source[i];
            if !(byte.is_ascii_hexdigit() || byte == b'-') {
                return None;
            }
            bytes[i] = byte;
            i += 1;
        }
        // CAPACITY fits a u8.
        #[allow(clippy::cast_possible_truncation)]
        let len = source.len() as u8;
        Some(Self { bytes, len })
    }

    /// The id as `PostgreSQL` writes it.
    #[must_use]
    pub fn as_str(&self) -> &str {
        // `new` only stores ASCII.
        core::str::from_utf8(&self.bytes[..usize::from(self.len)]).unwrap_or_default()
    }
}

impl core::fmt::Debug for SnapshotId {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_tuple("SnapshotId")
            .field(&self.as_str())
            .finish()
    }
}

impl core::fmt::Display for SnapshotId {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(self.as_str())
    }
}

/// Options for starting a `PostgreSQL` transaction: isolation level, access
/// mode, `DEFERRABLE`, and a snapshot to read from.
///
/// The default leaves every choice to the server. Use [`Self::builder`] when
/// the choices are known in code; it only offers `.deferrable()` after
/// `.serializable().read_only()`, the only combination where `PostgreSQL`
/// gives `DEFERRABLE` a meaning. Use the setters on this type for choices
/// made at runtime.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::{AccessMode, IsolationLevel, TransactionConfig};
///
/// let config = TransactionConfig::builder()
///     .serializable()
///     .read_only()
///     .deferrable()
///     .build();
/// assert_eq!(config.isolation(), Some(IsolationLevel::Serializable));
/// assert_eq!(config.access(), Some(AccessMode::ReadOnly));
/// assert!(config.is_deferrable());
///
/// // Runtime choices:
/// let config = TransactionConfig::new().isolation_level(IsolationLevel::RepeatableRead);
/// assert_eq!(config.access(), None); // server default
/// ```
///
/// # Compile-time checks
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
///
/// ```compile_fail
/// use drizzle_postgres::{SnapshotId, TransactionConfig};
///
/// // PostgreSQL imports a snapshot only into REPEATABLE READ or SERIALIZABLE.
/// let snapshot = SnapshotId::new("00000003-0000001B-1").unwrap();
/// let _ = TransactionConfig::builder()
///     .read_committed()
///     .snapshot(snapshot);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct TransactionConfig {
    isolation_level: Option<IsolationLevel>,
    access_mode: Option<AccessMode>,
    deferrable: bool,
    snapshot: Option<SnapshotId>,
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
            snapshot: None,
            legacy_read_committed: false,
        }
    }

    /// Starts a [`ConfigBuilder`], which checks option combinations at compile time.
    pub const fn builder() -> ConfigBuilder {
        ConfigBuilder::new()
    }

    /// Sets the isolation level.
    ///
    /// Any level other than `SERIALIZABLE` turns `DEFERRABLE` off, and one
    /// below `REPEATABLE READ` drops the snapshot.
    #[must_use]
    pub const fn isolation_level(mut self, level: IsolationLevel) -> Self {
        self.isolation_level = Some(level);
        if !matches!(level, IsolationLevel::Serializable) {
            self.deferrable = false;
        }
        if !matches!(
            level,
            IsolationLevel::RepeatableRead | IsolationLevel::Serializable
        ) {
            self.snapshot = None;
        }
        self.legacy_read_committed = false;
        self
    }

    /// Sets the access mode.
    ///
    /// `READ WRITE` turns `DEFERRABLE` off.
    #[must_use]
    pub const fn access_mode(mut self, mode: AccessMode) -> Self {
        self.access_mode = Some(mode);
        if !matches!(mode, AccessMode::ReadOnly) {
            self.deferrable = false;
        }
        self
    }

    /// Makes the transaction `SERIALIZABLE READ ONLY DEFERRABLE`.
    ///
    /// `DEFERRABLE` only has a meaning for a serializable, read-only
    /// transaction, so this also sets those two options. Prefer the builder
    /// when the choices are known in code.
    #[must_use]
    pub const fn deferrable(mut self) -> Self {
        self.isolation_level = Some(IsolationLevel::Serializable);
        self.access_mode = Some(AccessMode::ReadOnly);
        self.deferrable = true;
        self.legacy_read_committed = false;
        self
    }

    /// Returns the isolation level, or `None` for the server default.
    #[must_use]
    pub const fn isolation(&self) -> Option<IsolationLevel> {
        self.isolation_level
    }

    /// Returns the access mode, or `None` for the server default.
    #[must_use]
    pub const fn access(&self) -> Option<AccessMode> {
        self.access_mode
    }

    /// Returns `true` if `DEFERRABLE` is set.
    #[must_use]
    pub const fn is_deferrable(&self) -> bool {
        self.deferrable
    }

    /// The snapshot the transaction reads from, if any.
    #[must_use]
    pub const fn snapshot(&self) -> Option<SnapshotId> {
        self.snapshot
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

/// Builds a [`TransactionConfig`], rejecting meaningless combinations at compile time.
///
/// Start with [`TransactionConfig::builder`], pick an isolation level and an
/// access mode in either order, then call [`build`](Self::build). The type
/// parameters track the choices and are inferred.
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
        self.config = self.config.isolation_level(level);
        ConfigBuilder {
            config: self.config,
            state: PhantomData,
        }
    }

    const fn access<Next>(mut self, mode: AccessMode) -> ConfigBuilder<Isolation, Next> {
        self.config.access_mode = Some(mode);
        if !matches!(mode, AccessMode::ReadOnly) {
            self.config.deferrable = false;
        }
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

    /// Returns the finished [`TransactionConfig`].
    #[must_use]
    pub const fn build(self) -> TransactionConfig {
        self.config
    }
}

impl<Access> ConfigBuilder<state::RepeatableRead, Access> {
    /// Reads from `snapshot`, exported by another open transaction, so both
    /// see the same data (`SET TRANSACTION SNAPSHOT`).
    ///
    /// Only available with `REPEATABLE READ` or `SERIALIZABLE`, the levels
    /// `PostgreSQL` imports a snapshot into.
    pub const fn snapshot(mut self, snapshot: SnapshotId) -> Self {
        self.config.snapshot = Some(snapshot);
        self
    }
}

impl<Access> ConfigBuilder<state::Serializable, Access> {
    /// Reads from `snapshot`, exported by another open transaction, so both
    /// see the same data (`SET TRANSACTION SNAPSHOT`).
    ///
    /// Only available with `REPEATABLE READ` or `SERIALIZABLE`, the levels
    /// `PostgreSQL` imports a snapshot into.
    pub const fn snapshot(mut self, snapshot: SnapshotId) -> Self {
        self.config.snapshot = Some(snapshot);
        self
    }
}

impl ConfigBuilder<state::Serializable, state::ReadOnly> {
    /// Adds `DEFERRABLE`: the transaction waits for a snapshot that cannot
    /// cause a serialization failure, then runs without that risk.
    ///
    /// Only available after `.serializable()` and `.read_only()`.
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

    #[test]
    fn compatible_runtime_setters_preserve_deferrable() {
        let config = TransactionConfig::new()
            .deferrable()
            .isolation_level(IsolationLevel::Serializable)
            .access_mode(AccessMode::ReadOnly);

        assert!(config.is_deferrable());
    }

    #[test]
    fn compatible_builder_transitions_preserve_deferrable() {
        let config = TransactionConfig::builder()
            .serializable()
            .read_only()
            .deferrable()
            .serializable()
            .read_only()
            .build();

        assert!(config.is_deferrable());
    }

    #[test]
    fn access_mode_preserves_legacy_server_default_isolation() {
        let config = TransactionConfig::from(PostgresTransactionType::ReadCommitted)
            .access_mode(AccessMode::ReadOnly);

        assert_eq!(config.isolation(), Some(IsolationLevel::ReadCommitted));
        assert!(config.uses_server_default_isolation());
    }

    #[test]
    fn runtime_deferrable_selects_valid_modes() {
        let config = TransactionConfig::new().deferrable();

        assert_eq!(config.isolation(), Some(IsolationLevel::Serializable));
        assert_eq!(config.access(), Some(AccessMode::ReadOnly));
        assert!(config.is_deferrable());
        assert!(!config.uses_server_default_isolation());
    }

    #[test]
    fn snapshot_ids_hold_only_hex_digits_and_dashes() {
        let id = SnapshotId::new("00000003-0000001B-1").unwrap();
        assert_eq!(id.as_str(), "00000003-0000001B-1");
        assert_eq!(id.to_string(), "00000003-0000001B-1");

        assert_eq!(SnapshotId::new(""), None);
        assert_eq!(SnapshotId::new("0000-1'; DROP TABLE users; --"), None);
        assert_eq!(SnapshotId::new(&"0".repeat(65)), None);
    }

    #[test]
    fn lowering_the_isolation_level_drops_the_snapshot() {
        let snapshot = SnapshotId::new("00000003-0000001B-1").unwrap();
        let config = TransactionConfig::builder()
            .repeatable_read()
            .snapshot(snapshot)
            .serializable()
            .build();
        assert_eq!(config.snapshot(), Some(snapshot));

        let config = config.isolation_level(IsolationLevel::ReadCommitted);
        assert_eq!(config.snapshot(), None);
    }
}
