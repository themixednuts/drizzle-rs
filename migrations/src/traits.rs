//! Core traits for type-safe migration infrastructure
//!
//! This module provides idiomatic Rust traits replacing TypeScript patterns:
//! - `Version` - Type-safe version markers with const NUMBER
//! - `Upgradable` - Trait for upgrading snapshots between versions
//! - `Entity` - Trait for DDL entities with const KIND
//! - `EntityKind` - Enum replacing string entity_type discrimination

use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

// Import dialect-specific types for associated type definitions
use crate::postgres::{
    PostgresDDL, PostgresSnapshot, ddl::PostgresEntity, statements::PostgresGenerator,
};
use crate::sqlite::{SQLiteDDL, SQLiteSnapshot, ddl::SqliteEntity, statements::SqliteGenerator};

// =============================================================================
// Version System
// =============================================================================

/// Type-safe version marker trait.
///
/// Each schema version is represented as a zero-sized type implementing this trait.
/// The version number is available at compile time via the associated constant.
pub trait Version: Copy + Clone + Default + 'static {
    /// The version number (5, 6, 7, 8, etc.)
    const NUMBER: u32;
}

/// Helper to get version as string at runtime
pub fn version_str<V: Version>() -> String {
    V::NUMBER.to_string()
}

/// Version 5 marker
#[derive(Copy, Clone, Default, Debug)]
pub struct V5;
impl Version for V5 {
    const NUMBER: u32 = 5;
}

/// Version 6 marker
#[derive(Copy, Clone, Default, Debug)]
pub struct V6;
impl Version for V6 {
    const NUMBER: u32 = 6;
}

/// Version 7 marker
#[derive(Copy, Clone, Default, Debug)]
pub struct V7;
impl Version for V7 {
    const NUMBER: u32 = 7;
}

/// Version 8 marker
#[derive(Copy, Clone, Default, Debug)]
pub struct V8;
impl Version for V8 {
    const NUMBER: u32 = 8;
}

/// Latest version aliases per dialect
pub type SqliteLatest = V7;
pub type PostgresLatest = V8;
pub type MysqlLatest = V5;

// =============================================================================
// Upgradable Trait
// =============================================================================

/// Trait for upgrading a snapshot from one version to another.
///
/// Implementations are provided for each dialect's version transitions.
/// The trait is generic over the snapshot type `S`, source version `From`,
/// and target version `To`.
///
/// # Example
/// ```ignore
/// impl Upgradable<V5, V6> for SqliteSnapshot<V5> {
///     type Output = SqliteSnapshot<V6>;
///     type Error = UpgradeError;
///
///     fn upgrade(self) -> Result<Self::Output, Self::Error> {
///         // Transform v5 -> v6
///     }
/// }
/// ```
pub trait Upgradable<From: Version, To: Version> {
    /// The output snapshot type (same shape, different version)
    type Output;
    /// Error type for upgrade failures
    type Error;

    /// Perform the upgrade transformation
    fn upgrade(self) -> Result<Self::Output, Self::Error>;
}

/// Type-level version comparison.
///
/// This trait enables compile-time assertions about version ordering.
/// For example, you can require `From: VersionLt<To>` to ensure From < To.
pub trait VersionLt<Other: Version>: Version {}

// Declare version ordering relationships
impl VersionLt<V6> for V5 {}
impl VersionLt<V7> for V5 {}
impl VersionLt<V8> for V5 {}
impl VersionLt<V7> for V6 {}
impl VersionLt<V8> for V6 {}
impl VersionLt<V8> for V7 {}

