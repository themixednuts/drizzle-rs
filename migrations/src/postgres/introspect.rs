//! `PostgreSQL` database introspection
//!
//! This module provides functionality to introspect an existing `PostgreSQL` database
//! and extract its schema as DDL entities, matching drizzle-kit introspect.ts

use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, IndexColumn, Policy, PostgresEntity,
    PrimaryKey, Role, Schema, Sequence, Table, UniqueConstraint, View,
};
use super::grammar::{
    extract_nextval_sequence, is_serial_expression, is_system_namespace, is_system_role,
};
use super::snapshot::PostgresSnapshot;
use std::collections::HashSet;

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

/// Raw table info from `information_schema`
#[derive(Debug, Clone)]
pub struct RawTableInfo {
    pub schema: String,
    pub name: String,
    pub is_rls_enabled: bool,
    pub is_unlogged: bool,
    pub is_temporary: bool,
    pub tablespace: Option<String>,
    pub comment: Option<String>,
}

/// Raw column info from `information_schema`
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
    pub generated_stored: bool,
    pub dimensions: Option<i32>,
    pub comment: Option<String>,
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
///
/// Value columns are `Option` because `pg_sequences` returns NULL when the
/// current user lacks privilege on the sequence.
#[derive(Debug, Clone)]
pub struct RawSequenceInfo {
    pub schema: String,
    pub name: String,
    pub data_type: Option<String>,
    pub start_value: Option<String>,
    pub min_value: Option<String>,
    pub max_value: Option<String>,
    pub increment: Option<String>,
    pub cycle: Option<bool>,
    pub cache_value: Option<String>,
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
    pub deferrable: bool,
    pub initially_deferred: bool,
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
    pub deferrable: bool,
    pub initially_deferred: bool,
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

/// Transport-decoded PostgreSQL catalog rows used by every driver.
#[derive(Debug, Clone, Default)]
pub struct RawIntrospection {
    pub schemas: Vec<Schema>,
    pub tables: Vec<RawTableInfo>,
    pub columns: Vec<RawColumnInfo>,
    pub enums: Vec<RawEnumInfo>,
    pub sequences: Vec<RawSequenceInfo>,
    pub views: Vec<RawViewInfo>,
    pub indexes: Vec<RawIndexInfo>,
    pub foreign_keys: Vec<RawForeignKeyInfo>,
    pub primary_keys: Vec<RawPrimaryKeyInfo>,
    pub unique_constraints: Vec<RawUniqueInfo>,
    pub check_constraints: Vec<RawCheckInfo>,
    pub roles: Vec<RawRoleInfo>,
    pub policies: Vec<RawPolicyInfo>,
}

/// Assemble transport-decoded PostgreSQL metadata into the canonical DDL.
#[must_use]
pub fn assemble_ddl(raw: RawIntrospection) -> super::PostgresDDL {
    let mut ddl = super::PostgresDDL::new();
    for schema in raw.schemas {
        ddl.schemas.push(schema);
    }
    for value in process_enums(&raw.enums) {
        ddl.enums.push(value);
    }
    for sequence in process_sequences(&raw.sequences) {
        ddl.sequences.push(sequence);
    }
    for role in process_roles(&raw.roles) {
        ddl.roles.push(role);
    }
    for policy in process_policies(&raw.policies) {
        ddl.policies.push(policy);
    }
    for table in process_tables(&raw.tables) {
        ddl.tables.push(table);
    }
    for column in process_columns(&raw.columns) {
        ddl.columns.push(column);
    }
    for index in process_indexes(&raw.indexes) {
        ddl.indexes.push(index);
    }
    for foreign_key in process_foreign_keys(&raw.foreign_keys) {
        ddl.fks.push(foreign_key);
    }
    for primary_key in process_primary_keys(&raw.primary_keys) {
        ddl.pks.push(primary_key);
    }
    for unique in process_unique_constraints(&raw.unique_constraints) {
        ddl.uniques.push(unique);
    }
    for check in process_check_constraints(&raw.check_constraints) {
        ddl.checks.push(check);
    }
    for view in process_views(&raw.views) {
        ddl.views.push(view);
    }
    ddl
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
    /// Collect (schema, name) pairs for sequences owned by serial/bigserial columns.
    ///
    /// These sequences are auto-managed by `PostgreSQL` and should not appear in the
    /// snapshot used for diffing — otherwise `push()` would try to DROP them.
    fn serial_owned_sequences(&self) -> HashSet<(String, String)> {
        let mut owned = HashSet::new();
        for col in &self.columns {
            if let Some(ref default) = col.default
                && is_serial_expression(default, &col.schema)
                && let Some(seq_name) = extract_nextval_sequence(default)
            {
                owned.insert((col.schema.to_string(), seq_name));
            }
        }
        owned
    }

    /// Convert to a snapshot
    #[must_use]
    pub fn to_snapshot(&self) -> PostgresSnapshot {
        let mut snapshot = PostgresSnapshot::new();
        let serial_seqs = self.serial_owned_sequences();

        for schema in &self.schemas {
            snapshot.add_entity(PostgresEntity::Schema(schema.clone()));
        }
        for e in &self.enums {
            snapshot.add_entity(PostgresEntity::Enum(e.clone()));
        }
        for seq in &self.sequences {
            // Skip sequences owned by serial/bigserial columns — they are
            // auto-managed by PostgreSQL and must not appear in the diff.
            if serial_seqs.contains(&(seq.schema.to_string(), seq.name.to_string())) {
                continue;
            }
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
    #[must_use]
    pub const fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }

    /// Get all entities as a vector
    #[must_use]
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
#[must_use]
pub fn process_tables(raw_tables: &[RawTableInfo]) -> Vec<Table> {
    raw_tables
        .iter()
        .filter(|t| !is_system_namespace(&t.schema))
        .map(|t| Table {
            schema: t.schema.clone().into(),
            name: t.name.clone().into(),
            is_unlogged: if t.is_unlogged { Some(true) } else { None },
            is_temporary: if t.is_temporary { Some(true) } else { None },
            inherits: None,
            tablespace: t.tablespace.clone().map(Into::into),
            is_rls_enabled: Some(t.is_rls_enabled),
            comment: t.comment.clone().map(Into::into),
        })
        .collect()
}

/// Process raw column info into Column entities
#[must_use]
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
                        gen_type: if c.generated_stored {
                            GeneratedType::Stored
                        } else {
                            GeneratedType::Virtual
                        },
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

            let dimensions = c.dimensions.filter(|dims| *dims > 0).or_else(|| {
                if c.column_type.starts_with('_') {
                    Some(1)
                } else {
                    None
                }
            });
            let column_type = if dimensions.is_some() {
                c.column_type
                    .strip_prefix('_')
                    .unwrap_or(&c.column_type)
                    .to_string()
            } else {
                c.column_type.clone()
            };

            Column {
                schema: c.schema.clone().into(),
                table: c.table.clone().into(),
                name: c.name.clone().into(),
                sql_type: column_type.into(),
                type_schema: c.type_schema.clone().map(std::convert::Into::into),
                not_null: c.not_null,
                default: c.default_value.clone().map(std::convert::Into::into),
                generated,
                identity,
                dimensions,
                comment: c.comment.clone().map(std::convert::Into::into),
                // pg_attribute exposes attcollation but we don't read it yet
                // (the introspect SQL doesn't pull it). Collation drift
                // detection requires extending the SELECT — defer to a
                // follow-up.
                collate: None,
                ordinal_position: Some(c.ordinal_position),
            }
        })
        .collect()
}

