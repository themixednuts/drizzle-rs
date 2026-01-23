//! PostgreSQL database introspection
//!
//! This module provides functionality to introspect an existing PostgreSQL database
//! and extract its schema as DDL entities, matching drizzle-kit introspect.ts

use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, IndexColumn, Policy, PostgresEntity,
    PrimaryKey, Role, Schema, Sequence, Table, UniqueConstraint, View,
};
use super::grammar::{is_system_namespace, is_system_role};
use super::snapshot::PostgresSnapshot;

/// Error type for introspection operations
#[derive(Debug, Clone)]
pub struct IntrospectError {
    pub message: String,
    pub table: Option<String>,
    pub schema: Option<String>,
}

impl std::fmt::Display for IntrospectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.schema, &self.table) {
            (Some(s), Some(t)) => {
                write!(f, "Introspection error for '{}.{}': {}", s, t, self.message)
            }
            (Some(s), None) => write!(f, "Introspection error in schema '{}': {}", s, self.message),
            (None, Some(t)) => write!(f, "Introspection error for '{}': {}", t, self.message),
            (None, None) => write!(f, "Introspection error: {}", self.message),
        }
    }
}

impl std::error::Error for IntrospectError {}

/// Result type for introspection
pub type IntrospectResult<T> = Result<T, IntrospectError>;

// =============================================================================
// Raw Query Result Types
// =============================================================================

/// Raw table info from information_schema
#[derive(Debug, Clone)]
pub struct RawTableInfo {
    pub schema: String,
    pub name: String,
    pub is_rls_enabled: bool,
}

/// Raw column info from information_schema
#[derive(Debug, Clone)]
pub struct RawColumnInfo {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub column_type: String,
    pub type_schema: Option<String>,
    pub not_null: bool,
    pub default_value: Option<String>,
    pub is_identity: bool,
    pub identity_type: Option<String>,
    pub is_generated: bool,
    pub generated_expression: Option<String>,
    pub ordinal_position: i32,
}

/// Raw enum info
#[derive(Debug, Clone)]
pub struct RawEnumInfo {
    pub schema: String,
    pub name: String,
    pub values: Vec<String>,
}

/// Raw sequence info
#[derive(Debug, Clone)]
pub struct RawSequenceInfo {
    pub schema: String,
    pub name: String,
    pub data_type: String,
    pub start_value: String,
    pub min_value: String,
    pub max_value: String,
    pub increment: String,
    pub cycle: bool,
    pub cache_value: String,
}

/// Raw index info
#[derive(Debug, Clone)]
pub struct RawIndexInfo {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub is_unique: bool,
    pub is_primary: bool,
    pub method: String,
    pub columns: Vec<RawIndexColumnInfo>,
    pub where_clause: Option<String>,
    pub concurrent: bool,
}

/// Raw index column info
#[derive(Debug, Clone)]
pub struct RawIndexColumnInfo {
    pub name: String,
    pub is_expression: bool,
    pub asc: bool,
    pub nulls_first: bool,
    pub opclass: Option<String>,
}

/// Raw foreign key info
#[derive(Debug, Clone)]
pub struct RawForeignKeyInfo {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub columns: Vec<String>,
    pub schema_to: String,
    pub table_to: String,
    pub columns_to: Vec<String>,
    pub on_update: String,
    pub on_delete: String,
}

/// Raw primary key info
#[derive(Debug, Clone)]
pub struct RawPrimaryKeyInfo {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub columns: Vec<String>,
}

/// Raw unique constraint info
#[derive(Debug, Clone)]
pub struct RawUniqueInfo {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub columns: Vec<String>,
    pub nulls_not_distinct: bool,
}

/// Raw check constraint info
#[derive(Debug, Clone)]
pub struct RawCheckInfo {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub expression: String,
}

/// Raw view info
#[derive(Debug, Clone)]
pub struct RawViewInfo {
    pub schema: String,
    pub name: String,
    pub definition: String,
    pub is_materialized: bool,
}

/// Raw policy info (RLS)
#[derive(Debug, Clone)]
pub struct RawPolicyInfo {
    pub schema: String,
    pub table: String,
    pub name: String,
    pub as_clause: String,
    pub for_clause: String,
    pub to: Vec<String>,
    pub using: Option<String>,
    pub with_check: Option<String>,
}

