//! SQLite schema code generation
//!
//! This module generates Rust source code from introspected DDL entities.
//! The generated code uses the lowercase attribute syntax (e.g., `primary` instead of `PRIMARY`)
//! that is the current recommended style.

use super::collection::SQLiteDDL;
use super::ddl::{Column, ForeignKey, Index, Table, View};
use crate::utils::escape_for_rust_literal;
use drizzle_types::sqlite::SQLTypeCategory;
use heck::{ToPascalCase, ToSnakeCase};
use std::collections::{HashMap, HashSet};

/// Result of code generation
#[derive(Debug, Clone, Default)]
pub struct GeneratedSchema {
    /// The generated Rust source code
    pub code: String,
    /// Tables that were generated
    pub tables: Vec<String>,
    /// Indexes that were generated
    pub indexes: Vec<String>,
    /// Views that were generated
    pub views: Vec<String>,
    /// Any warnings during generation
    pub warnings: Vec<String>,
}

/// Options for code generation
#[derive(Debug, Clone, Default)]
pub struct CodegenOptions {
    /// Module documentation
    pub module_doc: Option<String>,
    /// Whether to include a schema struct
    pub include_schema: bool,
    /// Schema struct name
    pub schema_name: String,
    /// Whether to use public visibility
    pub use_pub: bool,
}

/// Generate Rust schema code from DDL
pub fn generate_rust_schema(ddl: &SQLiteDDL, options: &CodegenOptions) -> GeneratedSchema {
    let mut result = GeneratedSchema::default();
    let mut code = String::new();

    // Module header
    code.push_str("//! Auto-generated SQLite schema from introspection\n");
    code.push_str("//!\n");
    if let Some(doc) = &options.module_doc {
        for line in doc.lines() {
            code.push_str("//! ");
            code.push_str(line);
            code.push('\n');
        }
    }
    code.push('\n');

    // Imports
    code.push_str("use drizzle::sqlite::prelude::*;\n\n");

    // Build a map of table name -> columns
    let mut table_columns: HashMap<String, Vec<&Column>> = HashMap::new();
    for column in ddl.columns.list() {
        table_columns
            .entry(column.table.to_string())
            .or_default()
            .push(column);
    }

    // Build a map of table name -> primary key columns
    let mut table_pks: HashMap<String, HashSet<String>> = HashMap::new();
    for pk in ddl.pks.list() {
        for col in pk.columns.iter() {
            table_pks
                .entry(pk.table.to_string())
                .or_default()
                .insert(col.to_string());
        }
    }

    // Build a map of table name -> unique constraints (single-column only for inline)
    let mut table_uniques: HashMap<String, HashSet<String>> = HashMap::new();
    for unique in ddl.uniques.list() {
        if unique.columns.len() == 1 {
            table_uniques
                .entry(unique.table.to_string())
                .or_default()
                .insert(unique.columns[0].to_string());
        }
    }

    // Build a map of foreign keys by (table, column) -> (ref_table, ref_column)
    let mut fk_map: HashMap<(String, String), (&ForeignKey, usize)> = HashMap::new();
    for fk in ddl.fks.list() {
        for (idx, col) in fk.columns.iter().enumerate() {
            fk_map.insert((fk.table.to_string(), col.to_string()), (fk, idx));
        }
    }

    // Generate table structs
    for table in ddl.tables.list() {
        let table_name = table.name.to_string();
        let columns = table_columns
            .get(&table_name)
            .map(|c| c.as_slice())
            .unwrap_or(&[]);

        // Preserve DB/introspection order when available (cid -> ordinal_position).
        let mut columns_sorted: Vec<&Column> = columns.to_vec();
        columns_sorted.sort_by(|a, b| {
            let ao = a.ordinal_position.unwrap_or(i32::MAX);
            let bo = b.ordinal_position.unwrap_or(i32::MAX);
            ao.cmp(&bo).then_with(|| a.name.cmp(&b.name))
        });
        let pk_columns = table_pks.get(&table_name);
        let unique_columns = table_uniques.get(&table_name);
        let is_composite_pk = pk_columns.map(|pks| pks.len() > 1).unwrap_or(false);

        let table_code = generate_table_struct(
            table,
            &columns_sorted,
            pk_columns,
            unique_columns,
            is_composite_pk,
            &fk_map,
            options.use_pub,
        );

        code.push_str(&table_code);
        code.push('\n');
        result.tables.push(table_name);
    }

    // Generate index structs
    for index in ddl.indexes.list() {
        let index_code = generate_index_struct(index, options.use_pub);
        code.push_str(&index_code);
        code.push('\n');
        result.indexes.push(index.name.to_string());
    }

    // Generate view structs
    for view in ddl.views.list() {
        // Skip existing views (not managed by drizzle)
        if view.is_existing {
            continue;
        }
        let view_name = view.name.to_string();
        let columns = table_columns.get(&view_name).map(|c| c.as_slice()).unwrap_or(&[]);
        let view_code = generate_view_struct(view, columns, options.use_pub);
        code.push_str(&view_code);
        code.push('\n');
        result.views.push(view_name);
    }

    // Generate schema struct if requested
    if options.include_schema {
        let schema_code = generate_schema_struct(
            &options.schema_name,
            &result.tables,
            &result.indexes,
            options.use_pub,
        );
        code.push_str(&schema_code);
    }

    result.code = code;
    result
}

