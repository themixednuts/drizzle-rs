//! Type-level building blocks for snapshots, versions, and dialects.
//!
//! - [`Version`] and `V5`..`V8`: snapshot format versions as types.
//! - [`Upgradable`] and [`CanUpgrade`]: which version upgrades exist.
//! - [`Dialect`] with markers [`Sqlite`], [`Postgres`], [`Mysql`].
//! - [`Entity`], [`EntityKind`], [`EntityKey`]: DDL entity identity.

use std::fmt;
use std::hash::Hash;
use std::marker::PhantomData;

// Import dialect-specific types for associated type definitions
use crate::mysql::{MySQLDDL, MySQLEntity, MySQLSnapshot, statements::Generator as MySQLGenerator};
use crate::postgres::{
    PostgresDDL, PostgresSnapshot, ddl::PostgresEntity, statements::Generator as PostgresGenerator,
};
use crate::sqlite::{
    SQLiteDDL, SQLiteSnapshot, ddl::SqliteEntity, statements::Generator as SqliteGenerator,
};

// =============================================================================
// Version System
// =============================================================================

/// A snapshot format version, as a zero-sized type.
///
/// The number is available at compile time as [`NUMBER`](Self::NUMBER).
pub trait Version: Copy + Clone + Default + 'static {
    /// The version number (5, 6, 7, 8, ...).
    const NUMBER: u32;
}

/// Returns `V::NUMBER` as a string.
#[must_use]
pub fn version_str<V: Version>() -> String {
    V::NUMBER.to_string()
}

/// Snapshot format version 5.
#[derive(Copy, Clone, Default, Debug)]
pub struct V5;
impl Version for V5 {
    const NUMBER: u32 = 5;
}

/// Snapshot format version 6.
#[derive(Copy, Clone, Default, Debug)]
pub struct V6;
impl Version for V6 {
    const NUMBER: u32 = 6;
}

/// Snapshot format version 7.
#[derive(Copy, Clone, Default, Debug)]
pub struct V7;
impl Version for V7 {
    const NUMBER: u32 = 7;
}

/// Snapshot format version 8.
#[derive(Copy, Clone, Default, Debug)]
pub struct V8;
impl Version for V8 {
    const NUMBER: u32 = 8;
}

/// Latest SQLite snapshot version.
pub type SqliteLatest = V7;
/// Latest PostgreSQL snapshot version.
pub type PostgresLatest = V8;
/// Latest MySQL snapshot version.
pub type MysqlLatest = V6;

// =============================================================================
// Upgradable Trait
// =============================================================================

/// Upgrades a value from snapshot version `From` to version `To`.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::{Upgradable, V5, V6};
///
/// struct DocV5 { tables: Vec<String> }
/// struct DocV6 { tables: Vec<String>, views: Vec<String> }
///
/// impl Upgradable<V5, V6> for DocV5 {
///     type Output = DocV6;
///     type Error = std::convert::Infallible;
///
///     fn upgrade(self) -> Result<DocV6, Self::Error> {
///         Ok(DocV6 { tables: self.tables, views: Vec::new() })
///     }
/// }
///
/// let v6 = DocV5 { tables: vec!["users".into()] }.upgrade().unwrap();
/// assert!(v6.views.is_empty());
/// ```
pub trait Upgradable<From: Version, To: Version> {
    /// The upgraded value.
    type Output;
    /// Error returned when the upgrade fails.
    type Error;

    /// Performs the upgrade.
    ///
    /// # Errors
    ///
    /// Returns the implementation's [`Self::Error`] if the upgrade
    /// transformation fails (e.g., due to an unsupported input format or a
    /// corrupted snapshot).
    fn upgrade(self) -> Result<Self::Output, Self::Error>;
}

/// Compiles only if dialect `D` can upgrade from `From` to `To`.
///
/// Does nothing at runtime.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::{Sqlite, V5, V7, assert_can_upgrade};
///
/// assert_can_upgrade::<Sqlite, V5, V7>();
/// ```
///
/// # Compile-time checks
///
/// SQLite snapshots stop at version 7, so this does not compile:
///
/// ```compile_fail
/// use drizzle_migrations::{Sqlite, V7, V8, assert_can_upgrade};
///
/// assert_can_upgrade::<Sqlite, V7, V8>();
/// ```
#[inline]
pub const fn assert_can_upgrade<D, From, To>()
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

