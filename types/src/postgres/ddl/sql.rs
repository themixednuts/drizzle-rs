//! SQL generation for `PostgreSQL` DDL types
//!
//! This module provides SQL generation methods for DDL types, enabling
//! unified SQL output from both compile-time and runtime schema definitions.

use crate::alloc_prelude::*;
use std::fmt::Write;

use super::{
    CheckConstraint, Column, Enum, ForeignKey, Generated, GeneratedType, Identity, IdentityType,
    Index, IndexColumnDef, Policy, PrimaryKey, Sequence, Table, UniqueConstraint, View,
};

// =============================================================================
// Table SQL Generation
// =============================================================================

/// A complete table definition with all related entities for SQL generation
#[derive(Clone, Debug)]
pub struct TableSql<'a> {
    pub table: &'a Table,
    pub columns: &'a [Column],
    pub primary_key: Option<&'a PrimaryKey>,
    pub foreign_keys: &'a [ForeignKey],
    pub unique_constraints: &'a [UniqueConstraint],
    pub check_constraints: &'a [CheckConstraint],
    pub indexes: &'a [Index],
    pub policies: &'a [Policy],
}

impl<'a> TableSql<'a> {
    /// Create a new `TableSql` for SQL generation
    #[must_use]
    pub const fn new(table: &'a Table) -> Self {
        Self {
            table,
            columns: &[],
            primary_key: None,
            foreign_keys: &[],
            unique_constraints: &[],
            check_constraints: &[],
            indexes: &[],
            policies: &[],
        }
    }

    /// Set columns
    #[must_use]
    pub const fn columns(mut self, columns: &'a [Column]) -> Self {
        self.columns = columns;
        self
    }

    /// Set primary key
    #[must_use]
    pub const fn primary_key(mut self, pk: Option<&'a PrimaryKey>) -> Self {
        self.primary_key = pk;
        self
    }

    /// Set foreign keys
    #[must_use]
    pub const fn foreign_keys(mut self, fks: &'a [ForeignKey]) -> Self {
        self.foreign_keys = fks;
        self
    }

    /// Set unique constraints
    #[must_use]
    pub const fn unique_constraints(mut self, uniques: &'a [UniqueConstraint]) -> Self {
        self.unique_constraints = uniques;
        self
    }

    /// Set check constraints
    #[must_use]
    pub const fn check_constraints(mut self, checks: &'a [CheckConstraint]) -> Self {
        self.check_constraints = checks;
        self
    }

    /// Set indexes
    #[must_use]
    pub const fn indexes(mut self, indexes: &'a [Index]) -> Self {
        self.indexes = indexes;
        self
    }

    /// Set policies
    #[must_use]
    pub const fn policies(mut self, policies: &'a [Policy]) -> Self {
        self.policies = policies;
        self
    }

