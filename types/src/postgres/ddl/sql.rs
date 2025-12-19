//! SQL generation for PostgreSQL DDL types
//!
//! This module provides SQL generation methods for DDL types, enabling
//! unified SQL output from both compile-time and runtime schema definitions.

use crate::alloc_prelude::*;

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
    /// Create a new TableSql for SQL generation
    pub fn new(table: &'a Table) -> Self {
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
    pub fn columns(mut self, columns: &'a [Column]) -> Self {
        self.columns = columns;
        self
    }

    /// Set primary key
    pub fn primary_key(mut self, pk: Option<&'a PrimaryKey>) -> Self {
        self.primary_key = pk;
        self
    }

    /// Set foreign keys
    pub fn foreign_keys(mut self, fks: &'a [ForeignKey]) -> Self {
        self.foreign_keys = fks;
        self
    }

    /// Set unique constraints
    pub fn unique_constraints(mut self, uniques: &'a [UniqueConstraint]) -> Self {
        self.unique_constraints = uniques;
        self
    }

    /// Set check constraints
    pub fn check_constraints(mut self, checks: &'a [CheckConstraint]) -> Self {
        self.check_constraints = checks;
        self
    }

    /// Set indexes
    pub fn indexes(mut self, indexes: &'a [Index]) -> Self {
        self.indexes = indexes;
        self
    }

    /// Set policies
    pub fn policies(mut self, policies: &'a [Policy]) -> Self {
        self.policies = policies;
        self
    }

    fn schema_prefix(&self) -> String {
        if self.table.schema() != "public" {
            format!("\"{}\".", self.table.schema())
        } else {
            String::new()
        }
    }

    /// Generate CREATE TABLE SQL
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
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ");
            if pk.name_explicit {
                lines.push(format!(
                    "\tCONSTRAINT \"{}\" PRIMARY KEY({})",
                    pk.name(),
                    cols
                ));
            } else {
                lines.push(format!("\tPRIMARY KEY({})", cols));
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
                .map(|c| format!("\"{}\"", c))
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
    pub fn drop_table_sql(&self) -> String {
        let schema_prefix = self.schema_prefix();
        format!("DROP TABLE {}\"{}\";", schema_prefix, self.table.name())
    }

    /// Generate all related indexes
    pub fn create_indexes_sql(&self) -> Vec<String> {
        self.indexes.iter().map(|i| i.create_index_sql()).collect()
    }

    /// Generate RLS enable statement if needed
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
    pub fn create_policies_sql(&self) -> Vec<String> {
        self.policies
            .iter()
            .map(|p| p.create_policy_sql())
            .collect()
    }
}

// =============================================================================
// Column SQL Generation
// =============================================================================

impl Column {
    /// Generate the column definition SQL (without leading/trailing punctuation)
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
            sql.push_str(&format!(" DEFAULT {}", default));
        }

        // NOT NULL
        if self.not_null {
            sql.push_str(" NOT NULL");
        }

        sql
    }

    /// Generate ADD COLUMN SQL
    pub fn add_column_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD COLUMN {};",
            schema_prefix,
            self.table(),
            self.to_column_sql()
        )
    }

    /// Generate DROP COLUMN SQL
    pub fn drop_column_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
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
    pub fn to_sql(&self) -> String {
        let identity_type = match self.type_ {
            IdentityType::Always => "ALWAYS",
            IdentityType::ByDefault => "BY DEFAULT",
        };

        let mut sql = format!(" GENERATED {} AS IDENTITY", identity_type);

        // Add sequence options if any are specified
        let mut options = Vec::new();

        if let Some(increment) = self.increment.as_ref() {
            options.push(format!("INCREMENT BY {}", increment));
        }
        if let Some(min) = self.min_value.as_ref() {
            options.push(format!("MINVALUE {}", min));
        }
        if let Some(max) = self.max_value.as_ref() {
            options.push(format!("MAXVALUE {}", max));
        }
        if let Some(start) = self.start_with.as_ref() {
            options.push(format!("START WITH {}", start));
        }
        if let Some(cache) = self.cache {
            options.push(format!("CACHE {}", cache));
        }
        if self.cycle.unwrap_or(false) {
            options.push("CYCLE".to_string());
        }

        if !options.is_empty() {
            sql.push_str(&format!(" ({})", options.join(" ")));
        }

        sql
    }
}

// =============================================================================
// Generated Column SQL
// =============================================================================

impl Generated {
    /// Generate the GENERATED clause SQL
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
    pub fn to_constraint_sql(&self) -> String {
        let from_cols = self
            .columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        let to_cols = self
            .columns_to
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        let to_schema_prefix = if self.schema_to() != "public" {
            format!("\"{}\".", self.schema_to())
        } else {
            String::new()
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
            sql.push_str(&format!(" ON UPDATE {}", on_update.to_uppercase()));
        }

        if let Some(on_delete) = self.on_delete.as_ref()
            && on_delete != "NO ACTION"
        {
            sql.push_str(&format!(" ON DELETE {}", on_delete.to_uppercase()));
        }

        sql
    }

    /// Generate ADD FOREIGN KEY SQL
    pub fn add_fk_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD {};",
            schema_prefix,
            self.table(),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP FOREIGN KEY SQL
    pub fn drop_fk_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
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
    pub fn create_index_sql(&self) -> String {
        let unique = if self.is_unique { "UNIQUE " } else { "" };

        let concurrently = if self.concurrently {
            "CONCURRENTLY "
        } else {
            ""
        };

        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };

        let columns = self
            .columns
            .iter()
            .map(|c| c.to_sql())
            .collect::<Vec<_>>()
            .join(", ");

