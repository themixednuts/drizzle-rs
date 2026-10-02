//! Generates Rust schema source (`#[PostgresTable]` structs) from
//! introspected PostgreSQL DDL entities.
//!
//! Output uses the lowercase attribute style (`primary`, not `PRIMARY`).

use super::collection::PostgresDDL;
use super::ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Index, Policy, PrimaryKey, Table, UniqueConstraint,
    View,
};
use crate::utils::{
    ViewSelectItem, default_expression, escape_for_rust_literal, macro_default_name_matches,
    rust_ident, unsupported_default_comment, view_select_columns,
};
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::borrow::Cow;
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

/// The Rust identifier for a generated field or schema member named after
/// the SQL identifier `name`.
fn apply_field_casing(name: &str, casing: FieldCasing) -> String {
    rust_ident(&match casing {
        FieldCasing::Snake => name.to_snake_case(),
        FieldCasing::Camel => name.to_lower_camel_case(),
        FieldCasing::Preserve => name.to_string(),
    })
}

/// The Rust type name for a generated struct named after `name`.
fn struct_ident(name: &str) -> String {
    rust_ident(&name.to_pascal_case())
}

/// Rust primitive type names; an enum named like one would shadow it.
const RUST_PRIMITIVES: &[&str] = &[
    "bool", "char", "str", "i8", "i16", "i32", "i64", "i128", "isize", "u8", "u16", "u32", "u64",
    "u128", "usize", "f32", "f64",
];

/// Rust names for a `PostgreSQL` enum and its values.
struct EnumNames {
    type_name: String,
    variants: Vec<String>,
}