/// Process raw enum info into Enum entities
#[must_use]
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
#[must_use]
pub fn process_sequences(raw_sequences: &[RawSequenceInfo]) -> Vec<Sequence> {
    raw_sequences
        .iter()
        .filter(|s| !is_system_namespace(&s.schema))
        .map(|s| Sequence {
            schema: s.schema.clone().into(),
            name: s.name.clone().into(),
            increment_by: s.increment.clone().map(Into::into),
            min_value: s.min_value.clone().map(Into::into),
            max_value: s.max_value.clone().map(Into::into),
            start_with: s.start_value.clone().map(Into::into),
            cache_size: s.cache_value.as_deref().and_then(|v| v.parse().ok()),
            cycle: s.cycle,
        })
        .collect()
}

/// Process raw index info into Index entities
#[must_use]
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
                where_clause: i.where_clause.clone().map(std::convert::Into::into),
                method: Some(i.method.clone().into()),
                concurrently: i.concurrent,
                r#with: None,
            }
        })
        .collect()
}

/// Process raw foreign key info into `ForeignKey` entities
#[must_use]
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
            deferrable: f.deferrable,
            initially_deferred: f.initially_deferred,
        })
        .collect()
}

/// Process raw primary key info into `PrimaryKey` entities
#[must_use]
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

