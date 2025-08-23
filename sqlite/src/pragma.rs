//! SQLite PRAGMA statements for database configuration and introspection
//!
//! This module provides type-safe, ergonomic access to SQLite's PRAGMA statements.
//! PRAGMA statements are SQL extension specific to SQLite and are used to modify 
//! the operation of the SQLite library or to query the SQLite library for internal 
//! (non-table) data.
//!
//! [SQLite PRAGMA Documentation](https://sqlite.org/pragma.html)
//!
//! ## Features
//!
//! - **Type Safety**: Enums for all pragma values (no string literals needed)
//! - **Ergonomic API**: Uses `&'static str` instead of `String` - no `.to_string()` calls
//! - **Documentation Links**: Each pragma links to official SQLite documentation
//! - **ToSQL Integration**: Seamless integration with the query builder
//!
//! ## Categories
//!
//! - **Configuration**: `foreign_keys`, `journal_mode`, `synchronous`, etc.
//! - **Introspection**: `table_info`, `index_list`, `compile_options`, etc.
//! - **Maintenance**: `integrity_check`, `optimize`, `quick_check`, etc.
//!
//! ## Examples
//!
//! ```
//! use drizzle_sqlite::pragma::{Pragma, JournalMode, AutoVacuum};
//! use drizzle_core::ToSQL;
//!
//! // Enable foreign key constraints
//! let pragma = Pragma::foreign_keys(true);
//! assert_eq!(pragma.to_sql().sql(), "PRAGMA foreign_keys = ON");
//!
//! // Set journal mode to WAL
//! let pragma = Pragma::journal_mode(JournalMode::Wal);
//! assert_eq!(pragma.to_sql().sql(), "PRAGMA journal_mode = WAL");
//!
//! // Get table schema information
//! let pragma = Pragma::table_info("users");
//! assert_eq!(pragma.to_sql().sql(), "PRAGMA table_info(users)");
//!
//! // Check database integrity
//! let pragma = Pragma::integrity_check(None);
//! assert_eq!(pragma.to_sql().sql(), "PRAGMA integrity_check");
//! ```

use drizzle_core::{ToSQL, sql::SQL};
use crate::values::SQLiteValue;

/// Auto-vacuum modes for SQLite databases
/// 
/// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_auto_vacuum)
#[derive(Debug, Clone, PartialEq)]
pub enum AutoVacuum {
    /// Disable auto-vacuum
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::AutoVacuum;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(AutoVacuum::None.to_sql().sql(), "NONE");
    /// ```
    None,
    
    /// Enable full auto-vacuum
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::AutoVacuum;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(AutoVacuum::Full.to_sql().sql(), "FULL");
    /// ```
    Full,
    
    /// Enable incremental auto-vacuum
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::AutoVacuum;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(AutoVacuum::Incremental.to_sql().sql(), "INCREMENTAL");
    /// ```
    Incremental,
}

/// Journal modes for SQLite databases
/// 
/// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_journal_mode)
#[derive(Debug, Clone, PartialEq)]
pub enum JournalMode {
    /// Delete journal file after each transaction
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::JournalMode;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(JournalMode::Delete.to_sql().sql(), "DELETE");
    /// ```
    Delete,
    
    /// Truncate journal file after each transaction
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::JournalMode;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(JournalMode::Truncate.to_sql().sql(), "TRUNCATE");
    /// ```
    Truncate,
    
    /// Keep journal file persistent
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::JournalMode;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(JournalMode::Persist.to_sql().sql(), "PERSIST");
    /// ```
    Persist,
    
    /// Store journal in memory
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::JournalMode;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(JournalMode::Memory.to_sql().sql(), "MEMORY");
    /// ```
    Memory,
    
    /// Write-Ahead Logging mode
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::JournalMode;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(JournalMode::Wal.to_sql().sql(), "WAL");
    /// ```
    Wal,
    
    /// Disable journaling
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::JournalMode;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(JournalMode::Off.to_sql().sql(), "OFF");
    /// ```
    Off,
}

/// Synchronous modes for SQLite databases
/// 
/// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_synchronous)
#[derive(Debug, Clone, PartialEq)]
pub enum Synchronous {
    /// No syncing - fastest but least safe
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Synchronous;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(Synchronous::Off.to_sql().sql(), "OFF");
    /// ```
    Off,
    
    /// Sync at critical moments - good balance
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Synchronous;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(Synchronous::Normal.to_sql().sql(), "NORMAL");
    /// ```
    Normal,
    