/// The kind of a DDL entity (table, column, index, ...).
///
/// [`as_str`](Self::as_str) gives the snapshot JSON name (`"tables"`,
/// `"fks"`, ...).
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum EntityKind {
    // Schema-level entities
    /// A PostgreSQL schema (`"schemas"`).
    Schema = 0,
    /// An enum type (`"enums"`).
    Enum = 1,
    /// A sequence (`"sequences"`).
    Sequence = 2,
    /// A role (`"roles"`).
    Role = 3,

    // Table-level entities
    /// A table (`"tables"`).
    Table = 10,
    /// A column (`"columns"`).
    Column = 11,
    /// An index (`"indexes"`).
    Index = 12,
    /// A foreign key (`"fks"`).
    ForeignKey = 13,
    /// A primary key (`"pks"`).
    PrimaryKey = 14,
    /// A unique constraint (`"uniques"`).
    UniqueConstraint = 15,
    /// A check constraint (`"checks"`).
    CheckConstraint = 16,

    // Other entities
    /// A row-level security policy (`"policies"`).
    Policy = 20,
    /// A view (`"views"`).
    View = 21,
}

impl EntityKind {
    /// Returns the snapshot JSON name, e.g. `"tables"`.
    #[must_use]
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

    /// Parses a snapshot JSON name; `None` if unknown.
    #[must_use]
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

/// The key that identifies an entity within a snapshot.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum EntityKey {
    /// Simple name (e.g., table name, schema name)
    Simple(String),
    /// Two-part key (e.g., table.column)
    Composite2(String, String),
    /// Three-part key (e.g., schema.table.column for `PostgreSQL`)
    Composite3(String, String, String),
}

impl EntityKey {
    /// Creates a [`EntityKey::Simple`] key.
    pub fn simple(name: impl Into<String>) -> Self {
        Self::Simple(name.into())
    }

    /// Creates a [`EntityKey::Composite2`] key.
    pub fn composite2(a: impl Into<String>, b: impl Into<String>) -> Self {
        Self::Composite2(a.into(), b.into())
    }

    /// Creates a [`EntityKey::Composite3`] key.
    pub fn composite3(a: impl Into<String>, b: impl Into<String>, c: impl Into<String>) -> Self {
        Self::Composite3(a.into(), b.into(), c.into())
    }
}

/// A DDL entity type (table, column, index, ...) with a fixed [`EntityKind`].
pub trait Entity: Clone + PartialEq {
    /// The kind of this entity type.
    const KIND: EntityKind;

    /// Returns the key that identifies this entity.
    fn key(&self) -> EntityKey;

    /// Returns the owning entity's key (for example the table of a column),
    /// or `None` (the default).
    fn parent_key(&self) -> Option<EntityKey> {
        None
    }
}

// =============================================================================
// Versioned Snapshot
// =============================================================================

/// A value tagged with a snapshot version `V` at the type level.
///
/// Keeps data of different versions from being mixed up.
#[derive(Clone, Debug)]
pub struct Versioned<Data, V: Version> {
    /// The actual snapshot data
    pub data: Data,
    /// Phantom marker for version
    _version: PhantomData<V>,
}

impl<Data, V: Version> Versioned<Data, V> {
    /// Wraps `data` as version `V`.
    pub const fn new(data: Data) -> Self {
        Self {
            data,
            _version: PhantomData,
        }
    }

    /// Returns `V::NUMBER`.
    #[must_use]
    pub const fn version() -> u32 {
        V::NUMBER
    }

    /// Returns `V::NUMBER` as a string.
    #[must_use]
    pub fn version_str() -> String {
        version_str::<V>()
    }

    /// Returns the wrapped data.
    pub fn into_inner(self) -> Data {
        self.data
    }
}

// =============================================================================
// Dialect Trait
// =============================================================================

