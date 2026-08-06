//! SQL generation for `SQLite` DDL types
//!
//! This module provides SQL generation methods for DDL types, enabling
//! unified SQL output from both compile-time and runtime schema definitions.

use crate::alloc_prelude::*;
use core::fmt::Write;

use super::{
    CheckConstraint, Column, ForeignKey, Generated, GeneratedType, Index, IndexColumnDef,
    PrimaryKey, Table, UniqueConstraint, View,
};

fn quote_ident(ident: &str) -> String {
    format!("`{}`", ident.replace('`', "``"))
}

/// Returns `true` when `expr` is fully wrapped in a single pair of balanced
/// parentheses, e.g. `(a + b)` but not `(a) + (b)`.
///
/// This is a tolerant scanner (it does not understand string literals), which
/// matches the tolerance level of the rest of this module.
fn is_wrapped_in_parens(expr: &str) -> bool {
    let bytes = expr.as_bytes();
    if bytes.len() < 2 || bytes[0] != b'(' || bytes[bytes.len() - 1] != b')' {
        return false;
    }
    let mut depth = 0i32;
    for (i, ch) in expr.char_indices() {
        match ch {
            '(' => depth += 1,
            ')' => {
                depth -= 1;
                if depth == 0 {
                    return i == expr.len() - 1;
                }
            }
            _ => {}
        }
    }
    false
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

    /// Generate CREATE TABLE SQL
    #[must_use]
    pub fn create_table_sql(&self) -> String {
        let mut sql = format!("CREATE TABLE {} (\n", quote_ident(self.table.name()));

        let mut lines = Vec::new();

        // Columns whose `primary_key` flag is set but which no PrimaryKey
        // entity covers still need a PRIMARY KEY clause (e.g. columns built via
        // `ColumnDef::new(..).primary_key()` without a PrimaryKey entity).
        let flag_pk_columns: Vec<&str> = self
            .columns
            .iter()
            .filter(|c| {
                c.is_primary_key()
                    && !self
                        .primary_key
                        .as_ref()
                        .is_some_and(|pk| pk.columns.iter().any(|pc| *pc == c.name()))
            })
            .map(Column::name)
            .collect();

        // Column definitions
        for column in self.columns {
            let is_entity_inline_pk = self.primary_key.as_ref().is_some_and(|pk| {
                pk.columns.len() == 1
                    && pk.columns.iter().any(|c| *c == column.name())
                    && !pk.name_explicit
            });
            let is_flag_inline_pk =
                flag_pk_columns.len() == 1 && flag_pk_columns[0] == column.name();
            let is_inline_pk = is_entity_inline_pk || is_flag_inline_pk;

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
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "\tCONSTRAINT {} PRIMARY KEY({})",
                quote_ident(pk.name()),
                cols
            ));
        }

        // Composite primary key declared only through column flags (no entity):
        // rendering each column with an inline PRIMARY KEY would be invalid SQL,
        // so emit a single table-level PRIMARY KEY clause instead.
        if self.primary_key.is_none() && flag_pk_columns.len() > 1 {
            let cols = flag_pk_columns
                .iter()
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("\tPRIMARY KEY({cols})"));
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
                .map(|c| quote_ident(c))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!(
                "\tCONSTRAINT {} UNIQUE({})",
                quote_ident(unique.name()),
                cols
            ));
        }

        // Check constraints
        for check in self.check_constraints {
            lines.push(format!(
                "\tCONSTRAINT {} CHECK({})",
                quote_ident(check.name()),
                check.value
            ));
        }

        sql.push_str(&lines.join(",\n"));
        sql.push_str("\n)");

        // Table options
        let mut options = Vec::new();
        if self.table.without_rowid {
            options.push("WITHOUT ROWID");
        }
        if self.table.strict {
            options.push("STRICT");
        }
        if !options.is_empty() {
            let _ = write!(sql, " {}", options.join(", "));
        }

        sql.push(';');
        sql
    }

    /// Generate DROP TABLE SQL
    #[must_use]
    pub fn drop_table_sql(&self) -> String {
        format!("DROP TABLE {};", quote_ident(self.table.name()))
    }
}

// =============================================================================
// Column SQL Generation
// =============================================================================