/// `#[derive(PostgresEnum)]` writes the Rust names into the database: the
/// type is created as `CREATE TYPE <EnumName>` (unquoted, so `PostgreSQL`
/// folds it to lowercase) and each value is its variant's name. A
/// lowercase SQL name and identifier-shaped values therefore keep their
/// exact spelling (with `#[allow(non_camel_case_types)]` on the enum);
/// anything else falls back to `PascalCase` with a warning.
fn enum_names(e: &Enum, warnings: &mut Vec<String>) -> EnumNames {
    let is_exact_ident = |name: &str| {
        rust_ident(name) == name && !name.starts_with('_') && !RUST_PRIMITIVES.contains(&name)
    };

    let type_name = if is_exact_ident(&e.name) && !e.name.chars().any(|c| c.is_ascii_uppercase()) {
        e.name.to_string()
    } else {
        let fallback = struct_ident(&e.name);
        warnings.push(format!(
            "enum `{}`: #[derive(PostgresEnum)] creates the type from the Rust name, so it is generated as `{fallback}` (PostgreSQL type `{}`); rename the type or keep the original name another way",
            e.name,
            fallback.to_ascii_lowercase()
        ));
        fallback
    };

    let mut seen = HashSet::new();
    let exact_values = e
        .values
        .iter()
        .all(|value| is_exact_ident(value) && seen.insert(value.as_ref()));
    let variants = if exact_values {
        e.values.iter().map(ToString::to_string).collect()
    } else {
        warnings.push(format!(
            "enum `{}`: its values ({}) are not all Rust identifiers, and #[derive(PostgresEnum)] stores variant names, so the variants were renamed; the stored values change",
            e.name,
            e.values
                .iter()
                .map(|value| format!("'{value}'"))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        e.values.iter().map(|value| struct_ident(value)).collect()
    };

    EnumNames {
        type_name,
        variants,
    }
}

/// The macro's default name for a column-level foreign key.
fn default_fk_name(table: &str, column: &str) -> String {
    format!("{table}_{column}_fkey")
}

/// Whether a foreign key can be written as `#[column(references = ...)]`
/// (one column), which also gives the table relation accessors.
fn is_column_level_fk(fk: &ForeignKey) -> bool {
    fk.columns.len() == 1 && fk.columns_to.len() == 1
}

/// Picks `preferred` unless a schema struct field already uses it.
fn unique_member_name(preferred: String, suffix: &str, used: &mut HashSet<String>) -> String {
    if used.insert(preferred.clone()) {
        return preferred;
    }
    let mut candidate = format!("{preferred}_{suffix}");
    let mut n = 2;
    while !used.insert(candidate.clone()) {
        candidate = format!("{preferred}_{suffix}{n}");
        n += 1;
    }
    candidate
}

/// Lookup tables derived from a [`PostgresDDL`] and shared across every
/// per-entity generation pass.
struct SchemaMaps<'a> {
    enum_map: HashMap<(String, String), String>,
    enum_names: HashMap<(String, String), EnumNames>,
    table_columns: HashMap<(String, String), Vec<&'a Column>>,
    table_pks: HashMap<(String, String), HashSet<String>>,
    table_pk: HashMap<(String, String), &'a PrimaryKey>,
    /// Foreign keys written as table-level `foreign_key(...)` attributes.
    table_fks: HashMap<(String, String), Vec<&'a ForeignKey>>,
    single_unique_columns: HashMap<(String, String), HashSet<String>>,
    table_uniques: HashMap<(String, String), Vec<&'a UniqueConstraint>>,
    table_checks: HashMap<(String, String), Vec<&'a CheckConstraint>>,
    fk_map: HashMap<(String, String, String), (&'a ForeignKey, usize)>,
}

fn build_schema_maps<'a>(ddl: &'a PostgresDDL, warnings: &mut Vec<String>) -> SchemaMaps<'a> {
    let mut enum_map: HashMap<(String, String), String> = HashMap::new();
    let mut enum_names_map: HashMap<(String, String), EnumNames> = HashMap::new();
    for e in ddl.enums.list() {
        let names = enum_names(e, warnings);
        let key = (e.schema.to_string(), e.name.to_string());
        enum_map.insert(key.clone(), names.type_name.clone());
        enum_names_map.insert(key, names);
    }

    let mut table_columns: HashMap<(String, String), Vec<&Column>> = HashMap::new();
    for column in ddl.columns.list() {
        table_columns
            .entry((column.schema.to_string(), column.table.to_string()))
            .or_default()
            .push(column);
    }

    let mut table_pks: HashMap<(String, String), HashSet<String>> = HashMap::new();
    let mut table_pk: HashMap<(String, String), &PrimaryKey> = HashMap::new();
    for pk in ddl.pks.list() {
        table_pk.insert((pk.schema.to_string(), pk.table.to_string()), pk);
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
            && unique.name == default_unique_name(&unique.table, &unique.columns)
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
    let mut table_fks: HashMap<(String, String), Vec<&ForeignKey>> = HashMap::new();
    for fk in ddl.fks.list() {
        let column_key = (
            fk.schema.to_string(),
            fk.table.to_string(),
            fk.columns[0].to_string(),
        );
        // A column carries at most one `references`.
        if is_column_level_fk(fk) && !fk_map.contains_key(&column_key) {
            fk_map.insert(column_key, (fk, 0));
        } else {
            table_fks
                .entry((fk.schema.to_string(), fk.table.to_string()))
                .or_default()
                .push(fk);
        }
    }

    SchemaMaps {
        enum_map,
        enum_names: enum_names_map,
        table_columns,
        table_pks,
        table_pk,
        table_fks,
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

    let maps = build_schema_maps(ddl, &mut result.warnings);

    // Generate enum definitions
    let mut enum_types = Vec::new();
    for e in ddl.enums.list() {
        let names = &maps.enum_names[&(e.schema.to_string(), e.name.to_string())];
        code.push_str(&generate_enum_struct(e, names, options.use_pub));
        code.push('\n');
        result.enums.push(e.name.to_string());
        enum_types.push(names.type_name.clone());
    }

    for sequence in ddl.sequences.list() {
        result.warnings.push(format!(
            "sequence `{}.{}` has no Rust schema equivalent and was not generated; `drizzle generate` would drop it",
            sequence.schema, sequence.name
        ));
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

        code.push_str(&generate_table_struct(
            &TableGenContext {
                table,
                columns,
                pk_columns,
                pk: maps.table_pk.get(&key).copied(),
                unique_columns,
                unique_constraints,
                check_constraints,
                table_fks: maps
                    .table_fks
                    .get(&key)
                    .map_or(&[][..], std::vec::Vec::as_slice),
                fk_map: &maps.fk_map,
                enum_map: &maps.enum_map,
                use_pub: options.use_pub,
                field_casing: options.field_casing,
            },
            &mut result.warnings,
        ));
        code.push('\n');
        result.tables.push(table.name.to_string());
    }

    // Generate index structs
    for index in ddl.indexes.list() {
        code.push_str(&generate_index_struct(
            index,
            options.use_pub,
            options.field_casing,
            &mut result.warnings,
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
        let listed = maps
            .table_columns
            .get(&key)
            .map_or(&[][..], std::vec::Vec::as_slice);
        // drizzle-kit snapshots carry no view columns; read them from the
        // definition instead.
        let inferred = if listed.is_empty() {
            inferred_view_columns(view, ddl, &mut result.warnings)
        } else {
            Vec::new()
        };
        let inferred_refs: Vec<&Column> = inferred.iter().collect();
        let columns = if listed.is_empty() {
            &inferred_refs[..]
        } else {
            listed
        };
        if columns.is_empty() {
            let _ = writeln!(
                code,
                "// TODO: view `{}` was not generated: its columns could not be read from\n// its definition. Declare it with #[PostgresView] and its fields.\n",
                view.name
            );
            result.warnings.push(format!(
                "view `{}`: its columns could not be read from the definition, so it was not generated; `drizzle generate` would drop it until you declare it",
                view.name
            ));
            continue;
        }
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
            &SchemaMembers {
                enums: &enum_types,
                tables: &result.tables,
                indexes: &result.indexes,
                views: &result.views,
                policies: &result.policies,
            },
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
    pk: Option<&'a PrimaryKey>,
    unique_columns: Option<&'a HashSet<String>>,
    unique_constraints: &'a [&'a UniqueConstraint],
    check_constraints: &'a [&'a CheckConstraint],
    table_fks: &'a [&'a ForeignKey],
    fk_map: &'a HashMap<(String, String, String), (&'a ForeignKey, usize)>,
    enum_map: &'a HashMap<(String, String), String>,
    use_pub: bool,
    field_casing: FieldCasing,
}

/// Generate a single table struct
fn generate_table_struct(ctx: &TableGenContext<'_>, warnings: &mut Vec<String>) -> String {
    let struct_name = struct_ident(&ctx.table.name);
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
        let field_code = generate_column_field(column, ctx, warnings);
        code.push_str(&field_code);
    }

    code.push_str("}\n");
    code
}