    fn schema_prefix(&self) -> String {
        if self.table.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.table.schema())
        }
    }

    /// Generate CREATE TABLE SQL
    #[must_use]
    pub fn create_table_sql(&self) -> String {
        let schema_prefix = self.schema_prefix();
        let mut sql = format!(
            "CREATE TABLE {}\"{}\" (\n",
            schema_prefix,
            self.table.name()
        );

        let mut lines = Vec::new();

        // Column definitions
        for column in self.columns {
            lines.push(format!("\t{}", column.to_column_sql()));
        }

        // Primary key
        if let Some(pk) = &self.primary_key {
            let cols = pk
                .columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            if pk.name_explicit {
                lines.push(format!(
                    "\tCONSTRAINT \"{}\" PRIMARY KEY({})",
                    pk.name(),
                    cols
                ));
            } else {
                lines.push(format!("\tPRIMARY KEY({cols})"));
            }
        }

        // Foreign keys
        for fk in self.foreign_keys {
            lines.push(format!("\t{}", fk.to_constraint_sql()));
        }

        // Unique constraints
        for unique in self.unique_constraints {
            let cols = unique
                .columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "\tCONSTRAINT \"{}\" UNIQUE({})",
                unique.name(),
                cols
            ));
        }

        // Check constraints
        for check in self.check_constraints {
            lines.push(format!(
                "\tCONSTRAINT \"{}\" CHECK ({})",
                check.name(),
                &check.value
            ));
        }

        sql.push_str(&lines.join(",\n"));
        sql.push_str("\n);");

        sql
    }

    /// Generate DROP TABLE SQL
    #[must_use]
    pub fn drop_table_sql(&self) -> String {
        let schema_prefix = self.schema_prefix();
        format!("DROP TABLE {}\"{}\";", schema_prefix, self.table.name())
    }

    /// Generate all related indexes
    #[must_use]
    pub fn create_indexes_sql(&self) -> Vec<String> {
        self.indexes
            .iter()
            .map(super::index::Index::create_index_sql)
            .collect()
    }

    /// Generate RLS enable statement if needed
    #[must_use]
    pub fn enable_rls_sql(&self) -> Option<String> {
        if self.table.is_rls_enabled.unwrap_or(false) {
            let schema_prefix = self.schema_prefix();
            Some(format!(
                "ALTER TABLE {}\"{}\" ENABLE ROW LEVEL SECURITY;",
                schema_prefix,
                self.table.name()
            ))
        } else {
            None
        }
    }

    /// Generate all policies
    #[must_use]
    pub fn create_policies_sql(&self) -> Vec<String> {
        self.policies
            .iter()
            .map(super::policy::Policy::create_policy_sql)
            .collect()
    }
}

// =============================================================================
// Column SQL Generation
// =============================================================================

impl Column {
    /// Generate the column definition SQL (without leading/trailing punctuation)
    #[must_use]
    pub fn to_column_sql(&self) -> String {
        let mut sql = format!("\"{}\" {}", self.name(), self.sql_type());

        // Handle identity columns
        if let Some(identity) = &self.identity {
            sql.push_str(&identity.to_sql());
        }

        // Handle generated columns
        if let Some(generated) = &self.generated {
            sql.push_str(&generated.to_sql());
        }

        // Default value (skip if identity or generated - PostgreSQL doesn't allow both)
        if self.identity.is_none()
            && self.generated.is_none()
            && let Some(default) = self.default.as_ref()
        {
            let _ = write!(sql, " DEFAULT {default}");
        }

        // NOT NULL
        if self.not_null {
            sql.push_str(" NOT NULL");
        }

        sql
    }

    /// Generate ADD COLUMN SQL
    #[must_use]
    pub fn add_column_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD COLUMN {};",
            schema_prefix,
            self.table(),
            self.to_column_sql()
        )
    }

    /// Generate DROP COLUMN SQL
    #[must_use]
    pub fn drop_column_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" DROP COLUMN \"{}\";",
            schema_prefix,
            self.table(),
            self.name()
        )
    }
}

// =============================================================================
// Identity Column SQL
// =============================================================================

impl Identity {
    /// Generate the GENERATED AS IDENTITY clause
    #[must_use]
    pub fn to_sql(&self) -> String {
        let identity_type = match self.type_ {
            IdentityType::Always => "ALWAYS",
            IdentityType::ByDefault => "BY DEFAULT",
        };

        let mut sql = format!(" GENERATED {identity_type} AS IDENTITY");

        // Add sequence options if any are specified
        let mut options = Vec::new();

        if let Some(increment) = self.increment.as_ref() {
            options.push(format!("INCREMENT BY {increment}"));
        }
        if let Some(min) = self.min_value.as_ref() {
            options.push(format!("MINVALUE {min}"));
        }
        if let Some(max) = self.max_value.as_ref() {
            options.push(format!("MAXVALUE {max}"));
        }
        if let Some(start) = self.start_with.as_ref() {
            options.push(format!("START WITH {start}"));
        }
        if let Some(cache) = self.cache {
            options.push(format!("CACHE {cache}"));
        }
        if self.cycle.unwrap_or(false) {
            options.push("CYCLE".to_string());
        }

        if !options.is_empty() {
            let _ = write!(sql, " ({})", options.join(" "));
        }

        sql
    }
}

// =============================================================================
// Generated Column SQL
// =============================================================================

