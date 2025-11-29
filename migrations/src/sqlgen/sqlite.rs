//! SQLite SQL generation from schema metadata

use crate::sqlite::{AlteredTable, Column, ForeignKey, Index, SchemaDiff, Table};

/// SQL statement breakpoint marker (used by drizzle-kit)
pub const BREAKPOINT: &str = "--> statement-breakpoint";

/// SQLite SQL generator
pub struct SqliteGenerator {
    /// Whether to include statement breakpoints
    pub breakpoints: bool,
}

impl Default for SqliteGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl SqliteGenerator {
    pub fn new() -> Self {
        Self { breakpoints: true }
    }

    pub fn with_breakpoints(mut self, breakpoints: bool) -> Self {
        self.breakpoints = breakpoints;
        self
    }

    /// Generate SQL from a schema diff
    pub fn generate_migration(&self, diff: &SchemaDiff) -> Vec<String> {
        let mut statements = Vec::new();

        // Drop deleted tables first
        for table_name in &diff.tables.deleted {
            statements.push(format!("DROP TABLE `{}`;", table_name));
        }

        // Create new tables
        for table in &diff.tables.created {
            statements.push(self.generate_create_table(table));
        }

        // Alter existing tables
        for altered in &diff.tables.altered {
            statements.extend(self.generate_alter_table(altered));
        }

        statements
    }

    /// Generate SQL from migration statements
    pub fn statements_to_sql(&self, statements: &[String]) -> String {
        if self.breakpoints {
            statements.join(&format!("\n{}\n", BREAKPOINT))
        } else {
            statements.join("\n")
        }
    }

    /// Generate CREATE TABLE SQL
    pub fn generate_create_table(&self, table: &Table) -> String {
        let mut sql = format!("CREATE TABLE `{}` (\n", table.name);

        let mut parts = Vec::new();

        // Column definitions
        for column in table.columns.values() {
            parts.push(format!("\t{}", self.column_to_sql(column)));
        }

        // Composite primary keys
        for pk in table.composite_primary_keys.values() {
            let cols = pk
                .columns
                .iter()
                .map(|c| format!("`{}`", c))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("\tPRIMARY KEY ({})", cols));
        }

        // Foreign keys
        for fk in table.foreign_keys.values() {
            parts.push(format!("\t{}", self.foreign_key_to_sql(fk)));
        }

        sql.push_str(&parts.join(",\n"));
        sql.push_str("\n);");