/// Raw role info
#[derive(Debug, Clone)]
pub struct RawRoleInfo {
    pub name: String,
    pub create_db: bool,
    pub create_role: bool,
    pub inherit: bool,
}

// =============================================================================
// Introspection Result
// =============================================================================

/// Introspection result containing all extracted entities
#[derive(Debug, Clone, Default)]
pub struct IntrospectionResult {
    pub schemas: Vec<Schema>,
    pub enums: Vec<Enum>,
    pub sequences: Vec<Sequence>,
    pub roles: Vec<Role>,
    pub tables: Vec<Table>,
    pub columns: Vec<Column>,
    pub indexes: Vec<Index>,
    pub foreign_keys: Vec<ForeignKey>,
    pub primary_keys: Vec<PrimaryKey>,
    pub unique_constraints: Vec<UniqueConstraint>,
    pub check_constraints: Vec<CheckConstraint>,
    pub views: Vec<View>,
    pub policies: Vec<Policy>,
    pub errors: Vec<IntrospectError>,
}

impl IntrospectionResult {
    /// Convert to a snapshot
    pub fn to_snapshot(&self) -> PostgresSnapshot {
        let mut snapshot = PostgresSnapshot::new();

        for schema in &self.schemas {
            snapshot.add_entity(PostgresEntity::Schema(schema.clone()));
        }
        for e in &self.enums {
            snapshot.add_entity(PostgresEntity::Enum(e.clone()));
        }
        for seq in &self.sequences {
            snapshot.add_entity(PostgresEntity::Sequence(seq.clone()));
        }
        for role in &self.roles {
            snapshot.add_entity(PostgresEntity::Role(role.clone()));
        }
        for table in &self.tables {
            snapshot.add_entity(PostgresEntity::Table(table.clone()));
        }
        for column in &self.columns {
            snapshot.add_entity(PostgresEntity::Column(column.clone()));
        }
        for index in &self.indexes {
            snapshot.add_entity(PostgresEntity::Index(index.clone()));
        }
        for fk in &self.foreign_keys {
            snapshot.add_entity(PostgresEntity::ForeignKey(fk.clone()));
        }
        for pk in &self.primary_keys {
            snapshot.add_entity(PostgresEntity::PrimaryKey(pk.clone()));
        }
        for unique in &self.unique_constraints {
            snapshot.add_entity(PostgresEntity::UniqueConstraint(unique.clone()));
        }
        for check in &self.check_constraints {
            snapshot.add_entity(PostgresEntity::CheckConstraint(check.clone()));
        }
        for view in &self.views {
            snapshot.add_entity(PostgresEntity::View(view.clone()));
        }
        for policy in &self.policies {
            snapshot.add_entity(PostgresEntity::Policy(policy.clone()));
        }

        snapshot
    }

    /// Check if introspection had any errors
    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all entities as a vector
    pub fn to_entities(&self) -> Vec<PostgresEntity> {
        let mut entities = Vec::new();

        for s in &self.schemas {
            entities.push(PostgresEntity::Schema(s.clone()));
        }
        for e in &self.enums {
            entities.push(PostgresEntity::Enum(e.clone()));
        }
        for s in &self.sequences {
            entities.push(PostgresEntity::Sequence(s.clone()));
        }
        for r in &self.roles {
            entities.push(PostgresEntity::Role(r.clone()));
        }
        for t in &self.tables {
            entities.push(PostgresEntity::Table(t.clone()));
        }
        for c in &self.columns {
            entities.push(PostgresEntity::Column(c.clone()));
        }
        for i in &self.indexes {
            entities.push(PostgresEntity::Index(i.clone()));
        }
        for f in &self.foreign_keys {
            entities.push(PostgresEntity::ForeignKey(f.clone()));
        }
        for p in &self.primary_keys {
            entities.push(PostgresEntity::PrimaryKey(p.clone()));
        }
        for u in &self.unique_constraints {
            entities.push(PostgresEntity::UniqueConstraint(u.clone()));
        }
        for c in &self.check_constraints {
            entities.push(PostgresEntity::CheckConstraint(c.clone()));
        }
        for v in &self.views {
            entities.push(PostgresEntity::View(v.clone()));
        }
        for p in &self.policies {
            entities.push(PostgresEntity::Policy(p.clone()));
        }

        entities
    }
}