    /// Sync frequently - safest but slower
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Synchronous;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(Synchronous::Full.to_sql().sql(), "FULL");
    /// ```
    Full,
    
    /// Like FULL with additional syncing
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Synchronous;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(Synchronous::Extra.to_sql().sql(), "EXTRA");
    /// ```
    Extra,
}

/// Storage modes for temporary tables and indices
/// 
/// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_temp_store)
#[derive(Debug, Clone, PartialEq)]
pub enum TempStore {
    /// Use default storage mode
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::TempStore;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(TempStore::Default.to_sql().sql(), "DEFAULT");
    /// ```
    Default,
    
    /// Store temporary tables in files
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::TempStore;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(TempStore::File.to_sql().sql(), "FILE");
    /// ```
    File,
    
    /// Store temporary tables in memory
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::TempStore;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(TempStore::Memory.to_sql().sql(), "MEMORY");
    /// ```
    Memory,
}

/// Database locking modes
/// 
/// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_locking_mode)
#[derive(Debug, Clone, PartialEq)]
pub enum LockingMode {
    /// Normal locking mode - allows multiple readers
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::LockingMode;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(LockingMode::Normal.to_sql().sql(), "NORMAL");
    /// ```
    Normal,
    
    /// Exclusive locking mode - single connection only
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::LockingMode;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(LockingMode::Exclusive.to_sql().sql(), "EXCLUSIVE");
    /// ```
    Exclusive,
}

/// Secure delete modes for SQLite
/// 
/// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_secure_delete)
#[derive(Debug, Clone, PartialEq)]
pub enum SecureDelete {
    /// Disable secure delete
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::SecureDelete;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(SecureDelete::Off.to_sql().sql(), "OFF");
    /// ```
    Off,
    
    /// Enable secure delete - overwrite deleted data
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::SecureDelete;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(SecureDelete::On.to_sql().sql(), "ON");
    /// ```
    On,
    
    /// Fast secure delete - partial overwriting
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::SecureDelete;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(SecureDelete::Fast.to_sql().sql(), "FAST");
    /// ```
    Fast,
}

/// Encoding types for SQLite databases
/// 
/// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_encoding)
#[derive(Debug, Clone, PartialEq)]
pub enum Encoding {
    /// UTF-8 encoding
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Encoding;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(Encoding::Utf8.to_sql().sql(), "UTF-8");
    /// ```
    Utf8,
    
    /// UTF-16 little endian encoding
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Encoding;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(Encoding::Utf16Le.to_sql().sql(), "UTF-16LE");
    /// ```
    Utf16Le,
    
    /// UTF-16 big endian encoding
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Encoding;
    /// # use drizzle_core::ToSQL;
    /// assert_eq!(Encoding::Utf16Be.to_sql().sql(), "UTF-16BE");
    /// ```
    Utf16Be,
}

/// SQLite pragma statements for database configuration and introspection
#[derive(Debug, Clone, PartialEq)]
pub enum Pragma {
    // Read/Write Configuration Pragmas
    