/// Type-safe upgrade function that only compiles for valid upgrade paths.
///
/// This function leverages the `CanUpgrade` trait to enforce at compile time
/// that the specified dialect supports the given version transition.
///
/// # Example
/// ```ignore
/// use drizzle_migrations::{Sqlite, V5, V7, CanUpgrade, Versioned};
///
/// // This compiles because Sqlite: CanUpgrade<V5, V7>
/// fn upgrade_sqlite_snapshot<D>(data: Versioned<MyData, V5>) -> Versioned<MyData, V7>
/// where
///     D: CanUpgrade<V5, V7>,
/// {
///     // Perform the upgrade
///     Versioned::new(data.into_inner())
/// }
/// ```
#[inline]
pub fn assert_can_upgrade<D, From, To>()
where
    D: CanUpgrade<From, To>,
    From: Version,
    To: Version,
{
    // This function exists to provide a clear compile-time error
    // when an invalid upgrade path is attempted.
}

// =============================================================================
// Entity System
// =============================================================================

/// Entity kind discriminator enum.
///
/// Replaces string-based `entity_type` fields with a proper enum.
/// Uses `#[repr(u8)]` for efficient storage and comparison.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EntityKind {
    // Schema-level entities
    Schema = 0,
    Enum = 1,
    Sequence = 2,
    Role = 3,

    // Table-level entities
    Table = 10,
    Column = 11,
    Index = 12,
    ForeignKey = 13,
    PrimaryKey = 14,
    UniqueConstraint = 15,
    CheckConstraint = 16,

    // Other entities
    Policy = 20,
    View = 21,
}

impl EntityKind {
    /// Get the string representation for JSON serialization compatibility
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Schema => "schemas",
            Self::Enum => "enums",
            Self::Sequence => "sequences",
            Self::Role => "roles",
            Self::Table => "tables",
            Self::Column => "columns",
            Self::Index => "indexes",
            Self::ForeignKey => "fks",
            Self::PrimaryKey => "pks",
            Self::UniqueConstraint => "uniques",
            Self::CheckConstraint => "checks",
            Self::Policy => "policies",
            Self::View => "views",
        }
    }

    /// Parse from string (for deserialization)
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "schemas" => Some(Self::Schema),
            "enums" => Some(Self::Enum),
            "sequences" => Some(Self::Sequence),
            "roles" => Some(Self::Role),
            "tables" => Some(Self::Table),
            "columns" => Some(Self::Column),
            "indexes" => Some(Self::Index),
            "fks" => Some(Self::ForeignKey),
            "pks" => Some(Self::PrimaryKey),
            "uniques" => Some(Self::UniqueConstraint),
            "checks" => Some(Self::CheckConstraint),
            "policies" => Some(Self::Policy),
            "views" => Some(Self::View),
            _ => None,
        }
    }
}

impl std::str::FromStr for EntityKind {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s).ok_or(())
    }
}

impl fmt::Display for EntityKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Entity key types for unique identification
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityKey {
    /// Simple name (e.g., table name, schema name)
    Simple(String),
    /// Two-part key (e.g., table.column)
    Composite2(String, String),
    /// Three-part key (e.g., schema.table.column for PostgreSQL)
    Composite3(String, String, String),
}

impl EntityKey {
    pub fn simple(name: impl Into<String>) -> Self {
        Self::Simple(name.into())
    }

    pub fn composite2(a: impl Into<String>, b: impl Into<String>) -> Self {
        Self::Composite2(a.into(), b.into())
    }

    pub fn composite3(a: impl Into<String>, b: impl Into<String>, c: impl Into<String>) -> Self {
        Self::Composite3(a.into(), b.into(), c.into())
    }
}

/// Trait for DDL entities.
///
/// All DDL entity types (Table, Column, Index, etc.) implement this trait.
/// The `KIND` constant enables compile-time entity type discrimination.
pub trait Entity: Clone + PartialEq {
    /// The entity kind (discriminator)
    const KIND: EntityKind;

    /// Get the unique key for this entity
    fn key(&self) -> EntityKey;

    /// Get the parent entity key (if this entity belongs to a parent)
    fn parent_key(&self) -> Option<EntityKey> {
        None
    }
}

// =============================================================================
// Versioned Snapshot
// =============================================================================

/// A snapshot with compile-time version tracking.
///
/// Wraps snapshot data with a phantom type parameter for the version.
/// This enables type-safe upgrade chains and prevents accidental version mixing.
#[derive(Clone, Debug)]
pub struct Versioned<Data, V: Version> {
    /// The actual snapshot data
    pub data: Data,
    /// Phantom marker for version
    _version: PhantomData<V>,
}