        sql
    }

    /// Convert a column to SQL definition
    pub fn column_to_sql(&self, column: &Column) -> String {
        let mut parts = vec![format!("`{}`", column.name), column.sql_type.to_uppercase()];

        if column.primary_key {
            parts.push("PRIMARY KEY".to_string());
        }

        if column.autoincrement.unwrap_or(false) {
            parts.push("AUTOINCREMENT".to_string());
        }

        if column.not_null && !column.primary_key {
            parts.push("NOT NULL".to_string());
        }

        if let Some(ref default) = column.default {
            parts.push(format!("DEFAULT {}", default_to_sql(default)));
        }

        if let Some(ref generated) = column.generated {
            let gen_type = match generated.gen_type {
                crate::sqlite::GeneratedType::Stored => "STORED",
                crate::sqlite::GeneratedType::Virtual => "VIRTUAL",
            };
            parts.push(format!(
                "GENERATED ALWAYS AS ({}) {}",
                generated.expression, gen_type
            ));
        }

        parts.join(" ")
    }

    /// Convert a foreign key to SQL
    pub fn foreign_key_to_sql(&self, fk: &ForeignKey) -> String {
        let cols_from = fk
            .columns_from
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");
        let cols_to = fk
            .columns_to
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!(
            "FOREIGN KEY ({}) REFERENCES `{}`({})",
            cols_from, fk.table_to, cols_to
        );

        if let Some(ref on_update) = fk.on_update {
            sql.push_str(&format!(" ON UPDATE {}", on_update));
        }
        if let Some(ref on_delete) = fk.on_delete {
            sql.push_str(&format!(" ON DELETE {}", on_delete));
        }

        sql
    }

    /// Generate CREATE INDEX SQL
    pub fn generate_create_index(&self, table_name: &str, index: &Index) -> String {
        let unique = if index.is_unique { "UNIQUE " } else { "" };
        let cols = index
            .columns
            .iter()
            .map(|c| format!("`{}`", c))
            .collect::<Vec<_>>()
            .join(", ");

        let mut sql = format!(
            "CREATE {}INDEX `{}` ON `{}`({});",
            unique, index.name, table_name, cols
        );

        if let Some(ref where_clause) = index.r#where {
            sql = format!(
                "CREATE {}INDEX `{}` ON `{}`({}) WHERE {};",
                unique, index.name, table_name, cols, where_clause
            );
        }

        sql
    }

    /// Generate DROP INDEX SQL
    pub fn generate_drop_index(&self, index_name: &str) -> String {
        format!("DROP INDEX `{}`;", index_name)
    }

    /// Generate ALTER TABLE statements for an altered table
    pub fn generate_alter_table(&self, altered: &AlteredTable) -> Vec<String> {
        let mut statements = Vec::new();

        // Check if we need to recreate the table (SQLite doesn't support many ALTER operations)
        let needs_recreate = self.needs_table_recreation(altered);

        if needs_recreate {
            statements.extend(self.generate_table_recreation(altered));
        } else {
            // Simple column additions can use ALTER TABLE ADD COLUMN
            for column in &altered.columns.added {
                statements.push(format!(
                    "ALTER TABLE `{}` ADD COLUMN {};",
                    altered.name,
                    self.column_to_sql(column)
                ));
            }
        }

        // Handle index changes (these can be done without recreation)
        for index_name in &altered.indexes.deleted {
            statements.push(self.generate_drop_index(index_name));
        }

        for index in &altered.indexes.added {
            statements.push(self.generate_create_index(&altered.name, index));
        }

        // Altered indexes need drop + create
        for (old_idx, new_idx) in &altered.indexes.altered {
            statements.push(self.generate_drop_index(&old_idx.name));
            statements.push(self.generate_create_index(&altered.name, new_idx));
        }

        statements
    }

    /// Check if table recreation is needed for the alterations
    fn needs_table_recreation(&self, altered: &AlteredTable) -> bool {
        // Column deletions require recreation
        if !altered.columns.deleted.is_empty() {
            return true;
        }

        // Column alterations (type changes, nullability, etc.) require recreation
        if !altered.columns.altered.is_empty() {
            return true;
        }

        // Foreign key changes require recreation
        if altered.foreign_keys.has_changes() {
            return true;
        }

        false
    }

    /// Generate SQL to recreate a table with modifications
    fn generate_table_recreation(&self, altered: &AlteredTable) -> Vec<String> {
        let mut statements = Vec::new();

        // SQLite table recreation pattern:
        // 1. PRAGMA foreign_keys=OFF
        // 2. CREATE TABLE __new_table (...)
        // 3. INSERT INTO __new_table SELECT ... FROM old_table
        // 4. DROP TABLE old_table
        // 5. ALTER TABLE __new_table RENAME TO old_table
        // 6. PRAGMA foreign_keys=ON

        statements.push("PRAGMA foreign_keys=OFF;".to_string());

        // We need to build the new table definition
        // This is a simplified version - in production you'd need the full table info
        let new_table_name = format!("__new_{}", altered.name);

        // For now, we'll generate a placeholder comment
        // In a real implementation, you'd need the full table definition
        statements.push(format!(
            "-- Table recreation for `{}` requires full table definition",
            altered.name
        ));
        statements.push(format!(
            "-- CREATE TABLE `{}` (...new schema...);",
            new_table_name
        ));
        statements.push(format!(
            "-- INSERT INTO `{}` SELECT ... FROM `{}`;",
            new_table_name, altered.name
        ));
        statements.push(format!("-- DROP TABLE `{}`;", altered.name));
        statements.push(format!(
            "-- ALTER TABLE `{}` RENAME TO `{}`;",
            new_table_name, altered.name
        ));

        statements.push("PRAGMA foreign_keys=ON;".to_string());

        statements
    }
}

