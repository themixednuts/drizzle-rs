//! SQLite DDL (Data Definition Language) entity types
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
//! # Examples
//!
//! ## Compile-time Schema Definition
//!
//! ```
//! use drizzle_types::sqlite::ddl::{TableDef, ColumnDef};
//!
//! // These are all const - zero runtime allocation
//! const USERS_TABLE: TableDef = TableDef::new("users").strict();
//!
//! const USERS_COLUMNS: &[ColumnDef] = &[
//!     ColumnDef::new("users", "id", "INTEGER").primary_key().autoincrement(),
//!     ColumnDef::new("users", "name", "TEXT").not_null(),
//!     ColumnDef::new("users", "email", "TEXT").unique(),
//! ];
//! ```
//!
//! ## Converting to Runtime Types
//!
//! ```
//! use drizzle_types::sqlite::ddl::{TableDef, Table};
//!
//! const DEF: TableDef = TableDef::new("users").strict();
//!
//! // Convert when you need serde or runtime manipulation
//! let table: Table = DEF.into_table();
//! ```
//!
//! ## Runtime Deserialization
//!
//! ```ignore
//! use drizzle_types::sqlite::ddl::Table;
//!
//! let table: Table = serde_json::from_str(r#"{"name": "users", "strict": true}"#)?;
//! ```

mod check_constraint;
mod column;
mod foreign_key;
mod index;
mod primary_key;
pub mod sql;
mod table;
mod unique_constraint;
mod view;

// Const-friendly definition types
pub use check_constraint::CheckConstraintDef;
pub use column::{ColumnDef, GeneratedDef, GeneratedType};
pub use foreign_key::{ForeignKeyDef, ReferentialAction};
pub use index::{IndexColumn, IndexColumnDef, IndexDef, IndexOrigin};
pub use primary_key::PrimaryKeyDef;
pub use table::TableDef;
pub use unique_constraint::UniqueConstraintDef;
pub use view::ViewDef;

// Runtime types for serde
pub use check_constraint::CheckConstraint;
pub use column::{Column, Generated};
pub use foreign_key::ForeignKey;
pub use index::Index;
pub use primary_key::PrimaryKey;
pub use table::Table;
pub use unique_constraint::UniqueConstraint;
pub use view::View;

// SQL generation
pub use sql::TableSql;

#[cfg(feature = "serde")]
pub use crate::serde_helpers::{cow_from_string, cow_option_from_string};

// =============================================================================
// Entity Type Constants (for compatibility with migrations)
// =============================================================================

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

// =============================================================================
// Unified Entity Enum
// =============================================================================

/// Unified SQLite DDL entity enum for serialization
///
/// Uses internally-tagged enum representation where `entityType` discriminates variants.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "serde", serde(tag = "entityType"))]
pub enum SqliteEntity {
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

// =============================================================================
// Naming Helpers (matching drizzle-kit grammar.ts patterns)
// =============================================================================

/// Generate a default name for a foreign key constraint
#[must_use]
pub fn name_for_fk(table: &str, columns: &[&str], table_to: &str, columns_to: &[&str]) -> String {
    format!(
        "fk_{}_{}_{}_{}_fk",
        table,
        columns.join("_"),
        table_to,
        columns_to.join("_")
    )
}

/// Generate a default name for a unique constraint
#[must_use]
pub fn name_for_unique(table: &str, columns: &[&str]) -> String {
    format!("{}_{}_unique", table, columns.join("_"))
}

/// Generate a default name for a primary key constraint
#[must_use]
pub fn name_for_pk(table: &str) -> String {
    format!("{}_pk", table)
}

/// Generate a default name for an index
#[must_use]
pub fn name_for_index(table: &str, columns: &[&str]) -> String {
    format!("{}_{}_idx", table, columns.join("_"))
}

/// Generate a default name for a check constraint
#[must_use]
pub fn name_for_check(table: &str, index: usize) -> String {
    format!("{}_check_{}", table, index)
}