impl<Data, V: Version> Versioned<Data, V> {
    /// Create a new versioned wrapper
    pub fn new(data: Data) -> Self {
        Self {
            data,
            _version: PhantomData,
        }
    }

    /// Get the version number
    pub fn version() -> u32 {
        V::NUMBER
    }

    /// Get the version as a string
    pub fn version_str() -> String {
        version_str::<V>()
    }

    /// Unwrap to get the inner data
    pub fn into_inner(self) -> Data {
        self.data
    }
}

// =============================================================================
// Dialect Trait
// =============================================================================

/// Trait representing a database dialect.
///
/// This trait uses associated types to provide compile-time type safety across
/// dialect-specific operations. Both `MinVersion` and `LatestVersion` are
/// associated types implementing `Version`, enabling const operations.
///
/// # Example
/// ```ignore
/// fn process<D: Dialect>() {
///     // All const at compile time
///     const MIN: u32 = D::MinVersion::NUMBER;
///     const LATEST: u32 = D::LatestVersion::NUMBER;
///     println!("Processing {} (v{} to v{})", D::NAME, MIN, LATEST);
/// }
/// ```
pub trait Dialect: Sized + 'static {
    /// Display name of the dialect
    const NAME: &'static str;

    /// Minimum supported snapshot version (as a type)
    type MinVersion: Version;

    /// Latest/current snapshot version (as a type)
    type LatestVersion: Version;

    /// Dialect-specific snapshot type
    type Snapshot: Clone + Default + std::fmt::Debug;

    /// Dialect-specific DDL collection type
    type DDL: Clone + Default + std::fmt::Debug;

    /// Dialect-specific entity enum (e.g., SqliteEntity, PostgresEntity)
    type Entity: Clone + std::fmt::Debug + PartialEq;

    /// Dialect-specific SQL generator
    type Generator: Default;

    /// Check if a version number is supported
    #[inline]
    fn is_supported_version(version: u32) -> bool {
        version >= Self::MinVersion::NUMBER && version <= Self::LatestVersion::NUMBER
    }

    /// Check if a version is the latest
    #[inline]
    fn is_latest_version(version: u32) -> bool {
        version == Self::LatestVersion::NUMBER
    }

    /// Check if a version needs upgrade
    #[inline]
    fn needs_upgrade_from(version: u32) -> bool {
        version < Self::LatestVersion::NUMBER && version >= Self::MinVersion::NUMBER
    }

    /// Diff two snapshots and generate SQL migration statements
    fn diff_and_generate(
        prev: &Self::Snapshot,
        cur: &Self::Snapshot,
        breakpoints: bool,
    ) -> MigrationResult;
}

/// Marker trait for compile-time upgrade path validation.
///
/// Implement this trait to declare that a dialect supports upgrading from
/// version `From` to version `To`. The compiler will enforce that only
/// valid upgrade paths are used.
///
/// # Example
/// ```ignore
/// // Declare SQLite can upgrade V5 -> V6
/// impl CanUpgrade<V5, V6> for Sqlite {}
/// impl CanUpgrade<V6, V7> for Sqlite {}
///
/// // This function only compiles if the upgrade is valid
/// fn upgrade<D, From, To>(data: Versioned<Data, From>) -> Versioned<Data, To>
/// where
///     D: Dialect + CanUpgrade<From, To>,
///     From: Version,
///     To: Version,
/// {
///     // ...
/// }
/// ```
pub trait CanUpgrade<From: Version, To: Version>: Dialect {}

// =============================================================================
// Dialect Operations Trait
// =============================================================================

/// Migration result from diffing two snapshots
#[derive(Debug, Clone)]
pub struct MigrationResult {
    /// Generated SQL statements
    pub sql_statements: Vec<String>,
    /// Whether there are any changes
    pub has_changes: bool,
}