impl Generated {
    /// Generate the GENERATED clause SQL
    #[must_use]
    pub fn to_sql(&self) -> String {
        let gen_type = match self.gen_type {
            GeneratedType::Stored => "STORED",
        };
        format!(" GENERATED ALWAYS AS ({}) {}", self.expression, gen_type)
    }
}

// =============================================================================
// Foreign Key SQL Generation
// =============================================================================

impl ForeignKey {
    /// Generate the CONSTRAINT ... FOREIGN KEY clause SQL
    #[must_use]
    pub fn to_constraint_sql(&self) -> String {
        let from_cols = self
            .columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let to_cols = self
            .columns_to
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let to_schema_prefix = if self.schema_to() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema_to())
        };

        let mut sql = format!(
            "CONSTRAINT \"{}\" FOREIGN KEY ({}) REFERENCES {}\"{}\"({})",
            self.name(),
            from_cols,
            to_schema_prefix,
            self.table_to(),
            to_cols
        );

        if let Some(on_update) = self.on_update.as_ref()
            && on_update != "NO ACTION"
        {
            let _ = write!(sql, " ON UPDATE {}", on_update.to_uppercase());
        }

        if let Some(on_delete) = self.on_delete.as_ref()
            && on_delete != "NO ACTION"
        {
            let _ = write!(sql, " ON DELETE {}", on_delete.to_uppercase());
        }

        sql
    }

    /// Generate ADD FOREIGN KEY SQL
    #[must_use]
    pub fn add_fk_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD {};",
            schema_prefix,
            self.table(),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP FOREIGN KEY SQL
    #[must_use]
    pub fn drop_fk_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" DROP CONSTRAINT \"{}\";",
            schema_prefix,
            self.table(),
            self.name()
        )
    }
}

// =============================================================================
// Index SQL Generation
// =============================================================================

impl Index {
    /// Generate CREATE INDEX SQL
    #[must_use]
    pub fn create_index_sql(&self) -> String {
        let unique = if self.is_unique { "UNIQUE " } else { "" };

        let concurrently = if self.concurrently {
            "CONCURRENTLY "
        } else {
            ""
        };

        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };

        let columns = self
            .columns
            .iter()
            .map(super::index::IndexColumn::to_sql)
            .collect::<Vec<_>>()
            .join(", ");

        let using = self
            .method
            .as_ref()
            .map(|m| format!(" USING {m}"))
            .unwrap_or_default();

        let mut sql = format!(
            "CREATE {}{}INDEX \"{}\" ON {}\"{}\"{}({});",
            unique,
            concurrently,
            self.name(),
            schema_prefix,
            self.table(),
            using,
            columns
        );

        if let Some(where_clause) = self.where_clause.as_ref() {
            // Remove trailing semicolon to add WHERE
            sql.pop();
            let _ = write!(sql, " WHERE {where_clause};");
        }

        sql
    }

    /// Generate DROP INDEX SQL
    #[must_use]
    pub fn drop_index_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!("DROP INDEX {}\"{}\";", schema_prefix, self.name())
    }
}

impl IndexColumnDef {
    /// Generate the column reference for an index
    #[must_use]
    pub fn to_sql(&self) -> String {
        let mut sql = if self.is_expression {
            format!("({})", self.value)
        } else {
            format!("\"{}\"", self.value)
        };

        if let Some(op) = self.opclass.as_ref() {
            let _ = write!(sql, " {op}");
        }

        if !self.asc {
            sql.push_str(" DESC");
        }

        if self.nulls_first {
            sql.push_str(" NULLS FIRST");
        }

        sql
    }
}

// =============================================================================
// Enum SQL Generation
// =============================================================================