/// Process raw unique constraint info into `UniqueConstraint` entities
#[must_use]
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
            deferrable: u.deferrable,
            initially_deferred: u.initially_deferred,
        })
        .collect()
}

/// Process raw check constraint info into `CheckConstraint` entities
#[must_use]
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
#[must_use]
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
#[must_use]
pub fn process_policies(raw_policies: &[RawPolicyInfo]) -> Vec<Policy> {
    use std::borrow::Cow;

    raw_policies
        .iter()
        .filter(|p| !is_system_namespace(&p.schema))
        .map(|p| {
            let roles = p.to.iter().cloned().map(Cow::Owned).collect();

            Policy {
                schema: p.schema.clone().into(),
                table: p.table.clone().into(),
                name: p.name.clone().into(),
                as_clause: Some(p.as_clause.clone().into()),
                for_clause: Some(p.for_clause.clone().into()),
                to: Some(roles),
                using: p.using.clone().map(std::convert::Into::into),
                with_check: p.with_check.clone().map(std::convert::Into::into),
            }
        })
        .collect()
}

/// Process raw role info into Role entities
#[must_use]
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

/// SQL queries for `PostgreSQL` introspection
pub mod queries {
    /// Query to get all schemas
    pub const SCHEMAS_QUERY: &str = r"
        SELECT n.nspname AS name
        FROM pg_namespace n
        WHERE n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
          AND has_schema_privilege(current_user, n.oid, 'USAGE')
        ORDER BY n.nspname
    ";

    /// Query to get all tables
    pub const TABLES_QUERY: &str = r"
        SELECT 
            n.nspname AS schema,
            c.relname AS name,
            c.relrowsecurity AS is_rls_enabled,
            c.relpersistence = 'u' AS is_unlogged,
            c.relpersistence = 't' AS is_temporary,
            tsp.spcname AS tablespace,
            obj_description(c.oid, 'pg_class') AS comment
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        LEFT JOIN pg_tablespace tsp ON tsp.oid = c.reltablespace
        WHERE c.relkind IN ('r', 'p')
          AND n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
          AND has_schema_privilege(current_user, n.oid, 'USAGE')
          AND has_table_privilege(current_user, c.oid, 'SELECT')
        ORDER BY n.nspname, c.relname
    ";

    /// Query to get all columns
    pub const COLUMNS_QUERY: &str = r"
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
            COALESCE(a.attgenerated = 's', false) AS generated_stored,
            NULLIF(a.attndims::int4, 0) AS dimensions,
            col_description(cls.oid, a.attnum) AS comment,
            c.ordinal_position
        FROM information_schema.columns c
        LEFT JOIN pg_namespace n
          ON n.nspname = c.table_schema
        LEFT JOIN pg_class cls
          ON cls.relnamespace = n.oid
         AND cls.relname = c.table_name
        LEFT JOIN pg_attribute a
          ON a.attrelid = cls.oid
         AND a.attname = c.column_name
         AND a.attnum > 0
         AND NOT a.attisdropped
        WHERE c.table_schema NOT LIKE 'pg_%'
          AND c.table_schema != 'information_schema'
          AND n.oid IS NOT NULL
          AND has_schema_privilege(current_user, n.oid, 'USAGE')
        ORDER BY c.table_schema, c.table_name, c.ordinal_position
    ";

    /// Query to get all enums
    pub const ENUMS_QUERY: &str = r"
        SELECT 
            n.nspname AS schema,
            t.typname AS name,
            array_agg(e.enumlabel ORDER BY e.enumsortorder) AS values
        FROM pg_type t
        JOIN pg_enum e ON t.oid = e.enumtypid
        JOIN pg_namespace n ON n.oid = t.typnamespace
        WHERE n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
          AND has_schema_privilege(current_user, n.oid, 'USAGE')
        GROUP BY n.nspname, t.typname
        ORDER BY n.nspname, t.typname
    ";