/// A database dialect at the type level, with its snapshot, DDL, and
/// generator types.
///
/// Implemented by [`Sqlite`], [`Postgres`], and [`Mysql`]. Re-exported at the
/// crate root as `DialectTrait`.
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::{DialectTrait, Postgres, Version};
///
/// fn version_range<D: DialectTrait>() -> (u32, u32) {
///     (D::MinVersion::NUMBER, D::LatestVersion::NUMBER)
/// }
///
/// assert_eq!(Postgres::NAME, "postgresql");
/// assert_eq!(version_range::<Postgres>(), (5, 8));
/// ```
pub trait Dialect: Sized + 'static {
    /// Dialect name as written in snapshots (`"sqlite"`, `"postgresql"`,
    /// `"mysql"`).
    const NAME: &'static str;

    /// Oldest supported snapshot version.
    type MinVersion: Version;

    /// Current snapshot version.
    type LatestVersion: Version;

    /// Dialect-specific snapshot type
    type Snapshot: Clone + Default + std::fmt::Debug;

    /// Dialect-specific DDL collection type
    type DDL: Clone + Default + std::fmt::Debug;

    /// Dialect-specific entity enum (e.g., `SqliteEntity`, `PostgresEntity`)
    type Entity: Clone + std::fmt::Debug + PartialEq;

    /// Dialect-specific SQL generator
    type Generator: Default;

    /// Returns `true` if `version` is between the min and latest versions.
    #[inline]
    #[must_use]
    fn is_supported_version(version: u32) -> bool {
        version >= Self::MinVersion::NUMBER && version <= Self::LatestVersion::NUMBER
    }

    /// Returns `true` if `version` is the latest version.
    #[inline]
    #[must_use]
    fn is_latest_version(version: u32) -> bool {
        version == Self::LatestVersion::NUMBER
    }

    /// Returns `true` if `version` is supported but older than the latest.
    #[inline]
    #[must_use]
    fn needs_upgrade_from(version: u32) -> bool {
        version < Self::LatestVersion::NUMBER && version >= Self::MinVersion::NUMBER
    }

    /// Diffs two snapshots and returns the SQL statements.
    ///
    /// `breakpoints` is currently ignored by all built-in dialects.
    ///
    /// # Errors
    ///
    /// Returns a [`MigrationError`](crate::MigrationError) if the diff
    /// cannot be computed or rendered.
    fn diff_and_generate(
        prev: &Self::Snapshot,
        cur: &Self::Snapshot,
        breakpoints: bool,
    ) -> Result<DiffResult, crate::MigrationError>;
}

/// Marks that dialect `Self` can upgrade snapshots from `From` to `To`.
///
/// Use it as a bound so only valid upgrade paths compile. Built-in paths:
/// SQLite 5→6→7, PostgreSQL 5→6→7→8, MySQL 5→6 (plus the transitive pairs).
///
/// # Examples
///
/// ```rust
/// use drizzle_migrations::{CanUpgrade, Sqlite, V5, V7, Version, Versioned};
///
/// fn upgrade<D, From, To, T>(data: Versioned<T, From>) -> Versioned<T, To>
/// where
///     D: CanUpgrade<From, To>,
///     From: Version,
///     To: Version,
/// {
///     Versioned::new(data.into_inner())
/// }
///
/// let v7: Versioned<&str, V7> = upgrade::<Sqlite, _, _, _>(Versioned::<_, V5>::new("ddl"));
/// assert_eq!(Versioned::<&str, V7>::version(), 7);
/// # let _ = v7;
/// ```
pub trait CanUpgrade<From: Version, To: Version>: Dialect {}

// =============================================================================
// Dialect Operations Trait
// =============================================================================

/// Statements and warnings from [`Dialect::diff_and_generate`].
#[derive(Debug, Clone)]
pub struct DiffResult {
    /// Generated SQL statements
    pub sql_statements: Vec<String>,
    /// `true` when `sql_statements` is not empty.
    pub has_changes: bool,
    /// Structural warnings produced while planning the migration.
    pub warnings: Vec<String>,
}

impl DiffResult {
    /// Creates a result with no statements.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            sql_statements: Vec::new(),
            has_changes: false,
            warnings: Vec::new(),
        }
    }

    /// Creates a result from `sql_statements` with no warnings.
    #[must_use]
    pub const fn with_changes(sql_statements: Vec<String>) -> Self {
        let has_changes = !sql_statements.is_empty();
        Self {
            sql_statements,
            has_changes,
            warnings: Vec::new(),
        }
    }
}

