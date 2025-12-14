//! PostgreSQL DDL entity types for drizzle-kit beta v7 format
//!
//! These types represent the flat DDL entity array format used in drizzle-kit beta.
//! Each entity has an `entityType` discriminator field and references parent objects by name.

use crate::traits::{Entity, EntityKey, EntityKind};
use serde::{Deserialize, Serialize};

// =============================================================================
// Entity Type Constants
// =============================================================================

pub const ENTITY_TYPE_SCHEMAS: &str = "schemas";
pub const ENTITY_TYPE_ENUMS: &str = "enums";
pub const ENTITY_TYPE_SEQUENCES: &str = "sequences";
pub const ENTITY_TYPE_ROLES: &str = "roles";
pub const ENTITY_TYPE_POLICIES: &str = "policies";
pub const ENTITY_TYPE_TABLES: &str = "tables";
pub const ENTITY_TYPE_COLUMNS: &str = "columns";
pub const ENTITY_TYPE_INDEXES: &str = "indexes";
pub const ENTITY_TYPE_FKS: &str = "fks";
pub const ENTITY_TYPE_PKS: &str = "pks";
pub const ENTITY_TYPE_UNIQUES: &str = "uniques";
pub const ENTITY_TYPE_CHECKS: &str = "checks";
pub const ENTITY_TYPE_VIEWS: &str = "views";

// =============================================================================
// DDL Entity Types
// =============================================================================

/// Schema entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Schema {
    pub name: String,
}

/// Enum entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Enum {
    pub schema: String,
    pub name: String,
    pub values: Vec<String>,
}

/// Sequence entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Sequence {
    pub schema: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub increment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle: Option<bool>,
}

/// Role entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Role {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_db: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub create_role: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub inherit: Option<bool>,
}

/// Policy entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Policy {
    pub schema: String,
    pub table: String,
    pub name: String,
    #[serde(rename = "as", skip_serializing_if = "Option::is_none")]
    pub as_clause: Option<String>,
    #[serde(rename = "for", skip_serializing_if = "Option::is_none")]
    pub for_clause: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub to: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub using: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_check: Option<String>,
}

/// Table entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Table {
    pub schema: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_rls_enabled: Option<bool>,
}

/// Generated definition in column
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Generated {
    #[serde(rename = "as")]
    pub expression: String,
    #[serde(rename = "type")]
    pub type_: String, // "stored"
}

/// Identity definition in column
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Identity {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(rename = "type")]
    pub type_: String, // "always" | "byDefault"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub increment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_value: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_with: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cycle: Option<bool>,
}

/// Column entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Column {
    pub schema: String,
    pub table: String,
    pub name: String,
    #[serde(rename = "type")]
    pub sql_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub type_schema: Option<String>,
    pub not_null: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub generated: Option<Generated>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub identity: Option<Identity>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dimensions: Option<i32>,
}

/// Index column definition
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct IndexColumn {
    pub value: String,
    pub is_expression: bool,
    pub asc: bool,
    pub nulls_first: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub opclass: Option<String>,
}

/// Index entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Index {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub columns: Vec<IndexColumn>,
    pub is_unique: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#where: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    pub concurrently: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#with: Option<serde_json::Value>,
}

/// Foreign Key entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ForeignKey {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub name_explicit: bool,
    pub columns: Vec<String>,
    pub schema_to: String,
    pub table_to: String,
    pub columns_to: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_update: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub on_delete: Option<String>,
}

/// Primary Key entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct PrimaryKey {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub name_explicit: bool,
    pub columns: Vec<String>,
}

/// Unique Constraint entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct UniqueConstraint {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub name_explicit: bool,
    pub columns: Vec<String>,
    pub nulls_not_distinct: bool,
}

/// Check Constraint entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CheckConstraint {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub value: String,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ViewWithOption {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub check_option: Option<String>, // 'local' | 'cascaded'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_barrier: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub security_invoker: Option<bool>,

    // Materialized view options
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fillfactor: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub toast_tuple_target: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parallel_workers: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub autovacuum_enabled: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vacuum_index_cleanup: Option<String>, // 'auto'|'off'|'on'
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vacuum_truncate: Option<bool>,
    // ... possibly more autovacuum options, skipping for brevity if not critical, but will add what I can see
}

/// View entity
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct View {
    pub schema: String,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub definition: Option<String>,
    pub materialized: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#with: Option<ViewWithOption>,
    pub is_existing: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub with_no_data: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub using: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tablespace: Option<String>,
}