impl Column {
    /// Generate the column definition SQL (without leading/trailing punctuation)
    #[must_use]
    pub fn to_column_sql(&self, inline_pk: bool, inline_unique: bool) -> String {
        let mut sql = format!(
            "{} {}",
            quote_ident(self.name()),
            self.sql_type().to_uppercase()
        );

        if inline_pk {
            sql.push_str(" PRIMARY KEY");
            // AUTOINCREMENT is only valid immediately after an inline
            // `PRIMARY KEY`; emitting it anywhere else is a syntax error, so it
            // is intentionally dropped when this column is not the inline PK.
            if self.autoincrement.unwrap_or(false) {
                sql.push_str(" AUTOINCREMENT");
            }
        }

        if let Some(default) = self.default.as_ref() {
            let _ = write!(sql, " DEFAULT {default}");
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

        // COLLATE applies to comparisons on this column. SQLite parses it as a
        // column-constraint, so it follows other inline constraints.
        if let Some(collate) = self.collate.as_ref() {
            let _ = write!(sql, " COLLATE {collate}");
        }

        sql
    }

    /// Generate ADD COLUMN SQL
    #[must_use]
    pub fn add_column_sql(&self) -> String {
        format!(
            "ALTER TABLE {} ADD COLUMN {};",
            quote_ident(self.table()),
            self.to_column_sql(false, false)
        )
    }

    /// Generate DROP COLUMN SQL
    #[must_use]
    pub fn drop_column_sql(&self) -> String {
        format!(
            "ALTER TABLE {} DROP COLUMN {};",
            quote_ident(self.table()),
            quote_ident(self.name())
        )
    }
}

// =============================================================================
// Generated Column SQL
// =============================================================================

impl Generated {
    /// Generate the GENERATED clause SQL
    ///
    /// `SQLite` requires the generation expression to be parenthesized
    /// (`GENERATED ALWAYS AS (expr)`), so the expression is wrapped in parens
    /// unless it is already fully parenthesized (the table macros store
    /// pre-parenthesized expressions; introspection stores bare expressions).
    #[must_use]
    pub fn to_sql(&self) -> String {
        let gen_type = match self.gen_type {
            GeneratedType::Stored => "STORED",
            GeneratedType::Virtual => "VIRTUAL",
        };
        let expression = self.expression.trim();
        if is_wrapped_in_parens(expression) {
            format!(" GENERATED ALWAYS AS {expression} {gen_type}")
        } else {
            format!(" GENERATED ALWAYS AS ({expression}) {gen_type}")
        }
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
            quote_ident(&self.table_to),
            to_cols
        );

        if let Some(on_update) = self.on_update.as_ref()
            && on_update != "NO ACTION"
        {
            let _ = write!(sql, " ON UPDATE {on_update}");
        }

        if let Some(on_delete) = self.on_delete.as_ref()
            && on_delete != "NO ACTION"
        {
            let _ = write!(sql, " ON DELETE {on_delete}");
        }

        sql
    }

    /// Generate ADD FOREIGN KEY SQL (via new table constraint)
    #[must_use]
    pub fn add_fk_sql(&self) -> String {
        // SQLite doesn't support ADD CONSTRAINT for foreign keys directly
        // This would require table recreation
        format!(
            "-- SQLite requires table recreation to add foreign keys\n-- FK: {} on {}",
            self.name(),
            quote_ident(self.table())
        )
    }