    /// Query to get all sequences
    ///
    /// Uses the underlying `pg_sequence` + `pg_class` catalog tables instead
    /// of the `pg_sequences` convenience view.  The view internally calls
    /// `pg_sequence_parameters()` which can fail when sequences are being
    /// dropped concurrently (e.g. during parallel test runs).  Direct
    /// catalog access is fully MVCC-protected and avoids this issue.
    ///
    /// Value columns are nullable because the current user may lack
    /// privilege on the sequence.
    pub const SEQUENCES_QUERY: &str = r"
        SELECT
            n.nspname AS schema,
            c.relname AS name,
            format_type(s.seqtypid, NULL)::text AS data_type,
            s.seqstart::text AS start_value,
            s.seqmin::text AS min_value,
            s.seqmax::text AS max_value,
            s.seqincrement::text AS increment,
            s.seqcycle AS cycle,
            s.seqcache::text AS cache_value
        FROM pg_sequence s
        JOIN pg_class c ON c.oid = s.seqrelid
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE n.nspname NOT LIKE 'pg_%'
          AND n.nspname != 'information_schema'
          AND has_schema_privilege(current_user, n.oid, 'USAGE')
          AND (
              -- Reference s.seqrelid (not c.oid) so this qual only depends on
              -- pg_sequence: PostgreSQL 18's planner can push it down to the
              -- pg_class scan before the join filters to sequences, and
              -- has_sequence_privilege errors on non-sequence relations.
              has_sequence_privilege(current_user, s.seqrelid, 'USAGE')
              OR has_sequence_privilege(current_user, s.seqrelid, 'SELECT')
          )
        ORDER BY n.nspname, c.relname
    ";

    /// Query to get all views.
    ///
    /// Accepts `$1::text[]` — when non-NULL, scopes to those schemas;
    /// when NULL, returns all non-system views.
    pub const VIEWS_QUERY: &str = r"
        SELECT
            n.nspname AS schema,
            c.relname AS name,
            pg_get_viewdef(c.oid) AS definition,
            FALSE AS is_materialized
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE (
            ($1::text[] IS NOT NULL AND n.nspname = ANY($1::text[]))
            OR ($1::text[] IS NULL AND n.nspname NOT LIKE 'pg_%'
                AND n.nspname != 'information_schema')
        )
          AND c.relkind = 'v'
          AND has_schema_privilege(current_user, n.oid, 'USAGE')
          AND has_table_privilege(current_user, c.oid, 'SELECT')
          AND pg_get_viewdef(c.oid) IS NOT NULL
        UNION ALL
        SELECT
            n.nspname AS schema,
            c.relname AS name,
            pg_get_viewdef(c.oid) AS definition,
            TRUE AS is_materialized
        FROM pg_class c
        JOIN pg_namespace n ON n.oid = c.relnamespace
        WHERE (
            ($1::text[] IS NOT NULL AND n.nspname = ANY($1::text[]))
            OR ($1::text[] IS NULL AND n.nspname NOT LIKE 'pg_%'
                AND n.nspname != 'information_schema')
        )
          AND c.relkind = 'm'
          AND has_schema_privilege(current_user, n.oid, 'USAGE')
          AND has_table_privilege(current_user, c.oid, 'SELECT')
          AND pg_get_viewdef(c.oid) IS NOT NULL
        ORDER BY schema, name
    ";

    /// Query to get all indexes
    pub const INDEXES_QUERY: &str = r"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    idx.relname AS name,
    ix.indisunique AS is_unique,
    ix.indisprimary AS is_primary,
    am.amname AS method,
    array_agg(pg_get_indexdef(ix.indexrelid, s.n, true) ORDER BY s.n) AS columns,
    pg_get_expr(ix.indpred, ix.indrelid) AS where_clause
FROM pg_index ix
JOIN pg_class idx ON idx.oid = ix.indexrelid
JOIN pg_class tbl ON tbl.oid = ix.indrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN pg_am am ON am.oid = idx.relam
JOIN generate_series(1, ix.indnkeyatts) AS s(n) ON TRUE
WHERE ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
  AND has_schema_privilege(current_user, ns.oid, 'USAGE')
  AND has_table_privilege(current_user, tbl.oid, 'SELECT')
GROUP BY ns.nspname, tbl.relname, idx.relname, ix.indisunique, ix.indisprimary, am.amname, ix.indpred, ix.indrelid
ORDER BY ns.nspname, tbl.relname, idx.relname
";

