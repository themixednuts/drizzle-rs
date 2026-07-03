//! `PostgreSQL` schema code generation
//!
//! This module generates Rust source code from introspected DDL entities.
//! The generated code uses the lowercase attribute syntax (e.g., `primary` instead of `PRIMARY`)
//! that is the current recommended style.

use super::collection::PostgresDDL;
use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, Policy, Table, UniqueConstraint, View,
};
use crate::utils::escape_for_rust_literal;
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

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
    /// Policies that were generated
    pub policies: Vec<String>,
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

/// Lookup tables derived from a [`PostgresDDL`] and shared across every
/// per-entity generation pass.
struct SchemaMaps<'a> {
    enum_map: HashMap<(String, String), String>,
    table_columns: HashMap<(String, String), Vec<&'a Column>>,
    table_pks: HashMap<(String, String), HashSet<String>>,
    single_unique_columns: HashMap<(String, String), HashSet<String>>,
    table_uniques: HashMap<(String, String), Vec<&'a UniqueConstraint>>,
    table_checks: HashMap<(String, String), Vec<&'a CheckConstraint>>,
    fk_map: HashMap<(String, String, String), (&'a ForeignKey, usize)>,
}

fn build_schema_maps(ddl: &PostgresDDL) -> SchemaMaps<'_> {
    let mut enum_map: HashMap<(String, String), String> = HashMap::new();
    for e in ddl.enums.list() {
        let type_name = e.name.to_pascal_case();
        enum_map.insert((e.schema.to_string(), e.name.to_string()), type_name);
    }

    let mut table_columns: HashMap<(String, String), Vec<&Column>> = HashMap::new();
    for column in ddl.columns.list() {
        table_columns
            .entry((column.schema.to_string(), column.table.to_string()))
            .or_default()
            .push(column);
    }

    let mut table_pks: HashMap<(String, String), HashSet<String>> = HashMap::new();
    for pk in ddl.pks.list() {
        for col in pk.columns.iter() {
            table_pks
                .entry((pk.schema.to_string(), pk.table.to_string()))
                .or_default()
                .insert(col.to_string());
        }
    }

    let mut single_unique_columns: HashMap<(String, String), HashSet<String>> = HashMap::new();
    let mut table_uniques: HashMap<(String, String), Vec<&UniqueConstraint>> = HashMap::new();
    for unique in ddl.uniques.list() {
        let key = (unique.schema.to_string(), unique.table.to_string());
        table_uniques.entry(key.clone()).or_default().push(unique);
        if unique.columns.len() == 1
            && !unique.name_explicit
            && !unique.deferrable
            && !unique.initially_deferred
            && !unique.nulls_not_distinct
        {
            single_unique_columns
                .entry((unique.schema.to_string(), unique.table.to_string()))
                .or_default()
                .insert(unique.columns[0].to_string());
        }
    }

    let mut table_checks: HashMap<(String, String), Vec<&CheckConstraint>> = HashMap::new();
    for check in ddl.checks.list() {
        table_checks
            .entry((check.schema.to_string(), check.table.to_string()))
            .or_default()
            .push(check);
    }

    let mut fk_map: HashMap<(String, String, String), (&ForeignKey, usize)> = HashMap::new();
    for fk in ddl.fks.list() {
        for (idx, col) in fk.columns.iter().enumerate() {
            fk_map.insert(
                (fk.schema.to_string(), fk.table.to_string(), col.to_string()),
                (fk, idx),
            );
        }
    }

    SchemaMaps {
        enum_map,
        table_columns,
        table_pks,
        single_unique_columns,
        table_uniques,
        table_checks,
        fk_map,
    }
}

fn write_module_header(code: &mut String, options: &CodegenOptions) {
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
    code.push_str("use drizzle::postgres::prelude::*;\n\n");
}