impl Enum {
    /// Generate CREATE TYPE ... AS ENUM SQL
    #[must_use]
    pub fn create_enum_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        let values = self
            .values
            .iter()
            .map(|v| format!("'{v}'"))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "CREATE TYPE {}\"{}\" AS ENUM ({});",
            schema_prefix,
            self.name(),
            values
        )
    }

    /// Generate DROP TYPE SQL
    #[must_use]
    pub fn drop_enum_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!("DROP TYPE {}\"{}\";", schema_prefix, self.name())
    }

    /// Generate ALTER TYPE ... ADD VALUE SQL
    #[must_use]
    pub fn add_value_sql(&self, value: &str, before: Option<&str>) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        before.map_or_else(
            || {
                format!(
                    "ALTER TYPE {}\"{}\" ADD VALUE '{}';",
                    schema_prefix,
                    self.name(),
                    value
                )
            },
            |before_value| {
                format!(
                    "ALTER TYPE {}\"{}\" ADD VALUE '{}' BEFORE '{}';",
                    schema_prefix,
                    self.name(),
                    value,
                    before_value
                )
            },
        )
    }
}

// =============================================================================
// Sequence SQL Generation
// =============================================================================

impl Sequence {
    /// Generate CREATE SEQUENCE SQL
    #[must_use]
    pub fn create_sequence_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };

        let mut sql = format!("CREATE SEQUENCE {}\"{}\"", schema_prefix, self.name());

        if let Some(inc) = self.increment_by.as_ref() {
            let _ = write!(sql, " INCREMENT BY {inc}");
        }
        if let Some(min) = self.min_value.as_ref() {
            let _ = write!(sql, " MINVALUE {min}");
        }
        if let Some(max) = self.max_value.as_ref() {
            let _ = write!(sql, " MAXVALUE {max}");
        }
        if let Some(start) = self.start_with.as_ref() {
            let _ = write!(sql, " START WITH {start}");
        }
        if let Some(cache) = self.cache_size {
            let _ = write!(sql, " CACHE {cache}");
        }
        if self.cycle.unwrap_or(false) {
            sql.push_str(" CYCLE");
        }

        sql.push(';');
        sql
    }

    /// Generate DROP SEQUENCE SQL
    #[must_use]
    pub fn drop_sequence_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!("DROP SEQUENCE {}\"{}\";", schema_prefix, self.name())
    }
}

// =============================================================================
// View SQL Generation
// =============================================================================

/// Append WITH-options clause for `CREATE VIEW`, returning the uppercased
/// CHECK OPTION clause (to be emitted after `AS <definition>`), if any.
fn append_view_with_options(
    sql: &mut String,
    with_opts: &super::view::ViewWithOption,
) -> Option<String> {
    let mut options = String::new();
    let mut has_option = false;

    let mut push_option = |name: &str, value: &dyn core::fmt::Display| {
        if has_option {
            options.push_str(", ");
        } else {
            has_option = true;
        }
        let _ = write!(options, "{name} = {value}");
    };

    if let Some(value) = with_opts.security_barrier {
        push_option("security_barrier", &value);
    }
    if let Some(value) = with_opts.security_invoker {
        push_option("security_invoker", &value);
    }
    if let Some(value) = with_opts.fillfactor {
        push_option("fillfactor", &value);
    }
    if let Some(value) = with_opts.toast_tuple_target {
        push_option("toast_tuple_target", &value);
    }
    if let Some(value) = with_opts.parallel_workers {
        push_option("parallel_workers", &value);
    }
    if let Some(value) = with_opts.autovacuum_enabled {
        push_option("autovacuum_enabled", &value);
    }
    if let Some(value) = with_opts.vacuum_index_cleanup.as_ref() {
        push_option("vacuum_index_cleanup", value);
    }
    if let Some(value) = with_opts.vacuum_truncate {
        push_option("vacuum_truncate", &value);
    }
    if let Some(value) = with_opts.autovacuum_vacuum_threshold {
        push_option("autovacuum_vacuum_threshold", &value);
    }
    if let Some(value) = with_opts.autovacuum_vacuum_scale_factor {
        push_option("autovacuum_vacuum_scale_factor", &value);
    }
    if let Some(value) = with_opts.autovacuum_vacuum_cost_delay {
        push_option("autovacuum_vacuum_cost_delay", &value);
    }
    if let Some(value) = with_opts.autovacuum_vacuum_cost_limit {
        push_option("autovacuum_vacuum_cost_limit", &value);
    }
    if let Some(value) = with_opts.autovacuum_freeze_min_age {
        push_option("autovacuum_freeze_min_age", &value);
    }
    if let Some(value) = with_opts.autovacuum_freeze_max_age {
        push_option("autovacuum_freeze_max_age", &value);
    }
    if let Some(value) = with_opts.autovacuum_freeze_table_age {
        push_option("autovacuum_freeze_table_age", &value);
    }
    if let Some(value) = with_opts.autovacuum_multixact_freeze_min_age {
        push_option("autovacuum_multixact_freeze_min_age", &value);
    }
    if let Some(value) = with_opts.autovacuum_multixact_freeze_max_age {
        push_option("autovacuum_multixact_freeze_max_age", &value);
    }
    if let Some(value) = with_opts.autovacuum_multixact_freeze_table_age {
        push_option("autovacuum_multixact_freeze_table_age", &value);
    }
    if let Some(value) = with_opts.log_autovacuum_min_duration {
        push_option("log_autovacuum_min_duration", &value);
    }
    if let Some(value) = with_opts.user_catalog_table {
        push_option("user_catalog_table", &value);
    }

    if has_option {
        let _ = write!(sql, " WITH ({options})");
    }

    with_opts
        .check_option
        .as_deref()
        .map(str::to_ascii_uppercase)
}