/// Generate a single table struct
fn generate_table_struct(
    table: &Table,
    columns: &[&Column],
    pk_columns: Option<&HashSet<String>>,
    unique_columns: Option<&HashSet<String>>,
    is_composite_pk: bool,
    fk_map: &HashMap<(String, String), (&ForeignKey, usize)>,
    use_pub: bool,
) -> String {
    let mut code = String::new();
    let vis = if use_pub { "pub " } else { "" };

    // Struct name is PascalCase of table name
    let struct_name = table.name.to_pascal_case();

    // Check if table name differs from struct name
    let needs_name_attr = struct_name.to_snake_case() != table.name;

    // Build table attribute options
    let mut table_attrs = Vec::new();
    if needs_name_attr {
        table_attrs.push(format!("name = \"{}\"", table.name));
    }
    if table.strict {
        table_attrs.push("strict".to_string());
    }
    if table.without_rowid {
        table_attrs.push("without_rowid".to_string());
    }

    // Table attribute
    if table_attrs.is_empty() {
        code.push_str("#[SQLiteTable]\n");
    } else {
        code.push_str(&format!("#[SQLiteTable({})]\n", table_attrs.join(", ")));
    }

    // Struct definition
    code.push_str(&format!("{}struct {} {{\n", vis, struct_name));

    // Fields
    for column in columns {
        let field_code = generate_column_field(
            column,
            pk_columns,
            unique_columns,
            is_composite_pk,
            fk_map,
            use_pub,
        );
        code.push_str(&field_code);
    }

    code.push_str("}\n");
    code
}