    /// Schema-filtered variant of [`INDEXES_QUERY`].
    ///
    /// `pg_get_indexdef()` calls `relation_open()` which is not
    /// MVCC-protected and can fail if concurrent DDL drops an index.
    /// Scoping to specific schemas (`$1::text[]`) avoids encountering
    /// OIDs from schemas being modified by other sessions.
    pub const INDEXES_QUERY_FILTERED: &str = r"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    idx.relname AS name,
    ix.indisunique AS is_unique,
    ix.indisprimary AS is_primary,
    am.amname AS method,
    array_agg(pg_get_indexdef(ix.indexrelid, s.n, true) ORDER BY s.n) AS columns,
    pg_get_expr(ix.indpred, ix.indrelid) AS where_clause
FROM pg_index ix
JOIN pg_class idx ON idx.oid = ix.indexrelid
JOIN pg_class tbl ON tbl.oid = ix.indrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN pg_am am ON am.oid = idx.relam
JOIN generate_series(1, ix.indnkeyatts) AS s(n) ON TRUE
WHERE ns.nspname = ANY($1::text[])
  AND has_schema_privilege(current_user, ns.oid, 'USAGE')
  AND has_table_privilege(current_user, tbl.oid, 'SELECT')
GROUP BY ns.nspname, tbl.relname, idx.relname, ix.indisunique, ix.indisprimary, am.amname, ix.indpred, ix.indrelid
ORDER BY ns.nspname, tbl.relname, idx.relname
";

    /// Query to get all foreign keys
    pub const FOREIGN_KEYS_QUERY: &str = r"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    array_agg(src.attname ORDER BY s.ord) AS columns,
    ns_to.nspname AS schema_to,
    tbl_to.relname AS table_to,
    array_agg(dst.attname ORDER BY s.ord) AS columns_to,
    con.confupdtype::text AS on_update,
    con.confdeltype::text AS on_delete,
    con.condeferrable AS deferrable,
    con.condeferred AS initially_deferred
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN pg_class tbl_to ON tbl_to.oid = con.confrelid
JOIN pg_namespace ns_to ON ns_to.oid = tbl_to.relnamespace
JOIN unnest(con.conkey) WITH ORDINALITY AS s(attnum, ord) ON TRUE
JOIN pg_attribute src ON src.attrelid = tbl.oid AND src.attnum = s.attnum
JOIN unnest(con.confkey) WITH ORDINALITY AS r(attnum, ord) ON r.ord = s.ord
JOIN pg_attribute dst ON dst.attrelid = tbl_to.oid AND dst.attnum = r.attnum
WHERE con.contype = 'f'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
  AND has_schema_privilege(current_user, ns.oid, 'USAGE')
  AND has_table_privilege(current_user, tbl.oid, 'SELECT')
GROUP BY ns.nspname, tbl.relname, con.conname, ns_to.nspname, tbl_to.relname, con.confupdtype, con.confdeltype, con.condeferrable, con.condeferred
ORDER BY ns.nspname, tbl.relname, con.conname
";

    /// Query to get all primary keys
    pub const PRIMARY_KEYS_QUERY: &str = r"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    array_agg(att.attname ORDER BY s.ord) AS columns
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN unnest(con.conkey) WITH ORDINALITY AS s(attnum, ord) ON TRUE
JOIN pg_attribute att ON att.attrelid = tbl.oid AND att.attnum = s.attnum
WHERE con.contype = 'p'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
  AND has_schema_privilege(current_user, ns.oid, 'USAGE')
  AND has_table_privilege(current_user, tbl.oid, 'SELECT')
GROUP BY ns.nspname, tbl.relname, con.conname
ORDER BY ns.nspname, tbl.relname, con.conname
";