impl View {
    /// Generate CREATE VIEW SQL
    #[must_use]
    pub fn create_view_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };

        let materialized = if self.materialized {
            "MATERIALIZED "
        } else {
            ""
        };

        let Some(def) = self.definition.as_ref() else {
            return format!(
                "-- {}View {}\"{}\" has no definition",
                materialized,
                schema_prefix,
                self.name()
            );
        };

        let mut sql = String::with_capacity(def.len() + 64);
        let _ = write!(
            sql,
            "CREATE {}VIEW {}\"{}\"",
            materialized,
            schema_prefix,
            self.name(),
        );

        if let Some(using) = self.using.as_ref() {
            let _ = write!(sql, " USING {using}");
        }

        let check_option_clause = self
            .with
            .as_ref()
            .and_then(|with_opts| append_view_with_options(&mut sql, with_opts));

        if let Some(tablespace) = self.tablespace.as_ref() {
            let _ = write!(sql, " TABLESPACE \"{tablespace}\"");
        }

        sql.push_str(" AS ");
        sql.push_str(def);

        if let Some(check_option) = check_option_clause {
            let _ = write!(sql, " WITH {check_option} CHECK OPTION");
        }

        if self.materialized && matches!(self.with_no_data, Some(true)) {
            sql.push_str(" WITH NO DATA");
        }

        sql.push(';');
        sql
    }

    /// Generate DROP VIEW SQL
    #[must_use]
    pub fn drop_view_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        let materialized = if self.materialized {
            "MATERIALIZED "
        } else {
            ""
        };
        format!(
            "DROP {}VIEW {}\"{}\";",
            materialized,
            schema_prefix,
            self.name()
        )
    }
}

// =============================================================================
// Policy SQL Generation
// =============================================================================

impl Policy {
    /// Generate CREATE POLICY SQL
    #[must_use]
    pub fn create_policy_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };

        let mut sql = format!(
            "CREATE POLICY \"{}\" ON {}\"{}\"",
            self.name(),
            schema_prefix,
            self.table()
        );

        if let Some(r#for) = self.for_clause.as_ref() {
            let _ = write!(sql, " FOR {}", r#for.to_uppercase());
        }

        if let Some(to) = self.to.as_ref()
            && !to.is_empty()
        {
            let to_roles = to
                .iter()
                .map(|r| {
                    if *r == "public" {
                        "PUBLIC".to_string()
                    } else {
                        format!("\"{r}\"")
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            let _ = write!(sql, " TO {to_roles}");
        }

        if let Some(using) = self.using.as_ref() {
            let _ = write!(sql, " USING ({using})");
        }

        if let Some(with_check) = self.with_check.as_ref() {
            let _ = write!(sql, " WITH CHECK ({with_check})");
        }

        sql.push(';');
        sql
    }

    /// Generate DROP POLICY SQL
    #[must_use]
    pub fn drop_policy_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "DROP POLICY \"{}\" ON {}\"{}\";",
            self.name(),
            schema_prefix,
            self.table()
        )
    }
}

