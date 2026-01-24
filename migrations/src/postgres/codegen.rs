//! PostgreSQL schema code generation
//!
//! This module generates Rust source code from introspected DDL entities.
//! The generated code uses the lowercase attribute syntax (e.g., `primary` instead of `PRIMARY`)
//! that is the current recommended style.

use super::collection::PostgresDDL;
use super::ddl::{Column, Enum, ForeignKey, Index, Table, View};
use crate::utils::escape_for_rust_literal;
use heck::{ToPascalCase, ToSnakeCase};
use std::collections::{HashMap, HashSet};

/// Result of code generation
#[derive(Debug, Clone, Default)]
pub struct GeneratedSchema {
    /// The generated Rust source code
    pub code: String,
    /// Enums that were generated
    pub enums: Vec<String>,
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
pub fn generate_rust_schema(ddl: &PostgresDDL, options: &CodegenOptions) -> GeneratedSchema {
    let mut result = GeneratedSchema::default();
    let mut code = String::new();

    // Module header
    code.push_str("//! Auto-generated PostgreSQL schema from introspection\n");
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
    code.push_str("use drizzle::postgres::prelude::*;\n\n");

    // Build a map of (schema, enum_name) -> enum type name for column type resolution
    let mut enum_map: HashMap<(String, String), String> = HashMap::new();
    for e in ddl.enums.list() {
        let type_name = e.name.to_pascal_case();
        enum_map.insert((e.schema.to_string(), e.name.to_string()), type_name);
    }

    // Generate enum definitions
    for e in ddl.enums.list() {
        let enum_code = generate_enum_struct(e, options.use_pub);
        code.push_str(&enum_code);
        code.push('\n');
        result.enums.push(e.name.to_string());
    }

    // Build a map of (schema, table) -> columns
    let mut table_columns: HashMap<(String, String), Vec<&Column>> = HashMap::new();
    for column in ddl.columns.list() {
        table_columns
            .entry((column.schema.to_string(), column.table.to_string()))
            .or_default()
            .push(column);
    }

    // Build a map of (schema, table) -> primary key columns
    let mut table_pks: HashMap<(String, String), HashSet<String>> = HashMap::new();
    for pk in ddl.pks.list() {
        for col in pk.columns.iter() {
            table_pks
                .entry((pk.schema.to_string(), pk.table.to_string()))
                .or_default()
                .insert(col.to_string());
        }
    }

    // Build a map of (schema, table) -> unique constraints (single-column only for inline)
    let mut table_uniques: HashMap<(String, String), HashSet<String>> = HashMap::new();
    for unique in ddl.uniques.list() {
        if unique.columns.len() == 1 {
            table_uniques
                .entry((unique.schema.to_string(), unique.table.to_string()))
                .or_default()
                .insert(unique.columns[0].to_string());
        }
    }

    // Build a map of foreign keys by (schema, table, column) -> (FK, ref_column_idx)
    let mut fk_map: HashMap<(String, String, String), (&ForeignKey, usize)> = HashMap::new();
    for fk in ddl.fks.list() {
        for (idx, col) in fk.columns.iter().enumerate() {
            fk_map.insert(
                (fk.schema.to_string(), fk.table.to_string(), col.to_string()),
                (fk, idx),
            );
        }
    }

    // Generate table structs
    for table in ddl.tables.list() {
        let key = (table.schema.to_string(), table.name.to_string());
        let columns = table_columns.get(&key).map(|c| c.as_slice()).unwrap_or(&[]);
        let pk_columns = table_pks.get(&key);
        let unique_columns = table_uniques.get(&key);
        let is_composite_pk = pk_columns.map(|pks| pks.len() > 1).unwrap_or(false);

        let table_code = generate_table_struct(&TableGenContext {
            table,
            columns,
            pk_columns,
            unique_columns,
            is_composite_pk,
            fk_map: &fk_map,
            enum_map: &enum_map,
            use_pub: options.use_pub,
        });

        code.push_str(&table_code);
        code.push('\n');
        result.tables.push(table.name.to_string());
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
        let key = (view.schema.to_string(), view.name.to_string());
        let columns = table_columns.get(&key).map(|c| c.as_slice()).unwrap_or(&[]);
        let view_code = generate_view_struct(view, columns, &enum_map, options.use_pub);
        code.push_str(&view_code);
        code.push('\n');
        result.views.push(view.name.to_string());
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

/// Context for generating a table struct
struct TableGenContext<'a> {
    table: &'a Table,
    columns: &'a [&'a Column],
    pk_columns: Option<&'a HashSet<String>>,
    unique_columns: Option<&'a HashSet<String>>,
    is_composite_pk: bool,
    fk_map: &'a HashMap<(String, String, String), (&'a ForeignKey, usize)>,
    enum_map: &'a HashMap<(String, String), String>,
    use_pub: bool,
}