    /// Query to get all unique constraints
    pub const UNIQUES_QUERY: &str = r"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    array_agg(att.attname ORDER BY s.ord) AS columns,
    FALSE AS nulls_not_distinct,
    con.condeferrable AS deferrable,
    con.condeferred AS initially_deferred
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN unnest(con.conkey) WITH ORDINALITY AS s(attnum, ord) ON TRUE
JOIN pg_attribute att ON att.attrelid = tbl.oid AND att.attnum = s.attnum
WHERE con.contype = 'u'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
  AND has_schema_privilege(current_user, ns.oid, 'USAGE')
  AND has_table_privilege(current_user, tbl.oid, 'SELECT')
GROUP BY ns.nspname, tbl.relname, con.conname, con.condeferrable, con.condeferred
ORDER BY ns.nspname, tbl.relname, con.conname
";

    /// Query to get all check constraints
    pub const CHECKS_QUERY: &str = r"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    pg_get_expr(con.conbin, con.conrelid) AS expression
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
WHERE con.contype = 'c'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
  AND has_schema_privilege(current_user, ns.oid, 'USAGE')
  AND has_table_privilege(current_user, tbl.oid, 'SELECT')
ORDER BY ns.nspname, tbl.relname, con.conname
";

    /// Schema-filtered variant of [`CHECKS_QUERY`].
    ///
    /// `pg_get_expr()` calls `relation_open()` which is not MVCC-protected.
    /// Scoping to specific schemas avoids encountering OIDs from
    /// schemas being modified by other sessions.
    pub const CHECKS_QUERY_FILTERED: &str = r"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    pg_get_expr(con.conbin, con.conrelid) AS expression
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
WHERE con.contype = 'c'
  AND ns.nspname = ANY($1::text[])
  AND has_schema_privilege(current_user, ns.oid, 'USAGE')
  AND has_table_privilege(current_user, tbl.oid, 'SELECT')
ORDER BY ns.nspname, tbl.relname, con.conname
";

    /// Query to get all roles
    pub const ROLES_QUERY: &str = r"
SELECT
    rolname AS name,
    rolcreatedb AS create_db,
    rolcreaterole AS create_role,
    rolinherit AS inherit
FROM pg_roles
ORDER BY rolname
";

