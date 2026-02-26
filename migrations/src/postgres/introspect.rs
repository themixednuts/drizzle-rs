//! PostgreSQL database introspection
//!
//! This module provides functionality to introspect an existing PostgreSQL database
//! and extract its schema as DDL entities, matching drizzle-kit introspect.ts

use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, IndexColumn, Policy, PostgresEntity,
    PrimaryKey, Role, Schema, Sequence, Table, UniqueConstraint, View,
};
use super::grammar::{is_serial_expression, is_system_namespace, is_system_role};
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

/// Extract the sequence name from a `nextval('...'::regclass)` expression.
///
/// Handles patterns like:
/// - `nextval('push_creates_id_seq'::regclass)` → `push_creates_id_seq`
/// - `nextval('public.push_creates_id_seq'::regclass)` → `push_creates_id_seq`
/// - `nextval('"public"."push_creates_id_seq"'::regclass)` → `push_creates_id_seq`
fn extract_nextval_sequence(expr: &str) -> Option<String> {
    let inner = expr
        .strip_prefix("nextval('")?
        .strip_suffix("'::regclass)")?;
    // Take the part after the last '.' (schema separator)
    let name_part = match inner.rfind('.') {
        Some(pos) => &inner[pos + 1..],
        None => inner,
    };
    // Strip surrounding quotes
    let name = name_part.trim_matches('"');
    if name.is_empty() {
        return None;
    }
    Some(name.to_string())
}

impl IntrospectionResult {
    /// Collect (schema, name) pairs for sequences owned by serial/bigserial columns.
    ///
    /// These sequences are auto-managed by PostgreSQL and should not appear in the
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
    ///
    /// Uses the underlying `pg_sequence` + `pg_class` catalog tables instead
    /// of the `pg_sequences` convenience view.  The view internally calls
    /// `pg_sequence_parameters()` which can fail when sequences are being
    /// dropped concurrently (e.g. during parallel test runs).  Direct
    /// catalog access is fully MVCC-protected and avoids this issue.
    ///
    /// Value columns are nullable because the current user may lack
    /// privilege on the sequence.
    pub const SEQUENCES_QUERY: &str = r#"
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
        ORDER BY n.nspname, c.relname
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

    /// Query to get all indexes
    pub const INDEXES_QUERY: &str = r#"
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
GROUP BY ns.nspname, tbl.relname, idx.relname, ix.indisunique, ix.indisprimary, am.amname, ix.indpred, ix.indrelid
ORDER BY ns.nspname, tbl.relname, idx.relname
"#;

    /// Schema-filtered variant of [`INDEXES_QUERY`].
    ///
    /// `pg_get_indexdef()` calls `relation_open()` which is not
    /// MVCC-protected and can fail if concurrent DDL drops an index.
    /// Scoping to specific schemas (`$1::text[]`) avoids encountering
    /// OIDs from schemas being modified by other sessions.
    pub const INDEXES_QUERY_FILTERED: &str = r#"
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
GROUP BY ns.nspname, tbl.relname, idx.relname, ix.indisunique, ix.indisprimary, am.amname, ix.indpred, ix.indrelid
ORDER BY ns.nspname, tbl.relname, idx.relname
"#;

    /// Query to get all foreign keys
    pub const FOREIGN_KEYS_QUERY: &str = r#"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    array_agg(src.attname ORDER BY s.ord) AS columns,
    ns_to.nspname AS schema_to,
    tbl_to.relname AS table_to,
    array_agg(dst.attname ORDER BY s.ord) AS columns_to,
    con.confupdtype::text AS on_update,
    con.confdeltype::text AS on_delete
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
GROUP BY ns.nspname, tbl.relname, con.conname, ns_to.nspname, tbl_to.relname, con.confupdtype, con.confdeltype
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

    /// Query to get all primary keys
    pub const PRIMARY_KEYS_QUERY: &str = r#"
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
GROUP BY ns.nspname, tbl.relname, con.conname
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

    /// Query to get all unique constraints
    pub const UNIQUES_QUERY: &str = r#"
SELECT
    ns.nspname AS schema,
    tbl.relname AS table,
    con.conname AS name,
    array_agg(att.attname ORDER BY s.ord) AS columns,
    FALSE AS nulls_not_distinct
FROM pg_constraint con
JOIN pg_class tbl ON tbl.oid = con.conrelid
JOIN pg_namespace ns ON ns.oid = tbl.relnamespace
JOIN unnest(con.conkey) WITH ORDINALITY AS s(attnum, ord) ON TRUE
JOIN pg_attribute att ON att.attrelid = tbl.oid AND att.attnum = s.attnum
WHERE con.contype = 'u'
  AND ns.nspname NOT LIKE 'pg_%'
  AND ns.nspname <> 'information_schema'
GROUP BY ns.nspname, tbl.relname, con.conname
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

    /// Query to get all check constraints
    pub const CHECKS_QUERY: &str = r#"
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
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

    /// Schema-filtered variant of [`CHECKS_QUERY`].
    ///
    /// `pg_get_expr()` calls `relation_open()` which is not MVCC-protected.
    /// Scoping to specific schemas avoids encountering OIDs from
    /// schemas being modified by other sessions.
    pub const CHECKS_QUERY_FILTERED: &str = r#"
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
ORDER BY ns.nspname, tbl.relname, con.conname
"#;

    /// Query to get all roles
    pub const ROLES_QUERY: &str = r#"
SELECT
    rolname AS name,
    rolcreatedb AS create_db,
    rolcreaterole AS create_role,
    rolinherit AS inherit
FROM pg_roles
ORDER BY rolname
"#;

    /// Query to get all policies
    pub const POLICIES_QUERY: &str = r#"
SELECT
    schemaname AS schema,
    tablename AS table,
    policyname AS name,
    upper(permissive) AS as_clause,
    upper(cmd) AS for_clause,
    roles AS to,
    qual AS using,
    with_check AS with_check
FROM pg_policies
WHERE schemaname NOT LIKE 'pg_%'
  AND schemaname <> 'information_schema'
ORDER BY schemaname, tablename, policyname
"#;
}

// =============================================================================
// Utility Functions
// =============================================================================

/// Convert PostgreSQL foreign key action codes to human-readable strings.
///
/// PostgreSQL stores FK actions as single-character codes in `pg_constraint`.
pub fn pg_action_code_to_string(code: &str) -> String {
    match code {
        "a" => "NO ACTION",
        "r" => "RESTRICT",
        "c" => "CASCADE",
        "n" => "SET NULL",
        "d" => "SET DEFAULT",
        _ => "NO ACTION",
    }
    .to_string()
}

/// Parse raw index column strings from `pg_get_indexdef` into `RawIndexColumnInfo`.
///
/// Each string is a single column expression like `"name"`, `"name DESC"`,
/// `"lower(name)"`, or `"name text_pattern_ops"`.
pub fn parse_index_columns(cols: Vec<String>) -> Vec<RawIndexColumnInfo> {
    cols.into_iter()
        .map(|c| {
            let trimmed = c.trim().to_string();
            let upper = trimmed.to_uppercase();

            let asc = !upper.contains(" DESC");
            let nulls_first = upper.contains(" NULLS FIRST");

            // Strip sort/nulls directives for opclass parsing / expression detection.
            let mut core = trimmed.clone();
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