// =============================================================================
// Processing Functions
// =============================================================================

/// Process raw table info into Table entities
pub fn process_tables(raw_tables: &[RawTableInfo]) -> Vec<Table> {
    raw_tables
        .iter()
        .filter(|t| !is_system_namespace(&t.schema))
        .map(|t| Table {
            schema: t.schema.clone().into(),
            name: t.name.clone().into(),
            is_rls_enabled: Some(t.is_rls_enabled),
        })
        .collect()
}

/// Process raw column info into Column entities
pub fn process_columns(raw_columns: &[RawColumnInfo]) -> Vec<Column> {
    use super::ddl::{GeneratedType, IdentityType};

    raw_columns
        .iter()
        .filter(|c| !is_system_namespace(&c.schema))
        .map(|c| {
            let generated = if c.is_generated {
                c.generated_expression
                    .as_ref()
                    .map(|expr| super::ddl::Generated {
                        expression: expr.clone().into(),
                        gen_type: GeneratedType::Stored,
                    })
            } else {
                None
            };

            let identity = if c.is_identity {
                c.identity_type.as_ref().map(|t| {
                    let identity_type = if t.eq_ignore_ascii_case("always") {
                        IdentityType::Always
                    } else {
                        IdentityType::ByDefault
                    };
                    super::ddl::Identity {
                        name: format!("{}_{}_seq", c.table, c.name).into(),
                        schema: Some(c.schema.clone().into()),
                        type_: identity_type,
                        increment: None,
                        min_value: None,
                        max_value: None,
                        start_with: None,
                        cache: None,
                        cycle: None,
                    }
                })
            } else {
                None
            };

            Column {
                schema: c.schema.clone().into(),
                table: c.table.clone().into(),
                name: c.name.clone().into(),
                sql_type: c.column_type.clone().into(),
                type_schema: c.type_schema.clone().map(|s| s.into()),
                not_null: c.not_null,
                default: c.default_value.clone().map(|s| s.into()),
                generated,
                identity,
                dimensions: None,
                ordinal_position: Some(c.ordinal_position),
            }
        })
        .collect()
}

/// Process raw enum info into Enum entities
pub fn process_enums(raw_enums: &[RawEnumInfo]) -> Vec<Enum> {
    raw_enums
        .iter()
        .filter(|e| !is_system_namespace(&e.schema))
        .map(|e| Enum {
            schema: e.schema.clone().into(),
            name: e.name.clone().into(),
            values: e.values.iter().map(|v| v.clone().into()).collect(),
        })
        .collect()
}

/// Process raw sequence info into Sequence entities
pub fn process_sequences(raw_sequences: &[RawSequenceInfo]) -> Vec<Sequence> {
    raw_sequences
        .iter()
        .filter(|s| !is_system_namespace(&s.schema))
        .map(|s| Sequence {
            schema: s.schema.clone().into(),
            name: s.name.clone().into(),
            increment_by: Some(s.increment.clone().into()),
            min_value: Some(s.min_value.clone().into()),
            max_value: Some(s.max_value.clone().into()),
            start_with: Some(s.start_value.clone().into()),
            cache_size: s.cache_value.parse().ok(),
            cycle: Some(s.cycle),
        })
        .collect()
}

/// Process raw index info into Index entities
pub fn process_indexes(raw_indexes: &[RawIndexInfo]) -> Vec<Index> {
    use super::ddl::Opclass;

    raw_indexes
        .iter()
        .filter(|i| !is_system_namespace(&i.schema) && !i.is_primary)
        .map(|i| {
            let columns: Vec<IndexColumn> = i
                .columns
                .iter()
                .map(|c| IndexColumn {
                    value: c.name.clone().into(),
                    is_expression: c.is_expression,
                    asc: c.asc,
                    nulls_first: c.nulls_first,
                    opclass: c.opclass.clone().map(Opclass::new),
                })
                .collect();

            Index {
                schema: i.schema.clone().into(),
                table: i.table.clone().into(),
                name: i.name.clone().into(),
                name_explicit: true,
                columns,
                is_unique: i.is_unique,
                where_clause: i.where_clause.clone().map(|s| s.into()),
                method: Some(i.method.clone().into()),
                concurrently: i.concurrent,
                r#with: None,
            }
        })
        .collect()
}