    /// Set or query the 32-bit signed big-endian application ID
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_application_id)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::ApplicationId(12345);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA application_id = 12345");
    /// ```
    ApplicationId(i32),
    
    /// Query or set the auto-vacuum status in the database
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_auto_vacuum)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::{Pragma, AutoVacuum};
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::AutoVacuum(AutoVacuum::Full);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA auto_vacuum = FULL");
    /// ```
    AutoVacuum(AutoVacuum),
    
    /// Suggest maximum number of database disk pages in memory
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_cache_size)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::CacheSize(-2000);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA cache_size = -2000");
    /// ```
    CacheSize(i32),
    
    /// Query, set, or clear the enforcement of foreign key constraints
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_foreign_keys)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::ForeignKeys(true);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA foreign_keys = ON");
    /// 
    /// let pragma = Pragma::ForeignKeys(false);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA foreign_keys = OFF");
    /// ```
    ForeignKeys(bool),
    
    /// Query or set the journal mode for databases
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_journal_mode)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::{Pragma, JournalMode};
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::JournalMode(JournalMode::Wal);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA journal_mode = WAL");
    /// ```
    JournalMode(JournalMode),
    
    /// Control how aggressively SQLite will write data
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_synchronous)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::{Pragma, Synchronous};
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::Synchronous(Synchronous::Normal);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA synchronous = NORMAL");
    /// ```
    Synchronous(Synchronous),
    
    /// Query or set the storage mode used by temporary tables and indices
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_temp_store)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::{Pragma, TempStore};
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::TempStore(TempStore::Memory);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA temp_store = MEMORY");
    /// ```
    TempStore(TempStore),
    
    /// Query or set the database connection locking-mode
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_locking_mode)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::{Pragma, LockingMode};
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::LockingMode(LockingMode::Exclusive);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA locking_mode = EXCLUSIVE");
    /// ```
    LockingMode(LockingMode),
    
    /// Query or set the secure-delete setting
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_secure_delete)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::{Pragma, SecureDelete};
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::SecureDelete(SecureDelete::Fast);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA secure_delete = FAST");
    /// ```
    SecureDelete(SecureDelete),
    
    /// Set or get the user-version integer
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_user_version)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::UserVersion(42);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA user_version = 42");
    /// ```
    UserVersion(i32),
    
    /// Query or set the text encoding used by the database
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_encoding)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::{Pragma, Encoding};
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::Encoding(Encoding::Utf8);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA encoding = UTF-8");
    /// ```
    Encoding(Encoding),
    
    /// Query or set the database page size in bytes
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_page_size)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::PageSize(4096);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA page_size = 4096");
    /// ```
    PageSize(i32),
    
    /// Query or set the maximum memory map size
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_mmap_size)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::MmapSize(268435456);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA mmap_size = 268435456");
    /// ```
    MmapSize(i64),
    
    /// Enable or disable recursive trigger firing
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_recursive_triggers)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::RecursiveTriggers(true);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA recursive_triggers = ON");
    /// ```
    RecursiveTriggers(bool),
    
    // Read-Only Query Pragmas
    
    /// Return a list of collating sequences
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_collation_list)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::CollationList;
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA collation_list");
    /// ```
    CollationList,
    
    /// Return compile-time options used when building SQLite
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_compile_options)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::CompileOptions;
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA compile_options");
    /// ```
    CompileOptions,
    
    /// Return information about attached databases
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_database_list)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::DatabaseList;
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA database_list");
    /// ```
    DatabaseList,
    
    /// Return a list of SQL functions
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_function_list)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::FunctionList;
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA function_list");
    /// ```
    FunctionList,
    
    /// Return information about tables and views in the schema
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_table_list)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::TableList;
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA table_list");
    /// ```
    TableList,
    
    /// Return extended table information including hidden columns
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_table_xinfo)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::TableXInfo("users");
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA table_xinfo(users)");
    /// ```
    TableXInfo(&'static str),
    
    /// Return a list of available virtual table modules
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_module_list)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::ModuleList;
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA module_list");
    /// ```
    ModuleList,
    
    // Utility Pragmas
    
    /// Perform database integrity check
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_integrity_check)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// // Check entire database
    /// let pragma = Pragma::IntegrityCheck(None);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA integrity_check");
    /// 
    /// // Check specific table
    /// let pragma = Pragma::IntegrityCheck(Some("users"));
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA integrity_check(users)");
    /// ```
    IntegrityCheck(Option<&'static str>),
    
    /// Perform faster database integrity check
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_quick_check)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::QuickCheck(None);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA quick_check");
    /// 
    /// let pragma = Pragma::QuickCheck(Some("users"));
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA quick_check(users)");
    /// ```
    QuickCheck(Option<&'static str>),
    
    /// Attempt to optimize the database
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_optimize)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::Optimize(None);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA optimize");
    /// 
    /// let pragma = Pragma::Optimize(Some(0x10002));
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA optimize(65538)");
    /// ```
    Optimize(Option<u32>),
    
    /// Check foreign key constraints for a table
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_foreign_key_check)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::ForeignKeyCheck(None);
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA foreign_key_check");
    /// 
    /// let pragma = Pragma::ForeignKeyCheck(Some("orders"));
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA foreign_key_check(orders)");
    /// ```
    ForeignKeyCheck(Option<&'static str>),
    
    // Table-specific Pragmas
    
    /// Return information about table columns
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_table_info)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::TableInfo("users");
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA table_info(users)");
    /// ```
    TableInfo(&'static str),
    
    /// Return information about table indexes
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_index_list)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::IndexList("users");
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA index_list(users)");
    /// ```
    IndexList(&'static str),
    
    /// Return information about index columns
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_index_info)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::IndexInfo("idx_users_email");
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA index_info(idx_users_email)");
    /// ```
    IndexInfo(&'static str),
    
    /// Return foreign key information for a table
    /// 
    /// [SQLite Documentation](https://sqlite.org/pragma.html#pragma_foreign_key_list)
    /// 
    /// # Example
    /// ```
    /// # use drizzle_sqlite::pragma::Pragma;
    /// # use drizzle_core::ToSQL;
    /// let pragma = Pragma::ForeignKeyList("orders");
    /// assert_eq!(pragma.to_sql().sql(), "PRAGMA foreign_key_list(orders)");
    /// ```
    ForeignKeyList(&'static str),
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for AutoVacuum {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            AutoVacuum::None => SQL::raw("NONE"),
            AutoVacuum::Full => SQL::raw("FULL"),
            AutoVacuum::Incremental => SQL::raw("INCREMENTAL"),
        }
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for JournalMode {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            JournalMode::Delete => SQL::raw("DELETE"),
            JournalMode::Truncate => SQL::raw("TRUNCATE"),
            JournalMode::Persist => SQL::raw("PERSIST"),
            JournalMode::Memory => SQL::raw("MEMORY"),
            JournalMode::Wal => SQL::raw("WAL"),
            JournalMode::Off => SQL::raw("OFF"),
        }
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for Synchronous {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            Synchronous::Off => SQL::raw("OFF"),
            Synchronous::Normal => SQL::raw("NORMAL"),
            Synchronous::Full => SQL::raw("FULL"),
            Synchronous::Extra => SQL::raw("EXTRA"),
        }
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for TempStore {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            TempStore::Default => SQL::raw("DEFAULT"),
            TempStore::File => SQL::raw("FILE"),
            TempStore::Memory => SQL::raw("MEMORY"),
        }
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for LockingMode {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            LockingMode::Normal => SQL::raw("NORMAL"),
            LockingMode::Exclusive => SQL::raw("EXCLUSIVE"),
        }
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for SecureDelete {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            SecureDelete::Off => SQL::raw("OFF"),
            SecureDelete::On => SQL::raw("ON"),
            SecureDelete::Fast => SQL::raw("FAST"),
        }
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for Encoding {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            Encoding::Utf8 => SQL::raw("UTF-8"),
            Encoding::Utf16Le => SQL::raw("UTF-16LE"),
            Encoding::Utf16Be => SQL::raw("UTF-16BE"),
        }
    }
}