/// Generate a single column as a struct field
fn generate_column_field(
    column: &Column,
    pk_columns: Option<&HashSet<String>>,
    unique_columns: Option<&HashSet<String>>,
    is_composite_pk: bool,
    fk_map: &HashMap<(String, String), (&ForeignKey, usize)>,
    use_pub: bool,
) -> String {
    let vis = if use_pub { "pub " } else { "" };

    // Determine column attributes
    let mut attrs = Vec::new();
    let column_name = column.name.to_string();

    // Check if primary key
    let is_pk = pk_columns.is_some_and(|pks| pks.contains(&column_name));

    // Only add primary if it's a single-column PK (not composite)
    if is_pk && !is_composite_pk {
        attrs.push("primary".to_string());
    }

    // Check autoincrement
    if column.autoincrement == Some(true) {
        attrs.push("autoincrement".to_string());
    }

    // Check unique (only for single-column constraints)
    let is_unique = unique_columns.is_some_and(|uniques| uniques.contains(&column_name));
    if is_unique {
        attrs.push("unique".to_string());
    }

    // Check default value
    if let Some(default) = &column.default {
        // Format default value for Rust
        let default_str = format_default_value(default, &column.sql_type);
        if let Some(d) = default_str {
            attrs.push(format!("default = {}", d));
        }
    }

    // Check foreign key
    if let Some((fk, idx)) = fk_map.get(&(column.table.to_string(), column_name.clone()))
        && let Some(ref_col) = fk.columns_to.get(*idx)
    {
        let ref_table_struct = fk.table_to.to_pascal_case();
        attrs.push(format!("references = {}::{}", ref_table_struct, ref_col));

        // Add ON DELETE if specified
        if let Some(on_delete) = &fk.on_delete
            && !on_delete.eq_ignore_ascii_case("NO ACTION")
        {
            let action = on_delete.replace(' ', "_").to_lowercase();
            attrs.push(format!("on_delete = {}", action));
        }

        // Add ON UPDATE if specified
        if let Some(on_update) = &fk.on_update
            && !on_update.eq_ignore_ascii_case("NO ACTION")
        {
            let action = on_update.replace(' ', "_").to_lowercase();
            attrs.push(format!("on_update = {}", action));
        }
    }

    // Build the #[column(...)] attribute if there are any modifiers
    let attr_str = if attrs.is_empty() {
        String::new()
    } else {
        format!("    #[column({})]\n", attrs.join(", "))
    };

    // Determine if column is effectively NOT NULL:
    // Per SQLite docs (https://sqlite.org/lang_createtable.html):
    // - Explicit NOT NULL constraint
    // - INTEGER PRIMARY KEY is implicitly NOT NULL (special case)
    // - Other PRIMARY KEY types can technically be NULL due to SQLite legacy bug
    let is_integer_pk =
        is_pk && SQLTypeCategory::from_sql_type(&column.sql_type) == SQLTypeCategory::Integer;
    let is_not_null = column.not_null || is_integer_pk;

    // Determine Rust type from SQL type
    let rust_type = sql_type_to_rust_type(&column.sql_type, is_not_null);

    // Field name (snake_case)
    let field_name = column.name.to_snake_case();

    format!("{}    {}{}: {},\n", attr_str, vis, field_name, rust_type)
}

/// Format a default value for Rust syntax
fn format_default_value(default: &str, sql_type: &str) -> Option<String> {
    let category = SQLTypeCategory::from_sql_type(sql_type);

    // Skip function calls or complex expressions - these need default_fn
    if default.contains('(') && default.contains(')') {
        // Could be a function like CURRENT_TIMESTAMP - we'll return None
        // and add a warning instead
        return None;
    }

    match category {
        SQLTypeCategory::Integer => {
            // Boolean defaults
            if default == "0" || default == "1" {
                return Some(default.to_string());
            }
            // Integer defaults
            default.parse::<i64>().ok().map(|v| v.to_string())
        }
        SQLTypeCategory::Real => default.parse::<f64>().ok().map(|v| v.to_string()),
        SQLTypeCategory::Text | SQLTypeCategory::Blob => {
            // Remove surrounding quotes if present
            let trimmed = default.trim_matches(|c| c == '\'' || c == '"');
            Some(format!("\"{}\"", trimmed))
        }
        SQLTypeCategory::Numeric => {
            // Try as integer first, then float
            if let Ok(v) = default.parse::<i64>() {
                Some(v.to_string())
            } else if let Ok(v) = default.parse::<f64>() {
                Some(v.to_string())
            } else {
                None
            }
        }
    }
}

