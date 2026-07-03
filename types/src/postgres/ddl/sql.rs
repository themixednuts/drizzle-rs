//! SQL generation for `PostgreSQL` DDL types
//!
//! This module provides SQL generation methods for DDL types, enabling
//! unified SQL output from both compile-time and runtime schema definitions.

use crate::alloc_prelude::*;
use core::fmt::Write;

use super::{
    CheckConstraint, Column, Enum, ForeignKey, Generated, GeneratedType, Identity, IdentityType,
    Index, IndexColumn, IndexColumnDef, Policy, PrimaryKey, Sequence, Table, UniqueConstraint,
    View,
};

fn quote_ident(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

fn quote_literal(value: &str) -> String {
    format!("'{}'", value.replace('\'', "''"))
}

fn schema_prefix(schema: &str) -> String {
    if schema == "public" {
        String::new()
    } else {
        format!("{}.", quote_ident(schema))
    }
}

fn qualified_name(schema: &str, name: &str) -> String {
    format!("{}{}", schema_prefix(schema), quote_ident(name))
}

fn index_column_sql(column: &IndexColumn) -> String {
    let mut sql = if column.is_expression {
        format!("({})", column.value)
    } else {
        quote_ident(&column.value)
    };

    if let Some(op) = column.opclass.as_ref() {
        let _ = write!(sql, " {op}");
    }

    if !column.asc {
        sql.push_str(" DESC");
    }

    if column.nulls_first {
        sql.push_str(" NULLS FIRST");
    }

    sql
}

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

    /// Generate CREATE TABLE SQL
    #[must_use]
    pub fn create_table_sql(&self) -> String {
        let table_kind = if self.table.is_temporary.unwrap_or(false) {
            "TEMPORARY "
        } else if self.table.is_unlogged.unwrap_or(false) {
            "UNLOGGED "
        } else {
            ""
        };
        let mut sql = format!(
            "CREATE {}TABLE {} (\n",
            table_kind,
            qualified_name(self.table.schema(), self.table.name())
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
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", ");
            if pk.name_explicit {
                lines.push(format!(
                    "\tCONSTRAINT {} PRIMARY KEY({})",
                    quote_ident(pk.name()),
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
            lines.push(format!("\t{}", unique.to_constraint_sql()));
        }

        // Check constraints
        for check in self.check_constraints {
            lines.push(format!(
                "\tCONSTRAINT {} CHECK ({})",
                quote_ident(check.name()),
                check.value
            ));
        }

        sql.push_str(&lines.join(",\n"));
        sql.push('\n');
        sql.push(')');

        if let Some(inherits) = self.table.inherits.as_ref() {
            let _ = write!(sql, " INHERITS ({})", quote_ident(inherits));
        }

        if let Some(tablespace) = self.table.tablespace.as_ref() {
            let _ = write!(sql, " TABLESPACE {}", quote_ident(tablespace));
        }

        sql.push(';');

        sql
    }

    /// Generate DROP TABLE SQL
    #[must_use]
    pub fn drop_table_sql(&self) -> String {
        format!(
            "DROP TABLE {};",
            qualified_name(self.table.schema(), self.table.name())
        )
    }

    /// Generate all related indexes
    #[must_use]
    pub fn create_indexes_sql(&self) -> Vec<String> {
        self.indexes
            .iter()
            .map(super::index::Index::create_index_sql)
            .collect()
    }

    /// Generate COMMENT ON statements for the table and its columns.
    #[must_use]
    pub fn create_comments_sql(&self) -> Vec<String> {
        let mut comments = Vec::new();
        let table_name = qualified_name(self.table.schema(), self.table.name());

        if let Some(comment) = self.table.comment.as_ref() {
            comments.push(format!(
                "COMMENT ON TABLE {} IS {};",
                table_name,
                quote_literal(comment)
            ));
        }

        for column in self.columns {
            if let Some(comment) = column.comment.as_ref() {
                comments.push(format!(
                    "COMMENT ON COLUMN {}.{} IS {};",
                    table_name,
                    quote_ident(column.name()),
                    quote_literal(comment)
                ));
            }
        }

        comments
    }

    /// Generate RLS enable statement if needed
    #[must_use]
    pub fn enable_rls_sql(&self) -> Option<String> {
        if self.table.is_rls_enabled.unwrap_or(false) {
            Some(format!(
                "ALTER TABLE {} ENABLE ROW LEVEL SECURITY;",
                qualified_name(self.table.schema(), self.table.name())
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
        let mut sql = format!("{} {}", quote_ident(self.name()), self.sql_type());

        if let Some(dimensions) = self.dimensions
            && dimensions > 0
        {
            for _ in 0..dimensions {
                sql.push_str("[]");
            }
        }

        // COLLATE follows the type in the PostgreSQL grammar. Collation
        // names are double-quoted identifiers (`COLLATE "en_US"`,
        // `COLLATE "C"`).
        if let Some(collate) = self.collate.as_ref() {
            let _ = write!(sql, " COLLATE {}", quote_ident(collate));
        }

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
        format!(
            "ALTER TABLE {} ADD COLUMN {};",
            qualified_name(self.schema(), self.table()),
            self.to_column_sql()
        )
    }

    /// Generate DROP COLUMN SQL
    #[must_use]
    pub fn drop_column_sql(&self) -> String {
        format!(
            "ALTER TABLE {} DROP COLUMN {};",
            qualified_name(self.schema(), self.table()),
            quote_ident(self.name())
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
            GeneratedType::Virtual => "VIRTUAL",
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
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");

        let to_cols = self
            .columns_to
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!(
            "CONSTRAINT {} FOREIGN KEY ({}) REFERENCES {}({})",
            quote_ident(self.name()),
            from_cols,
            qualified_name(self.schema_to(), self.table_to()),
            to_cols
        );

        if let Some(on_delete) = self.on_delete.as_ref()
            && !on_delete.eq_ignore_ascii_case("NO ACTION")
        {
            let _ = write!(sql, " ON DELETE {}", on_delete.to_uppercase());
        }

        if let Some(on_update) = self.on_update.as_ref()
            && !on_update.eq_ignore_ascii_case("NO ACTION")
        {
            let _ = write!(sql, " ON UPDATE {}", on_update.to_uppercase());
        }

        if self.deferrable || self.initially_deferred {
            sql.push_str(" DEFERRABLE");
            if self.initially_deferred {
                sql.push_str(" INITIALLY DEFERRED");
            }
        }

        sql
    }

    /// Generate ADD FOREIGN KEY SQL
    #[must_use]
    pub fn add_fk_sql(&self) -> String {
        format!(
            "ALTER TABLE {} ADD {};",
            qualified_name(self.schema(), self.table()),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP FOREIGN KEY SQL
    #[must_use]
    pub fn drop_fk_sql(&self) -> String {
        format!(
            "ALTER TABLE {} DROP CONSTRAINT {};",
            qualified_name(self.schema(), self.table()),
            quote_ident(self.name())
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

        let columns = self
            .columns
            .iter()
            .map(index_column_sql)
            .collect::<Vec<_>>()
            .join(", ");

        let using = self
            .method
            .as_ref()
            .map(|m| format!(" USING {m}"))
            .unwrap_or_default();

        let mut sql = format!(
            "CREATE {}INDEX {}{} ON {}{}({})",
            unique,
            concurrently,
            quote_ident(self.name()),
            qualified_name(self.schema(), self.table()),
            using,
            columns
        );

        if let Some(with) = self.with.as_ref() {
            let _ = write!(sql, " WITH ({with})");
        }

        if let Some(where_clause) = self.where_clause.as_ref() {
            let _ = write!(sql, " WHERE {where_clause}");
        }

        sql.push(';');
        sql
    }

    /// Generate DROP INDEX SQL
    #[must_use]
    pub fn drop_index_sql(&self) -> String {
        format!("DROP INDEX {};", qualified_name(self.schema(), self.name()))
    }
}

impl IndexColumnDef {
    /// Generate the column reference for an index
    #[must_use]
    pub fn to_sql(&self) -> String {
        let mut sql = if self.is_expression {
            format!("({})", self.value)
        } else {
            quote_ident(self.value)
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
        let values = self
            .values
            .iter()
            .map(|v| quote_literal(v))
            .collect::<Vec<_>>()
            .join(", ");
        format!(
            "CREATE TYPE {} AS ENUM ({});",
            qualified_name(self.schema(), self.name()),
            values
        )
    }

    /// Generate DROP TYPE SQL
    #[must_use]
    pub fn drop_enum_sql(&self) -> String {
        format!("DROP TYPE {};", qualified_name(self.schema(), self.name()))
    }

    /// Generate ALTER TYPE ... ADD VALUE SQL
    #[must_use]
    pub fn add_value_sql(&self, value: &str, before: Option<&str>) -> String {
        before.map_or_else(
            || {
                format!(
                    "ALTER TYPE {} ADD VALUE {};",
                    qualified_name(self.schema(), self.name()),
                    quote_literal(value)
                )
            },
            |before_value| {
                format!(
                    "ALTER TYPE {} ADD VALUE {} BEFORE {};",
                    qualified_name(self.schema(), self.name()),
                    quote_literal(value),
                    quote_literal(before_value)
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
        let mut sql = format!(
            "CREATE SEQUENCE {}",
            qualified_name(self.schema(), self.name())
        );

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
        format!(
            "DROP SEQUENCE {};",
            qualified_name(self.schema(), self.name())
        )
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
        let materialized = if self.materialized {
            "MATERIALIZED "
        } else {
            ""
        };

        let Some(def) = self.definition.as_ref() else {
            return format!(
                "-- {}View {} has no definition",
                materialized,
                qualified_name(self.schema(), self.name())
            );
        };

        let mut sql = String::with_capacity(def.len() + 64);
        let _ = write!(
            sql,
            "CREATE {}VIEW {}",
            materialized,
            qualified_name(self.schema(), self.name()),
        );

        if let Some(using) = self.using.as_ref() {
            let _ = write!(sql, " USING {using}");
        }

        let check_option_clause = self
            .with
            .as_ref()
            .and_then(|with_opts| append_view_with_options(&mut sql, with_opts));

        if let Some(tablespace) = self.tablespace.as_ref() {
            let _ = write!(sql, " TABLESPACE {}", quote_ident(tablespace));
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
        let materialized = if self.materialized {
            "MATERIALIZED "
        } else {
            ""
        };
        format!(
            "DROP {}VIEW {};",
            materialized,
            qualified_name(self.schema(), self.name())
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
        let mut sql = format!(
            "CREATE POLICY {} ON {}",
            quote_ident(self.name()),
            qualified_name(self.schema(), self.table())
        );

        let as_clause = self.as_clause.as_deref().unwrap_or("PERMISSIVE");
        let _ = write!(sql, " AS {}", as_clause.to_uppercase());

        if let Some(r#for) = self.for_clause.as_ref() {
            let _ = write!(sql, " FOR {}", r#for.to_uppercase());
        }

        if let Some(to) = self.to.as_ref()
            && !to.is_empty()
        {
            let to_roles = to
                .iter()
                .map(|r| {
                    if r.eq_ignore_ascii_case("public") {
                        "PUBLIC".to_string()
                    } else {
                        quote_ident(r)
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
        format!(
            "DROP POLICY {} ON {};",
            quote_ident(self.name()),
            qualified_name(self.schema(), self.table())
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
        format!("DROP TABLE {};", qualified_name(self.schema(), self.name()))
    }

    /// Generate RENAME TABLE SQL
    #[must_use]
    pub fn rename_table_sql(&self, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {};",
            qualified_name(self.schema(), self.name()),
            quote_ident(new_name)
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
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "CONSTRAINT {} PRIMARY KEY({})",
            quote_ident(self.name()),
            cols
        )
    }

    /// Generate ADD PRIMARY KEY SQL
    #[must_use]
    pub fn add_pk_sql(&self) -> String {
        format!(
            "ALTER TABLE {} ADD {};",
            qualified_name(self.schema(), self.table()),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP PRIMARY KEY SQL
    #[must_use]
    pub fn drop_pk_sql(&self) -> String {
        format!(
            "ALTER TABLE {} DROP CONSTRAINT {};",
            qualified_name(self.schema(), self.table()),
            quote_ident(self.name())
        )
    }
}

// =============================================================================
// Unique Constraint SQL Generation
// =============================================================================

impl UniqueConstraint {
    fn constraint_sql(&self, space_before_columns: bool) -> String {
        let cols = self
            .columns
            .iter()
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");

        let separator = if space_before_columns { " " } else { "" };
        let mut sql = format!(
            "CONSTRAINT {} UNIQUE{}({})",
            quote_ident(self.name()),
            separator,
            cols
        );
        if self.deferrable || self.initially_deferred {
            sql.push_str(" DEFERRABLE");
            if self.initially_deferred {
                sql.push_str(" INITIALLY DEFERRED");
            }
        }
        sql
    }

    /// Generate the UNIQUE constraint clause
    #[must_use]
    pub fn to_constraint_sql(&self) -> String {
        self.constraint_sql(false)
    }

    /// Generate ADD UNIQUE SQL
    #[must_use]
    pub fn add_unique_sql(&self) -> String {
        format!(
            "ALTER TABLE {} ADD {};",
            qualified_name(self.schema(), self.table()),
            self.constraint_sql(true)
        )
    }

    /// Generate DROP UNIQUE SQL
    #[must_use]
    pub fn drop_unique_sql(&self) -> String {
        format!(
            "ALTER TABLE {} DROP CONSTRAINT {};",
            qualified_name(self.schema(), self.table()),
            quote_ident(self.name())
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
        format!(
            "CONSTRAINT {} CHECK ({})",
            quote_ident(self.name()),
            self.value
        )
    }

    /// Generate ADD CHECK SQL
    #[must_use]
    pub fn add_check_sql(&self) -> String {
        format!(
            "ALTER TABLE {} ADD {};",
            qualified_name(self.schema(), self.table()),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP CHECK SQL
    #[must_use]
    pub fn drop_check_sql(&self) -> String {
        format!(
            "ALTER TABLE {} DROP CONSTRAINT {};",
            qualified_name(self.schema(), self.table()),
            quote_ident(self.name())
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

    #[test]
    fn test_unique_concurrently_index_word_order() {
        let mut index = Index::new(
            "public",
            "users",
            "users_email_idx",
            vec![IndexColumn::new("email")],
        )
        .unique();
        index.concurrently = true;

        assert_eq!(
            index.create_index_sql(),
            "CREATE UNIQUE INDEX CONCURRENTLY \"users_email_idx\" ON \"users\"(\"email\");"
        );
    }

    #[test]
    fn test_policy_uses_explicit_as_and_public_role() {
        let mut policy = Policy::new("public", "users", "users_policy");
        policy.to = Some(vec![Cow::Borrowed("public")]);

        assert_eq!(
            policy.create_policy_sql(),
            "CREATE POLICY \"users_policy\" ON \"users\" AS PERMISSIVE TO PUBLIC;"
        );
    }
}