/// Generate a single table struct
fn generate_table_struct(ctx: &TableGenContext<'_>) -> String {
    let struct_name = ctx.table.name.to_pascal_case();
    let vis = if ctx.use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Table attribute
    code.push_str("#[PostgresTable]\n");

    // Struct definition
    code.push_str(&format!("{vis}struct {struct_name} {{\n"));

    // Sort columns by ordinal position if available, falling back to name.
    let mut sorted_columns: Vec<&&Column> = ctx.columns.iter().collect();
    sorted_columns.sort_by(|a, b| {
        let ao = a.ordinal_position.unwrap_or(i32::MAX);
        let bo = b.ordinal_position.unwrap_or(i32::MAX);
        ao.cmp(&bo).then_with(|| a.name.cmp(&b.name))
    });

    // Generate fields
    for column in sorted_columns {
        let field_code = generate_column_field(
            column,
            ctx.pk_columns,
            ctx.unique_columns,
            ctx.is_composite_pk,
            ctx.fk_map,
            ctx.enum_map,
            ctx.use_pub,
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
    fk_map: &HashMap<(String, String, String), (&ForeignKey, usize)>,
    enum_map: &HashMap<(String, String), String>,
    use_pub: bool,
) -> String {
    let field_name = column.name.to_snake_case();
    let vis = if use_pub { "pub " } else { "" };

    let col_name_str = column.name.to_string();
    let is_pk = pk_columns
        .map(|pks| pks.contains(&col_name_str))
        .unwrap_or(false);
    let is_unique = unique_columns
        .map(|uqs| uqs.contains(&col_name_str))
        .unwrap_or(false);

    // For single-column PKs, add primary. For composite, skip (handled at table level)
    let should_add_primary = is_pk && !is_composite_pk;

    // Check for serial (nextval default without identity)
    let is_serial = column
        .default
        .as_ref()
        .map(|d| d.contains("nextval"))
        .unwrap_or(false)
        && column.identity.is_none();

    // Get FK info if present
    let fk_info = fk_map.get(&(
        column.schema.to_string(),
        column.table.to_string(),
        col_name_str.clone(),
    ));

    // Check if this column uses an enum type
    let type_schema = column.type_schema.as_deref().unwrap_or(&column.schema);
    let enum_type = enum_map.get(&(type_schema.to_string(), column.sql_type.to_string()));

    // Build column attributes
    let mut attrs = Vec::new();

    // For SERIAL columns (auto-increment via nextval), use "serial" attribute
    if is_serial {
        attrs.push("serial".to_string());
    }

    // For GENERATED IDENTITY columns, use identity(always) or identity(by_default)
    // with optional sequence options
    if let Some(identity) = &column.identity {
        use super::ddl::IdentityType;
        let identity_type = match identity.type_ {
            IdentityType::Always => "always",
            IdentityType::ByDefault => "by_default",
        };

        // Build sequence options if any are non-default
        let mut seq_opts: Vec<String> = Vec::new();
        if let Some(increment) = &identity.increment
            && increment != "1"
        {
            seq_opts.push(format!("increment = {}", increment));
        }
        if let Some(start) = &identity.start_with
            && start != "1"
        {
            seq_opts.push(format!("start = {}", start));
        }
        if let Some(min) = &identity.min_value {
            seq_opts.push(format!("min_value = {}", min));
        }
        if let Some(max) = &identity.max_value {
            seq_opts.push(format!("max_value = {}", max));
        }
        if let Some(cache) = &identity.cache
            && *cache != 1
        {
            seq_opts.push(format!("cache = {}", cache));
        }
        if identity.cycle == Some(true) {
            seq_opts.push("cycle".to_string());
        }

        if seq_opts.is_empty() {
            attrs.push(format!("identity({})", identity_type));
        } else {
            attrs.push(format!(
                "identity({}, {})",
                identity_type,
                seq_opts.join(", ")
            ));
        }
    }

    if should_add_primary {
        attrs.push("primary".to_string());
    }

    if is_unique {
        attrs.push("unique".to_string());
    }

    // Add "enum" attribute for enum-typed columns
    if enum_type.is_some() {
        attrs.push("enum".to_string());
    }

    // Add generated column attribute for GENERATED AS columns
    if let Some(generated) = &column.generated {
        use super::ddl::GeneratedType;
        let gen_type = match generated.gen_type {
            GeneratedType::Stored => "stored",
        };
        // Escape quotes in expression
        let expr = generated.expression.replace('"', "\\\"");
        attrs.push(format!("generated({}, \"{}\")", gen_type, expr));
    }

    // Add default if present (but skip nextval for serial columns)
    if let Some(default) = &column.default
        && !is_serial
        && column.generated.is_none()
        && let Some(formatted) = format_default_value(default, &column.sql_type)
    {
        attrs.push(format!("default = {formatted}"));
    }

    // Add FK reference if present
    if let Some((fk, idx)) = fk_info {
        let ref_table = fk.table_to.to_pascal_case();
        let ref_column = fk.columns_to.get(*idx).cloned().unwrap_or_default();
        attrs.push(format!("references = {ref_table}::{ref_column}"));

        // Add on_delete if not NO ACTION
        if let Some(on_delete) = &fk.on_delete
            && on_delete != "NO ACTION"
        {
            let action = on_delete.to_lowercase().replace(' ', "_");
            attrs.push(format!("on_delete = {action}"));
        }

        // Add on_update if not NO ACTION
        if let Some(on_update) = &fk.on_update
            && on_update != "NO ACTION"
        {
            let action = on_update.to_lowercase().replace(' ', "_");
            attrs.push(format!("on_update = {action}"));
        }
    }

    // Generate attribute line if there are any
    let mut result = String::new();
    if !attrs.is_empty() {
        result.push_str(&format!("    #[column({})]\n", attrs.join(", ")));
    }

    // Determine Rust type - use enum type if available, otherwise map SQL type
    let rust_type = if let Some(enum_name) = enum_type {
        if column.not_null {
            enum_name.clone()
        } else {
            format!("Option<{}>", enum_name)
        }
    } else {
        sql_type_to_rust_type(&column.sql_type, column.not_null)
    };

    result.push_str(&format!("    {vis}{field_name}: {rust_type},\n"));
    result
}

/// Generate a Rust enum definition from a PostgreSQL enum
fn generate_enum_struct(e: &Enum, use_pub: bool) -> String {
    let enum_name = e.name.to_pascal_case();
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Enum derive attribute with PostgresEnum - matches the project's actual usage
    // #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
    code.push_str("#[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]\n");

    // Enum definition
    code.push_str(&format!("{vis}enum {enum_name} {{\n"));

    // Generate variants from enum values
    for (idx, value) in e.values.iter().enumerate() {
        let variant_name = value.to_pascal_case();
        // First variant gets #[default] attribute
        if idx == 0 {
            code.push_str("    #[default]\n");
        }
        code.push_str(&format!("    {},\n", variant_name));
    }

    code.push_str("}\n");
    code
}

/// Format a default value for Rust syntax
fn format_default_value(default: &str, sql_type: &str) -> Option<String> {
    let default = default.trim();

    // Skip defaults that are function calls (like now(), nextval(), etc.)
    if default.contains('(') || default.starts_with("nextval") {
        return None;
    }

    // Handle NULL
    if default.eq_ignore_ascii_case("null") {
        return None;
    }

    // Handle boolean
    if default.eq_ignore_ascii_case("true") || default.eq_ignore_ascii_case("false") {
        return Some(default.to_lowercase());
    }

    // Handle numeric types
    if sql_type.contains("int")
        || sql_type.contains("numeric")
        || sql_type.contains("decimal")
        || sql_type == "float4"
        || sql_type == "float8"
    {
        // Remove type casts like ::integer
        let value = default.split("::").next().unwrap_or(default);
        return Some(value.trim_matches('\'').to_string());
    }

    // Handle text/string types
    if sql_type.contains("text")
        || sql_type.contains("varchar")
        || sql_type.contains("char")
        || sql_type == "bpchar"
    {
        // Keep as quoted string, removing Postgres specific casts
        let value = default.split("::").next().unwrap_or(default);
        let trimmed = value.trim_matches('\'');
        return Some(format!("\"{}\"", trimmed));
    }

    // For other types, just return as-is
    Some(default.to_string())
}

/// Convert PostgreSQL type to Rust type
pub fn sql_type_to_rust_type(sql_type: &str, not_null: bool) -> String {
    // Handle PostgreSQL array types, which are often represented as "_typename" (udt_name).
    // Keep this intentionally simple: one-dimensional arrays map to Vec<T>.
    if let Some(elem) = sql_type.strip_prefix('_') {
        let elem_ty = sql_type_to_rust_type(elem, true);
        let base = format!("Vec<{}>", elem_ty);
        return if not_null {
            base
        } else {
            format!("Option<{}>", base)
        };
    }

    let base_type = match sql_type {
        // Integer types
        s if s.eq_ignore_ascii_case("int2") || s.eq_ignore_ascii_case("smallint") => "i16",
        s if s.eq_ignore_ascii_case("int4")
            || s.eq_ignore_ascii_case("integer")
            || s.eq_ignore_ascii_case("int") =>
        {
            "i32"
        }
        s if s.eq_ignore_ascii_case("int8") || s.eq_ignore_ascii_case("bigint") => "i64",
        s if s.eq_ignore_ascii_case("serial") || s.eq_ignore_ascii_case("serial4") => "i32",
        s if s.eq_ignore_ascii_case("bigserial") || s.eq_ignore_ascii_case("serial8") => "i64",
        s if s.eq_ignore_ascii_case("smallserial") || s.eq_ignore_ascii_case("serial2") => "i16",

        // Floating point
        s if s.eq_ignore_ascii_case("float4") || s.eq_ignore_ascii_case("real") => "f32",
        s if s.eq_ignore_ascii_case("float8") || s.eq_ignore_ascii_case("double precision") => {
            "f64"
        }
        s if s.eq_ignore_ascii_case("numeric") || s.eq_ignore_ascii_case("decimal") => "String", // Use String for precise decimals

        // Boolean
        s if s.eq_ignore_ascii_case("bool") || s.eq_ignore_ascii_case("boolean") => "bool",

        // Text types
        s if s.eq_ignore_ascii_case("text")
            || s.eq_ignore_ascii_case("varchar")
            || s.eq_ignore_ascii_case("char")
            || s.eq_ignore_ascii_case("bpchar")
            || s.eq_ignore_ascii_case("name") =>
        {
            "String"
        }

        // Binary
        s if s.eq_ignore_ascii_case("bytea") => "Vec<u8>",

        // UUID
        s if s.eq_ignore_ascii_case("uuid") => "uuid::Uuid",

        // Date/Time types
        s if s.eq_ignore_ascii_case("date") => "chrono::NaiveDate",
        s if s.eq_ignore_ascii_case("time") => "chrono::NaiveTime",
        s if s.eq_ignore_ascii_case("timestamp") => "chrono::NaiveDateTime",
        s if s.eq_ignore_ascii_case("timestamptz") => "chrono::DateTime<chrono::Utc>",

        // JSON
        s if s.eq_ignore_ascii_case("json") || s.eq_ignore_ascii_case("jsonb") => {
            "serde_json::Value"
        }

        // Default to String for unknown types
        _ => "String",
    };

    if not_null {
        base_type.to_string()
    } else {
        format!("Option<{}>", base_type)
    }
}

/// Generate an index struct
fn generate_index_struct(index: &Index, use_pub: bool) -> String {
    let struct_name = index.name.to_pascal_case();
    let table_name = index.table.to_pascal_case();
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Index attribute
    let attrs = if index.is_unique {
        "#[PostgresIndex(unique)]"
    } else {
        "#[PostgresIndex]"
    };
    code.push_str(&format!("{attrs}\n"));

    // Tuple struct with column references
    let columns: Vec<String> = index
        .columns
        .iter()
        .map(|c| {
            if c.is_expression {
                format!("\"{}\"", c.value) // Expression indexes use string literals
            } else {
                format!("{}::{}", table_name, c.value.to_snake_case())
            }
        })
        .collect();

    code.push_str(&format!(
        "{vis}struct {struct_name}({});\n",
        columns.join(", ")
    ));
    code
}

/// Generate a view struct
fn generate_view_struct(
    view: &View,
    columns: &[&Column],
    enum_map: &HashMap<(String, String), String>,
    use_pub: bool,
) -> String {
    let struct_name = view.name.to_pascal_case();
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Build view attributes
    let mut attrs = Vec::new();

    // Check if view name differs from struct name (snake_case version)
    if struct_name.to_snake_case() != view.name.as_ref() {
        attrs.push(format!("name = \"{}\"", view.name));
    }

    // Add schema if not public
    if view.schema != "public" {
        attrs.push(format!("schema = \"{}\"", view.schema));
    }

    // Add materialized flag if true
    if view.materialized {
        attrs.push("materialized".to_string());
    }

    // Add WITH NO DATA for materialized views
    if view.with_no_data == Some(true) {
        attrs.push("with_no_data".to_string());
    }

    // Add USING clause for materialized views
    if let Some(using) = &view.using {
        attrs.push(format!("using = \"{}\"", using));
    }

    // Add TABLESPACE for materialized views
    if let Some(tablespace) = &view.tablespace {
        attrs.push(format!("tablespace = \"{}\"", tablespace));
    }

    // Add definition
    if let Some(def) = &view.definition {
        let escaped_def = escape_for_rust_literal(def);
        attrs.push(format!("definition = \"{}\"", escaped_def));
    }

    // Build the attribute line
    if attrs.is_empty() {
        code.push_str("#[PostgresView]\n");
    } else {
        code.push_str(&format!("#[PostgresView({})]\n", attrs.join(", ")));
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

        // Check if this column uses an enum type
        let type_schema = column.type_schema.as_deref().unwrap_or(&column.schema);
        let enum_type = enum_map.get(&(type_schema.to_string(), column.sql_type.to_string()));

        // Determine Rust type - use enum type if available, otherwise map SQL type
        let rust_type = if let Some(enum_name) = enum_type {
            if column.not_null {
                enum_name.clone()
            } else {
                format!("Option<{}>", enum_name)
            }
        } else {
            sql_type_to_rust_type(&column.sql_type, column.not_null)
        };

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
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Schema derive
    code.push_str("#[derive(PostgresSchema)]\n");
    code.push_str(&format!("{vis}struct {schema_name} {{\n"));

    // Table fields
    for table in tables {
        let field_name = table.to_snake_case();
        let type_name = table.to_pascal_case();
        code.push_str(&format!("    {vis}{field_name}: {type_name},\n"));
    }

    // Index fields (commented as they're typically not needed in schema)
    if !indexes.is_empty() {
        code.push_str("    // Indexes:\n");
        for index in indexes {
            let field_name = index.to_snake_case();
            let type_name = index.to_pascal_case();
            code.push_str(&format!("    // {field_name}: {type_name},\n"));
        }
    }

    code.push_str("}\n");
    code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sql_type_to_rust_type() {
        assert_eq!(sql_type_to_rust_type("int4", true), "i32");
        assert_eq!(sql_type_to_rust_type("int8", true), "i64");
        assert_eq!(sql_type_to_rust_type("text", true), "String");
        assert_eq!(sql_type_to_rust_type("bool", true), "bool");
        assert_eq!(sql_type_to_rust_type("bytea", true), "Vec<u8>");

        // Nullable types
        assert_eq!(sql_type_to_rust_type("int4", false), "Option<i32>");
        assert_eq!(sql_type_to_rust_type("text", false), "Option<String>");
    }

    #[test]
    fn test_format_default_value() {
        // Numeric
        assert_eq!(format_default_value("42", "int4"), Some("42".to_string()));
        assert_eq!(
            format_default_value("3.14::numeric", "numeric"),
            Some("3.14".to_string())
        );

        // Boolean
        assert_eq!(
            format_default_value("true", "bool"),
            Some("true".to_string())
        );

        // String
        assert_eq!(
            format_default_value("'hello'::text", "text"),
            Some("\"hello\"".to_string())
        );

        // Function calls should be None
        assert_eq!(format_default_value("now()", "timestamp"), None);
        assert_eq!(
            format_default_value("nextval('seq'::regclass)", "int4"),
            None
        );
    }
}
