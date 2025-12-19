//! PostgreSQL DDL (Data Definition Language) entity types
//!
//! This module provides two complementary types for each DDL entity:
//!
//! - **`*Def` types** - Const-friendly definitions using only `Copy` types (`&'static str`, `bool`)
//!   for compile-time schema definitions
//! - **Runtime types** - Full types with `Cow<'static, str>` for serde serialization/deserialization
//!
//! # Design Pattern
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────────────────┐
//! │  Compile Time (const)           Runtime (serde)                         │
//! │  ─────────────────────           ────────────────                        │
//! │                                                                          │
//! │  const DEF: TableDef = ...;     let table: Table = DEF.into_table();     │
//! │  const COLS: &[ColumnDef] = ... let cols: Vec<Column> = ...              │
//! │                                                                          │
//! │  Uses: &'static str, bool       Uses: Cow<'static, str>, Vec, Option     │
//! │  All types are Copy             Supports serde, owned strings            │
//! └─────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! # PostgreSQL-Specific Features
//!
//! PostgreSQL DDL types include additional features not present in SQLite:
//!
//! - **Schemas** - Namespace support (`public`, `custom_schema`, etc.)
//! - **Enums** - User-defined enumerated types
//! - **Sequences** - Auto-increment sequences (alternative to SERIAL)
//! - **Roles** - Database roles/permissions
//! - **Policies** - Row-level security policies
//! - **Identity Columns** - GENERATED ALWAYS/BY DEFAULT AS IDENTITY
//! - **Generated Columns** - GENERATED AS expression STORED
//! - **Index Options** - Operator classes, nulls ordering, etc.

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
// Unified Entity Enum
// =============================================================================

/// Unified PostgreSQL DDL entity enum for serialization
///
/// Uses internally-tagged enum representation where `entityType` discriminates variants.
#[derive(Clone, Debug, PartialEq)]
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