fn format_table_attrs(ctx: &TableGenContext<'_>) -> Vec<String> {
    let table = ctx.table;
    let mut attrs = Vec::new();
    if !macro_default_name_matches(&struct_ident(&table.name), &table.name) {
        attrs.push(format!(
            "name = \"{}\"",
            escape_for_rust_literal(&table.name)
        ));
    }
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
    // The macro names the primary key `{table}_pkey` unless told otherwise.
    if let Some(pk) = ctx.pk
        && pk.name != format!("{}_pkey", table.name)
    {
        attrs.push(format!(
            "primary_key(name = \"{}\")",
            escape_for_rust_literal(&pk.name)
        ));
    }
    for fk in ctx.table_fks {
        attrs.push(format_table_fk_attr(fk, ctx.field_casing));
    }
    attrs
}

/// A table-level `foreign_key(...)` attribute (composite keys).
fn format_table_fk_attr(fk: &ForeignKey, field_casing: FieldCasing) -> String {
    let columns: Vec<String> = fk
        .columns
        .iter()
        .map(|col| apply_field_casing(col, field_casing))
        .collect();
    let target_columns: Vec<String> = fk
        .columns_to
        .iter()
        .map(|col| apply_field_casing(col, field_casing))
        .collect();
    let mut args = vec![
        format!("columns({})", columns.join(", ")),
        format!(
            "references({}, {})",
            struct_ident(&fk.table_to),
            target_columns.join(", ")
        ),
    ];
    if fk.name != default_fk_name(&fk.table, &fk.columns[0]) {
        args.push(format!("name = \"{}\"", escape_for_rust_literal(&fk.name)));
    }
    for (key, action) in [("on_delete", &fk.on_delete), ("on_update", &fk.on_update)] {
        if let Some(action) = action
            && !action.eq_ignore_ascii_case("NO ACTION")
        {
            args.push(format!("{key} = \"{}\"", action.to_ascii_uppercase()));
        }
    }
    if fk.initially_deferred {
        args.push("initially_deferred".to_string());
    } else if fk.deferrable {
        args.push("deferrable".to_string());
    }
    format!("foreign_key({})", args.join(", "))
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
    if unique.nulls_not_distinct {
        args.push("nulls_not_distinct".to_string());
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
///
/// `sql_type` is used to suppress options equal to `PostgreSQL`'s defaults
/// for the column's type — introspection materializes every option, and
/// re-emitting the defaults would clutter every identity column with
/// `min_value = 1, max_value = 2147483647`.
fn format_identity_attr(identity: &super::ddl::Identity, sql_type: &str) -> String {
    use super::ddl::IdentityType;
    use super::grammar::{IdentityDefaults, PgTypeCategory};

    let identity_type = match identity.type_ {
        IdentityType::Always => "always",
        IdentityType::ByDefault => "by_default",
    };

    let default_range_type = match PgTypeCategory::from_sql_type(sql_type) {
        PgTypeCategory::SmallInt => "smallint",
        PgTypeCategory::BigInt => "bigint",
        _ => "integer",
    };

    let mut seq_opts: Vec<String> = Vec::new();
    if let Some(increment) = &identity.increment
        && increment != IdentityDefaults::INCREMENT
    {
        seq_opts.push(format!("increment = {increment}"));
    }
    if let Some(start) = &identity.start_with
        && start != IdentityDefaults::START_WITH
    {
        seq_opts.push(format!("start = {start}"));
    }
    if let Some(min) = &identity.min_value
        && min != IdentityDefaults::MIN
    {
        seq_opts.push(format!("min_value = {min}"));
    }
    if let Some(max) = &identity.max_value
        && max != IdentityDefaults::max_for(default_range_type)
    {
        seq_opts.push(format!("max_value = {max}"));
    }
    if let Some(cache) = &identity.cache
        && *cache != IdentityDefaults::CACHE
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
fn push_fk_attrs(attrs: &mut Vec<String>, fk: &ForeignKey, idx: usize, casing: FieldCasing) {
    let ref_table = struct_ident(&fk.table_to);
    let ref_column = fk.columns_to.get(idx).cloned().unwrap_or_default();
    let ref_field = apply_field_casing(&ref_column, casing);
    attrs.push(format!("references = {ref_table}::{ref_field}"));
    if fk.name != default_fk_name(&fk.table, &fk.columns[idx]) {
        attrs.push(format!(
            "fk_name = \"{}\"",
            escape_for_rust_literal(&fk.name)
        ));
    }

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
fn generate_column_field(
    column: &Column,
    ctx: &TableGenContext<'_>,
    warnings: &mut Vec<String>,
) -> String {
    let field_name = apply_field_casing(column.name.as_ref(), ctx.field_casing);
    let vis = if ctx.use_pub { "pub " } else { "" };

    let col_name_str = column.name.to_string();
    let is_pk = ctx
        .pk_columns
        .is_some_and(|pks| pks.contains(&col_name_str));
    let is_unique = ctx
        .unique_columns
        .is_some_and(|uqs| uqs.contains(&col_name_str));

    // `primary` on several fields forms a composite key.
    let should_add_primary = is_pk;

    // Serial columns: drizzle-kit records the `serial` type; a database
    // reports the integer type with a `nextval(...)` default.
    let serial_kind = serial_attr(&column.sql_type).or_else(|| {
        (column
            .default
            .as_ref()
            .is_some_and(|d| d.contains("nextval"))
            && column.identity.is_none())
        .then_some("serial")
    });
    let is_serial = serial_kind.is_some();

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

    // The macro names the column after the field in snake_case.
    if !macro_default_name_matches(&field_name, &column.name) {
        attrs.push(format!(
            "name = \"{}\"",
            escape_for_rust_literal(&column.name)
        ));
    }

    if let Some(physical_type) = bounded_character_type_attr(column) {
        attrs.push(physical_type);
    }

    // `serde_json::Value` maps to JSONB; plain JSON needs the marker.
    if enum_type.is_none()
        && super::collection::normalize_type_for_compare(&column.sql_type) == "json"
    {
        attrs.push("json".to_string());
    }

    if let Some(serial) = serial_kind {
        attrs.push(serial.to_string());
    }

    // For GENERATED IDENTITY columns, use identity(always) or identity(by_default)
    // with optional sequence options
    if let Some(identity) = &column.identity {
        attrs.push(format_identity_attr(identity, &column.sql_type));
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
    let mut unsupported_default = None;
    if let Some(default) = &column.default
        && !is_serial
        && column.generated.is_none()
    {
        if let Some(formatted) = format_default_value(default, &column.sql_type) {
            attrs.push(format!("default = {formatted}"));
        } else if !default.trim().eq_ignore_ascii_case("null") {
            match default_expression(default) {
                Some(expression) => attrs.push(format!("default = {expression}")),
                None => unsupported_default = Some(default.as_ref()),
            }
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
        push_fk_attrs(&mut attrs, fk, *idx, ctx.field_casing);
    }

    // Generate attribute line if there are any
    let mut result = String::new();
    if let Some(default) = unsupported_default {
        result.push_str(&unsupported_default_comment("    ", default));
    }
    if enum_type.is_none()
        && serial_kind.is_none()
        && let Some(note) = unsupported_type_note(column)
    {
        let _ = writeln!(result, "    // TODO: {note}");
        warnings.push(format!("column `{}.{}`: {note}", column.table, column.name));
    }
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

/// Columns of a view whose snapshot lists none, read from its definition:
/// plain column references copy the source column's type, `count(...)` is
/// a `bigint`, and other expressions become nullable text (with a warning).
fn inferred_view_columns(
    view: &View,
    ddl: &PostgresDDL,
    warnings: &mut Vec<String>,
) -> Vec<Column> {
    let Some((from, items)) = view.definition.as_deref().and_then(view_select_columns) else {
        return Vec::new();
    };
    let source = |table: Option<&str>, column: &str| {
        let table = table.or(from.as_deref());
        ddl.columns
            .list()
            .iter()
            .find(|c| Some(c.table.as_ref()) == table && c.name == column)
            .or_else(|| ddl.columns.list().iter().find(|c| c.name == column))
    };
    let view_column = |name: &str, src: Option<&Column>, sql_type: &str, not_null: bool| {
        let mut column = src.cloned().unwrap_or_else(|| {
            Column::new(
                view.schema.to_string(),
                view.name.to_string(),
                name.to_string(),
                sql_type.to_string(),
            )
        });
        column.schema = Cow::Owned(view.schema.to_string());
        column.table = Cow::Owned(view.name.to_string());
        column.name = Cow::Owned(name.to_string());
        column.not_null = not_null;
        column.default = None;
        column.generated = None;
        column.identity = None;
        column.comment = None;
        // A serial source column is a plain integer in the view.
        if let Some(serial) = serial_attr(&column.sql_type) {
            column.sql_type = Cow::Borrowed(match serial {
                "bigserial" => "bigint",
                "smallserial" => "smallint",
                _ => "integer",
            });
        }
        column
    };
    let mut columns = Vec::new();
    for item in items {
        match item {
            ViewSelectItem::Column {
                output,
                table,
                column,
            } => match source(table.as_deref(), &column) {
                Some(src) => columns.push(view_column(&output, Some(src), "", src.not_null)),
                None => {
                    warnings.push(format!(
                        "view `{}`: column `{output}` refers to an unknown column; typed as text",
                        view.name
                    ));
                    columns.push(view_column(&output, None, "text", false));
                }
            },
            ViewSelectItem::Expression { output, expression } => {
                if expression.to_ascii_lowercase().starts_with("count(") {
                    columns.push(view_column(&output, None, "bigint", true));
                } else {
                    warnings.push(format!(
                        "view `{}`: the type of `{expression}` (column `{output}`) is unknown; typed as text",
                        view.name
                    ));
                    columns.push(view_column(&output, None, "text", false));
                }
            }
            ViewSelectItem::Star { table } => {
                let table = table.or_else(|| from.clone());
                for src in ddl
                    .columns
                    .list()
                    .iter()
                    .filter(|c| Some(c.table.as_ref()) == table.as_deref())
                {
                    columns.push(view_column(&src.name, Some(src), "", src.not_null));
                }
            }
        }
    }
    columns
}

/// The `#[column(...)]` marker for a `serial`-family SQL type.
fn serial_attr(sql_type: &str) -> Option<&'static str> {
    match sql_type.trim().to_ascii_lowercase().as_str() {
        "serial" | "serial4" => Some("serial"),
        "bigserial" | "serial8" => Some("bigserial"),
        "smallserial" | "serial2" => Some("smallserial"),
        _ => None,
    }
}

/// Explains why a column's type cannot be kept by the generated field, or
/// `None` when the field's Rust type (plus any `varchar(n)` / `char(n)` /
/// `json` marker) produces the same SQL type.
fn unsupported_type_note(column: &Column) -> Option<String> {
    let normalized = super::collection::normalize_type_for_compare(&column.sql_type);
    let base = normalized.trim_end_matches("[]");
    let base_name = base.split('(').next().unwrap_or(base).trim();
    let supported = matches!(
        base_name,
        "smallint"
            | "integer"
            | "bigint"
            | "real"
            | "double precision"
            | "float4"
            | "float8"
            | "boolean"
            | "text"
            | "bytea"
            | "uuid"
            | "date"
            | "time"
            | "timestamp"
            | "timestamp with time zone"
            | "json"
            | "jsonb"
            | "inet"
            | "cidr"
    ) || (matches!(base_name, "character varying" | "character")
        && bounded_character_type_attr(column).is_some());
    (!supported).then(|| {
        format!(
            "`{}` has no built-in Rust mapping in #[PostgresTable]; the field type `{}` maps to a different SQL type, so `drizzle generate` would change the column. Use a type implementing `DrizzlePostgresColumn`.",
            column.sql_type,
            sql_type_to_rust_type_with_dimensions(&column.sql_type, column.dimensions, true)
        )
    })
}

fn bounded_character_type_attr(column: &Column) -> Option<String> {
    let normalized = super::collection::normalize_type_for_compare(&column.sql_type);
    for (prefix, marker) in [("character varying", "VARCHAR"), ("character", "CHAR")] {
        let Some(arguments) = normalized
            .strip_prefix(prefix)
            .and_then(|value| value.strip_prefix('('))
        else {
            continue;
        };
        let Some(length) = arguments.strip_suffix(')') else {
            continue;
        };
        if !length.is_empty() && length.chars().all(|character| character.is_ascii_digit()) {
            return Some(format!("{marker}({length})"));
        }
    }
    None
}

/// Generate a Rust enum definition from a `PostgreSQL` enum
fn generate_enum_struct(e: &Enum, names: &EnumNames, use_pub: bool) -> String {
    let enum_name = &names.type_name;
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // The SQL type and values are spelled exactly like the Rust names.
    let needs_allow = enum_name.starts_with(|c: char| c.is_ascii_lowercase())
        || enum_name.contains('_')
        || names
            .variants
            .iter()
            .any(|v| v.starts_with(|c: char| c.is_ascii_lowercase()) || v.contains('_'));
    if needs_allow {
        code.push_str("#[allow(non_camel_case_types)]\n");
    }
    code.push_str("#[derive(PostgresEnum, Clone, Copy, Debug, Default, PartialEq)]\n");
    if e.schema != "public" {
        let _ = writeln!(
            code,
            "#[postgres_enum(schema = \"{}\")]",
            escape_for_rust_literal(&e.schema)
        );
    }

    // Enum definition
    let _ = writeln!(code, "{vis}enum {enum_name} {{");

    // Generate variants from enum values
    for (idx, variant_name) in names.variants.iter().enumerate() {
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

    // Handle numeric types. Match whole type names: a substring test would
    // also catch `interval` and `point`, whose defaults are not numbers.
    let base_type = sql_type_lower
        .split(['(', '['])
        .next()
        .unwrap_or_default()
        .trim();
    if matches!(
        base_type,
        "int2"
            | "int4"
            | "int8"
            | "smallint"
            | "integer"
            | "int"
            | "bigint"
            | "smallserial"
            | "serial"
            | "bigserial"
            | "serial2"
            | "serial4"
            | "serial8"
            | "numeric"
            | "decimal"
            | "float4"
            | "float8"
            | "real"
            | "double precision"
    ) {
        // Remove type casts like ::integer
        let value = default.split("::").next().unwrap_or(default);
        let value = value.trim_matches('\'');
        if matches!(base_type, "float4" | "float8" | "real" | "double precision") {
            return value.parse::<f64>().ok().map(|v| format!("{v:?}"));
        }
        // `numeric` fields are generated as `String`.
        if matches!(base_type, "numeric" | "decimal") {
            return value.parse::<f64>().is_ok().then(|| format!("\"{value}\""));
        }
        if value.parse::<f64>().is_ok() {
            return Some(value.to_string());
        }
        return None;
    }

    // String literals (and every other expression, e.g. `CURRENT_USER` on a
    // text column) are handled by `default_expression`, which distinguishes
    // quoted strings from bare SQL identifiers.
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

    // drizzle-kit snapshots spell arrays `text[]`.
    if let Some(elem) = sql_type.trim().strip_suffix("[]") {
        let elem_ty = sql_type_to_rust_type(elem, true);
        let base = format!("Vec<{elem_ty}>");
        return if not_null {
            base
        } else {
            format!("Option<{base}>")
        };
    }

    // Match on the type name without its parameters (`varchar(255)`,
    // `timestamp(3) with time zone`).
    let lowered = sql_type.trim().to_ascii_lowercase();
    let without_params = match (lowered.find('('), lowered.find(')')) {
        (Some(open), Some(close)) if open < close => {
            format!("{}{}", &lowered[..open], &lowered[close + 1..])
        }
        _ => lowered.clone(),
    };
    let sql_type = without_params.trim();

    let base_type = match sql_type {
        "timestamp with time zone" => "chrono::DateTime<chrono::Utc>",
        "timestamp without time zone" => "chrono::NaiveDateTime",
        "time without time zone" => "chrono::NaiveTime",
        "character varying" | "character" => "String",
        "inet" => "cidr::IpInet",
        "cidr" => "cidr::IpCidr",
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
fn generate_index_struct(
    index: &Index,
    use_pub: bool,
    field_casing: FieldCasing,
    warnings: &mut Vec<String>,
) -> String {
    let struct_name = struct_ident(&index.name);
    let table_name = struct_ident(&index.table);
    if let Some(column) = index
        .columns
        .iter()
        .find(|c| !c.asc || c.nulls_first || c.opclass.is_some())
    {
        warnings.push(format!(
            "index `{}`: the ordering or operator class of `{}` (DESC, NULLS FIRST, opclass) cannot be expressed in #[PostgresIndex] and was dropped",
            index.name, column.value
        ));
    }
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    let mut attrs = Vec::new();
    let inferred_name = struct_name.to_snake_case();
    let inferred_name = if inferred_name.ends_with("_idx") || inferred_name.ends_with("_index") {
        inferred_name
    } else {
        format!("{inferred_name}_idx")
    };
    if inferred_name != index.name.as_ref() {
        attrs.push(format!(
            "name = \"{}\"",
            escape_for_rust_literal(&index.name)
        ));
    }
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
    let struct_name = struct_ident(&view.name);
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Build view attributes
    let mut attrs = Vec::new();

    // The macro names the view after the struct in snake_case.
    if !macro_default_name_matches(&struct_name, &view.name) {
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
        if !macro_default_name_matches(&field_name, &column.name) {
            let _ = writeln!(
                code,
                "    #[column(name = \"{}\")]",
                escape_for_rust_literal(&column.name)
            );
        }

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

/// Everything listed in the generated schema struct.
struct SchemaMembers<'a> {
    enums: &'a [String],
    tables: &'a [String],
    indexes: &'a [String],
    views: &'a [String],
    policies: &'a [String],
}

/// Generate a schema struct
fn generate_schema_struct(
    schema_name: &str,
    members: &SchemaMembers<'_>,
    use_pub: bool,
    field_casing: FieldCasing,
) -> String {
    let vis = if use_pub { "pub " } else { "" };

    let mut code = String::new();

    // Schema derive
    code.push_str("#[derive(PostgresSchema)]\n");
    let _ = writeln!(code, "{vis}struct {schema_name} {{");

    let mut used = HashSet::new();
    // Enum types first: their `CREATE TYPE` must run before the tables.
    for type_name in members.enums {
        let field_name = unique_member_name(
            apply_field_casing(type_name, FieldCasing::Snake),
            "enum",
            &mut used,
        );
        let _ = writeln!(code, "    {vis}{field_name}: {type_name},");
    }
    for (items, suffix) in [
        (members.tables, "table"),
        (members.indexes, "index"),
        (members.views, "view"),
        (members.policies, "policy"),
    ] {
        for item in items {
            let field_name =
                unique_member_name(apply_field_casing(item, field_casing), suffix, &mut used);
            let type_name = struct_ident(item);
            let _ = writeln!(code, "    {vis}{field_name}: {type_name},");
        }
    }

    code.push_str("}\n");
    code
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parser::SchemaParser, schema::Snapshot};
    use drizzle_types::Dialect;
    use drizzle_types::postgres::ddl::Schema;

    fn assert_round_trips(ddl: &PostgresDDL, generated: &GeneratedSchema) {
        let parsed = SchemaParser::parse(&generated.code);
        assert!(
            parsed.errors.is_empty(),
            "generated source:\n{}\nerrors: {:#?}",
            generated.code,
            parsed.errors
        );
        let Snapshot::Postgres(snapshot) =
            Snapshot::from_parse_result(&parsed, Dialect::PostgreSQL, None)
        else {
            panic!("expected generated PostgreSQL schema snapshot");
        };
        let reparsed = PostgresDDL::from_entities(snapshot.ddl);
        let migration = crate::postgres::diff::compute_migration(ddl, &reparsed);
        assert!(
            migration.sql_statements.is_empty(),
            "generated PostgreSQL schema changed the DDL: {:#?}",
            migration.sql_statements
        );
    }

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
    fn bounded_character_types_round_trip_without_becoming_text() {
        let mut ddl = PostgresDDL::new();
        ddl.schemas.push(Schema::new("public"));
        ddl.tables.push(Table::new("public", "users"));
        ddl.columns
            .push(Column::new("public", "users", "name", "varchar(255)"));
        ddl.columns
            .push(Column::new("public", "users", "code", "character(8)"));
        let mut aliases = Column::new("public", "users", "aliases", "varchar(64)");
        aliases.dimensions = Some(1);
        ddl.columns.push(aliases);

        let generated = generate_rust_schema(&ddl, &CodegenOptions::default());

        assert!(
            generated.code.contains("#[column(VARCHAR(255))]"),
            "generated schema must preserve the VARCHAR length:\n{}",
            generated.code
        );
        assert!(
            generated.code.contains("#[column(CHAR(8))]"),
            "generated schema must preserve the CHAR length:\n{}",
            generated.code
        );
        assert!(
            generated.code.contains("#[column(VARCHAR(64))]"),
            "generated schema must preserve bounded array element types:\n{}",
            generated.code
        );
        assert!(generated.code.contains("aliases: Option<Vec<String>>"));
        assert_round_trips(&ddl, &generated);
    }

    #[test]
    fn test_format_default_value() {
        // Numeric
        assert_eq!(format_default_value("42", "int4"), Some("42".to_string()));
        // `numeric` maps to a `String` field, so the default stays a string.
        assert_eq!(
            format_default_value("3.14::numeric", "numeric"),
            Some("\"3.14\"".to_string())
        );
        assert_eq!(
            format_default_value("0", "double precision"),
            Some("0.0".to_string())
        );

        // Boolean
        assert_eq!(
            format_default_value("true", "bool"),
            Some("true".to_string())
        );

        // Strings are left to `default_expression`, which also handles bare
        // identifiers such as `CURRENT_USER` on text columns.
        assert_eq!(format_default_value("'hello'::text", "text"), None);
        assert_eq!(format_default_value("CURRENT_USER", "text"), None);

        // `interval` / `point` are not numeric types despite containing `int`.
        assert_eq!(format_default_value("'1 year'::interval", "interval"), None);
        assert_eq!(format_default_value("'(0,0)'::point", "point"), None);
        assert_eq!(
            format_default_value("'-1'::integer", "int4"),
            Some("-1".to_string())
        );
        assert_eq!(
            format_default_value("1.5", "double precision"),
            Some("1.5".to_string())
        );

        // Function calls should be None
        assert_eq!(format_default_value("now()", "timestamp"), None);
        assert_eq!(
            format_default_value("nextval('seq'::regclass)", "int4"),
            None
        );
    }

    #[test]
    fn table_name_that_does_not_round_trip_through_rust_is_explicit() {
        let mut ddl = PostgresDDL::new();
        ddl.schemas.push(Schema::new("public"));
        ddl.tables.push(Table::new("public", "audit_logs_42"));
        ddl.columns
            .push(Column::new("public", "audit_logs_42", "id", "integer"));

        let generated = generate_rust_schema(&ddl, &CodegenOptions::default());

        assert!(
            generated
                .code
                .contains("#[PostgresTable(name = \"audit_logs_42\")]"),
            "generated schema must preserve the SQL table name:\n{}",
            generated.code
        );
        assert_round_trips(&ddl, &generated);
    }

    #[test]
    fn index_name_that_does_not_round_trip_through_rust_is_explicit() {
        use drizzle_types::postgres::ddl::IndexColumn;

        let mut ddl = PostgresDDL::new();
        ddl.schemas.push(Schema::new("public"));
        ddl.tables.push(Table::new("public", "users"));
        ddl.columns
            .push(Column::new("public", "users", "email", "text"));
        ddl.indexes.push(Index::new(
            "public",
            "users",
            "users_email_42",
            vec![IndexColumn::new("email")],
        ));

        let generated = generate_rust_schema(&ddl, &CodegenOptions::default());

        assert!(
            generated
                .code
                .contains("#[PostgresIndex(name = \"users_email_42\")]"),
            "generated schema must preserve the SQL index name:\n{}",
            generated.code
        );
        assert_round_trips(&ddl, &generated);
    }

    #[test]
    fn index_name_without_derived_suffix_is_explicit() {
        use drizzle_types::postgres::ddl::IndexColumn;

        let mut ddl = PostgresDDL::new();
        ddl.schemas.push(Schema::new("public"));
        ddl.tables.push(Table::new("public", "users"));
        ddl.columns
            .push(Column::new("public", "users", "email", "text"));
        ddl.indexes.push(Index::new(
            "public",
            "users",
            "users_email",
            vec![IndexColumn::new("email")],
        ));

        let generated = generate_rust_schema(&ddl, &CodegenOptions::default());

        assert!(
            generated
                .code
                .contains("#[PostgresIndex(name = \"users_email\")]"),
            "generated schema must override the macro's `_idx` default:\n{}",
            generated.code
        );
        assert_round_trips(&ddl, &generated);
    }
}