impl MigrationResult {
    /// Create an empty result (no changes)
    pub fn empty() -> Self {
        Self {
            sql_statements: Vec::new(),
            has_changes: false,
        }
    }

    /// Create a result with changes
    pub fn with_changes(sql_statements: Vec<String>) -> Self {
        let has_changes = !sql_statements.is_empty();
        Self {
            sql_statements,
            has_changes,
        }
    }
}

/// SQLite dialect marker type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sqlite;

impl Sqlite {
    /// Minimum supported snapshot version (inherent alias)
    pub const MIN_VERSION: u32 = V5::NUMBER;
    /// Latest snapshot version (inherent alias)
    pub const LATEST_VERSION: u32 = V7::NUMBER;
}

impl Dialect for Sqlite {
    const NAME: &'static str = "sqlite";
    type MinVersion = V5;
    type LatestVersion = V7;
    type Snapshot = SQLiteSnapshot;
    type DDL = SQLiteDDL;
    type Entity = SqliteEntity;
    type Generator = SqliteGenerator;

    fn diff_and_generate(
        prev: &Self::Snapshot,
        cur: &Self::Snapshot,
        breakpoints: bool,
    ) -> MigrationResult {
        let diff = crate::sqlite::diff_snapshots(prev, cur);
        if !diff.has_changes() {
            return MigrationResult::empty();
        }
        let generator = SqliteGenerator::new().with_breakpoints(breakpoints);
        let sql = generator.generate_migration(&diff);
        MigrationResult::with_changes(sql)
    }
}

// Declare valid SQLite upgrade paths
impl CanUpgrade<V5, V6> for Sqlite {}
impl CanUpgrade<V6, V7> for Sqlite {}
// Transitive: V5 -> V7 requires going through V6
impl CanUpgrade<V5, V7> for Sqlite {}

/// PostgreSQL dialect marker type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Postgres;

impl Postgres {
    /// Minimum supported snapshot version (inherent alias)
    pub const MIN_VERSION: u32 = V5::NUMBER;
    /// Latest snapshot version (inherent alias)
    pub const LATEST_VERSION: u32 = V8::NUMBER;
}

impl Dialect for Postgres {
    const NAME: &'static str = "postgresql";
    type MinVersion = V5;
    type LatestVersion = V8;
    type Snapshot = PostgresSnapshot;
    type DDL = PostgresDDL;
    type Entity = PostgresEntity;
    type Generator = PostgresGenerator;

    fn diff_and_generate(
        prev: &Self::Snapshot,
        cur: &Self::Snapshot,
        breakpoints: bool,
    ) -> MigrationResult {
        let diff = crate::postgres::diff_snapshots(&prev.ddl, &cur.ddl);
        if !diff.has_changes() {
            return MigrationResult::empty();
        }
        let generator = PostgresGenerator::new().with_breakpoints(breakpoints);
        let sql = generator.generate(&diff.diffs);
        MigrationResult::with_changes(sql)
    }
}

// Declare valid PostgreSQL upgrade paths
impl CanUpgrade<V5, V6> for Postgres {}
impl CanUpgrade<V6, V7> for Postgres {}
impl CanUpgrade<V7, V8> for Postgres {}
// Transitive paths
impl CanUpgrade<V5, V7> for Postgres {}
impl CanUpgrade<V5, V8> for Postgres {}
impl CanUpgrade<V6, V8> for Postgres {}

/// MySQL dialect marker type
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mysql;

impl Mysql {
    /// Minimum supported snapshot version (inherent alias)
    pub const MIN_VERSION: u32 = V5::NUMBER;
    /// Latest snapshot version (inherent alias)
    pub const LATEST_VERSION: u32 = V5::NUMBER;
}

impl Dialect for Mysql {
    const NAME: &'static str = "mysql";
    type MinVersion = V5;
    type LatestVersion = V5;
    // MySQL not yet implemented - use placeholder types
    type Snapshot = ();
    type DDL = ();
    type Entity = ();
    type Generator = ();