/// Generate Rust schema code from DDL
#[must_use]
pub fn generate_rust_schema(ddl: &PostgresDDL, options: &CodegenOptions) -> GeneratedSchema {
    let mut result = GeneratedSchema::default();
    let mut code = String::new();

    write_module_header(&mut code, options);

    let maps = build_schema_maps(ddl);

    // Generate enum definitions
    for e in ddl.enums.list() {
        code.push_str(&generate_enum_struct(e, options.use_pub));
        code.push('\n');
        result.enums.push(e.name.to_string());
    }

    // Generate table structs
    for table in ddl.tables.list() {
        let key = (table.schema.to_string(), table.name.to_string());
        let columns = maps
            .table_columns
            .get(&key)
            .map_or(&[][..], std::vec::Vec::as_slice);
        let pk_columns = maps.table_pks.get(&key);
        let unique_columns = maps.single_unique_columns.get(&key);
        let unique_constraints = maps
            .table_uniques
            .get(&key)
            .map_or(&[][..], std::vec::Vec::as_slice);
        let check_constraints = maps
            .table_checks
            .get(&key)
            .map_or(&[][..], std::vec::Vec::as_slice);
        let is_composite_pk = pk_columns.is_some_and(|pks| pks.len() > 1);

        code.push_str(&generate_table_struct(&TableGenContext {
            table,
            columns,
            pk_columns,
            unique_columns,
            unique_constraints,
            check_constraints,
            is_composite_pk,
            fk_map: &maps.fk_map,
            enum_map: &maps.enum_map,
            use_pub: options.use_pub,
            field_casing: options.field_casing,
        }));
        code.push('\n');
        result.tables.push(table.name.to_string());
    }

    // Generate index structs
    for index in ddl.indexes.list() {
        code.push_str(&generate_index_struct(
            index,
            options.use_pub,
            options.field_casing,
        ));
        code.push('\n');
        result.indexes.push(index.name.to_string());
    }

    // Generate view structs
    for view in ddl.views.list() {
        if view.is_existing {
            continue;
        }
        let key = (view.schema.to_string(), view.name.to_string());
        let columns = maps
            .table_columns
            .get(&key)
            .map_or(&[][..], std::vec::Vec::as_slice);
        code.push_str(&generate_view_struct(
            view,
            columns,
            &maps.enum_map,
            options.use_pub,
            options.field_casing,
        ));
        code.push('\n');
        result.views.push(view.name.to_string());
    }

    for policy in ddl.policies.list() {
        code.push_str(&generate_policy_struct(policy, options.use_pub));
        code.push('\n');
        result.policies.push(policy.name.to_string());
    }

    if options.include_schema {
        code.push_str(&generate_schema_struct(
            &options.schema_name,
            &result.tables,
            &result.indexes,
            &result.policies,
            options.use_pub,
            options.field_casing,
        ));
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
    unique_constraints: &'a [&'a UniqueConstraint],
    check_constraints: &'a [&'a CheckConstraint],
    is_composite_pk: bool,
    fk_map: &'a HashMap<(String, String, String), (&'a ForeignKey, usize)>,
    enum_map: &'a HashMap<(String, String), String>,
    use_pub: bool,
    field_casing: FieldCasing,
}

/// Generate a single table struct
fn generate_table_struct(ctx: &TableGenContext<'_>) -> String {
    let struct_name = ctx.table.name.to_pascal_case();
    let vis = if ctx.use_pub { "pub " } else { "" };

    let mut code = String::new();

    if let Some(comment) = ctx.table.comment.as_deref() {
        write_doc_comment(&mut code, "", comment);
    }

    // Table attribute
    let table_attrs = format_table_attrs(ctx);
    if table_attrs.is_empty() {
        code.push_str("#[PostgresTable]\n");
    } else {
        let _ = writeln!(code, "#[PostgresTable({})]", table_attrs.join(", "));
    }

    // Struct definition
    let _ = writeln!(code, "{vis}struct {struct_name} {{");

    // Sort columns by ordinal position if available, falling back to name.
    let mut sorted_columns: Vec<&&Column> = ctx.columns.iter().collect();
    sorted_columns.sort_by(|a, b| {
        let ao = a.ordinal_position.unwrap_or(i32::MAX);
        let bo = b.ordinal_position.unwrap_or(i32::MAX);
        ao.cmp(&bo).then_with(|| a.name.cmp(&b.name))
    });

    // Generate fields
    for column in sorted_columns {
        let field_code = generate_column_field(column, ctx);
        code.push_str(&field_code);
    }

    code.push_str("}\n");
    code
}