        let using = self
            .method
            .as_ref()
            .map(|m| format!(" USING {}", m))
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
            sql.push_str(&format!(" WHERE {};", where_clause));
        }

        sql
    }

    /// Generate DROP INDEX SQL
    pub fn drop_index_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!("DROP INDEX {}\"{}\";", schema_prefix, self.name())
    }
}

impl IndexColumnDef {
    /// Generate the column reference for an index
    pub fn to_sql(&self) -> String {
        let mut sql = if self.is_expression {
            format!("({})", self.value)
        } else {
            format!("\"{}\"", self.value)
        };

        if let Some(op) = self.opclass.as_ref() {
            sql.push_str(&format!(" {}", op));
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
    pub fn create_enum_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        let values = self
            .values
            .iter()
            .map(|v| format!("'{}'", v))
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
    pub fn drop_enum_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!("DROP TYPE {}\"{}\";", schema_prefix, self.name())
    }

    /// Generate ALTER TYPE ... ADD VALUE SQL
    pub fn add_value_sql(&self, value: &str, before: Option<&str>) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        if let Some(before_value) = before {
            format!(
                "ALTER TYPE {}\"{}\" ADD VALUE '{}' BEFORE '{}';",
                schema_prefix,
                self.name(),
                value,
                before_value
            )
        } else {
            format!(
                "ALTER TYPE {}\"{}\" ADD VALUE '{}';",
                schema_prefix,
                self.name(),
                value
            )
        }
    }
}

// =============================================================================
// Sequence SQL Generation
// =============================================================================

impl Sequence {
    /// Generate CREATE SEQUENCE SQL
    pub fn create_sequence_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };

        let mut sql = format!("CREATE SEQUENCE {}\"{}\"", schema_prefix, self.name());

        if let Some(inc) = self.increment_by.as_ref() {
            sql.push_str(&format!(" INCREMENT BY {}", inc));
        }
        if let Some(min) = self.min_value.as_ref() {
            sql.push_str(&format!(" MINVALUE {}", min));
        }
        if let Some(max) = self.max_value.as_ref() {
            sql.push_str(&format!(" MAXVALUE {}", max));
        }
        if let Some(start) = self.start_with.as_ref() {
            sql.push_str(&format!(" START WITH {}", start));
        }
        if let Some(cache) = self.cache_size {
            sql.push_str(&format!(" CACHE {}", cache));
        }
        if self.cycle.unwrap_or(false) {
            sql.push_str(" CYCLE");
        }

        sql.push(';');
        sql
    }

    /// Generate DROP SEQUENCE SQL
    pub fn drop_sequence_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!("DROP SEQUENCE {}\"{}\";", schema_prefix, self.name())
    }
}

// =============================================================================
// View SQL Generation
// =============================================================================

impl View {
    /// Generate CREATE VIEW SQL
    pub fn create_view_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };

        let materialized = if self.materialized {
            "MATERIALIZED "
        } else {
            ""
        };

        if let Some(def) = self.definition.as_ref() {
            format!(
                "CREATE {}VIEW {}\"{}\" AS {};",
                materialized,
                schema_prefix,
                self.name(),
                def
            )
        } else {
            format!(
                "-- {}View {}\"{}\" has no definition",
                materialized,
                schema_prefix,
                self.name()
            )
        }
    }

    /// Generate DROP VIEW SQL
    pub fn drop_view_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
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
    pub fn create_policy_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };

        let mut sql = format!(
            "CREATE POLICY \"{}\" ON {}\"{}\"",
            self.name(),
            schema_prefix,
            self.table()
        );

        if let Some(r#for) = self.for_clause.as_ref() {
            sql.push_str(&format!(" FOR {}", r#for.to_uppercase()));
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
                        format!("\"{}\"", r)
                    }
                })
                .collect::<Vec<_>>()
                .join(", ");
            sql.push_str(&format!(" TO {}", to_roles));
        }

        if let Some(using) = self.using.as_ref() {
            sql.push_str(&format!(" USING ({})", using));
        }

        if let Some(with_check) = self.with_check.as_ref() {
            sql.push_str(&format!(" WITH CHECK ({})", with_check));
        }

        sql.push(';');
        sql
    }

    /// Generate DROP POLICY SQL
    pub fn drop_policy_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
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
    pub fn drop_table_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!("DROP TABLE {}\"{}\";", schema_prefix, self.name())
    }

    /// Generate RENAME TABLE SQL
    pub fn rename_table_sql(&self, new_name: &str) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
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
    pub fn to_constraint_sql(&self) -> String {
        let cols = self
            .columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        format!("CONSTRAINT \"{}\" PRIMARY KEY({})", self.name(), cols)
    }

    /// Generate ADD PRIMARY KEY SQL
    pub fn add_pk_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD {};",
            schema_prefix,
            self.table(),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP PRIMARY KEY SQL
    pub fn drop_pk_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
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
    pub fn to_constraint_sql(&self) -> String {
        let cols = self
            .columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        format!("CONSTRAINT \"{}\" UNIQUE({})", self.name(), cols)
    }

    /// Generate ADD UNIQUE SQL
    pub fn add_unique_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD {};",
            schema_prefix,
            self.table(),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP UNIQUE SQL
    pub fn drop_unique_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
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
    pub fn to_constraint_sql(&self) -> String {
        format!("CONSTRAINT \"{}\" CHECK ({})", self.name(), &self.value)
    }

    /// Generate ADD CHECK SQL
    pub fn add_check_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
        };
        format!(
            "ALTER TABLE {}\"{}\" ADD {};",
            schema_prefix,
            self.table(),
            self.to_constraint_sql()
        )
    }

    /// Generate DROP CHECK SQL
    pub fn drop_check_sql(&self) -> String {
        let schema_prefix = if self.schema() != "public" {
            format!("\"{}\".", self.schema())
        } else {
            String::new()
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