    fn diff_and_generate(
        _prev: &Self::Snapshot,
        _cur: &Self::Snapshot,
        _breakpoints: bool,
    ) -> MigrationResult {
        unimplemented!("MySQL migrations not yet supported")
    }
}
// No upgrade paths for MySQL - already at latest

// =============================================================================
// Diff Types
// =============================================================================

/// Diff operation type
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DiffType {
    Create,
    Drop,
    Alter,
}

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use super::*;

    #[test]
    fn test_version_numbers() {
        assert_eq!(V5::NUMBER, 5);
        assert_eq!(V6::NUMBER, 6);
        assert_eq!(V7::NUMBER, 7);
        assert_eq!(V8::NUMBER, 8);
    }

    #[test]
    fn test_version_str() {
        assert_eq!(version_str::<V5>(), "5");
        assert_eq!(version_str::<V7>(), "7");
    }

    #[test]
    fn test_entity_kind_str() {
        assert_eq!(EntityKind::Table.as_str(), "tables");
        assert_eq!(EntityKind::Column.as_str(), "columns");
        assert_eq!(EntityKind::ForeignKey.as_str(), "fks");
    }

    #[test]
    fn test_entity_kind_parse() {
        assert_eq!(EntityKind::from_str("tables"), Ok(EntityKind::Table));
        assert_eq!(EntityKind::from_str("columns"), Ok(EntityKind::Column));
        assert_eq!(EntityKind::from_str("invalid"), Err(()));
    }

    #[test]
    fn test_versioned_snapshot() {
        #[derive(Clone, Debug)]
        struct TestData {
            value: i32,
        }

        let versioned: Versioned<TestData, V7> = Versioned::new(TestData { value: 42 });
        assert_eq!(Versioned::<TestData, V7>::version(), 7);
        assert_eq!(versioned.data.value, 42);
    }

    #[test]
    fn test_dialect_version_info() {
        // SQLite: V5 to V7 - using inherent consts (no trait needed)
        assert_eq!(Sqlite::MIN_VERSION, 5);
        assert_eq!(Sqlite::LATEST_VERSION, 7);

        // PostgreSQL: V5 to V8
        assert_eq!(Postgres::MIN_VERSION, 5);
        assert_eq!(Postgres::LATEST_VERSION, 8);

        // MySQL: V5 only
        assert_eq!(Mysql::MIN_VERSION, 5);
        assert_eq!(Mysql::LATEST_VERSION, 5);
    }

    #[test]
    fn test_dialect_version_checks() {
        // SQLite checks
        assert!(Sqlite::is_supported_version(5));
        assert!(Sqlite::is_supported_version(6));
        assert!(Sqlite::is_supported_version(7));
        assert!(!Sqlite::is_supported_version(4));
        assert!(!Sqlite::is_supported_version(8));

        assert!(Sqlite::needs_upgrade_from(5));
        assert!(Sqlite::needs_upgrade_from(6));
        assert!(!Sqlite::needs_upgrade_from(7));

        assert!(!Sqlite::is_latest_version(5));
        assert!(Sqlite::is_latest_version(7));
    }

    #[test]
    fn test_can_upgrade_compiles() {
        // These calls verify that the CanUpgrade impls exist
        // If they don't, this test won't compile
        assert_can_upgrade::<Sqlite, V5, V6>();
        assert_can_upgrade::<Sqlite, V6, V7>();
        assert_can_upgrade::<Sqlite, V5, V7>(); // Transitive

        assert_can_upgrade::<Postgres, V5, V6>();
        assert_can_upgrade::<Postgres, V6, V7>();
        assert_can_upgrade::<Postgres, V7, V8>();
        assert_can_upgrade::<Postgres, V5, V8>(); // Transitive
    }

    // This test demonstrates a compile-time error if uncommented:
    // #[test]
    // fn test_invalid_upgrade_fails() {
    //     // This would fail to compile because Sqlite doesn't impl CanUpgrade<V5, V8>
    //     assert_can_upgrade::<Sqlite, V5, V8>();
    // }
}
