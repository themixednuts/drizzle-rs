//! SQL generation for SQLite DDL types
//!
//! This module provides SQL generation methods for DDL types, enabling
//! unified SQL output from both compile-time and runtime schema definitions.

use crate::alloc_prelude::*;

use super::{
    CheckConstraint, Column, ForeignKey, Generated, GeneratedType, Index, IndexColumnDef,
    PrimaryKey, Table, UniqueConstraint, View,
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

    /// Generate CREATE TABLE SQL
    pub fn create_table_sql(&self) -> String {
        let mut sql = format!("CREATE TABLE `{}` (\n", self.table.name());

        let mut lines = Vec::new();

        // Column definitions
        for column in self.columns {
            let is_inline_pk = self.primary_key.as_ref().is_some_and(|pk| {
                pk.columns.len() == 1
                    && pk.columns.iter().any(|c| *c == column.name())
                    && !pk.name_explicit
            });

            let is_inline_unique = self.unique_constraints.iter().any(|u| {
                u.columns.len() == 1
                    && u.columns.iter().any(|c| *c == column.name())
                    && !u.name_explicit
            });

            lines.push(format!(
                "\t{}",
                column.to_column_sql(is_inline_pk, is_inline_unique)
            ));
        }

        // Composite or named primary key
        if let Some(pk) = &self.primary_key
            && (pk.columns.len() > 1 || pk.name_explicit)
        {
            let cols = pk
                .columns
                .iter()
                .map(|c| format!("`{}`", c))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "\tCONSTRAINT `{}` PRIMARY KEY({})",
                pk.name(),
                cols
            ));
        }

        // Foreign keys
        for fk in self.foreign_keys {
            lines.push(format!("\t{}", fk.to_constraint_sql()));
        }

        // Multi-column unique constraints
        for unique in self
            .unique_constraints
            .iter()
            .filter(|u| u.columns.len() > 1 || u.name_explicit)
        {
            let cols = unique
                .columns
                .iter()
                .map(|c| format!("`{}`", c))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("\tCONSTRAINT `{}` UNIQUE({})", unique.name(), cols));
        }

        // Check constraints
        for check in self.check_constraints {
            lines.push(format!(
                "\tCONSTRAINT `{}` CHECK({})",
                check.name(),
                &check.value
            ));
        }

        sql.push_str(&lines.join(",\n"));
        sql.push_str("\n)");

        // Table options
        if self.table.without_rowid {
            sql.push_str(" WITHOUT ROWID");
        }
        if self.table.strict {
            sql.push_str(" STRICT");
        }

        sql.push(';');
        sql
    }

    /// Generate DROP TABLE SQL
    pub fn drop_table_sql(&self) -> String {
        format!("DROP TABLE `{}`;", self.table.name())
    }
}

// =============================================================================
// Column SQL Generation
// =============================================================================

impl Column {
    /// Generate the column definition SQL (without leading/trailing punctuation)
    pub fn to_column_sql(&self, inline_pk: bool, inline_unique: bool) -> String {
        let mut sql = format!("`{}` {}", self.name(), self.sql_type().to_uppercase());

        if inline_pk {
            sql.push_str(" PRIMARY KEY");
            if self.autoincrement.unwrap_or(false) {
                sql.push_str(" AUTOINCREMENT");
            }
        }

        if let Some(default) = self.default.as_ref() {
            sql.push_str(&format!(" DEFAULT {}", default));
        }

        if let Some(generated) = &self.generated {
            sql.push_str(&generated.to_sql());
        }

        // NOT NULL - skip for INTEGER PRIMARY KEY (allows NULL by default in SQLite)
        if self.not_null && !(inline_pk && self.sql_type().to_lowercase().starts_with("int")) {
            sql.push_str(" NOT NULL");
        }

        if inline_unique && !inline_pk {
            sql.push_str(" UNIQUE");
        }

        sql
    }

    /// Generate ADD COLUMN SQL
    pub fn add_column_sql(&self) -> String {
        format!(
            "ALTER TABLE `{}` ADD COLUMN {};",
            self.table(),
            self.to_column_sql(false, false)
        )
    }

    /// Generate DROP COLUMN SQL
    pub fn drop_column_sql(&self) -> String {
        format!(
            "ALTER TABLE `{}` DROP COLUMN `{}`;",
            self.table(),
            self.name()
        )
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
            GeneratedType::Virtual => "VIRTUAL",
        };
        format!(" GENERATED ALWAYS AS {} {}", self.expression, gen_type)
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
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");

        let to_cols = self
            .columns_to
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!(
            "CONSTRAINT `{}` FOREIGN KEY ({}) REFERENCES `{}`({})",
            self.name(),
            from_cols,
            &self.table_to,
            to_cols
        );

        if let Some(on_update) = self.on_update.as_ref()
            && on_update != "NO ACTION"
        {
            sql.push_str(&format!(" ON UPDATE {}", on_update));
        }

        if let Some(on_delete) = self.on_delete.as_ref()
            && on_delete != "NO ACTION"
        {
            sql.push_str(&format!(" ON DELETE {}", on_delete));
        }