/// Process raw foreign key info into ForeignKey entities
pub fn process_foreign_keys(raw_fks: &[RawForeignKeyInfo]) -> Vec<ForeignKey> {
    raw_fks
        .iter()
        .filter(|f| !is_system_namespace(&f.schema))
        .map(|f| ForeignKey {
            schema: f.schema.clone().into(),
            table: f.table.clone().into(),
            name: f.name.clone().into(),
            name_explicit: true,
            columns: f.columns.iter().map(|c| c.clone().into()).collect(),
            schema_to: f.schema_to.clone().into(),
            table_to: f.table_to.clone().into(),
            columns_to: f.columns_to.iter().map(|c| c.clone().into()).collect(),
            on_update: Some(f.on_update.clone().into()),
            on_delete: Some(f.on_delete.clone().into()),
        })
        .collect()
}

/// Process raw primary key info into PrimaryKey entities
pub fn process_primary_keys(raw_pks: &[RawPrimaryKeyInfo]) -> Vec<PrimaryKey> {
    raw_pks
        .iter()
        .filter(|p| !is_system_namespace(&p.schema))
        .map(|p| PrimaryKey {
            schema: p.schema.clone().into(),
            table: p.table.clone().into(),
            name: p.name.clone().into(),
            name_explicit: true,
            columns: p.columns.iter().map(|c| c.clone().into()).collect(),
        })
        .collect()
}

/// Process raw unique constraint info into UniqueConstraint entities
pub fn process_unique_constraints(raw_uniques: &[RawUniqueInfo]) -> Vec<UniqueConstraint> {
    raw_uniques
        .iter()
        .filter(|u| !is_system_namespace(&u.schema))
        .map(|u| UniqueConstraint {
            schema: u.schema.clone().into(),
            table: u.table.clone().into(),
            name: u.name.clone().into(),
            name_explicit: true,
            columns: u.columns.iter().map(|c| c.clone().into()).collect(),
            nulls_not_distinct: u.nulls_not_distinct,
        })
        .collect()
}

/// Process raw check constraint info into CheckConstraint entities
pub fn process_check_constraints(raw_checks: &[RawCheckInfo]) -> Vec<CheckConstraint> {
    raw_checks
        .iter()
        .filter(|c| !is_system_namespace(&c.schema))
        .map(|c| CheckConstraint {
            schema: c.schema.clone().into(),
            table: c.table.clone().into(),
            name: c.name.clone().into(),
            value: c.expression.clone().into(),
        })
        .collect()
}

/// Process raw view info into View entities
pub fn process_views(raw_views: &[RawViewInfo]) -> Vec<View> {
    raw_views
        .iter()
        .filter(|v| !is_system_namespace(&v.schema))
        .map(|v| View {
            schema: v.schema.clone().into(),
            name: v.name.clone().into(),
            definition: Some(v.definition.clone().into()),
            materialized: v.is_materialized,
            r#with: None,
            is_existing: false,
            with_no_data: None,
            using: None,
            tablespace: None,
        })
        .collect()
}