impl<'a> ToSQL<'a, SQLiteValue<'a>> for Pragma {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            // Read/Write Configuration Pragmas
            Pragma::ApplicationId(id) => SQL::raw(format!("PRAGMA application_id = {}", id)),
            Pragma::AutoVacuum(mode) => SQL::raw("PRAGMA auto_vacuum = ").append(mode.to_sql()),
            Pragma::CacheSize(size) => SQL::raw(format!("PRAGMA cache_size = {}", size)),
            Pragma::ForeignKeys(enabled) => {
                SQL::raw("PRAGMA foreign_keys = ").append(SQL::raw(if *enabled { "ON" } else { "OFF" }))
            }
            Pragma::JournalMode(mode) => SQL::raw("PRAGMA journal_mode = ").append(mode.to_sql()),
            Pragma::Synchronous(mode) => SQL::raw("PRAGMA synchronous = ").append(mode.to_sql()),
            Pragma::TempStore(store) => SQL::raw("PRAGMA temp_store = ").append(store.to_sql()),
            Pragma::LockingMode(mode) => SQL::raw("PRAGMA locking_mode = ").append(mode.to_sql()),
            Pragma::SecureDelete(mode) => SQL::raw("PRAGMA secure_delete = ").append(mode.to_sql()),
            Pragma::UserVersion(version) => SQL::raw(format!("PRAGMA user_version = {}", version)),
            Pragma::Encoding(encoding) => SQL::raw("PRAGMA encoding = ").append(encoding.to_sql()),
            Pragma::PageSize(size) => SQL::raw(format!("PRAGMA page_size = {}", size)),
            Pragma::MmapSize(size) => SQL::raw(format!("PRAGMA mmap_size = {}", size)),
            Pragma::RecursiveTriggers(enabled) => {
                SQL::raw("PRAGMA recursive_triggers = ").append(SQL::raw(if *enabled { "ON" } else { "OFF" }))
            }
            
            // Read-Only Query Pragmas
            Pragma::CollationList => SQL::raw("PRAGMA collation_list"),
            Pragma::CompileOptions => SQL::raw("PRAGMA compile_options"),
            Pragma::DatabaseList => SQL::raw("PRAGMA database_list"),
            Pragma::FunctionList => SQL::raw("PRAGMA function_list"),
            Pragma::TableList => SQL::raw("PRAGMA table_list"),
            Pragma::TableXInfo(table) => SQL::raw(format!("PRAGMA table_xinfo({})", table)),
            Pragma::ModuleList => SQL::raw("PRAGMA module_list"),
            
            // Utility Pragmas
            Pragma::IntegrityCheck(table) => match table {
                Some(t) => SQL::raw(format!("PRAGMA integrity_check({})", t)),
                None => SQL::raw("PRAGMA integrity_check"),
            },
            Pragma::QuickCheck(table) => match table {
                Some(t) => SQL::raw(format!("PRAGMA quick_check({})", t)),
                None => SQL::raw("PRAGMA quick_check"),
            },
            Pragma::Optimize(mask) => match mask {
                Some(m) => SQL::raw(format!("PRAGMA optimize({})", m)),
                None => SQL::raw("PRAGMA optimize"),
            },
            Pragma::ForeignKeyCheck(table) => match table {
                Some(t) => SQL::raw(format!("PRAGMA foreign_key_check({})", t)),
                None => SQL::raw("PRAGMA foreign_key_check"),
            },
            
            // Table-specific Pragmas
            Pragma::TableInfo(table) => SQL::raw(format!("PRAGMA table_info({})", table)),
            Pragma::IndexList(table) => SQL::raw(format!("PRAGMA index_list({})", table)),
            Pragma::IndexInfo(index) => SQL::raw(format!("PRAGMA index_info({})", index)),
            Pragma::ForeignKeyList(table) => SQL::raw(format!("PRAGMA foreign_key_list({})", table)),
        }
    }
}

