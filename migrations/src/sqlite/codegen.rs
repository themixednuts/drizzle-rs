//! `SQLite` schema code generation
//!
//! This module generates Rust source code from introspected DDL entities.
//! The generated code uses the lowercase attribute syntax (e.g., `primary` instead of `PRIMARY`)
//! that is the current recommended style.

use super::collection::SQLiteDDL;
use super::ddl::{CheckConstraint, Column, ForeignKey, Index, Table, UniqueConstraint, View};
use crate::utils::escape_for_rust_literal;
use drizzle_types::sqlite::SQLTypeCategory;
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

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
    /// Field naming style for generated Rust members
    pub field_casing: FieldCasing,
}

/// Casing strategy for generated Rust field names.
#[derive(Debug, Clone, Copy, Default)]
pub enum FieldCasing {
    /// `snake_case` (default)
    #[default]
    Snake,
    /// `camelCase`
    Camel,
    /// Preserve source casing as much as possible
    Preserve,
}

fn sanitize_rust_identifier(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (idx, ch) in name.chars().enumerate() {
        let valid = if idx == 0 {
            ch == '_' || ch.is_ascii_alphabetic()
        } else {
            ch == '_' || ch.is_ascii_alphanumeric()
        };

        if valid {
            out.push(ch);
        } else {
            out.push('_');
        }
    }

    if out.is_empty() { "_".to_string() } else { out }
}

fn apply_field_casing(name: &str, casing: FieldCasing) -> String {
    match casing {
        FieldCasing::Snake => name.to_snake_case(),
        FieldCasing::Camel => name.to_lower_camel_case(),
        FieldCasing::Preserve => sanitize_rust_identifier(name),
    }
}

struct SchemaMaps<'a> {
    table_columns: HashMap<String, Vec<&'a Column>>,
    table_pks: HashMap<String, HashSet<String>>,
    single_unique_columns: HashMap<String, HashSet<String>>,
    table_uniques: HashMap<String, Vec<&'a UniqueConstraint>>,
    table_checks: HashMap<String, Vec<&'a CheckConstraint>>,
    /// Single-column FKs, attached to their column as `references = ...`
    fk_map: HashMap<(String, String), (&'a ForeignKey, usize)>,
    /// Composite FKs, emitted as table-level `foreign_key(...)` attributes
    composite_fks: HashMap<String, Vec<&'a ForeignKey>>,
}

fn build_schema_maps(ddl: &SQLiteDDL) -> SchemaMaps<'_> {
    let mut table_columns: HashMap<String, Vec<&Column>> = HashMap::new();
    for column in ddl.columns.list() {
        table_columns
            .entry(column.table.to_string())
            .or_default()
            .push(column);
    }

    let mut table_pks: HashMap<String, HashSet<String>> = HashMap::new();
    for pk in ddl.pks.list() {
        for col in pk.columns.iter() {
            table_pks
                .entry(pk.table.to_string())
                .or_default()
                .insert(col.to_string());
        }
    }

    let mut single_unique_columns: HashMap<String, HashSet<String>> = HashMap::new();
    let mut table_uniques: HashMap<String, Vec<&UniqueConstraint>> = HashMap::new();
    for unique in ddl.uniques.list() {
        table_uniques
            .entry(unique.table.to_string())
            .or_default()
            .push(unique);
        if unique.columns.len() == 1 && !unique.name_explicit {
            single_unique_columns
                .entry(unique.table.to_string())
                .or_default()
                .insert(unique.columns[0].to_string());
        }
    }

    let mut table_checks: HashMap<String, Vec<&CheckConstraint>> = HashMap::new();
    for check in ddl.checks.list() {
        table_checks
            .entry(check.table.to_string())
            .or_default()
            .push(check);
    }

    let mut fk_map: HashMap<(String, String), (&ForeignKey, usize)> = HashMap::new();
    let mut composite_fks: HashMap<String, Vec<&ForeignKey>> = HashMap::new();
    for fk in ddl.fks.list() {
        if fk.columns.len() == 1 {
            fk_map.insert((fk.table.to_string(), fk.columns[0].to_string()), (fk, 0));
        } else {
            // A composite FK is one constraint — mapping it onto N per-column
            // `references = ...` attrs would generate N single-column FKs.
            composite_fks
                .entry(fk.table.to_string())
                .or_default()
                .push(fk);
        }
    }

    SchemaMaps {
        table_columns,
        table_pks,
        single_unique_columns,
        table_uniques,
        table_checks,
        fk_map,
        composite_fks,
    }
}