/// Generate full table recreation SQL (when you have the full table info)
pub fn generate_table_recreation_sql(
    table_name: &str,
    new_table: &Table,
    columns_to_copy: &[String],
) -> Vec<String> {
    let generator = SqliteGenerator::new();
    let mut statements = Vec::new();
    let new_table_name = format!("__new_{}", table_name);

    statements.push("PRAGMA foreign_keys=OFF;".to_string());

    // Create new table with temporary name
    let mut temp_table = new_table.clone();
    temp_table.name = new_table_name.clone();
    statements.push(generator.generate_create_table(&temp_table));

    // Copy data
    let cols = columns_to_copy
        .iter()
        .map(|c| format!("`{}`", c))
        .collect::<Vec<_>>()
        .join(", ");
    statements.push(format!(
        "INSERT INTO `{}`({}) SELECT {} FROM `{}`;",
        new_table_name, cols, cols, table_name
    ));

    // Drop old table
    statements.push(format!("DROP TABLE `{}`;", table_name));

    // Rename new table
    statements.push(format!(
        "ALTER TABLE `{}` RENAME TO `{}`;",
        new_table_name, table_name
    ));

    statements.push("PRAGMA foreign_keys=ON;".to_string());

    statements
}

/// Convert a JSON default value to SQL literal
fn default_to_sql(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            // Check if it's an expression (starts with `)
            if s.starts_with('`') && s.ends_with('`') {
                // Strip the backticks - it's a SQL expression
                s[1..s.len() - 1].to_string()
            } else {
                format!("'{}'", s.replace('\'', "''"))
            }
        }
        _ => format!("'{}'", value.to_string().replace('\'', "''")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sqlite::Column;

    #[test]
    fn test_column_to_sql() {
        let generator = SqliteGenerator::new();

        let col = Column::new("id", "integer").primary_key().not_null();
        assert_eq!(generator.column_to_sql(&col), "`id` INTEGER PRIMARY KEY");

        let col = Column::new("name", "text").not_null();
        assert_eq!(generator.column_to_sql(&col), "`name` TEXT NOT NULL");

        let col = Column::new("count", "integer").default_value(serde_json::json!(0));
        assert_eq!(generator.column_to_sql(&col), "`count` INTEGER DEFAULT 0");
    }

    #[test]
    fn test_create_table() {
        let generator = SqliteGenerator::new();

        let mut table = Table::new("users");
        table.add_column(Column::new("id", "integer").primary_key().not_null());
        table.add_column(Column::new("name", "text").not_null());

        let sql = generator.generate_create_table(&table);
        assert!(sql.contains("CREATE TABLE `users`"));
        assert!(sql.contains("`id` INTEGER PRIMARY KEY"));
        assert!(sql.contains("`name` TEXT NOT NULL"));
    }

    #[test]
    fn test_create_index() {
        let generator = SqliteGenerator::new();

        let index = Index::new("idx_users_name", vec!["name".to_string()]);
        let sql = generator.generate_create_index("users", &index);
        assert_eq!(sql, "CREATE INDEX `idx_users_name` ON `users`(`name`);");

        let unique_index = Index::new("idx_users_email", vec!["email".to_string()]).unique();
        let sql = generator.generate_create_index("users", &unique_index);
        assert_eq!(
            sql,
            "CREATE UNIQUE INDEX `idx_users_email` ON `users`(`email`);"
        );
    }
}
