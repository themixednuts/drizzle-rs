//! Transaction-type marker and per-driver behavior conversions for `SQLite` drivers.

/// `SQLite` transaction types
#[derive(Default, Debug, Clone, Copy)]
pub enum SQLiteTransactionType {
    #[default]
    /// A deferred transaction is the default - it does not acquire locks until needed
    Deferred,
    /// An immediate transaction acquires a RESERVED lock immediately
    Immediate,
    /// An exclusive transaction acquires an EXCLUSIVE lock immediately
    Exclusive,
}

#[cfg(feature = "rusqlite")]
impl From<SQLiteTransactionType> for ::rusqlite::TransactionBehavior {
    fn from(tx_type: SQLiteTransactionType) -> Self {
        match tx_type {
            SQLiteTransactionType::Deferred => Self::Deferred,
            SQLiteTransactionType::Immediate => Self::Immediate,
            SQLiteTransactionType::Exclusive => Self::Exclusive,
        }
    }
}

#[cfg(feature = "rusqlite")]
impl From<::rusqlite::TransactionBehavior> for SQLiteTransactionType {
    fn from(behavior: ::rusqlite::TransactionBehavior) -> Self {
        match behavior {
            ::rusqlite::TransactionBehavior::Immediate => Self::Immediate,
            ::rusqlite::TransactionBehavior::Exclusive => Self::Exclusive,
            // Deferred and any future variants default to Deferred.
            _ => Self::Deferred,
        }
    }
}

// Convert to libsql::TransactionBehavior
#[cfg(feature = "libsql")]
impl From<SQLiteTransactionType> for libsql::TransactionBehavior {
    fn from(tx_type: SQLiteTransactionType) -> Self {
        match tx_type {
            SQLiteTransactionType::Deferred => Self::Deferred,
            SQLiteTransactionType::Immediate => Self::Immediate,
            SQLiteTransactionType::Exclusive => Self::Exclusive,
        }
    }
}

// Convert from libsql::TransactionBehavior
#[cfg(feature = "libsql")]
impl From<libsql::TransactionBehavior> for SQLiteTransactionType {
    fn from(behavior: libsql::TransactionBehavior) -> Self {
        match behavior {
            libsql::TransactionBehavior::Immediate => Self::Immediate,
            libsql::TransactionBehavior::Exclusive => Self::Exclusive,
            // Deferred and ReadOnly (mapped as closest equivalent) both fall through to Deferred.
            libsql::TransactionBehavior::Deferred | libsql::TransactionBehavior::ReadOnly => {
                Self::Deferred
            }
        }
    }
}

// Convert to turso::TransactionBehavior
#[cfg(feature = "turso")]
impl From<SQLiteTransactionType> for turso::transaction::TransactionBehavior {
    fn from(tx_type: SQLiteTransactionType) -> Self {
        match tx_type {
            SQLiteTransactionType::Deferred => Self::Deferred,
            SQLiteTransactionType::Immediate => Self::Immediate,
            SQLiteTransactionType::Exclusive => Self::Exclusive,
        }
    }
}

// Convert from turso::TransactionBehavior
#[cfg(feature = "turso")]
impl From<turso::transaction::TransactionBehavior> for SQLiteTransactionType {
    fn from(behavior: turso::transaction::TransactionBehavior) -> Self {
        match behavior {
            turso::transaction::TransactionBehavior::Immediate => Self::Immediate,
            turso::transaction::TransactionBehavior::Exclusive => Self::Exclusive,
            // Deferred and any future variants default to Deferred.
            _ => Self::Deferred,
        }
    }
}