fn write_module_header(code: &mut String, options: &CodegenOptions) {
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
    code.push_str("use drizzle::sqlite::prelude::*;\n\n");
}

/// Generate Rust schema code from DDL
#[must_use]
pub fn generate_rust_schema(ddl: &SQLiteDDL, options: &CodegenOptions) -> GeneratedSchema {
    let mut result = GeneratedSchema::default();
    let mut code = String::new();

    write_module_header(&mut code, options);

    let SchemaMaps {
        table_columns,
        table_pks,
        single_unique_columns,
        table_uniques,
        table_checks,
        fk_map,
        composite_fks,
    } = build_schema_maps(ddl);

    // Generate table structs
    for table in ddl.tables.list() {
        let table_name = table.name.to_string();
        let columns = table_columns
            .get(&table_name)
            .map_or(&[][..], std::vec::Vec::as_slice);

        // Preserve DB/introspection order when available (cid -> ordinal_position).
        let mut columns_sorted: Vec<&Column> = columns.to_vec();
        columns_sorted.sort_by(|a, b| {
            let ao = a.ordinal_position.unwrap_or(i32::MAX);
            let bo = b.ordinal_position.unwrap_or(i32::MAX);
            ao.cmp(&bo).then_with(|| a.name.cmp(&b.name))
        });
        let pk_columns = table_pks.get(&table_name);
        let unique_columns = single_unique_columns.get(&table_name);
        let unique_constraints = table_uniques
            .get(&table_name)
            .map_or(&[][..], std::vec::Vec::as_slice);
        let check_constraints = table_checks
            .get(&table_name)
            .map_or(&[][..], std::vec::Vec::as_slice);
        let is_composite_pk = pk_columns.is_some_and(|pks| pks.len() > 1);

        let ctx = TableGenContext {
            table,
            columns: &columns_sorted,
            pk_columns,
            unique_columns,
            unique_constraints,
            check_constraints,
            is_composite_pk,
            fk_map: &fk_map,
            composite_fks: composite_fks
                .get(&table_name)
                .map_or(&[][..], std::vec::Vec::as_slice),
            use_pub: options.use_pub,
            field_casing: options.field_casing,
        };

        let table_code = generate_table_struct(&ctx);

        code.push_str(&table_code);
        code.push('\n');
        result.tables.push(table_name);
    }

    // Generate index structs
    for index in ddl.indexes.list() {
        if index.columns.iter().any(|c| c.is_expression) {
            // #[SQLiteIndex] tuple structs only accept `Table::column` paths;
            // expression columns cannot be expressed. Emit a TODO comment so
            // the user can recreate the index manually.
            // TODO: support expression columns once the index macro can
            // represent them.
            let expressions: Vec<&str> = index
                .columns
                .iter()
                .filter(|c| c.is_expression)
                .map(|c| c.value.as_ref())
                .collect();
            let _ = writeln!(
                code,
                "// TODO: index `{}` on `{}` uses expression column(s) ({}) which\n// #[SQLiteIndex] cannot express yet; recreate it manually.\n",
                index.name,
                index.table,
                expressions.join(", ")
            );
            result.warnings.push(format!(
                "index `{}` uses expression columns and was emitted as a TODO comment",
                index.name
            ));
            continue;
        }
        let index_code = generate_index_struct(index, options.use_pub, options.field_casing);
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
        let columns = table_columns
            .get(&view_name)
            .map_or(&[][..], std::vec::Vec::as_slice);
        let view_code = generate_view_struct(view, columns, options.use_pub, options.field_casing);
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
            options.field_casing,
        );
        code.push_str(&schema_code);
    }

    result.code = code;
    result
}

