//! PostgreSQL SQL generation from schema metadata

use crate::postgres::{
    AlteredColumn, AlteredEnum, AlteredTable, Column, Enum, ForeignKey, Index, IndexColumn,
    SchemaDiff, Table,
};

/// SQL statement breakpoint marker
pub const BREAKPOINT: &str = "--> statement-breakpoint";

/// PostgreSQL SQL generator
pub struct PostgresGenerator {
    pub breakpoints: bool,
}

impl Default for PostgresGenerator {
    fn default() -> Self {
        Self::new()
    }
}

impl PostgresGenerator {
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

        // Create new enums first (they might be referenced by tables)
        for e in &diff.enums.created {
            statements.push(self.generate_create_enum(e));
        }

        // Alter existing enums (add values)
        for altered in &diff.enums.altered {
            statements.extend(self.generate_alter_enum(altered));
        }

        // Drop deleted tables
        for table_name in &diff.tables.deleted {
            statements.push(format!("DROP TABLE \"{}\";", table_name));
        }

        // Create new tables
        for table in &diff.tables.created {
            statements.push(self.generate_create_table(table));
        }

        // Alter existing tables
        for altered in &diff.tables.altered {
            statements.extend(self.generate_alter_table(altered));
        }

        // Drop deleted enums (after tables that might reference them)
        for enum_name in &diff.enums.deleted {
            statements.push(format!("DROP TYPE \"{}\";", enum_name));
        }

        // Handle sequences
        for seq in &diff.sequences.created {
            statements.push(format!(
                "CREATE SEQUENCE \"{}\".\"{}\";",
                seq.schema, seq.name
            ));
        }

        for seq_name in &diff.sequences.deleted {
            statements.push(format!("DROP SEQUENCE \"{}\";", seq_name));
        }