    /// Query to get all policies
    pub const POLICIES_QUERY: &str = r#"
SELECT
    n.nspname AS schema,
    c.relname AS table,
    p.polname AS name,
    CASE
        WHEN p.polpermissive THEN 'PERMISSIVE'::text
        ELSE 'RESTRICTIVE'::text
    END AS as_clause,
    CASE p.polcmd
        WHEN 'r'::"char" THEN 'SELECT'::text
        WHEN 'a'::"char" THEN 'INSERT'::text
        WHEN 'w'::"char" THEN 'UPDATE'::text
        WHEN 'd'::"char" THEN 'DELETE'::text
        WHEN '*'::"char" THEN 'ALL'::text
        ELSE NULL::text
    END AS for_clause,
    CASE
        WHEN p.polroles = '{0}'::oid[] THEN (string_to_array('public'::text, ''::text))::name[]
        ELSE ARRAY(
            SELECT pg_authid.rolname
            FROM pg_authid
            WHERE pg_authid.oid = ANY(p.polroles)
            ORDER BY pg_authid.rolname
        )
    END AS to,
    pg_get_expr(p.polqual, p.polrelid) AS using,
    pg_get_expr(p.polwithcheck, p.polrelid) AS with_check
FROM pg_policy p
JOIN pg_class c ON c.oid = p.polrelid
JOIN pg_namespace n ON n.oid = c.relnamespace
WHERE n.nspname NOT LIKE 'pg_%'
  AND n.nspname <> 'information_schema'
  AND has_schema_privilege(current_user, n.oid, 'USAGE')
  AND has_table_privilege(current_user, c.oid, 'SELECT')
ORDER BY n.nspname, c.relname, p.polname
"#;
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Convert `PostgreSQL` foreign key action codes to human-readable strings.
///
/// `PostgreSQL` stores FK actions as single-character codes in `pg_constraint`.
#[must_use]
pub fn pg_action_code_to_string(code: &str) -> String {
    match code {
        "r" => "RESTRICT",
        "c" => "CASCADE",
        "n" => "SET NULL",
        "d" => "SET DEFAULT",
        // "a" (NO ACTION) and any unknown code fall through to NO ACTION.
        _ => "NO ACTION",
    }
    .to_string()
}

/// Parse raw index column strings from `pg_get_indexdef` into `RawIndexColumnInfo`.
///
/// Each string is a single column expression like `"name"`, `"name DESC"`,
/// `"lower(name)"`, or `"name text_pattern_ops"`.
#[must_use]
pub fn parse_index_columns(cols: Vec<String>) -> Vec<RawIndexColumnInfo> {
    cols.into_iter()
        .map(|c| {
            let trimmed = c.trim().to_string();
            let upper = trimmed.to_uppercase();

            let asc = !upper.contains(" DESC");
            let nulls_first = upper.contains(" NULLS FIRST");

            // Strip sort/nulls directives for opclass parsing / expression detection.
            let mut core = trimmed;
            for token in [" ASC", " DESC", " NULLS FIRST", " NULLS LAST"] {
                if let Some(pos) = core.to_uppercase().find(token) {
                    core.truncate(pos);
                    break;
                }
            }
            let core = core.trim().to_string();

            // Heuristic: treat as expression if it contains parentheses or spaces.
            let is_expression = core.contains('(')
                || core.contains(')')
                || core.contains(' ')
                || core.contains("::");

            // Heuristic opclass parsing: split whitespace and take second token if it looks like opclass.
            let mut opclass: Option<String> = None;
            let mut name = core.clone();
            let parts: Vec<&str> = core.split_whitespace().collect();
            if parts.len() >= 2 {
                let second = parts[1];
                if !matches!(second.to_uppercase().as_str(), "ASC" | "DESC" | "NULLS") {
                    opclass = Some(second.to_string());
                    name = parts[0].to_string();
                }
            }

            RawIndexColumnInfo {
                name,
                is_expression,
                asc,
                nulls_first,
                opclass,
            }
        })
        .collect()
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
                is_unlogged: false,
                is_temporary: false,
                tablespace: None,
                comment: None,
                is_rls_enabled: false,
            },
            RawTableInfo {
                schema: "pg_catalog".to_string(),
                name: "pg_class".to_string(),
                is_unlogged: false,
                is_temporary: false,
                tablespace: None,
                comment: None,
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
            is_unlogged: None,
            is_temporary: None,
            inherits: None,
            tablespace: None,
            is_rls_enabled: None,
            comment: None,
        });

        let snapshot = result.to_snapshot();
        assert_eq!(snapshot.ddl.len(), 2);
    }

    #[test]
    fn postgres_catalog_queries_are_privilege_scoped() {
        use queries::{
            CHECKS_QUERY, COLUMNS_QUERY, ENUMS_QUERY, FOREIGN_KEYS_QUERY, INDEXES_QUERY,
            POLICIES_QUERY, PRIMARY_KEYS_QUERY, SCHEMAS_QUERY, SEQUENCES_QUERY, TABLES_QUERY,
            UNIQUES_QUERY, VIEWS_QUERY,
        };

        for query in [
            SCHEMAS_QUERY,
            TABLES_QUERY,
            COLUMNS_QUERY,
            ENUMS_QUERY,
            SEQUENCES_QUERY,
            VIEWS_QUERY,
            INDEXES_QUERY,
            FOREIGN_KEYS_QUERY,
            PRIMARY_KEYS_QUERY,
            UNIQUES_QUERY,
            CHECKS_QUERY,
            POLICIES_QUERY,
        ] {
            assert!(
                query.contains("has_schema_privilege"),
                "query is not schema-privilege scoped: {query}"
            );
        }

        for query in [
            TABLES_QUERY,
            VIEWS_QUERY,
            INDEXES_QUERY,
            FOREIGN_KEYS_QUERY,
            PRIMARY_KEYS_QUERY,
            UNIQUES_QUERY,
            CHECKS_QUERY,
            POLICIES_QUERY,
        ] {
            assert!(
                query.contains("has_table_privilege"),
                "query is not table-privilege scoped: {query}"
            );
        }
    }
}