/// Convert SQL type to Rust type
fn sql_type_to_rust_type(sql_type: &str, not_null: bool) -> String {
    let category = SQLTypeCategory::from_sql_type(sql_type);

    let base_type = match category {
        SQLTypeCategory::Integer => "i64",
        SQLTypeCategory::Real => "f64",
        SQLTypeCategory::Text => "String",
        SQLTypeCategory::Blob => "Vec<u8>",
        SQLTypeCategory::Numeric => "i64",
    };

    if not_null {
        base_type.to_string()
    } else {
        format!("Option<{}>", base_type)
    }
}

/// Generate an index struct
fn generate_index_struct(index: &Index, use_pub: bool) -> String {
    let mut code = String::new();
    let vis = if use_pub { "pub " } else { "" };

    // Index struct name is PascalCase
    let struct_name = index.name.to_pascal_case();
    let table_struct = index.table.to_pascal_case();

    // Build column references
    let columns: Vec<String> = index
        .columns
        .iter()
        .map(|c| format!("{}::{}", table_struct, c.value))
        .collect();

    // Index attribute
    if index.is_unique {
        code.push_str("#[SQLiteIndex(unique)]\n");
    } else {
        code.push_str("#[SQLiteIndex]\n");
    }

    // Struct definition (tuple struct with column references)
    code.push_str(&format!(
        "{}struct {}({});\n",
        vis,
        struct_name,
        columns.join(", ")
    ));

    code
}

/// Generate a view struct
fn generate_view_struct(view: &View, columns: &[&Column], use_pub: bool) -> String {
    let struct_name = view.name.to_pascal_case();
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Build view attributes
    let mut attrs = Vec::new();

    // Check if view name differs from struct name (snake_case version)
    if struct_name.to_snake_case() != view.name.as_ref() {
        attrs.push(format!("name = \"{}\"", view.name));
    }

    // Add definition
    if let Some(def) = &view.definition {
        let escaped_def = escape_for_rust_literal(def);
        attrs.push(format!("definition = \"{}\"", escaped_def));
    }

    // Build the attribute line
    if attrs.is_empty() {
        code.push_str("#[SQLiteView]\n");
    } else {
        code.push_str(&format!("#[SQLiteView({})]\n", attrs.join(", ")));
    }

    // Struct definition with column fields
    code.push_str(&format!("{vis}struct {struct_name} {{\n"));

    // Sort columns by ordinal position
    let mut sorted_columns: Vec<&&Column> = columns.iter().collect();
    sorted_columns.sort_by(|a, b| {
        let ao = a.ordinal_position.unwrap_or(i32::MAX);
        let bo = b.ordinal_position.unwrap_or(i32::MAX);
        ao.cmp(&bo).then_with(|| a.name.cmp(&b.name))
    });

    // Generate fields for each column
    for column in sorted_columns {
        let field_name = column.name.to_snake_case();
        let rust_type = sql_type_to_rust_type(&column.sql_type, column.not_null);
        code.push_str(&format!("    {vis}{field_name}: {rust_type},\n"));
    }

    code.push_str("}\n");
    code
}

/// Generate a schema struct
fn generate_schema_struct(
    schema_name: &str,
    tables: &[String],
    indexes: &[String],
    use_pub: bool,
) -> String {
    let mut code = String::new();
    let vis = if use_pub { "pub " } else { "" };

    code.push_str("#[derive(SQLiteSchema)]\n");
    code.push_str(&format!("{}struct {} {{\n", vis, schema_name));

    // Add tables
    for table in tables {
        let field_name = table.to_snake_case();
        let type_name = table.to_pascal_case();
        code.push_str(&format!("    {}{}: {},\n", vis, field_name, type_name));
    }

    // Add indexes
    for index in indexes {
        let field_name = index.to_snake_case();
        let type_name = index.to_pascal_case();
        code.push_str(&format!("    {}{}: {},\n", vis, field_name, type_name));
    }

    code.push_str("}\n");
    code
}

#[cfg(test)]
mod tests {
    use super::super::ddl::*;
    use super::*;

