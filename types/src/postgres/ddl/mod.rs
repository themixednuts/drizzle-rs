//! `PostgreSQL` schema objects (schemas, enums, sequences, roles, policies,
//! privileges, tables, columns, indexes, constraints, views) for migrations.
//!
//! Each object comes in two forms:
//!
//! - **`*Def` types** ([`TableDef`], [`ColumnDef`], ...) hold only `Copy`
//!   data (`&'static str`, `bool`, slices) so they can be built in `const`
//!   items. The schema macros generate these.
//! - **Runtime types** ([`Table`], [`Column`], ...) hold `Cow<'static, str>`
//!   and can be serialized with the `serde` feature. Migration snapshots
//!   store these, as a list of [`PostgresEntity`] values.
//!
//! Convert a definition with its `into_*` method or `From`. [`TableSql`]
//! renders `CREATE TABLE` and related statements.
//!
//! Compared with `SQLite`, `PostgreSQL` adds schemas, enum types, sequences,
//! roles, row-level security policies, privileges, identity columns, and
//! index options such as operator classes and `NULLS FIRST`/`LAST`.
//!
//! # Examples
//!
//! ```
//! use drizzle_types::postgres::ddl::{ColumnDef, Table, TableDef};
//!
//! const USERS: TableDef = TableDef::new("public", "users");
//! const COLUMNS: &[ColumnDef] = &[
//!     ColumnDef::new("public", "users", "id", "integer").not_null(),
//!     ColumnDef::new("public", "users", "name", "text").default_value("'anonymous'"),
//! ];
//!
//! let table: Table = USERS.into_table();
//! assert_eq!(table.schema(), "public");
//! # let _ = COLUMNS;
//! ```

mod check_constraint;
mod column;
mod enum_type;
mod foreign_key;
mod index;
mod policy;
mod primary_key;
mod privilege;
mod role;
mod schema;
mod sequence;
pub mod sql;
mod unique_constraint;
mod view;

// Const-friendly definition types
pub use check_constraint::CheckConstraintDef;
pub use column::{ColumnDef, GeneratedDef, GeneratedType, IdentityDef, IdentityType};
pub use enum_type::EnumDef;
pub use foreign_key::{ForeignKeyDef, ReferentialAction};
pub use index::{IndexColumn, IndexColumnDef, IndexDef, OpclassDef};
pub use policy::PolicyDef;
pub use primary_key::PrimaryKeyDef;
pub use privilege::{PrivilegeDef, PrivilegeType};
pub use role::RoleDef;
pub use schema::SchemaDef;
pub use sequence::SequenceDef;
pub use unique_constraint::UniqueConstraintDef;
pub use view::{ViewDef, ViewWithOptionDef};

// Runtime types for serde
pub use check_constraint::CheckConstraint;
pub use column::{Column, Generated, Identity};
pub use enum_type::Enum;
pub use foreign_key::ForeignKey;
pub use index::{Index, Opclass};
pub use policy::Policy;
pub use primary_key::PrimaryKey;
pub use privilege::Privilege;
pub use role::Role;
pub use schema::Schema;
pub use sequence::Sequence;
pub use unique_constraint::UniqueConstraint;
pub use view::{View, ViewWithOption};

// SQL generation
pub use sql::TableSql;

#[cfg(feature = "serde")]
pub use crate::serde_helpers::{
    cow_from_string, cow_option_from_string, cow_option_vec_from_strings, cow_vec_from_strings,
};

// =============================================================================
// Entity Type Constants (for compatibility with migrations)
// =============================================================================