fn format_table_attrs(ctx: &TableGenContext<'_>) -> Vec<String> {
    let table = ctx.table;
    let mut attrs = Vec::new();
    if table.schema != "public" {
        attrs.push(format!(
            "schema = \"{}\"",
            escape_for_rust_literal(&table.schema)
        ));
    }
    if table.is_unlogged == Some(true) {
        attrs.push("unlogged".to_string());
    }
    if table.is_temporary == Some(true) {
        attrs.push("temporary".to_string());
    }
    if let Some(inherits) = &table.inherits {
        attrs.push(format!(
            "inherits = \"{}\"",
            escape_for_rust_literal(inherits)
        ));
    }
    if let Some(tablespace) = &table.tablespace {
        attrs.push(format!(
            "tablespace = \"{}\"",
            escape_for_rust_literal(tablespace)
        ));
    }
    if table.is_rls_enabled == Some(true) {
        attrs.push("rls".to_string());
    }
    for unique in ctx.unique_constraints {
        if should_emit_table_unique(unique) {
            attrs.push(format_table_unique_attr(unique, ctx.field_casing));
        }
    }
    for (idx, check) in ctx.check_constraints.iter().enumerate() {
        if check_column_target(check, ctx).is_none() {
            attrs.push(format_table_check_attr(check, ctx, idx));
        }
    }
    attrs
}

fn should_emit_table_unique(unique: &UniqueConstraint) -> bool {
    unique.columns.len() > 1
        || unique.name_explicit
        || unique.deferrable
        || unique.initially_deferred
        || unique.nulls_not_distinct
}