// =============================================================================
// Table-level utilities
// =============================================================================

impl Table {
    /// Generate DROP TABLE SQL
    #[must_use]
    pub fn drop_table_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!("DROP TABLE {}\"{}\";", schema_prefix, self.name())
    }

    /// Generate RENAME TABLE SQL
    #[must_use]
    pub fn rename_table_sql(&self, new_name: &str) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" RENAME TO \"{}\";",
            schema_prefix,
            self.name(),
            new_name
        )
    }
}

// =============================================================================
// Primary Key SQL Generation
// =============================================================================

impl PrimaryKey {
    /// Generate the PRIMARY KEY constraint clause
    #[must_use]
    pub fn to_constraint_sql(&self) -> String {
        let cols = self
            .columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");

        format!("CONSTRAINT \"{}\" PRIMARY KEY({})", self.name(), cols)
    }

    /// Generate ADD PRIMARY KEY SQL
    #[must_use]
    pub fn add_pk_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD {};",
            schema_prefix,
            self.table(),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP PRIMARY KEY SQL
    #[must_use]
    pub fn drop_pk_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" DROP CONSTRAINT \"{}\";",
            schema_prefix,
            self.table(),
            self.name()
        )
    }
}

// =============================================================================
// Unique Constraint SQL Generation
// =============================================================================

impl UniqueConstraint {
    /// Generate the UNIQUE constraint clause
    #[must_use]
    pub fn to_constraint_sql(&self) -> String {
        let cols = self
            .columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");

        format!("CONSTRAINT \"{}\" UNIQUE({})", self.name(), cols)
    }

    /// Generate ADD UNIQUE SQL
    #[must_use]
    pub fn add_unique_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD {};",
            schema_prefix,
            self.table(),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP UNIQUE SQL
    #[must_use]
    pub fn drop_unique_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" DROP CONSTRAINT \"{}\";",
            schema_prefix,
            self.table(),
            self.name()
        )
    }
}

// =============================================================================
// Check Constraint SQL Generation
// =============================================================================

impl CheckConstraint {
    /// Generate the CHECK constraint clause
    #[must_use]
    pub fn to_constraint_sql(&self) -> String {
        format!("CONSTRAINT \"{}\" CHECK ({})", self.name(), &self.value)
    }

    /// Generate ADD CHECK SQL
    #[must_use]
    pub fn add_check_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD {};",
            schema_prefix,
            self.table(),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP CHECK SQL
    #[must_use]
    pub fn drop_check_sql(&self) -> String {
        let schema_prefix = if self.schema() == "public" {
            String::new()
        } else {
            format!("\"{}\".", self.schema())
        };
        format!(
            "ALTER TABLE {}\"{}\" DROP CONSTRAINT \"{}\";",
            schema_prefix,
            self.table(),
            self.name()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::postgres::ddl::{ColumnDef, PrimaryKeyDef, TableDef};
    use std::borrow::Cow;

    #[test]
    fn test_simple_create_table() {
        let table = TableDef::new("public", "users").into_table();
        let columns = [
            ColumnDef::new("public", "users", "id", "SERIAL")
                .not_null()
                .into_column(),
            ColumnDef::new("public", "users", "name", "TEXT")
                .not_null()
                .into_column(),
            ColumnDef::new("public", "users", "email", "TEXT").into_column(),
        ];
        const PK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
        let pk = PrimaryKeyDef::new("public", "users", "users_pkey")
            .columns(PK_COLS)
            .into_primary_key();

        let sql = TableSql::new(&table)
            .columns(&columns)
            .primary_key(Some(&pk))
            .create_table_sql();

        assert!(sql.contains("CREATE TABLE \"users\""));
        assert!(sql.contains("\"id\" SERIAL NOT NULL"));
        assert!(sql.contains("\"name\" TEXT NOT NULL"));
        assert!(sql.contains("\"email\" TEXT"));
    }

    #[test]
    fn test_table_with_schema() {
        let table = TableDef::new("myschema", "users").into_table();
        let sql = TableSql::new(&table).create_table_sql();
        assert!(sql.contains("\"myschema\".\"users\""));
    }
}