/// Process raw policy info into Policy entities
pub fn process_policies(raw_policies: &[RawPolicyInfo]) -> Vec<Policy> {
    use std::borrow::Cow;

    raw_policies
        .iter()
        .filter(|p| !is_system_namespace(&p.schema))
        .map(|p| {
            // Convert Vec<String> to Vec<&'static str> by leaking the strings
            // This is acceptable for migration tooling which runs once
            let roles: Vec<&'static str> =
                p.to.iter()
                    .map(|s| Box::leak(s.clone().into_boxed_str()) as &'static str)
                    .collect();

            Policy {
                schema: p.schema.clone().into(),
                table: p.table.clone().into(),
                name: p.name.clone().into(),
                as_clause: Some(p.as_clause.clone().into()),
                for_clause: Some(p.for_clause.clone().into()),
                to: Some(Cow::Owned(roles)),
                using: p.using.clone().map(|s| s.into()),
                with_check: p.with_check.clone().map(|s| s.into()),
            }
        })
        .collect()
}

/// Process raw role info into Role entities
pub fn process_roles(raw_roles: &[RawRoleInfo]) -> Vec<Role> {
    raw_roles
        .iter()
        .filter(|r| !is_system_role(&r.name))
        .map(|r| Role {
            name: r.name.clone().into(),
            superuser: None,
            create_db: Some(r.create_db),
            create_role: Some(r.create_role),
            inherit: Some(r.inherit),
            can_login: None,
            replication: None,
            bypass_rls: None,
            conn_limit: None,
            password: None,
            valid_until: None,
        })
        .collect()
}

// =============================================================================
// SQL Queries
// =============================================================================

/// SQL queries for PostgreSQL introspection
pub mod queries {
    /// Query to get all schemas
    pub const SCHEMAS_QUERY: &str = r#"
        SELECT nspname AS name
        FROM pg_namespace
        WHERE nspname NOT LIKE 'pg_%'
          AND nspname != 'information_schema'
        ORDER BY nspname
    "#;

    /// Query to get all tables
    pub const TABLES_QUERY: &str = r#"
        SELECT 
            schemaname AS schema,
            tablename AS name,
            rowsecurity AS is_rls_enabled
        FROM pg_tables
        WHERE schemaname NOT LIKE 'pg_%'
          AND schemaname != 'information_schema'
        ORDER BY schemaname, tablename
    "#;

    /// Query to get all columns
    pub const COLUMNS_QUERY: &str = r#"
        SELECT 
            c.table_schema AS schema,
            c.table_name AS table,
            c.column_name AS name,
            c.udt_name AS column_type,
            c.udt_schema AS type_schema,
            c.is_nullable = 'NO' AS not_null,
            c.column_default AS default_value,
            c.is_identity = 'YES' AS is_identity,
            c.identity_generation AS identity_type,
            c.is_generated = 'ALWAYS' AS is_generated,
            c.generation_expression AS generated_expression,
            c.ordinal_position
        FROM information_schema.columns c
        WHERE c.table_schema NOT LIKE 'pg_%'
          AND c.table_schema != 'information_schema'
        ORDER BY c.table_schema, c.table_name, c.ordinal_position
    "#;

    /// Query to get all enums
    pub const ENUMS_QUERY: &str = r#"
        SELECT 
            n.nspname AS schema,
            t.typname AS name,
            array_agg(e.enumlabel ORDER BY e.enumsortorder) AS values
        FROM pg_type t
        JOIN pg_enum e ON t.oid = e.enumtypid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
        GROUP BY n.nspname, t.typname
        ORDER BY n.nspname, t.typname
    "#;

    /// Query to get all sequences
    pub const SEQUENCES_QUERY: &str = r#"
        SELECT 
            schemaname AS schema,
            sequencename AS name,
            data_type,
            start_value::text,
            min_value::text,
            max_value::text,
            increment_by::text AS increment,
            cycle AS cycle,
            cache_size::text AS cache_value
        FROM pg_sequences
        WHERE schemaname NOT LIKE 'pg_%'
          AND schemaname != 'information_schema'
        ORDER BY schemaname, sequencename
    "#;

    /// Query to get all views
    pub const VIEWS_QUERY: &str = r#"
        SELECT 
            schemaname AS schema,
            viewname AS name,
            definition,
            FALSE AS is_materialized
        FROM pg_views
        WHERE schemaname NOT LIKE 'pg_%'
          AND schemaname != 'information_schema'
        UNION ALL
        SELECT 
            schemaname AS schema,
            matviewname AS name,
            definition,
            TRUE AS is_materialized
        FROM pg_matviews
        WHERE schemaname NOT LIKE 'pg_%'
          AND schemaname != 'information_schema'
        ORDER BY schema, name
    "#;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_process_tables() {
        let raw = vec![
            RawTableInfo {
                schema: "public".to_string(),
                name: "users".to_string(),
                is_rls_enabled: false,
            },
            RawTableInfo {
                schema: "pg_catalog".to_string(),
                name: "pg_class".to_string(),
                is_rls_enabled: false,
            },
        ];

        let tables = process_tables(&raw);
        assert_eq!(tables.len(), 1);
        assert_eq!(tables[0].name, "users");
    }

    #[test]
    fn test_introspection_result_to_snapshot() {
        let mut result = IntrospectionResult::default();
        result.schemas.push(Schema::new("public"));
        result.tables.push(Table {
            schema: "public".into(),
            name: "users".into(),
            is_rls_enabled: None,
        });

        let snapshot = result.to_snapshot();
        assert_eq!(snapshot.ddl.len(), 2);
    }
}