fn default_unique_name(table: &str, columns: &[impl AsRef<str>]) -> String {
    format!(
        "{}_{}_key",
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
    if unique.deferrable {
        args.push("deferrable".to_string());
    }
    if unique.initially_deferred {
        args.push("initially_deferred".to_string());
    }
    format!("unique({})", args.join(", "))
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
    if expr_lower.contains(&format!("\"{ident_lower}\"")) {
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

/// Format a column's IDENTITY metadata as a `#[column(identity(...))]` fragment.
fn format_identity_attr(identity: &super::ddl::Identity) -> String {
    use super::ddl::IdentityType;
    let identity_type = match identity.type_ {
        IdentityType::Always => "always",
        IdentityType::ByDefault => "by_default",
    };

    let mut seq_opts: Vec<String> = Vec::new();
    if let Some(increment) = &identity.increment
        && increment != "1"
    {
        seq_opts.push(format!("increment = {increment}"));
    }
    if let Some(start) = &identity.start_with
        && start != "1"
    {
        seq_opts.push(format!("start = {start}"));
    }
    if let Some(min) = &identity.min_value {
        seq_opts.push(format!("min_value = {min}"));
    }
    if let Some(max) = &identity.max_value {
        seq_opts.push(format!("max_value = {max}"));
    }
    if let Some(cache) = &identity.cache
        && *cache != 1
    {
        seq_opts.push(format!("cache = {cache}"));
    }
    if identity.cycle == Some(true) {
        seq_opts.push("cycle".to_string());
    }

    if seq_opts.is_empty() {
        format!("identity({identity_type})")
    } else {
        format!("identity({identity_type}, {})", seq_opts.join(", "))
    }
}

/// Push FK-related attributes (`references`, `on_delete`, `on_update`) for a
/// column onto the accumulator.
fn push_fk_attrs(attrs: &mut Vec<String>, fk: &ForeignKey, idx: usize) {
    let ref_table = fk.table_to.to_pascal_case();
    let ref_column = fk.columns_to.get(idx).cloned().unwrap_or_default();
    attrs.push(format!("references = {ref_table}::{ref_column}"));

    if let Some(on_delete) = &fk.on_delete
        && on_delete != "NO ACTION"
    {
        let action = on_delete.to_lowercase().replace(' ', "_");
        attrs.push(format!("on_delete = {action}"));
    }

    if let Some(on_update) = &fk.on_update
        && on_update != "NO ACTION"
    {
        let action = on_update.to_lowercase().replace(' ', "_");
        attrs.push(format!("on_update = {action}"));
    }

    if fk.deferrable {
        attrs.push("deferrable".to_string());
    }
    if fk.initially_deferred {
        attrs.push("initially_deferred".to_string());
    }
}

/// Generate a single column as a struct field
fn generate_column_field(column: &Column, ctx: &TableGenContext<'_>) -> String {
    let field_name = apply_field_casing(column.name.as_ref(), ctx.field_casing);
    let vis = if ctx.use_pub { "pub " } else { "" };

    let col_name_str = column.name.to_string();
    let is_pk = ctx
        .pk_columns
        .is_some_and(|pks| pks.contains(&col_name_str));
    let is_unique = ctx
        .unique_columns
        .is_some_and(|uqs| uqs.contains(&col_name_str));

    // For single-column PKs, add primary. For composite, skip (handled at table level)
    let should_add_primary = is_pk && !ctx.is_composite_pk;

    // Check for serial (nextval default without identity)
    let is_serial = column
        .default
        .as_ref()
        .is_some_and(|d| d.contains("nextval"))
        && column.identity.is_none();

    // Get FK info if present
    let fk_info = ctx.fk_map.get(&(
        column.schema.to_string(),
        column.table.to_string(),
        col_name_str,
    ));

    // Check if this column uses an enum type
    let type_schema = column.type_schema.as_deref().unwrap_or(&column.schema);
    let enum_type = ctx
        .enum_map
        .get(&(type_schema.to_string(), column.sql_type.to_string()));

    // Build column attributes
    let mut attrs = Vec::new();

    // For SERIAL columns (auto-increment via nextval), use "serial" attribute
    if is_serial {
        attrs.push("serial".to_string());
    }

    // For GENERATED IDENTITY columns, use identity(always) or identity(by_default)
    // with optional sequence options
    if let Some(identity) = &column.identity {
        attrs.push(format_identity_attr(identity));
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

    if let Some(collate) = &column.collate {
        attrs.push(format!(
            "collate = \"{}\"",
            escape_for_rust_literal(collate)
        ));
    }

    // Add generated column attribute for GENERATED AS columns
    if let Some(generated) = &column.generated {
        use super::ddl::GeneratedType;
        let gen_type = match generated.gen_type {
            GeneratedType::Stored => "stored",
            GeneratedType::Virtual => "virtual",
        };
        let expr = escape_for_rust_literal(&generated.expression);
        attrs.push(format!("generated({gen_type}, \"{expr}\")"));
    }

    // Add default if present (but skip nextval for serial columns)
    if let Some(default) = &column.default
        && !is_serial
        && column.generated.is_none()
    {
        if let Some(formatted) = format_default_value(default, &column.sql_type) {
            attrs.push(format!("default = {formatted}"));
        } else if !default.trim().eq_ignore_ascii_case("null") {
            attrs.push(format!(
                "default_sql = \"{}\"",
                escape_for_rust_literal(default)
            ));
        }
    }

    if let Some(check) = column_check_for(column, ctx) {
        attrs.push(format!(
            "check = \"{}\"",
            escape_for_rust_literal(&check.value)
        ));
    }

    // Add FK reference if present
    if let Some((fk, idx)) = fk_info {
        push_fk_attrs(&mut attrs, fk, *idx);
    }

    // Generate attribute line if there are any
    let mut result = String::new();
    if let Some(comment) = column.comment.as_deref() {
        write_doc_comment(&mut result, "    ", comment);
    }
    if !attrs.is_empty() {
        let _ = writeln!(result, "    #[column({})]", attrs.join(", "));
    }

    // Determine Rust type - use enum type if available, otherwise map SQL type
    let rust_type = enum_type.map_or_else(
        || {
            sql_type_to_rust_type_with_dimensions(
                &column.sql_type,
                column.dimensions,
                column.not_null,
            )
        },
        |enum_name| {
            if column.not_null {
                enum_name.clone()
            } else {
                format!("Option<{enum_name}>")
            }
        },
    );

    let _ = writeln!(result, "    {vis}{field_name}: {rust_type},");
    result
}

/// Generate a Rust enum definition from a `PostgreSQL` enum
fn generate_enum_struct(e: &Enum, use_pub: bool) -> String {
    let enum_name = e.name.to_pascal_case();
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Enum derive attribute with PostgresEnum - matches the project's actual usage
    // #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
    code.push_str("#[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]\n");

    // Enum definition
    let _ = writeln!(code, "{vis}enum {enum_name} {{");

    // Generate variants from enum values
    for (idx, value) in e.values.iter().enumerate() {
        let variant_name = value.to_pascal_case();
        // First variant gets #[default] attribute
        if idx == 0 {
            code.push_str("    #[default]\n");
        }
        let _ = writeln!(code, "    {variant_name},");
    }

    code.push_str("}\n");
    code
}

/// Format a default value for Rust syntax
fn format_default_value(default: &str, sql_type: &str) -> Option<String> {
    let default = default.trim();
    let sql_type_lower = sql_type.to_ascii_lowercase();

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
    if sql_type_lower.contains("int")
        || sql_type_lower.contains("numeric")
        || sql_type_lower.contains("decimal")
        || sql_type_lower == "float4"
        || sql_type_lower == "float8"
    {
        // Remove type casts like ::integer
        let value = default.split("::").next().unwrap_or(default);
        return Some(value.trim_matches('\'').to_string());
    }

    // Handle text/string types
    if sql_type_lower.contains("text")
        || sql_type_lower.contains("varchar")
        || sql_type_lower.contains("char")
        || sql_type_lower == "bpchar"
    {
        // Keep as quoted string, removing Postgres specific casts
        let value = default.split("::").next().unwrap_or(default);
        let trimmed = value.trim_matches('\'');
        return Some(format!("\"{}\"", escape_for_rust_literal(trimmed)));
    }

    None
}

/// Convert `PostgreSQL` type to Rust type
#[must_use]
pub fn sql_type_to_rust_type(sql_type: &str, not_null: bool) -> String {
    // Handle PostgreSQL array types, which are often represented as "_typename" (udt_name).
    // Keep this intentionally simple: one-dimensional arrays map to Vec<T>.
    if let Some(elem) = sql_type.strip_prefix('_') {
        let elem_ty = sql_type_to_rust_type(elem, true);
        let base = format!("Vec<{elem_ty}>");
        return if not_null {
            base
        } else {
            format!("Option<{base}>")
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
        format!("Option<{base_type}>")
    }
}

/// Convert `PostgreSQL` type and array dimensions to a Rust type.
#[must_use]
pub fn sql_type_to_rust_type_with_dimensions(
    sql_type: &str,
    dimensions: Option<i32>,
    not_null: bool,
) -> String {
    let Some(dimensions) = dimensions.filter(|dims| *dims > 0) else {
        return sql_type_to_rust_type(sql_type, not_null);
    };

    let mut base = sql_type_to_rust_type(sql_type.trim_start_matches('_'), true);
    for _ in 0..dimensions {
        base = format!("Vec<{base}>");
    }

    if not_null {
        base
    } else {
        format!("Option<{base}>")
    }
}

fn write_doc_comment(code: &mut String, indent: &str, comment: &str) {
    for line in comment.lines() {
        if line.is_empty() {
            let _ = writeln!(code, "{indent}///");
        } else {
            let _ = writeln!(code, "{indent}/// {line}");
        }
    }
}

/// Generate an index struct
fn generate_index_struct(index: &Index, use_pub: bool, field_casing: FieldCasing) -> String {
    let struct_name = index.name.to_pascal_case();
    let table_name = index.table.to_pascal_case();
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    let mut attrs = Vec::new();
    if index.is_unique {
        attrs.push("unique".to_string());
    }
    if index.concurrently {
        attrs.push("concurrent".to_string());
    }
    if let Some(method) = &index.method
        && !method.eq_ignore_ascii_case("btree")
    {
        attrs.push(format!("method = \"{}\"", escape_for_rust_literal(method)));
    }
    if let Some(where_clause) = &index.where_clause {
        attrs.push(format!(
            "where = \"{}\"",
            escape_for_rust_literal(where_clause)
        ));
    }
    if attrs.is_empty() {
        code.push_str("#[PostgresIndex]\n");
    } else {
        let _ = writeln!(code, "#[PostgresIndex({})]", attrs.join(", "));
    }

    // Tuple struct with column references
    let columns: Vec<String> = index
        .columns
        .iter()
        .map(|c| {
            if c.is_expression {
                format!("\"{}\"", c.value) // Expression indexes use string literals
            } else {
                format!(
                    "{}::{}",
                    table_name,
                    apply_field_casing(c.value.as_ref(), field_casing)
                )
            }
        })
        .collect();

    let _ = writeln!(code, "{vis}struct {struct_name}({});", columns.join(", "));
    code
}

/// Generate a view struct
fn generate_view_struct(
    view: &View,
    columns: &[&Column],
    enum_map: &HashMap<(String, String), String>,
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
        attrs.push(format!("using = \"{using}\""));
    }

    // Add TABLESPACE for materialized views
    if let Some(tablespace) = &view.tablespace {
        attrs.push(format!("tablespace = \"{tablespace}\""));
    }

    // Add definition
    if let Some(def) = &view.definition {
        let escaped_def = escape_for_rust_literal(def);
        attrs.push(format!("definition = \"{escaped_def}\""));
    }

    // Build the attribute line
    if attrs.is_empty() {
        code.push_str("#[PostgresView]\n");
    } else {
        let _ = writeln!(code, "#[PostgresView({})]", attrs.join(", "));
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

        // Check if this column uses an enum type
        let type_schema = column.type_schema.as_deref().unwrap_or(&column.schema);
        let enum_type = enum_map.get(&(type_schema.to_string(), column.sql_type.to_string()));

        // Determine Rust type - use enum type if available, otherwise map SQL type
        let rust_type = enum_type.map_or_else(
            || {
                sql_type_to_rust_type_with_dimensions(
                    &column.sql_type,
                    column.dimensions,
                    column.not_null,
                )
            },
            |enum_name| {
                if column.not_null {
                    enum_name.clone()
                } else {
                    format!("Option<{enum_name}>")
                }
            },
        );

        let _ = writeln!(code, "    {vis}{field_name}: {rust_type},");
    }

    code.push_str("}\n");
    code
}

fn generate_policy_struct(policy: &Policy, use_pub: bool) -> String {
    let mut struct_name = policy.name.to_pascal_case();
    if struct_name.is_empty() {
        struct_name = "Policy".to_string();
    }
    let table_type = policy.table.to_pascal_case();
    let vis = if use_pub { "pub " } else { "" };

    let mut attrs = Vec::new();
    if struct_name.to_snake_case() != policy.name.as_ref() {
        attrs.push(format!(
            "name = \"{}\"",
            escape_for_rust_literal(&policy.name)
        ));
    }
    if let Some(as_clause) = &policy.as_clause {
        attrs.push(format!("as = \"{}\"", escape_for_rust_literal(as_clause)));
    }
    if let Some(for_clause) = &policy.for_clause {
        attrs.push(format!("for = \"{}\"", escape_for_rust_literal(for_clause)));
    }
    if let Some(roles) = &policy.to
        && !roles.is_empty()
    {
        let roles = roles
            .iter()
            .map(|role| format!("\"{}\"", escape_for_rust_literal(role)))
            .collect::<Vec<_>>()
            .join(", ");
        attrs.push(format!("to({roles})"));
    }
    if let Some(using) = &policy.using {
        attrs.push(format!("using = \"{}\"", escape_for_rust_literal(using)));
    }
    if let Some(with_check) = &policy.with_check {
        attrs.push(format!(
            "with_check = \"{}\"",
            escape_for_rust_literal(with_check)
        ));
    }

    let mut code = String::new();
    if attrs.is_empty() {
        code.push_str("#[PostgresPolicy]\n");
    } else {
        let _ = writeln!(code, "#[PostgresPolicy({})]", attrs.join(", "));
    }
    let _ = writeln!(code, "{vis}struct {struct_name}({table_type});");
    code
}

/// Generate a schema struct
fn generate_schema_struct(
    schema_name: &str,
    tables: &[String],
    indexes: &[String],
    policies: &[String],
    use_pub: bool,
    field_casing: FieldCasing,
) -> String {
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Schema derive
    code.push_str("#[derive(PostgresSchema)]\n");
    let _ = writeln!(code, "{vis}struct {schema_name} {{");

    // Table fields
    for table in tables {
        let field_name = apply_field_casing(table, field_casing);
        let type_name = table.to_pascal_case();
        let _ = writeln!(code, "    {vis}{field_name}: {type_name},");
    }

    // Index fields (commented as they're typically not needed in schema)
    if !indexes.is_empty() {
        code.push_str("    // Indexes:\n");
        for index in indexes {
            let field_name = apply_field_casing(index, field_casing);
            let type_name = index.to_pascal_case();
            let _ = writeln!(code, "    // {field_name}: {type_name},");
        }
    }

    for policy in policies {
        let field_name = apply_field_casing(policy, field_casing);
        let type_name = policy.to_pascal_case();
        let _ = writeln!(code, "    {vis}{field_name}: {type_name},");
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