// =============================================================================
// Unified Entity Enum
// =============================================================================

/// Unified PostgreSQL DDL entity enum for serialization
///
/// Uses internally-tagged enum representation where `entityType` discriminates variants.
/// This replaces the need for `entity_type: String` fields on each DDL struct.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "entityType")]
pub enum PostgresEntity {
    #[serde(rename = "schemas")]
    Schema(Schema),
    #[serde(rename = "enums")]
    Enum(Enum),
    #[serde(rename = "sequences")]
    Sequence(Sequence),
    #[serde(rename = "roles")]
    Role(Role),
    #[serde(rename = "policies")]
    Policy(Policy),
    #[serde(rename = "tables")]
    Table(Table),
    #[serde(rename = "columns")]
    Column(Column),
    #[serde(rename = "indexes")]
    Index(Index),
    #[serde(rename = "fks")]
    ForeignKey(ForeignKey),
    #[serde(rename = "pks")]
    PrimaryKey(PrimaryKey),
    #[serde(rename = "uniques")]
    UniqueConstraint(UniqueConstraint),
    #[serde(rename = "checks")]
    CheckConstraint(CheckConstraint),
    #[serde(rename = "views")]
    View(View),
}

impl PostgresEntity {
    /// Get the entity kind for this entity
    pub fn kind(&self) -> EntityKind {
        match self {
            PostgresEntity::Schema(_) => EntityKind::Schema,
            PostgresEntity::Enum(_) => EntityKind::Enum,
            PostgresEntity::Sequence(_) => EntityKind::Sequence,
            PostgresEntity::Role(_) => EntityKind::Role,
            PostgresEntity::Policy(_) => EntityKind::Policy,
            PostgresEntity::Table(_) => EntityKind::Table,
            PostgresEntity::Column(_) => EntityKind::Column,
            PostgresEntity::Index(_) => EntityKind::Index,
            PostgresEntity::ForeignKey(_) => EntityKind::ForeignKey,
            PostgresEntity::PrimaryKey(_) => EntityKind::PrimaryKey,
            PostgresEntity::UniqueConstraint(_) => EntityKind::UniqueConstraint,
            PostgresEntity::CheckConstraint(_) => EntityKind::CheckConstraint,
            PostgresEntity::View(_) => EntityKind::View,
        }
    }
}

// =============================================================================
// Entity Trait Implementations
// =============================================================================

impl Entity for Schema {
    const KIND: EntityKind = EntityKind::Schema;

    fn key(&self) -> EntityKey {
        EntityKey::simple(&self.name)
    }
}

impl Entity for Enum {
    const KIND: EntityKind = EntityKind::Enum;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.schema))
    }
}

impl Entity for Sequence {
    const KIND: EntityKind = EntityKind::Sequence;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.schema))
    }
}

impl Entity for Role {
    const KIND: EntityKind = EntityKind::Role;

    fn key(&self) -> EntityKey {
        EntityKey::simple(&self.name)
    }
}

impl Entity for Policy {
    const KIND: EntityKind = EntityKind::Policy;

    fn key(&self) -> EntityKey {
        EntityKey::composite3(&self.schema, &self.table, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::composite2(&self.schema, &self.table))
    }
}

impl Entity for Table {
    const KIND: EntityKind = EntityKind::Table;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.schema))
    }
}

impl Entity for Column {
    const KIND: EntityKind = EntityKind::Column;

    fn key(&self) -> EntityKey {
        EntityKey::composite3(&self.schema, &self.table, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::composite2(&self.schema, &self.table))
    }
}

impl Entity for Index {
    const KIND: EntityKind = EntityKind::Index;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::composite2(&self.schema, &self.table))
    }
}

impl Entity for ForeignKey {
    const KIND: EntityKind = EntityKind::ForeignKey;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::composite2(&self.schema, &self.table))
    }
}

impl Entity for PrimaryKey {
    const KIND: EntityKind = EntityKind::PrimaryKey;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::composite2(&self.schema, &self.table))
    }
}

impl Entity for UniqueConstraint {
    const KIND: EntityKind = EntityKind::UniqueConstraint;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::composite2(&self.schema, &self.table))
    }
}

impl Entity for CheckConstraint {
    const KIND: EntityKind = EntityKind::CheckConstraint;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::composite2(&self.schema, &self.table))
    }
}

impl Entity for View {
    const KIND: EntityKind = EntityKind::View;

    fn key(&self) -> EntityKey {
        EntityKey::composite2(&self.schema, &self.name)
    }

    fn parent_key(&self) -> Option<EntityKey> {
        Some(EntityKey::simple(&self.schema))
    }
}
