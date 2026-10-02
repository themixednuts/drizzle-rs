//! `SQLite` schema objects (tables, columns, indexes, constraints, views) for
//! migrations.
//!
//! Each object comes in two forms:
//!
//! - **`*Def` types** ([`TableDef`], [`ColumnDef`], ...) hold only `Copy`
//!   data (`&'static str`, `bool`, slices) so they can be built in `const`
//!   items. The schema macros generate these.
//! - **Runtime types** ([`Table`], [`Column`], ...) hold `Cow<'static, str>`
//!   and can be serialized with the `serde` feature. Migration snapshots
//!   store these, as a list of [`SqliteEntity`] values.
//!
//! Convert a definition with its `into_*` method or `From`. [`TableSql`]
//! renders `CREATE TABLE` and related statements.
//!
//! # Examples
//!
//! Const definitions:
//!
//! ```
//! use drizzle_types::sqlite::ddl::{ColumnDef, TableDef};
//!
//! const USERS_TABLE: TableDef = TableDef::new("users").strict();
//!
//! const USERS_COLUMNS: &[ColumnDef] = &[
//!     ColumnDef::new("users", "id", "INTEGER").primary_key().autoincrement(),
//!     ColumnDef::new("users", "name", "TEXT").not_null(),
//!     ColumnDef::new("users", "email", "TEXT").unique(),
//! ];
//! # let _ = (USERS_TABLE, USERS_COLUMNS);
//! ```
//!
//! Converting to the runtime type:
//!
//! ```
//! use drizzle_types::sqlite::ddl::{Table, TableDef};
//!
//! const DEF: TableDef = TableDef::new("users").strict();
//!
//! let table: Table = DEF.into_table();
//! assert_eq!(table.name(), "users");
//! ```
//!
//! Deserializing (with the `serde` feature):
//!
//! ```
//! # #[cfg(feature = "serde")]
//! # {
//! use drizzle_types::sqlite::ddl::Table;
//!
//! let table: Table = serde_json::from_str(r#"{"name": "users", "strict": true}"#).unwrap();
//! assert!(table.strict);
//! # }
//! ```

use crate::alloc_prelude::*;

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

/// Any `SQLite` schema object: one element of a snapshot's `ddl` array.
///
/// With `serde`, the variant is stored in an `entityType` field, using the
/// `ENTITY_TYPE_*` names (`"tables"`, `"columns"`, `"indexes"`, `"fks"`,
/// `"pks"`, `"uniques"`, `"checks"`, `"views"`).
#[derive(Clone, Debug, PartialEq, Eq)]
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

/// Returns the default foreign key name:
/// `fk_{table}_{columns}_{table_to}_{columns_to}_fk`, with columns joined by `_`.
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

/// Returns the default unique constraint name: `{table}_{columns}_unique`.
///
/// # Examples
///
/// ```
/// use drizzle_types::sqlite::ddl::name_for_unique;
///
/// assert_eq!(name_for_unique("users", &["org_id", "email"]), "users_org_id_email_unique");
/// ```
#[must_use]
pub fn name_for_unique(table: &str, columns: &[&str]) -> String {
    format!("{}_{}_unique", table, columns.join("_"))
}

/// Returns the default primary key name: `{table}_pk`.
#[must_use]
pub fn name_for_pk(table: &str) -> String {
    format!("{table}_pk")
}

/// Returns the default index name: `{table}_{columns}_idx`.
#[must_use]
pub fn name_for_index(table: &str, columns: &[&str]) -> String {
    format!("{}_{}_idx", table, columns.join("_"))
}

/// Returns the default name of the `index`-th check constraint:
/// `{table}_check_{index}`.
#[must_use]
pub fn name_for_check(table: &str, index: usize) -> String {
    format!("{table}_check_{index}")
}