    #[test]
    fn test_generate_simple_table() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("users"));
        ddl.columns.push(
            Column::new("users", "id", "integer")
                .not_null()
                .autoincrement(),
        );
        ddl.columns
            .push(Column::new("users", "name", "text").not_null());
        ddl.columns.push(Column::new("users", "email", "text"));
        ddl.pks.push(PrimaryKey::from_strings(
            "users".to_string(),
            "users_pk".to_string(),
            vec!["id".to_string()],
        ));

        let options = CodegenOptions {
            include_schema: false,
            schema_name: "AppSchema".to_string(),
            use_pub: true,
            ..Default::default()
        };

        let result = generate_rust_schema(&ddl, &options);

        assert!(result.code.contains("#[SQLiteTable]"));
        assert!(result.code.contains("pub struct Users"));
        assert!(result.code.contains("#[column(primary, autoincrement)]"));
        assert!(result.code.contains("pub id: i64"));
        assert!(result.code.contains("pub name: String"));
        assert!(result.code.contains("pub email: Option<String>"));
        assert_eq!(result.tables.len(), 1);
    }

    #[test]
    fn test_generate_table_with_unique() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("accounts"));
        ddl.columns
            .push(Column::new("accounts", "id", "integer").not_null());
        ddl.columns
            .push(Column::new("accounts", "email", "text").not_null());
        ddl.uniques.push(UniqueConstraint::from_strings(
            "accounts".to_string(),
            "accounts_email_unique".to_string(),
            vec!["email".to_string()],
        ));

        let options = CodegenOptions::default();
        let result = generate_rust_schema(&ddl, &options);

        assert!(result.code.contains("#[column(unique)]"));
    }

    #[test]
    fn test_generate_table_with_foreign_key() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("posts"));
        ddl.columns
            .push(Column::new("posts", "id", "integer").not_null());
        ddl.columns
            .push(Column::new("posts", "author_id", "integer").not_null());

        let fk = ForeignKey::from_strings(
            "posts".to_string(),
            "fk_posts_author".to_string(),
            vec!["author_id".to_string()],
            "users".to_string(),
            vec!["id".to_string()],
        );
        ddl.fks.push(fk);

        let options = CodegenOptions::default();
        let result = generate_rust_schema(&ddl, &options);

        assert!(result.code.contains("references = Users::id"));
    }

    #[test]
    fn test_generate_index() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("users"));
        ddl.columns
            .push(Column::new("users", "email", "text").not_null());

        ddl.indexes.push(
            Index::new(
                "users",
                "users_email_idx",
                vec![IndexColumn {
                    value: "email".into(),
                    is_expression: false,
                }],
            )
            .unique(),
        );

        let options = CodegenOptions::default();
        let result = generate_rust_schema(&ddl, &options);

        assert!(result.code.contains("#[SQLiteIndex(unique)]"));
        assert!(result.code.contains("struct UsersEmailIdx(Users::email)"));
    }

    #[test]
    fn test_generate_schema_struct() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("users"));
        ddl.tables.push(Table::new("posts"));

        let options = CodegenOptions {
            include_schema: true,
            schema_name: "AppSchema".to_string(),
            use_pub: true,
            ..Default::default()
        };

        let result = generate_rust_schema(&ddl, &options);

        assert!(result.code.contains("#[derive(SQLiteSchema)]"));
        assert!(result.code.contains("pub struct AppSchema"));
        assert!(result.code.contains("pub users: Users"));
        assert!(result.code.contains("pub posts: Posts"));
    }

    #[test]
    fn test_sql_type_to_rust_type() {
        assert_eq!(sql_type_to_rust_type("integer", true), "i64");
        assert_eq!(sql_type_to_rust_type("integer", false), "Option<i64>");
        assert_eq!(sql_type_to_rust_type("text", true), "String");
        assert_eq!(sql_type_to_rust_type("text", false), "Option<String>");
        assert_eq!(sql_type_to_rust_type("real", true), "f64");
        assert_eq!(sql_type_to_rust_type("blob", true), "Vec<u8>");
    }
}