/// Generate a single table struct
struct TableGenContext<'a> {
    table: &'a Table,
    columns: &'a [&'a Column],
    pk_columns: Option<&'a HashSet<String>>,
    unique_columns: Option<&'a HashSet<String>>,
    unique_constraints: &'a [&'a UniqueConstraint],
    check_constraints: &'a [&'a CheckConstraint],
    is_composite_pk: bool,
    fk_map: &'a HashMap<(String, String), (&'a ForeignKey, usize)>,
    composite_fks: &'a [&'a ForeignKey],
    use_pub: bool,
    field_casing: FieldCasing,
}

/// Generate a single table struct
fn generate_table_struct(ctx: &TableGenContext<'_>) -> String {
    let mut code = String::new();
    let vis = if ctx.use_pub { "pub " } else { "" };

    // Struct name is PascalCase of table name
    let struct_name = ctx.table.name.to_pascal_case();

    // Check if table name differs from struct name
    let needs_name_attr = apply_field_casing(&struct_name, ctx.field_casing) != ctx.table.name;

    // Build table attribute options
    let mut table_attrs = Vec::new();
    if needs_name_attr {
        table_attrs.push(format!("name = \"{}\"", ctx.table.name));
    }
    if ctx.table.strict {
        table_attrs.push("strict".to_string());
    }
    if ctx.table.without_rowid {
        table_attrs.push("without_rowid".to_string());
    }
    for unique in ctx.unique_constraints {
        if should_emit_table_unique(unique) {
            table_attrs.push(format_table_unique_attr(unique, ctx.field_casing));
        }
    }
    for (idx, check) in ctx.check_constraints.iter().enumerate() {
        if check_column_target(check, ctx).is_none() {
            table_attrs.push(format_table_check_attr(check, ctx, idx));
        }
    }
    for fk in ctx.composite_fks {
        table_attrs.push(format_composite_fk_attr(fk, ctx.field_casing));
    }

    // Table attribute
    if table_attrs.is_empty() {
        code.push_str("#[SQLiteTable]\n");
    } else {
        let _ = writeln!(code, "#[SQLiteTable({})]", table_attrs.join(", "));
    }

    // Struct definition
    let _ = writeln!(code, "{vis}struct {struct_name} {{");

    // Fields
    for column in ctx.columns {
        let field_code = generate_column_field(column, ctx);
        code.push_str(&field_code);
    }

    code.push_str("}\n");
    code
}

fn should_emit_table_unique(unique: &UniqueConstraint) -> bool {
    unique.columns.len() > 1 || unique.name_explicit
}

fn default_unique_name(table: &str, columns: &[impl AsRef<str>]) -> String {
    format!(
        "{}_{}_unique",
        table,
        columns
            .iter()
            .map(AsRef::as_ref)
            .collect::<Vec<_>>()
            .join("_")
    )
}

fn format_table_unique_attr(unique: &UniqueConstraint, field_casing: FieldCasing) -> String {
    let columns: Vec<String> = unique
        .columns
        .iter()
        .map(|col| apply_field_casing(col.as_ref(), field_casing))
        .collect();
    let mut args = vec![format!("columns({})", columns.join(", "))];
    let default_name = default_unique_name(&unique.table, &unique.columns);
    if unique.name_explicit || unique.name != default_name {
        args.push(format!(
            "name = \"{}\"",
            escape_for_rust_literal(&unique.name)
        ));
    }
    format!("unique({})", args.join(", "))
}

/// Format a composite FK as a table-level attribute:
/// `foreign_key(columns(a, b), references(Parent, x, y), on_delete = "cascade")`
fn format_composite_fk_attr(fk: &ForeignKey, field_casing: FieldCasing) -> String {
    let columns: Vec<String> = fk
        .columns
        .iter()
        .map(|col| apply_field_casing(col.as_ref(), field_casing))
        .collect();
    let target_struct = fk.table_to.to_pascal_case();
    let target_columns: Vec<String> = fk
        .columns_to
        .iter()
        .map(|col| apply_field_casing(col.as_ref(), field_casing))
        .collect();

    let mut args = vec![
        format!("columns({})", columns.join(", ")),
        format!(
            "references({}, {})",
            target_struct,
            target_columns.join(", ")
        ),
    ];
    if let Some(on_delete) = &fk.on_delete
        && !on_delete.eq_ignore_ascii_case("NO ACTION")
    {
        args.push(format!("on_delete = \"{}\"", on_delete.to_lowercase()));
    }
    if let Some(on_update) = &fk.on_update
        && !on_update.eq_ignore_ascii_case("NO ACTION")
    {
        args.push(format!("on_update = \"{}\"", on_update.to_lowercase()));
    }
    format!("foreign_key({})", args.join(", "))
}