        statements
    }

    /// Generate SQL from statements
    pub fn statements_to_sql(&self, statements: &[String]) -> String {
        if self.breakpoints {
            statements.join(&format!("\n{}\n", BREAKPOINT))
        } else {
            statements.join("\n")
        }
    }

    /// Generate CREATE TYPE (enum)
    pub fn generate_create_enum(&self, e: &Enum) -> String {
        let values = e
            .values
            .iter()
            .map(|v| format!("'{}'", v))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "CREATE TYPE \"{}\".\"{}\" AS ENUM ({});",
            e.schema, e.name, values
        )
    }

    /// Generate ALTER TYPE for enum changes
    pub fn generate_alter_enum(&self, altered: &AlteredEnum) -> Vec<String> {
        let mut statements = Vec::new();

        for value in &altered.values_added {
            statements.push(format!(
                "ALTER TYPE \"{}\".\"{}\" ADD VALUE '{}';",
                altered.schema, altered.name, value
            ));
        }

        // Note: PostgreSQL doesn't support removing enum values easily
        // This would require recreating the type
        if !altered.values_removed.is_empty() {
            statements.push(
                "-- WARNING: Cannot remove enum values in PostgreSQL without recreation"
                    .to_string(),
            );
            statements.push(format!("-- Removed values: {:?}", altered.values_removed));
        }

        statements
    }

    /// Generate CREATE TABLE SQL
    pub fn generate_create_table(&self, table: &Table) -> String {
        let schema_prefix = if table.schema.is_empty() || table.schema == "public" {
            String::new()
        } else {
            format!("\"{}\".", table.schema)
        };

        let mut sql = format!("CREATE TABLE {}\"{}\" (\n", schema_prefix, table.name);

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
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ");
            if pk.name.is_empty() {
                parts.push(format!("\tPRIMARY KEY ({})", cols));
            } else {
                parts.push(format!(
                    "\tCONSTRAINT \"{}\" PRIMARY KEY ({})",
                    pk.name, cols
                ));
            }
        }

        // Foreign keys
        for fk in table.foreign_keys.values() {
            parts.push(format!("\t{}", self.foreign_key_to_sql(fk)));
        }

        // Unique constraints
        for uc in table.unique_constraints.values() {
            let cols = uc
                .columns
                .iter()
                .map(|c| format!("\"{}\"", c))
                .collect::<Vec<_>>()
                .join(", ");
            parts.push(format!("\tCONSTRAINT \"{}\" UNIQUE ({})", uc.name, cols));
        }

        // Check constraints
        for cc in table.check_constraints.values() {
            parts.push(format!("\tCONSTRAINT \"{}\" CHECK ({})", cc.name, cc.value));
        }

        sql.push_str(&parts.join(",\n"));
        sql.push_str("\n);");

        sql
    }

    /// Convert a column to SQL definition
    pub fn column_to_sql(&self, column: &Column) -> String {
        let mut parts = vec![format!("\"{}\"", column.name), column.sql_type.clone()];

        if column.primary_key {
            parts.push("PRIMARY KEY".to_string());
        }

        if column.not_null && !column.primary_key {
            parts.push("NOT NULL".to_string());
        }

        if let Some(ref default) = column.default {
            parts.push(format!("DEFAULT {}", default_to_sql(default)));
        }

        if let Some(ref generated) = column.generated {
            parts.push(format!(
                "GENERATED ALWAYS AS ({}) STORED",
                generated.expression
            ));
        }

        if let Some(ref identity) = column.identity {
            let identity_type = if identity.identity_type == "always" {
                "ALWAYS"
            } else {
                "BY DEFAULT"
            };
            parts.push(format!("GENERATED {} AS IDENTITY", identity_type));
        }

        parts.join(" ")
    }

    /// Convert a foreign key to SQL
    pub fn foreign_key_to_sql(&self, fk: &ForeignKey) -> String {
        let cols_from = fk
            .columns_from
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");
        let cols_to = fk
            .columns_to
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        let table_to = if let Some(ref schema) = fk.schema_to {
            format!("\"{}\".\"{}\"", schema, fk.table_to)
        } else {
            format!("\"{}\"", fk.table_to)
        };

        let mut sql = format!(
            "CONSTRAINT \"{}\" FOREIGN KEY ({}) REFERENCES {}({})",
            fk.name, cols_from, table_to, cols_to
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
    pub fn generate_create_index(&self, table: &Table, index: &Index) -> String {
        let unique = if index.is_unique { "UNIQUE " } else { "" };
        let concurrently = if index.concurrently {
            "CONCURRENTLY "
        } else {
            ""
        };

        let cols = index
            .columns
            .iter()
            .map(|c| self.index_column_to_sql(c))
            .collect::<Vec<_>>()
            .join(", ");

        let schema_prefix = if table.schema.is_empty() || table.schema == "public" {
            String::new()
        } else {
            format!("\"{}\".", table.schema)
        };

        let mut sql = format!(
            "CREATE {}{}INDEX \"{}\" ON {}\"{}\" USING {} ({});",
            unique, concurrently, index.name, schema_prefix, table.name, index.method, cols
        );

        if let Some(ref where_clause) = index.r#where {
            sql = format!(
                "CREATE {}{}INDEX \"{}\" ON {}\"{}\" USING {} ({}) WHERE {};",
                unique,
                concurrently,
                index.name,
                schema_prefix,
                table.name,
                index.method,
                cols,
                where_clause
            );
        }

        sql
    }

    fn index_column_to_sql(&self, col: &IndexColumn) -> String {
        let mut parts = vec![if col.is_expression {
            format!("({})", col.expression)
        } else {
            format!("\"{}\"", col.expression)
        }];

        if let Some(ref opclass) = col.opclass {
            parts.push(opclass.clone());
        }

        if !col.asc {
            parts.push("DESC".to_string());
        }

        if let Some(ref nulls) = col.nulls {
            parts.push(format!("NULLS {}", nulls.to_uppercase()));
        }

        parts.join(" ")
    }

    /// Generate DROP INDEX SQL
    pub fn generate_drop_index(&self, schema: &str, index_name: &str) -> String {
        if schema.is_empty() || schema == "public" {
            format!("DROP INDEX \"{}\";", index_name)
        } else {
            format!("DROP INDEX \"{}\".\"{}\";", schema, index_name)
        }
    }

    /// Generate ALTER TABLE statements
    pub fn generate_alter_table(&self, altered: &AlteredTable) -> Vec<String> {
        let mut statements = Vec::new();

        let schema_prefix = if altered.schema.is_empty() || altered.schema == "public" {
            String::new()
        } else {
            format!("\"{}\".", altered.schema)
        };
        let table_ref = format!("{}\"{}\"", schema_prefix, altered.name);

        // Add columns
        for column in &altered.columns.added {
            statements.push(format!(
                "ALTER TABLE {} ADD COLUMN {};",
                table_ref,
                self.column_to_sql(column)
            ));
        }

        // Drop columns
        for col_name in &altered.columns.deleted {
            statements.push(format!(
                "ALTER TABLE {} DROP COLUMN \"{}\";",
                table_ref, col_name
            ));
        }

        // Alter columns
        for alt_col in &altered.columns.altered {
            statements.extend(self.generate_alter_column(&table_ref, alt_col));
        }

        // Handle index changes
        for index_name in &altered.indexes.deleted {
            statements.push(self.generate_drop_index(&altered.schema, index_name));
        }

        // Create placeholder table for index generation
        let table = Table {
            name: altered.name.clone(),
            schema: altered.schema.clone(),
            ..Default::default()
        };

        for index in &altered.indexes.added {
            statements.push(self.generate_create_index(&table, index));
        }

        for (old_idx, new_idx) in &altered.indexes.altered {
            statements.push(self.generate_drop_index(&altered.schema, &old_idx.name));
            statements.push(self.generate_create_index(&table, new_idx));
        }

        // Foreign key changes
        for fk_name in &altered.foreign_keys.deleted {
            statements.push(format!(
                "ALTER TABLE {} DROP CONSTRAINT \"{}\";",
                table_ref, fk_name
            ));
        }

        for fk in &altered.foreign_keys.added {
            statements.push(format!(
                "ALTER TABLE {} ADD {};",
                table_ref,
                self.foreign_key_to_sql(fk)
            ));
        }

        statements
    }

    fn generate_alter_column(&self, table_ref: &str, alt_col: &AlteredColumn) -> Vec<String> {
        let mut statements = Vec::new();
        let col_ref = format!("\"{}\"", alt_col.name);

        // Type change
        if alt_col.old.sql_type != alt_col.new.sql_type {
            statements.push(format!(
                "ALTER TABLE {} ALTER COLUMN {} SET DATA TYPE {} USING {}::{};",
                table_ref, col_ref, alt_col.new.sql_type, col_ref, alt_col.new.sql_type
            ));
        }

        // Nullability change
        if alt_col.old.not_null != alt_col.new.not_null {
            if alt_col.new.not_null {
                statements.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} SET NOT NULL;",
                    table_ref, col_ref
                ));
            } else {
                statements.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} DROP NOT NULL;",
                    table_ref, col_ref
                ));
            }
        }

        // Default value change
        if alt_col.old.default != alt_col.new.default {
            if let Some(ref default) = alt_col.new.default {
                statements.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} SET DEFAULT {};",
                    table_ref,
                    col_ref,
                    default_to_sql(default)
                ));
            } else {
                statements.push(format!(
                    "ALTER TABLE {} ALTER COLUMN {} DROP DEFAULT;",
                    table_ref, col_ref
                ));
            }
        }

        statements
    }
}

/// Convert a JSON default value to SQL literal
fn default_to_sql(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "NULL".to_string(),
        serde_json::Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => {
            if s.starts_with('`') && s.ends_with('`') {
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

    #[test]
    fn test_column_to_sql() {
        let generator = PostgresGenerator::new();

        let col = Column::new("id", "serial").primary_key();
        assert_eq!(generator.column_to_sql(&col), "\"id\" serial PRIMARY KEY");

        let col = Column::new("name", "text").not_null();
        assert_eq!(generator.column_to_sql(&col), "\"name\" text NOT NULL");
    }

    #[test]
    fn test_create_enum() {
        let generator = PostgresGenerator::new();

        let e = Enum {
            name: "status".to_string(),
            schema: "public".to_string(),
            values: vec!["active".to_string(), "inactive".to_string()],
        };

        let sql = generator.generate_create_enum(&e);
        assert!(sql.contains("CREATE TYPE"));
        assert!(sql.contains("'active'"));
        assert!(sql.contains("'inactive'"));
    }
}