    /// Generate DROP FOREIGN KEY SQL (comment since `SQLite` doesn't support it)
    #[must_use]
    pub fn drop_fk_sql(&self) -> String {
        format!(
            "-- SQLite requires table recreation to drop foreign keys\n-- FK: {} on {}",
            self.name(),
            quote_ident(self.table())
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

        let columns = self
            .columns
            .iter()
            .map(super::index::IndexColumn::to_sql)
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!(
            "CREATE {}INDEX {} ON {}({});",
            unique,
            quote_ident(self.name()),
            quote_ident(self.table()),
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
        format!("DROP INDEX {};", quote_ident(self.name()))
    }
}

impl IndexColumnDef {
    /// Generate the column reference for an index
    #[must_use]
    pub fn to_sql(&self) -> String {
        if self.is_expression {
            self.value.to_string()
        } else {
            quote_ident(self.value)
        }
    }
}

// =============================================================================
// View SQL Generation
// =============================================================================

impl View {
    /// Generate CREATE VIEW SQL
    #[must_use]
    pub fn create_view_sql(&self) -> String {
        self.definition.as_ref().map_or_else(
            || format!("-- View {} has no definition", quote_ident(self.name())),
            |def| format!("CREATE VIEW {} AS {};", quote_ident(self.name()), def),
        )
    }

    /// Generate DROP VIEW SQL
    #[must_use]
    pub fn drop_view_sql(&self) -> String {
        format!("DROP VIEW {};", quote_ident(self.name()))
    }
}

// =============================================================================
// Table-level utilities
// =============================================================================

impl Table {
    /// Generate DROP TABLE SQL
    #[must_use]
    pub fn drop_table_sql(&self) -> String {
        format!("DROP TABLE {};", quote_ident(self.name()))
    }

    /// Generate RENAME TABLE SQL
    #[must_use]
    pub fn rename_table_sql(&self, new_name: &str) -> String {
        format!(
            "ALTER TABLE {} RENAME TO {};",
            quote_ident(self.name()),
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
            .map(|c| quote_ident(c))
            .collect::<Vec<_>>()
            .join(", ");

        format!("CONSTRAINT {} UNIQUE({})", quote_ident(self.name()), cols)
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
            "CONSTRAINT {} CHECK({})",
            quote_ident(self.name()),
            self.value
        )
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
    fn test_column_flag_primary_key_renders_inline_without_entity() {
        // A column built via ColumnDef::primary_key().autoincrement() must
        // render an inline PRIMARY KEY AUTOINCREMENT even when no PrimaryKey
        // entity exists.
        let table = TableDef::new("users").into_table();
        let columns = [
            ColumnDef::new("users", "id", "INTEGER")
                .primary_key()
                .autoincrement()
                .into_column(),
            ColumnDef::new("users", "name", "TEXT")
                .not_null()
                .into_column(),
        ];

        let sql = TableSql::new(&table).columns(&columns).create_table_sql();

        assert!(
            sql.contains("`id` INTEGER PRIMARY KEY AUTOINCREMENT"),
            "expected inline PRIMARY KEY AUTOINCREMENT, got: {sql}"
        );
        assert!(
            !sql.contains("INTEGER AUTOINCREMENT"),
            "orphan AUTOINCREMENT without PRIMARY KEY: {sql}"
        );
    }

    #[test]
    fn test_column_flag_composite_primary_key_renders_table_constraint() {
        let table = TableDef::new("pair").into_table();
        let columns = [
            ColumnDef::new("pair", "a", "INTEGER")
                .primary_key()
                .into_column(),
            ColumnDef::new("pair", "b", "INTEGER")
                .primary_key()
                .into_column(),
        ];

        let sql = TableSql::new(&table).columns(&columns).create_table_sql();

        assert!(
            sql.contains("PRIMARY KEY(`a`, `b`)"),
            "expected composite PRIMARY KEY clause, got: {sql}"
        );
        assert_eq!(
            sql.matches("PRIMARY KEY").count(),
            1,
            "composite flag PK must render exactly one PRIMARY KEY clause: {sql}"
        );
    }

    #[test]
    fn test_generated_expression_is_parenthesized() {
        use crate::sqlite::ddl::{Generated, GeneratedType};

        let bare = Generated {
            expression: Cow::Borrowed("length(name)"),
            gen_type: GeneratedType::Stored,
        };
        assert_eq!(bare.to_sql(), " GENERATED ALWAYS AS (length(name)) STORED");

        // Already-parenthesized expressions (macro canonical form) must not be
        // double-wrapped.
        let wrapped = Generated {
            expression: Cow::Borrowed("(length(name))"),
            gen_type: GeneratedType::Virtual,
        };
        assert_eq!(
            wrapped.to_sql(),
            " GENERATED ALWAYS AS (length(name)) VIRTUAL"
        );

        // `(a) + (b)` starts and ends with parens but is NOT fully wrapped.
        let tricky = Generated {
            expression: Cow::Borrowed("(a) + (b)"),
            gen_type: GeneratedType::Virtual,
        };
        assert_eq!(tricky.to_sql(), " GENERATED ALWAYS AS ((a) + (b)) VIRTUAL");
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

        assert!(sql.ends_with("WITHOUT ROWID, STRICT;"));
    }
}