fn format_table_check_attr(
    check: &CheckConstraint,
    _ctx: &TableGenContext<'_>,
    _idx: usize,
) -> String {
    let mut args = Vec::new();
    args.push(format!(
        "name = \"{}\"",
        escape_for_rust_literal(&check.name)
    ));
    args.push(format!(
        "expr = \"{}\"",
        escape_for_rust_literal(&check.value)
    ));
    format!("check({})", args.join(", "))
}

fn check_column_target(check: &CheckConstraint, ctx: &TableGenContext<'_>) -> Option<String> {
    let referenced = expression_referenced_columns(&check.value, ctx.columns);
    if referenced.len() != 1 {
        return None;
    }
    let column = referenced.into_iter().next()?;
    if check.name == format!("{}_{}_check", ctx.table.name, column) {
        Some(column)
    } else {
        None
    }
}

fn expression_referenced_columns(expr: &str, columns: &[&Column]) -> Vec<String> {
    columns
        .iter()
        .filter_map(|column| {
            let name = column.name.as_ref();
            if expression_references_identifier(expr, name) {
                Some(name.to_string())
            } else {
                None
            }
        })
        .collect()
}

fn expression_references_identifier(expr: &str, ident: &str) -> bool {
    let expr_lower = expr.to_ascii_lowercase();
    let ident_lower = ident.to_ascii_lowercase();
    if expr_lower.contains(&format!("`{ident_lower}`"))
        || expr_lower.contains(&format!("\"{ident_lower}\""))
    {
        return true;
    }

    let mut offset = 0;
    while let Some(pos) = expr_lower[offset..].find(&ident_lower) {
        let start = offset + pos;
        let end = start + ident_lower.len();
        let before = expr_lower[..start].chars().next_back();
        let after = expr_lower[end..].chars().next();
        let before_boundary = before.is_none_or(|c| !(c == '_' || c.is_ascii_alphanumeric()));
        let after_boundary = after.is_none_or(|c| !(c == '_' || c.is_ascii_alphanumeric()));
        if before_boundary && after_boundary {
            return true;
        }
        offset = end;
    }
    false
}

fn column_check_for<'a>(column: &Column, ctx: &TableGenContext<'a>) -> Option<&'a CheckConstraint> {
    ctx.check_constraints
        .iter()
        .copied()
        .find(|check| check_column_target(check, ctx).as_deref() == Some(column.name.as_ref()))
}