        sql
    }

    /// Generate ADD FOREIGN KEY SQL (via new table constraint)
    pub fn add_fk_sql(&self) -> String {
        // SQLite doesn't support ADD CONSTRAINT for foreign keys directly
        // This would require table recreation
        format!(
            "-- SQLite requires table recreation to add foreign keys\n-- FK: {} on `{}`",
            self.name(),
            self.table()
        )
    }

    /// Generate DROP FOREIGN KEY SQL (comment since SQLite doesn't support it)
    pub fn drop_fk_sql(&self) -> String {
        format!(
            "-- SQLite requires table recreation to drop foreign keys\n-- FK: {} on `{}`",
            self.name(),
            self.table()
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

        let columns = self
            .columns
            .iter()
            .map(|c| c.to_sql())
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!(
            "CREATE {}INDEX `{}` ON `{}`({});",
            unique,
            self.name(),
            self.table(),
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
        format!("DROP INDEX `{}`;", self.name())
    }
}

impl IndexColumnDef {
    /// Generate the column reference for an index
    pub fn to_sql(&self) -> String {
        if self.is_expression {
            self.value.to_string()
        } else {
            format!("`{}`", self.value)
        }
    }
}

// =============================================================================
// View SQL Generation
// =============================================================================

impl View {
    /// Generate CREATE VIEW SQL
    pub fn create_view_sql(&self) -> String {
        if let Some(def) = self.definition.as_ref() {
            format!("CREATE VIEW `{}` AS {};", self.name(), def)
        } else {
            format!("-- View `{}` has no definition", self.name())
        }
    }

    /// Generate DROP VIEW SQL
    pub fn drop_view_sql(&self) -> String {
        format!("DROP VIEW `{}`;", self.name())
    }
}

// =============================================================================
// Table-level utilities
// =============================================================================

impl Table {
    /// Generate DROP TABLE SQL
    pub fn drop_table_sql(&self) -> String {
        format!("DROP TABLE `{}`;", self.name())
    }

    /// Generate RENAME TABLE SQL
    pub fn rename_table_sql(&self, new_name: &str) -> String {
        format!("ALTER TABLE `{}` RENAME TO `{}`;", self.name(), new_name)
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
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");

        format!("CONSTRAINT `{}` PRIMARY KEY({})", self.name(), cols)
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
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");

        format!("CONSTRAINT `{}` UNIQUE({})", self.name(), cols)
    }
}

// =============================================================================
// Check Constraint SQL Generation
// =============================================================================

impl CheckConstraint {
    /// Generate the CHECK constraint clause
    pub fn to_constraint_sql(&self) -> String {
        format!("CONSTRAINT `{}` CHECK({})", self.name(), &self.value)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::ddl::{
        ColumnDef, ForeignKeyDef, IndexColumnDef, IndexDef, PrimaryKeyDef, ReferentialAction,
        TableDef,
    };
    use std::borrow::Cow;

    #[test]
    fn test_simple_create_table() {
        let table = TableDef::new("users").into_table();
        let columns = [
            ColumnDef::new("users", "id", "INTEGER")
                .primary_key()
                .autoincrement()
                .into_column(),
            ColumnDef::new("users", "name", "TEXT")
                .not_null()
                .into_column(),
            ColumnDef::new("users", "email", "TEXT").into_column(),
        ];
        const PK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
        let pk = PrimaryKeyDef::new("users", "users_pk")
            .columns(PK_COLS)
            .into_primary_key();

        let sql = TableSql::new(&table)
            .columns(&columns)
            .primary_key(Some(&pk))
            .create_table_sql();

        assert!(sql.contains("CREATE TABLE `users`"));
        assert!(sql.contains("`id` INTEGER PRIMARY KEY AUTOINCREMENT"));
        assert!(sql.contains("`name` TEXT NOT NULL"));
        assert!(sql.contains("`email` TEXT"));
    }

    #[test]
    fn test_table_with_foreign_key() {
        let table = TableDef::new("posts").into_table();
        let columns = [
            ColumnDef::new("posts", "id", "INTEGER")
                .primary_key()
                .into_column(),
            ColumnDef::new("posts", "user_id", "INTEGER")
                .not_null()
                .into_column(),
        ];
        const PK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
        let pk = PrimaryKeyDef::new("posts", "posts_pk")
            .columns(PK_COLS)
            .into_primary_key();
        const FK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("user_id")];
        const FK_REFS: &[Cow<'static, str>] = &[Cow::Borrowed("id")];
        let fks = [ForeignKeyDef::new("posts", "posts_user_id_fk")
            .columns(FK_COLS)
            .references("users", FK_REFS)
            .on_delete(ReferentialAction::Cascade)
            .into_foreign_key()];

        let sql = TableSql::new(&table)
            .columns(&columns)
            .primary_key(Some(&pk))
            .foreign_keys(&fks)
            .create_table_sql();

        assert!(sql.contains("FOREIGN KEY (`user_id`) REFERENCES `users`(`id`)"));
        assert!(sql.contains("ON DELETE CASCADE"));
    }

    #[test]
    fn test_create_index() {
        const COLS: &[IndexColumnDef] = &[IndexColumnDef::new("email")];
        let index = IndexDef::new("users", "users_email_idx")
            .columns(COLS)
            .unique()
            .into_index();

        let sql = index.create_index_sql();
        assert_eq!(
            sql,
            "CREATE UNIQUE INDEX `users_email_idx` ON `users`(`email`);"
        );
    }

    #[test]
    fn test_strict_without_rowid() {
        let table = TableDef::new("data").strict().without_rowid().into_table();
        let columns = [ColumnDef::new("data", "key", "TEXT")
            .primary_key()
            .not_null()
            .into_column()];
        const PK_COLS: &[Cow<'static, str>] = &[Cow::Borrowed("key")];
        let pk = PrimaryKeyDef::new("data", "data_pk")
            .columns(PK_COLS)
            .into_primary_key();

        let sql = TableSql::new(&table)
            .columns(&columns)
            .primary_key(Some(&pk))
            .create_table_sql();

        assert!(sql.ends_with("WITHOUT ROWID STRICT;"));
    }
}