impl From<crate::Plan> for DiffResult {
    fn from(plan: crate::Plan) -> Self {
        let has_changes = !plan.statements.is_empty();
        Self {
            sql_statements: plan.statements,
            has_changes,
            warnings: plan.warnings,
        }
    }
}

/// SQLite dialect marker (snapshot versions 5 to 7).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Sqlite;

impl Sqlite {
    /// Oldest supported snapshot version.
    pub const MIN_VERSION: u32 = V5::NUMBER;
    /// Current snapshot version.
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
        _breakpoints: bool,
    ) -> Result<DiffResult, crate::MigrationError> {
        crate::diff(
            &crate::Snapshot::Sqlite(prev.clone()),
            &crate::Snapshot::Sqlite(cur.clone()),
        )
        .map(Into::into)
    }
}

// Declare valid SQLite upgrade paths
impl CanUpgrade<V5, V6> for Sqlite {}
impl CanUpgrade<V6, V7> for Sqlite {}
// Transitive: V5 -> V7 requires going through V6
impl CanUpgrade<V5, V7> for Sqlite {}

/// PostgreSQL dialect marker (snapshot versions 5 to 8).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Postgres;

impl Postgres {
    /// Oldest supported snapshot version.
    pub const MIN_VERSION: u32 = V5::NUMBER;
    /// Current snapshot version.
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
        _breakpoints: bool,
    ) -> Result<DiffResult, crate::MigrationError> {
        crate::diff(
            &crate::Snapshot::Postgres(prev.clone()),
            &crate::Snapshot::Postgres(cur.clone()),
        )
        .map(Into::into)
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

/// MySQL dialect marker (snapshot versions 5 to 6).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Mysql;

impl Mysql {
    /// Oldest supported snapshot version.
    pub const MIN_VERSION: u32 = V5::NUMBER;
    /// Current snapshot version.
    pub const LATEST_VERSION: u32 = V6::NUMBER;
}

impl Dialect for Mysql {
    const NAME: &'static str = "mysql";
    type MinVersion = V5;
    type LatestVersion = V6;
    type Snapshot = MySQLSnapshot;
    type DDL = MySQLDDL;
    type Entity = MySQLEntity;
    type Generator = MySQLGenerator;

    fn diff_and_generate(
        prev: &Self::Snapshot,
        cur: &Self::Snapshot,
        _breakpoints: bool,
    ) -> Result<DiffResult, crate::MigrationError> {
        crate::diff(
            &crate::Snapshot::MySQL(prev.clone()),
            &crate::Snapshot::MySQL(cur.clone()),
        )
        .map(Into::into)
    }
}
impl CanUpgrade<V5, V6> for Mysql {}

// =============================================================================
// Diff Types
// =============================================================================

/// The kind of change a diff entry describes.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum DiffType {
    /// The entity is new.
    Create,
    /// The entity was removed.
    Drop,
    /// The entity changed.
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

        // MySQL: V5 to V6
        assert_eq!(Mysql::MIN_VERSION, 5);
        assert_eq!(Mysql::LATEST_VERSION, 6);
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

        assert_can_upgrade::<Mysql, V5, V6>();
    }

    #[test]
    fn mysql_typed_diff_delegates_to_the_fallible_shared_planner() {
        let previous = MySQLSnapshot::new();
        let mut invalid = MySQLSnapshot::new();
        invalid.add_entity(MySQLEntity::Column(crate::mysql::Column::new(
            "missing_table",
            "id",
            "bigint",
        )));

        let error = Mysql::diff_and_generate(&previous, &invalid, true)
            .expect_err("invalid public snapshots must return an error instead of panicking");
        assert!(error.to_string().contains("missing table"), "{error}");
    }

    // This test demonstrates a compile-time error if uncommented:
    // #[test]
    // fn test_invalid_upgrade_fails() {
    //     // This would fail to compile because Sqlite doesn't impl CanUpgrade<V5, V8>
    //     assert_can_upgrade::<Sqlite, V5, V8>();
    // }
}