/// Entity type discriminator for schemas
pub const ENTITY_TYPE_SCHEMAS: &str = "schemas";
/// Entity type discriminator for enums
pub const ENTITY_TYPE_ENUMS: &str = "enums";
/// Entity type discriminator for sequences
pub const ENTITY_TYPE_SEQUENCES: &str = "sequences";
/// Entity type discriminator for roles
pub const ENTITY_TYPE_ROLES: &str = "roles";
/// Entity type discriminator for policies
pub const ENTITY_TYPE_POLICIES: &str = "policies";
/// Entity type discriminator for tables
pub const ENTITY_TYPE_TABLES: &str = "tables";
/// Entity type discriminator for columns
pub const ENTITY_TYPE_COLUMNS: &str = "columns";
/// Entity type discriminator for indexes
pub const ENTITY_TYPE_INDEXES: &str = "indexes";
/// Entity type discriminator for foreign keys
pub const ENTITY_TYPE_FKS: &str = "fks";
/// Entity type discriminator for primary keys
pub const ENTITY_TYPE_PKS: &str = "pks";
/// Entity type discriminator for unique constraints
pub const ENTITY_TYPE_UNIQUES: &str = "uniques";
/// Entity type discriminator for check constraints
pub const ENTITY_TYPE_CHECKS: &str = "checks";
/// Entity type discriminator for views
pub const ENTITY_TYPE_VIEWS: &str = "views";
/// Entity type discriminator for privileges
pub const ENTITY_TYPE_PRIVILEGES: &str = "privileges";

mod table;

// Re-export Table types
pub use table::{Table, TableDef};

// =============================================================================
// Snapshot DDL channel for schema items
// =============================================================================

/// Const DDL metadata channel from `#[Postgres*]` schema items to the
/// `PostgresSchema` derive's runtime `to_snapshot()`.
///
/// Every `PostgreSQL` schema-item macro (table, index, enum, view, policy)
/// implements this trait; the defaults mean "no metadata of that kind". The
/// schema derive reads these consts so the runtime snapshot carries the same
/// fidelity as the compile-time DDL consts (identity sequence options, index
/// `method`/`where`/`concurrently`, `NULLS NOT DISTINCT`, enum schemas, one
/// composite `PrimaryKey` entity per table) without re-deriving any of it.
pub trait PostgresItemDdl {
    /// Column definitions in declaration order (tables only).
    const SNAPSHOT_COLUMNS: &'static [ColumnDef] = &[];
    /// The table's single (possibly composite) primary-key entity.
    const SNAPSHOT_PRIMARY_KEY: Option<PrimaryKeyDef> = None;
    /// Unique constraints (column-level and table-level) in declaration order.
    const SNAPSHOT_UNIQUE_CONSTRAINTS: &'static [UniqueConstraintDef] = &[];
    /// The index definition (index items only).
    const SNAPSHOT_INDEX: Option<IndexDef> = None;
    /// Schema the enum type is created in (enum items only).
    const ENUM_SCHEMA: &'static str = "public";
}

// =============================================================================
// Unified Entity Enum
// =============================================================================

/// Unified `PostgreSQL` DDL entity enum for serialization
///
/// Uses internally-tagged enum representation where `entityType` discriminates variants.
#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "entityType"))]
pub enum PostgresEntity {
    #[cfg_attr(feature = "serde", serde(rename = "schemas"))]
    Schema(Schema),
    #[cfg_attr(feature = "serde", serde(rename = "enums"))]
    Enum(Enum),
    #[cfg_attr(feature = "serde", serde(rename = "sequences"))]
    Sequence(Sequence),
    #[cfg_attr(feature = "serde", serde(rename = "roles"))]
    Role(Role),
    #[cfg_attr(feature = "serde", serde(rename = "policies"))]
    Policy(Policy),
    #[cfg_attr(feature = "serde", serde(rename = "privileges"))]
    Privilege(Privilege),
    #[cfg_attr(feature = "serde", serde(rename = "tables"))]
    Table(Table),
    #[cfg_attr(feature = "serde", serde(rename = "columns"))]
    Column(Column),
    #[cfg_attr(feature = "serde", serde(rename = "indexes"))]
    Index(Index),
    #[cfg_attr(feature = "serde", serde(rename = "fks"))]
    ForeignKey(ForeignKey),
    #[cfg_attr(feature = "serde", serde(rename = "pks"))]
    PrimaryKey(PrimaryKey),
    #[cfg_attr(feature = "serde", serde(rename = "uniques"))]
    UniqueConstraint(UniqueConstraint),
    #[cfg_attr(feature = "serde", serde(rename = "checks"))]
    CheckConstraint(CheckConstraint),
    #[cfg_attr(feature = "serde", serde(rename = "views"))]
    View(View),
}