impl Pragma {
    /// Create a PRAGMA query to get the current value (read-only operation)
    pub fn query(pragma_name: &str) -> SQL<'static, SQLiteValue<'static>> {
        SQL::raw(format!("PRAGMA {}", pragma_name))
    }
    
    /// Convenience constructor for foreign_keys pragma
    pub fn foreign_keys(enabled: bool) -> Self {
        Self::ForeignKeys(enabled)
    }
    
    /// Convenience constructor for journal_mode pragma
    pub fn journal_mode(mode: JournalMode) -> Self {
        Self::JournalMode(mode)
    }
    
    /// Convenience constructor for table_info pragma
    pub fn table_info(table: &'static str) -> Self {
        Self::TableInfo(table)
    }
    
    /// Convenience constructor for index_list pragma
    pub fn index_list(table: &'static str) -> Self {
        Self::IndexList(table)
    }
    
    /// Convenience constructor for foreign_key_list pragma
    pub fn foreign_key_list(table: &'static str) -> Self {
        Self::ForeignKeyList(table)
    }
    
    /// Convenience constructor for integrity_check pragma
    pub fn integrity_check(table: Option<&'static str>) -> Self {
        Self::IntegrityCheck(table)
    }
    
    /// Convenience constructor for foreign_key_check pragma
    pub fn foreign_key_check(table: Option<&'static str>) -> Self {
        Self::ForeignKeyCheck(table)
    }
    
    /// Convenience constructor for table_xinfo pragma
    pub fn table_xinfo(table: &'static str) -> Self {
        Self::TableXInfo(table)
    }
    
    /// Convenience constructor for encoding pragma
    pub fn encoding(encoding: Encoding) -> Self {
        Self::Encoding(encoding)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_pragma_helper() {
        // Test the static query helper function - not covered in doc tests
        assert_eq!(
            Pragma::query("foreign_keys").sql(),
            "PRAGMA foreign_keys"
        );
        assert_eq!(
            Pragma::query("custom_pragma").sql(),
            "PRAGMA custom_pragma"
        );
    }

    #[test]
    fn test_convenience_constructor_integration() {
        // Test that convenience constructors work the same as direct construction
        assert_eq!(
            Pragma::foreign_keys(true).to_sql().sql(),
            Pragma::ForeignKeys(true).to_sql().sql()
        );
        assert_eq!(
            Pragma::table_info("users").to_sql().sql(),
            Pragma::TableInfo("users").to_sql().sql()
        );
        assert_eq!(
            Pragma::encoding(Encoding::Utf8).to_sql().sql(),
            Pragma::Encoding(Encoding::Utf8).to_sql().sql()
        );
    }
}