/// Generate a single column as a struct field
fn generate_column_field(column: &Column, ctx: &TableGenContext<'_>) -> String {
    let vis = if ctx.use_pub { "pub " } else { "" };

    // Determine column attributes
    let mut attrs = Vec::new();
    let column_name = column.name.to_string();

    // Check if primary key
    let is_pk = ctx.pk_columns.is_some_and(|pks| pks.contains(&column_name));

    // Only add primary if it's a single-column PK (not composite)
    if is_pk && !ctx.is_composite_pk {
        attrs.push("primary".to_string());
    }

    // Check autoincrement
    if column.autoincrement == Some(true) {
        attrs.push("autoincrement".to_string());
    }

    // Check unique (only for single-column constraints)
    let is_unique = ctx
        .unique_columns
        .is_some_and(|uniques| uniques.contains(&column_name));
    if is_unique {
        attrs.push("unique".to_string());
    }

    if let Some(generated) = &column.generated {
        use super::ddl::GeneratedType;
        let gen_type = match generated.gen_type {
            GeneratedType::Stored => "stored",
            GeneratedType::Virtual => "virtual",
        };
        attrs.push(format!(
            "generated({gen_type}, \"{}\")",
            escape_for_rust_literal(&generated.expression)
        ));
    }

    // Check default value
    if let Some(default) = &column.default
        && column.generated.is_none()
    {
        if let Some(d) = format_default_value(default, &column.sql_type) {
            attrs.push(format!("default = {d}"));
        } else if !default.trim().eq_ignore_ascii_case("null") {
            attrs.push(format!(
                "default_sql = \"{}\"",
                escape_for_rust_literal(default)
            ));
        }
    }

    if let Some(collate) = &column.collate {
        attrs.push(format!(
            "collate = \"{}\"",
            escape_for_rust_literal(collate)
        ));
    }

    if let Some(check) = column_check_for(column, ctx) {
        attrs.push(format!(
            "check = \"{}\"",
            escape_for_rust_literal(&check.value)
        ));
    }

    // Check foreign key
    if let Some((fk, idx)) = ctx.fk_map.get(&(column.table.to_string(), column_name))
        && let Some(ref_col) = fk.columns_to.get(*idx)
    {
        let ref_table_struct = fk.table_to.to_pascal_case();
        attrs.push(format!("references = {ref_table_struct}::{ref_col}"));

        // Add ON DELETE if specified
        if let Some(on_delete) = &fk.on_delete
            && !on_delete.eq_ignore_ascii_case("NO ACTION")
        {
            let action = on_delete.replace(' ', "_").to_lowercase();
            attrs.push(format!("on_delete = {action}"));
        }

        // Add ON UPDATE if specified
        if let Some(on_update) = &fk.on_update
            && !on_update.eq_ignore_ascii_case("NO ACTION")
        {
            let action = on_update.replace(' ', "_").to_lowercase();
            attrs.push(format!("on_update = {action}"));
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
    let field_name = apply_field_casing(column.name.as_ref(), ctx.field_casing);

    format!("{attr_str}    {vis}{field_name}: {rust_type},\n")
}

/// Strip exactly one layer of matching SQL quotes and un-double the escaped
/// quote characters (`'it''s'` → `it's`). Returns `None` when not quoted.
fn unquote_sql_string(default: &str) -> Option<String> {
    let bytes = default.as_bytes();
    if bytes.len() >= 2 && bytes[0] == b'\'' && bytes[bytes.len() - 1] == b'\'' {
        Some(default[1..default.len() - 1].replace("''", "'"))
    } else if bytes.len() >= 2 && bytes[0] == b'"' && bytes[bytes.len() - 1] == b'"' {
        Some(default[1..default.len() - 1].replace("\"\"", "\""))
    } else {
        None
    }
}

/// Format a default value for Rust syntax
fn format_default_value(default: &str, sql_type: &str) -> Option<String> {
    let default = default.trim();
    let category = SQLTypeCategory::from_sql_type(sql_type);

    if default.eq_ignore_ascii_case("null") {
        return None;
    }

    let unquoted = unquote_sql_string(default);

    // Skip function calls or complex expressions; these use default_sql.
    // Quoted string literals are exempt — `'a(b)'` is a plain string default.
    if unquoted.is_none() && default.contains('(') && default.contains(')') {
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
            unquoted.map(|inner| format!("\"{}\"", escape_for_rust_literal(&inner)))
        }
        SQLTypeCategory::Numeric => default
            .parse::<i64>()
            .map(|v| v.to_string())
            .ok()
            .or_else(|| default.parse::<f64>().map(|v| v.to_string()).ok()),
    }
}

/// Convert SQL type to Rust type
fn sql_type_to_rust_type(sql_type: &str, not_null: bool) -> String {
    // Handle boolean specifically before the category match
    if sql_type.eq_ignore_ascii_case("boolean") {
        return if not_null {
            "bool".to_string()
        } else {
            "Option<bool>".to_string()
        };
    }

    let category = SQLTypeCategory::from_sql_type(sql_type);

    let base_type = match category {
        SQLTypeCategory::Integer | SQLTypeCategory::Numeric => "i64",
        SQLTypeCategory::Real => "f64",
        SQLTypeCategory::Text => "String",
        SQLTypeCategory::Blob => "Vec<u8>",
    };

    if not_null {
        base_type.to_string()
    } else {
        format!("Option<{base_type}>")
    }
}

/// Generate an index struct
fn generate_index_struct(index: &Index, use_pub: bool, field_casing: FieldCasing) -> String {
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
        .map(|s| {
            if let Some((table, col)) = s.split_once("::") {
                format!("{}::{}", table, apply_field_casing(col, field_casing))
            } else {
                s
            }
        })
        .collect();

    // Keep the generated attribute as the complete schema contract so an
    // introspected partial index survives parse -> snapshot -> diff unchanged.
    let mut attrs = Vec::new();
    if struct_name.to_snake_case() != index.name.as_ref() {
        attrs.push(format!(
            "name = \"{}\"",
            escape_for_rust_literal(&index.name)
        ));
    }
    if index.is_unique {
        attrs.push("unique".to_string());
    }
    if let Some(where_clause) = &index.where_clause {
        attrs.push(format!(
            "where = \"{}\"",
            escape_for_rust_literal(where_clause)
        ));
    }
    if attrs.is_empty() {
        code.push_str("#[SQLiteIndex]\n");
    } else {
        let _ = writeln!(code, "#[SQLiteIndex({})]", attrs.join(", "));
    }

    // Struct definition (tuple struct with column references)
    let _ = writeln!(
        code,
        "{}struct {}({});",
        vis,
        struct_name,
        columns.join(", ")
    );

    code
}

/// Generate a view struct
fn generate_view_struct(
    view: &View,
    columns: &[&Column],
    use_pub: bool,
    field_casing: FieldCasing,
) -> String {
    let struct_name = view.name.to_pascal_case();
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Build view attributes
    let mut attrs = Vec::new();

    // Check if view name differs from struct name (snake_case version)
    if apply_field_casing(&struct_name, field_casing) != view.name.as_ref() {
        attrs.push(format!("name = \"{}\"", view.name));
    }

    // Add definition
    if let Some(def) = &view.definition {
        let escaped_def = escape_for_rust_literal(def);
        attrs.push(format!("definition = \"{escaped_def}\""));
    }

    // Build the attribute line
    if attrs.is_empty() {
        code.push_str("#[SQLiteView]\n");
    } else {
        let _ = writeln!(code, "#[SQLiteView({})]", attrs.join(", "));
    }

    // Struct definition with column fields
    let _ = writeln!(code, "{vis}struct {struct_name} {{");

    // Sort columns by ordinal position
    let mut sorted_columns: Vec<&&Column> = columns.iter().collect();
    sorted_columns.sort_by(|a, b| {
        let ao = a.ordinal_position.unwrap_or(i32::MAX);
        let bo = b.ordinal_position.unwrap_or(i32::MAX);
        ao.cmp(&bo).then_with(|| a.name.cmp(&b.name))
    });

    // Generate fields for each column
    for column in sorted_columns {
        let field_name = apply_field_casing(column.name.as_ref(), field_casing);
        let rust_type = sql_type_to_rust_type(&column.sql_type, column.not_null);
        let _ = writeln!(code, "    {vis}{field_name}: {rust_type},");
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
    field_casing: FieldCasing,
) -> String {
    let mut code = String::new();
    let vis = if use_pub { "pub " } else { "" };

    code.push_str("#[derive(SQLiteSchema)]\n");
    let _ = writeln!(code, "{vis}struct {schema_name} {{");

    // Add tables
    for table in tables {
        let field_name = apply_field_casing(table, field_casing);
        let type_name = table.to_pascal_case();
        let _ = writeln!(code, "    {vis}{field_name}: {type_name},");
    }

    // Add indexes
    for index in indexes {
        let field_name = apply_field_casing(index, field_casing);
        let type_name = index.to_pascal_case();
        let _ = writeln!(code, "    {vis}{field_name}: {type_name},");
    }

    code.push_str("}\n");
    code
}

#[cfg(test)]
mod tests {
    use super::super::ddl::*;
    use super::*;
    use crate::parser::SchemaParser;
    use crate::schema::Snapshot;
    use crate::sqlite::introspect::{RawColumnInfo, RawIntrospection, assemble_ddl};
    use drizzle_types::Dialect;

    #[test]
    fn index_name_that_does_not_round_trip_through_rust_is_explicit() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("users"));
        ddl.columns.push(Column::new("users", "email", "text"));
        ddl.indexes.push(Index::new(
            "users",
            "users_email_42",
            vec![IndexColumn::new("email")],
        ));

        let generated = generate_rust_schema(&ddl, &CodegenOptions::default());

        assert!(
            generated
                .code
                .contains("#[SQLiteIndex(name = \"users_email_42\")]"),
            "generated schema must preserve the SQL index name:\n{}",
            generated.code
        );

        let parsed = SchemaParser::parse(&generated.code);
        assert!(
            parsed.errors.is_empty(),
            "generated source:\n{}\nerrors: {:#?}",
            generated.code,
            parsed.errors
        );
        let Snapshot::Sqlite(snapshot) =
            Snapshot::from_parse_result(&parsed, Dialect::SQLite, None)
        else {
            panic!("expected generated SQLite schema snapshot");
        };
        let reparsed = SQLiteDDL::from_entities(snapshot.ddl);
        let migration = crate::sqlite::diff::compute_migration(&ddl, &reparsed);
        assert!(
            migration.sql_statements.is_empty(),
            "generated SQLite schema changed the index: {:#?}",
            migration.sql_statements
        );
    }

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

        assert_eq!(
            result.code,
            "\
//! Auto-generated SQLite schema from introspection
//!

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
pub struct Users {
    pub email: Option<String>,
    #[column(primary, autoincrement)]
    pub id: i64,
    pub name: String,
}

"
        );
        assert_eq!(result.tables, vec!["users"]);
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

        assert_eq!(
            result.code,
            "\
//! Auto-generated SQLite schema from introspection
//!

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Accounts {
    #[column(unique)]
    email: String,
    id: i64,
}

"
        );
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

        assert_eq!(
            result.code,
            "\
//! Auto-generated SQLite schema from introspection
//!

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Posts {
    #[column(references = Users::id)]
    author_id: i64,
    id: i64,
}

"
        );
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

        assert_eq!(
            result.code,
            "\
//! Auto-generated SQLite schema from introspection
//!

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    email: String,
}

#[SQLiteIndex(unique)]
struct UsersEmailIdx(Users::email);

"
        );
    }

    #[test]
    fn test_generate_partial_index() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("jobs"));
        ddl.columns.push(Column::new("jobs", "builder", "text"));
        let mut index = Index::new(
            "jobs",
            "jobs_unclaimed_idx",
            vec![IndexColumn::new("builder")],
        );
        index.is_unique = true;
        index.where_clause = Some("builder IS NULL".into());
        ddl.indexes.push(index);

        let generated = generate_rust_schema(&ddl, &CodegenOptions::default());
        assert!(
            generated
                .code
                .contains("#[SQLiteIndex(unique, where = \"builder IS NULL\")]")
        );
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

        assert_eq!(
            result.code,
            "\
//! Auto-generated SQLite schema from introspection
//!

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
pub struct Users {
}

#[SQLiteTable]
pub struct Posts {
}

#[derive(SQLiteSchema)]
pub struct AppSchema {
    pub users: Users,
    pub posts: Posts,
}
"
        );
    }

    #[test]
    fn test_format_default_value_unquotes_one_layer() {
        // Exactly one quote layer stripped, doubled quotes un-escaped.
        assert_eq!(
            format_default_value("'it''s'", "text"),
            Some("\"it's\"".to_string())
        );
        // Quoted strings containing parens are still plain string defaults.
        assert_eq!(
            format_default_value("'a(b)'", "text"),
            Some("\"a(b)\"".to_string())
        );
        // Unquoted expressions with parens fall through to default_sql.
        assert_eq!(format_default_value("abs(-1)", "text"), None);
        // Double-quoted works too.
        assert_eq!(
            format_default_value("\"he said \"\"hi\"\"\"", "text"),
            Some("\"he said \\\"hi\\\"\"".to_string())
        );
    }

    #[test]
    fn test_composite_fk_emitted_as_table_level_attr() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("child"));
        ddl.columns
            .push(Column::new("child", "tenant_id", "integer").not_null());
        ddl.columns
            .push(Column::new("child", "parent_id", "integer").not_null());
        let fk = ForeignKey::from_strings(
            "child".to_string(),
            "fk_child_parent".to_string(),
            vec!["tenant_id".to_string(), "parent_id".to_string()],
            "parent".to_string(),
            vec!["tenant_id".to_string(), "id".to_string()],
        )
        .on_delete("CASCADE");
        ddl.fks.push(fk);

        let result = generate_rust_schema(&ddl, &CodegenOptions::default());

        assert!(
            result.code.contains(
                "#[SQLiteTable(foreign_key(columns(tenant_id, parent_id), references(Parent, tenant_id, id), on_delete = \"cascade\"))]"
            ),
            "composite FK should be a table-level attribute, got:\n{}",
            result.code
        );
        assert!(
            !result.code.contains("references = Parent::"),
            "composite FK must not be split into per-column references:\n{}",
            result.code
        );
    }

    #[test]
    fn test_expression_index_emits_todo_not_invalid_rust() {
        let mut ddl = SQLiteDDL::new();
        ddl.tables.push(Table::new("users"));
        ddl.columns
            .push(Column::new("users", "email", "text").not_null());
        ddl.indexes.push(Index::new(
            "users",
            "users_email_lower_idx",
            vec![IndexColumn {
                value: "lower(email)".into(),
                is_expression: true,
            }],
        ));

        let result = generate_rust_schema(&ddl, &CodegenOptions::default());

        assert!(
            result
                .code
                .contains("// TODO: index `users_email_lower_idx`"),
            "expected TODO comment for expression index, got:\n{}",
            result.code
        );
        assert!(
            !result.code.contains("Users::lower(email)"),
            "must not emit invalid Rust for expression columns:\n{}",
            result.code
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("users_email_lower_idx")),
            "expected a warning for the skipped index"
        );
    }

    #[test]
    fn test_sql_type_to_rust_type() {
        assert_eq!(sql_type_to_rust_type("integer", true), "i64");
        assert_eq!(sql_type_to_rust_type("integer", false), "Option<i64>");
        assert_eq!(sql_type_to_rust_type("text", true), "String");
        assert_eq!(sql_type_to_rust_type("text", false), "Option<String>");
        assert_eq!(sql_type_to_rust_type("real", true), "f64");
        assert_eq!(sql_type_to_rust_type("blob", true), "Vec<u8>");
        assert_eq!(sql_type_to_rust_type("boolean", true), "bool");
        assert_eq!(sql_type_to_rust_type("boolean", false), "Option<bool>");
        assert_eq!(sql_type_to_rust_type("BOOLEAN", true), "bool");
    }

    #[test]
    fn generated_schema_round_trips_sqlite_affinity_and_integer_primary_key() {
        let ddl = assemble_ddl(RawIntrospection {
            tables: vec![("audit_logs".to_string(), None)],
            columns: vec![
                RawColumnInfo {
                    table: "audit_logs".to_string(),
                    cid: 0,
                    name: "id".to_string(),
                    column_type: "INTEGER".to_string(),
                    // SQLite PRAGMA reports 0 for an inline INTEGER PRIMARY KEY.
                    not_null: false,
                    default_value: None,
                    pk: 1,
                    hidden: 0,
                    sql: None,
                },
                RawColumnInfo {
                    table: "audit_logs".to_string(),
                    cid: 1,
                    name: "user_name".to_string(),
                    column_type: "VARCHAR(255)".to_string(),
                    not_null: true,
                    default_value: None,
                    pk: 0,
                    hidden: 0,
                    sql: None,
                },
            ],
            ..RawIntrospection::default()
        });

        let generated = generate_rust_schema(
            &ddl,
            &CodegenOptions {
                field_casing: FieldCasing::Camel,
                include_schema: true,
                schema_name: "Schema".to_string(),
                use_pub: true,
                ..CodegenOptions::default()
            },
        );
        assert!(generated.code.contains("pub userName: String"));

        let parsed = SchemaParser::parse(&generated.code);
        let Snapshot::Sqlite(snapshot) =
            Snapshot::from_parse_result(&parsed, Dialect::SQLite, None)
        else {
            panic!("expected generated SQLite schema snapshot");
        };
        let regenerated = SQLiteDDL::from_entities(snapshot.ddl);

        let diffs = crate::sqlite::collection::diff_ddl(&ddl, &regenerated);
        assert!(
            diffs.is_empty(),
            "generated SQLite schema must round trip without a migration: {diffs:#?}"
        );
    }
}
