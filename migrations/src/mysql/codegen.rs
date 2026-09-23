//! MySQL schema code generation.
//!
//! The generator deliberately targets the public `MySQLTable`, `MySQLIndex`,
//! `MySQLView`, `MySQLSchema`, and `MySQLEnum` macro surface.  That makes a
//! generated file useful to both the source parser and a consuming Rust crate
//! instead of relying on parser-only attributes.

use super::collection::{MySQLDDL, TableEntities, ValidationError};
use super::ddl::{
    CheckConstraint, Column, ForeignKey, GeneratedType, Index, IndexAlgorithm, IndexLock,
    IndexMethod, InlineType, ReferentialAction, Table, UniqueConstraint, View, ViewAlgorithm,
    ViewCheckOption, ViewSqlSecurity,
};
use crate::utils::{default_expression, escape_for_rust_literal, unsupported_default_comment};
use drizzle_types::mysql::{MySQLType, MySQLTypeCategory};
use heck::{ToLowerCamelCase, ToPascalCase, ToSnakeCase};
use std::collections::{HashMap, HashSet};
use std::fmt::Write;

/// Result of MySQL Rust schema generation.
#[derive(Debug, Clone, Default)]
pub struct GeneratedSchema {
    /// The generated Rust source code.
    pub code: String,
    /// Inline MySQL enums emitted before the tables that use them.
    pub enums: Vec<String>,
    /// Tables that were generated.
    pub tables: Vec<String>,
    /// Indexes that were generated.
    pub indexes: Vec<String>,
    /// Views that were generated.
    pub views: Vec<String>,
    /// Metadata that the current macro surface cannot represent exactly.
    pub warnings: Vec<String>,
}

/// A MySQL schema cannot be emitted without changing its migration meaning.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CodegenError {
    #[error("invalid MySQL snapshot: {0}")]
    InvalidSnapshot(#[from] ValidationError),
    #[error(
        "cannot generate MySQL column {table}.{column}: type `{sql_type}` has no lossless MySQLTable representation"
    )]
    UnsupportedColumnType {
        table: String,
        column: String,
        sql_type: String,
    },
    #[error(
        "cannot generate MySQL ENUM column {table}.{column}: label {label:?} cannot be represented as a distinct fieldless Rust enum variant"
    )]
    UnsupportedEnumLabel {
        table: String,
        column: String,
        label: String,
    },
    #[error(
        "cannot generate MySQL SET column {table}.{column}: label {label:?} cannot be represented losslessly by MySQLTable"
    )]
    UnsupportedSetLabel {
        table: String,
        column: String,
        label: String,
    },
}

/// Options for Rust schema generation.
#[derive(Debug, Clone, Default)]
pub struct CodegenOptions {
    /// Optional module-level documentation placed after the generated header.
    pub module_doc: Option<String>,
    /// Whether to include a `#[derive(MySQLSchema)]` declaration.
    pub include_schema: bool,
    /// Name of the generated schema type. Empty uses `AppSchema`.
    pub schema_name: String,
    /// Whether generated declarations should be public.
    pub use_pub: bool,
    /// Casing strategy for generated Rust field names.
    pub field_casing: FieldCasing,
}

/// Casing strategy for generated Rust field names.
#[derive(Debug, Clone, Copy, Default)]
pub enum FieldCasing {
    /// `snake_case` (default).
    #[default]
    Snake,
    /// `camelCase`.
    Camel,
    /// Preserve source casing where it is a valid Rust identifier.
    Preserve,
}

type TableKey = (Option<String>, String);
type ColumnKey = (Option<String>, String, String);

#[derive(Default)]
struct IdentifierAllocator {
    used: HashSet<String>,
}

impl IdentifierAllocator {
    fn allocate(&mut self, preferred: &str, fallback: &str) -> String {
        let base = sanitize_rust_identifier(preferred, fallback);
        if self.used.insert(base.clone()) {
            return base;
        }

        let mut suffix = 2usize;
        loop {
            let candidate = format!("{base}_{suffix}");
            if self.used.insert(candidate.clone()) {
                return candidate;
            }
            suffix += 1;
        }
    }
}

#[derive(Default)]
struct NameMaps {
    tables: HashMap<TableKey, String>,
    columns: HashMap<ColumnKey, String>,
    table_columns: HashMap<TableKey, Vec<String>>,
    enum_types: HashMap<ColumnKey, String>,
    indexable_columns: HashMap<ColumnKey, bool>,
    prefix_columns: HashMap<ColumnKey, bool>,
}

impl NameMaps {
    fn table(&self, database: Option<&str>, table: &str) -> Option<&str> {
        self.tables
            .get(&table_key(database, table))
            .map(String::as_str)
    }

    fn column(&self, database: Option<&str>, table: &str, column: &str) -> Option<&str> {
        self.columns
            .get(&column_key(database, table, column))
            .map(String::as_str)
    }

    fn enum_type(&self, database: Option<&str>, table: &str, column: &str) -> Option<&str> {
        self.enum_types
            .get(&column_key(database, table, column))
            .map(String::as_str)
    }

    fn is_indexable(&self, database: Option<&str>, table: &str, column: &str) -> Option<bool> {
        self.indexable_columns
            .get(&column_key(database, table, column))
            .copied()
    }

    fn supports_prefix(&self, database: Option<&str>, table: &str, column: &str) -> Option<bool> {
        self.prefix_columns
            .get(&column_key(database, table, column))
            .copied()
    }

    fn first_column(&self, database: Option<&str>, table: &str) -> Option<&str> {
        self.table_columns
            .get(&table_key(database, table))
            .and_then(|columns| columns.first())
            .map(String::as_str)
    }
}

#[derive(Debug, Clone)]
struct TypeInfo {
    attribute: String,
    snapshot_sql_type: String,
    rust_type: String,
    category: MySQLTypeCategory,
}

/// Generate Rust declarations that use the public MySQL schema macros.
///
/// A normal migration snapshot has already passed [`MySQLDDL::try_from_entities`].
/// This function also attempts that normalization so database qualification and
/// view defaults match the parser/macro snapshot path. Invalid directly-built
/// DDL and columns without a lossless public macro representation are rejected.
///
/// # Errors
///
/// Returns [`CodegenError`] when the snapshot is invalid or generating public
/// MySQL macros would change a column's SQL type or inline values.
pub fn generate_rust_schema(
    ddl: &MySQLDDL,
    options: &CodegenOptions,
) -> Result<GeneratedSchema, CodegenError> {
    let mut result = GeneratedSchema::default();
    let normalized = MySQLDDL::try_from_entities(ddl.to_entities())?;
    validate_column_types(&normalized)?;

    let mut code = String::new();
    write_module_header(&mut code, options);

    let (maps, mut type_names) = build_name_maps(&normalized, options.field_casing, &mut result);

    for table in normalized.tables.list() {
        let entities = normalized.table_entities(table.database.as_deref(), &table.name);
        for enum_code in generate_table_enums(table, &entities, &maps, options, &mut result) {
            code.push_str(&enum_code);
            code.push('\n');
        }
    }

    for table in normalized.tables.list() {
        let entities = normalized.table_entities(table.database.as_deref(), &table.name);
        code.push_str(&generate_table_struct(
            table,
            &entities,
            &maps,
            options,
            &mut result.warnings,
        ));
        code.push('\n');
        result.tables.push(table.name.to_string());
    }

    let mut generated_indexes = Vec::new();
    for index in normalized.indexes.list() {
        if let Some((type_name, index_code)) =
            generate_index_struct(index, &maps, options, &mut type_names, &mut result.warnings)
        {
            code.push_str(&index_code);
            code.push('\n');
            result.indexes.push(index.name.to_string());
            generated_indexes.push(type_name);
        }
    }

    let mut generated_views = Vec::new();
    for view in normalized.views.list() {
        let preferred = view.name.to_pascal_case();
        let type_name = type_names.allocate(&preferred, "GeneratedView");
        code.push_str(&generate_view_struct(
            view,
            &type_name,
            options,
            &mut result.warnings,
        ));
        code.push('\n');
        result.views.push(view.name.to_string());
        generated_views.push(type_name);
    }

    if options.include_schema {
        let preferred_schema_name = if options.schema_name.trim().is_empty() {
            "AppSchema"
        } else {
            options.schema_name.trim()
        };
        let schema_name = type_names.allocate(preferred_schema_name, "AppSchema");
        code.push_str(&generate_schema_struct(
            &schema_name,
            normalized.tables.list(),
            &maps,
            &generated_indexes,
            &generated_views,
            options,
        ));
    }

    result.code = code;
    Ok(result)
}

fn validate_column_types(ddl: &MySQLDDL) -> Result<(), CodegenError> {
    for column in ddl.columns.list() {
        match &column.inline_type {
            Some(InlineType::Enum(values)) => {
                let mut variants = HashSet::new();
                for label in &values.values {
                    let Some(variant) = enum_variant_identifier(label) else {
                        return Err(CodegenError::UnsupportedEnumLabel {
                            table: column.table.to_string(),
                            column: column.name.to_string(),
                            label: label.to_string(),
                        });
                    };
                    if !variants.insert(variant.trim_start_matches("r#").to_string()) {
                        return Err(CodegenError::UnsupportedEnumLabel {
                            table: column.table.to_string(),
                            column: column.name.to_string(),
                            label: label.to_string(),
                        });
                    }
                }
                if values.values.is_empty() {
                    return Err(CodegenError::UnsupportedEnumLabel {
                        table: column.table.to_string(),
                        column: column.name.to_string(),
                        label: String::new(),
                    });
                }
            }
            Some(InlineType::Set(values)) => {
                let invalid = values
                    .values
                    .iter()
                    .find(|label| label.is_empty() || label.contains(['\0', '\'', '\\']));
                if values.values.len() > 64 || invalid.is_some() {
                    return Err(CodegenError::UnsupportedSetLabel {
                        table: column.table.to_string(),
                        column: column.name.to_string(),
                        label: invalid.map_or_else(String::new, ToString::to_string),
                    });
                }
            }
            None if parse_standard_type(&column.sql_type).is_none() => {
                return Err(CodegenError::UnsupportedColumnType {
                    table: column.table.to_string(),
                    column: column.name.to_string(),
                    sql_type: column.sql_type.to_string(),
                });
            }
            None => {}
        }
    }
    Ok(())
}

fn build_name_maps(
    ddl: &MySQLDDL,
    field_casing: FieldCasing,
    result: &mut GeneratedSchema,
) -> (NameMaps, IdentifierAllocator) {
    let mut maps = NameMaps::default();
    let mut types = IdentifierAllocator::default();

    for table in ddl.tables.list() {
        let preferred = table.name.to_pascal_case();
        let rust_name = types.allocate(&preferred, "GeneratedTable");
        maps.tables
            .insert(table_key(table.database.as_deref(), &table.name), rust_name);

        let mut fields = IdentifierAllocator::default();
        for column in ddl
            .columns
            .for_table(table.database.as_deref(), &table.name)
        {
            let preferred = apply_field_casing(&column.name, field_casing);
            let rust_name = fields.allocate(&preferred, "column");
            maps.columns.insert(
                column_key(column.database.as_deref(), &column.table, &column.name),
                rust_name.clone(),
            );
            maps.table_columns
                .entry(table_key(table.database.as_deref(), &table.name))
                .or_default()
                .push(rust_name);
            maps.indexable_columns.insert(
                column_key(column.database.as_deref(), &column.table, &column.name),
                column_is_indexable(column),
            );
            maps.prefix_columns.insert(
                column_key(column.database.as_deref(), &column.table, &column.name),
                column_supports_index_prefix(column),
            );
        }
    }

    for table in ddl.tables.list() {
        let Some(table_type) = maps
            .table(table.database.as_deref(), &table.name)
            .map(str::to_string)
        else {
            continue;
        };
        for column in ddl
            .columns
            .for_table(table.database.as_deref(), &table.name)
        {
            let Some(InlineType::Enum(values)) = &column.inline_type else {
                continue;
            };
            if !inline_enum_is_representable(values.values.iter().map(|value| value.as_ref())) {
                result.warnings.push(format!(
                    "{}.{} is an inline ENUM whose labels cannot be represented by MySQLEnum variants",
                    table.name, column.name
                ));
                continue;
            }
            let column_part = column.name.to_pascal_case();
            let preferred = format!("{table_type}{column_part}Enum");
            let enum_name = types.allocate(&preferred, "GeneratedEnum");
            maps.enum_types.insert(
                column_key(column.database.as_deref(), &column.table, &column.name),
                enum_name,
            );
        }
    }

    (maps, types)
}

fn generate_table_enums(
    table: &Table,
    entities: &TableEntities<'_>,
    maps: &NameMaps,
    options: &CodegenOptions,
    result: &mut GeneratedSchema,
) -> Vec<String> {
    let mut code = Vec::new();
    let visibility = visibility(options.use_pub);
    for column in &entities.columns {
        let Some(InlineType::Enum(values)) = &column.inline_type else {
            continue;
        };
        let Some(enum_name) = maps.enum_type(table.database.as_deref(), &table.name, &column.name)
        else {
            continue;
        };

        let variants = values
            .values
            .iter()
            .map(|value| value.as_ref())
            .map(enum_variant_identifier)
            .collect::<Option<Vec<_>>>();
        let Some(variants) = variants else {
            // `build_name_maps` recorded the diagnostic. Keep this branch
            // defensive in case the enum representation rule changes.
            continue;
        };

        let mut enum_code = String::new();
        enum_code.push_str("#[derive(Clone, Copy, Debug, PartialEq, Eq, MySQLEnum)]\n");
        let _ = writeln!(enum_code, "{visibility}enum {enum_name} {{");
        for variant in variants {
            let _ = writeln!(enum_code, "    {variant},");
        }
        enum_code.push_str("}\n");
        result.enums.push(format!("{}.{}", table.name, column.name));
        code.push(enum_code);
    }
    code
}

fn generate_table_struct(
    table: &Table,
    entities: &TableEntities<'_>,
    maps: &NameMaps,
    options: &CodegenOptions,
    warnings: &mut Vec<String>,
) -> String {
    let table_type = maps
        .table(table.database.as_deref(), &table.name)
        .expect("name map contains every table");
    let mut attrs = vec![format!("NAME = \"{}\"", rust_literal(&table.name))];

    if let Some(database) = &table.database {
        attrs.push(format!("DATABASE = \"{}\"", rust_literal(database)));
    }
    if table.temporary {
        attrs.push("TEMPORARY".to_string());
    }
    if let Some(engine) = &table.engine {
        push_option_attribute(&mut attrs, warnings, table, "ENGINE", engine);
    }
    if let Some(charset) = &table.charset {
        push_option_attribute(&mut attrs, warnings, table, "DEFAULT_CHARSET", charset);
    }
    if let Some(collation) = &table.collation {
        push_option_attribute(&mut attrs, warnings, table, "COLLATE", collation);
    }
    if let Some(comment) = &table.comment {
        attrs.push(format!("COMMENT = \"{}\"", rust_literal(comment)));
    }
    for option in &table.options {
        warnings.push(format!(
            "table option {}={} on {} cannot be represented by MySQLTable",
            option.name, option.value, table.name
        ));
    }

    let primary_columns = primary_columns(entities, table, warnings);

    if table.temporary && !entities.foreign_keys.is_empty() {
        warnings.push(format!(
            "temporary table {} has foreign keys; MySQLTable rejects that combination, so the foreign keys were not generated",
            table.name
        ));
    } else {
        for foreign_key in &entities.foreign_keys {
            if let Some(attr) = foreign_key_attribute(
                foreign_key,
                table,
                entities,
                &primary_columns,
                maps,
                warnings,
            ) {
                attrs.push(attr);
            }
        }
    }
    for unique in &entities.uniques {
        if let Some(attr) = unique_attribute(unique, table, maps, warnings) {
            attrs.push(attr);
        }
    }
    for check in &entities.checks {
        attrs.push(check_attribute(check, table, warnings));
    }

    let single_table_uniques = single_table_unique_columns(entities);

    let mut code = String::new();
    write_table_attribute(&mut code, &attrs);
    let _ = writeln!(
        code,
        "{}struct {table_type} {{",
        visibility(options.use_pub)
    );
    for column in &entities.columns {
        let is_primary = primary_columns.contains(column.name.as_ref());
        let has_table_unique = single_table_uniques.contains(column.name.as_ref());
        code.push_str(&generate_column_field(
            table,
            column,
            is_primary,
            has_table_unique,
            maps,
            options,
            warnings,
        ));
    }
    code.push_str("}\n");
    code
}

fn generate_column_field(
    table: &Table,
    column: &Column,
    is_primary: bool,
    has_table_unique: bool,
    maps: &NameMaps,
    options: &CodegenOptions,
    warnings: &mut Vec<String>,
) -> String {
    let field_name = maps
        .column(column.database.as_deref(), &column.table, &column.name)
        .expect("name map contains every column");
    let mut attrs = vec![format!("NAME = \"{}\"", rust_literal(&column.name))];
    let type_info = column_type_info(table, column, maps);
    attrs.push(type_info.attribute.clone());
    if macro_type_is_lossless(column, maps)
        && column.sql_type.as_ref() != type_info.snapshot_sql_type
    {
        warnings.push(format!(
            "MySQL type `{}` on {}.{} is emitted as macro-canonical `{}`; the exact original spelling cannot be preserved",
            column.sql_type, table.name, column.name, type_info.snapshot_sql_type
        ));
    }

    let mut emit_primary = is_primary;
    if emit_primary && !column_is_indexable(column) {
        warnings.push(format!(
            "primary-key column {}.{} needs an unsupported MySQL index prefix; PRIMARY was not generated",
            table.name, column.name
        ));
        emit_primary = false;
    }

    if emit_primary {
        attrs.push("PRIMARY".to_string());
        if !column.not_null {
            warnings.push(format!(
                "primary-key column {}.{} was nullable; generated Rust field is non-null because MySQLTable rejects nullable primary keys",
                table.name, column.name
            ));
        }
        if column.unique {
            warnings.push(format!(
                "column {}.{} is both PRIMARY and UNIQUE; MySQLTable emits PRIMARY only, so the redundant UNIQUE marker is not preserved",
                table.name, column.name
            ));
        }
    }

    let mut inline_unique = column.unique && !emit_primary;
    if inline_unique && !column_is_indexable(column) {
        warnings.push(format!(
            "unique column {}.{} needs an unsupported MySQL index prefix; UNIQUE was not generated",
            table.name, column.name
        ));
        inline_unique = false;
    }
    let mut emit_autoincrement = column.autoincrement;
    let emit_generated = column.generated.is_some();
    let mut emit_default = column.default.is_some();
    let mut emit_on_update = column.on_update.is_some();

    if emit_generated {
        if emit_autoincrement {
            warnings.push(format!(
                "generated column {}.{} cannot also be AUTO_INCREMENT; omitting AUTO_INCREMENT",
                table.name, column.name
            ));
            emit_autoincrement = false;
        }
        if emit_default {
            warnings.push(format!(
                "generated column {}.{} cannot also have DEFAULT; omitting DEFAULT",
                table.name, column.name
            ));
            emit_default = false;
        }
        if emit_on_update {
            warnings.push(format!(
                "generated column {}.{} cannot also have ON_UPDATE; omitting ON_UPDATE",
                table.name, column.name
            ));
            emit_on_update = false;
        }
    }

    if emit_autoincrement && !(emit_primary || inline_unique) {
        if has_table_unique {
            // The macro validates this at the field level. Keep the emitted
            // code valid, but make the duplicated physical uniqueness visible.
            inline_unique = true;
            warnings.push(format!(
                "AUTO_INCREMENT column {}.{} is keyed only by a table-level UNIQUE constraint; emitted field UNIQUE too because MySQLTable requires it",
                table.name, column.name
            ));
        } else {
            warnings.push(format!(
                "AUTO_INCREMENT column {}.{} is not primary or unique; omitting AUTO_INCREMENT because MySQLTable rejects it",
                table.name, column.name
            ));
            emit_autoincrement = false;
        }
    }

    if inline_unique {
        attrs.push("UNIQUE".to_string());
    }
    if emit_autoincrement {
        attrs.push("AUTO_INCREMENT".to_string());
    }
    if emit_generated && let Some(generated) = &column.generated {
        let kind = match generated.generation_type {
            GeneratedType::Stored => "STORED",
            GeneratedType::Virtual => "VIRTUAL",
        };
        attrs.push(format!(
            "generated({kind}, \"{}\")",
            rust_literal(&generated.expression)
        ));
    }
    let mut unsupported_default = None;
    if emit_default && let Some(default) = &column.default {
        match default_attribute(default) {
            Some(attribute) => attrs.push(attribute),
            None => {
                warnings.push(format!(
                    "default `{default}` on {}.{} cannot be expressed as `DEFAULT = ...`; omitting it",
                    table.name, column.name
                ));
                unsupported_default = Some(default.as_ref());
            }
        }
    }
    if emit_on_update {
        if type_supports_on_update(&type_info.category) {
            if let Some(on_update) = &column.on_update {
                attrs.push(format!("ON_UPDATE = \"{}\"", rust_literal(on_update)));
            }
        } else {
            warnings.push(format!(
                "ON_UPDATE on {}.{} requires DATETIME or TIMESTAMP; omitting it",
                table.name, column.name
            ));
        }
    }

    if let Some(charset) = &column.charset {
        if type_supports_character_options(&type_info.category) {
            attrs.push(format!("CHARSET = \"{}\"", rust_literal(charset)));
        } else {
            warnings.push(format!(
                "CHARSET on {}.{} is not supported for its generated MySQL type",
                table.name, column.name
            ));
        }
    }
    if let Some(collation) = &column.collation {
        if type_supports_character_options(&type_info.category) {
            attrs.push(format!("COLLATE = \"{}\"", rust_literal(collation)));
        } else {
            warnings.push(format!(
                "COLLATE on {}.{} is not supported for its generated MySQL type",
                table.name, column.name
            ));
        }
    }
    if let Some(comment) = &column.comment {
        attrs.push(format!("COMMENT = \"{}\"", rust_literal(comment)));
    }

    let not_null = column.not_null || emit_primary;
    let rust_type = nullable_type(type_info.rust_type, not_null);
    format!(
        "{}    #[column({})]\n    {}{}: {},\n",
        unsupported_default
            .map(|default| unsupported_default_comment("    ", default))
            .unwrap_or_default(),
        attrs.join(", "),
        visibility(options.use_pub),
        field_name,
        rust_type
    )
}

fn generate_index_struct(
    index: &Index,
    maps: &NameMaps,
    options: &CodegenOptions,
    type_names: &mut IdentifierAllocator,
    warnings: &mut Vec<String>,
) -> Option<(String, String)> {
    if index.columns.is_empty() {
        warnings.push(format!(
            "index {} has no columns and was not generated",
            index.name
        ));
        return None;
    }
    if index.comment.is_some() {
        warnings.push(format!(
            "index comment on {} cannot be represented by MySQLIndex; index was not generated",
            index.name
        ));
        return None;
    }
    if index.visible.is_some() {
        warnings.push(format!(
            "index visibility on {} cannot be represented by MySQLIndex; index was not generated",
            index.name
        ));
        return None;
    }
    if index
        .columns
        .iter()
        .any(|column| column.is_expression && column.length.is_some() || column.length == Some(0))
    {
        warnings.push(format!(
            "index {} has an invalid functional or zero-length prefix key part; index was not generated",
            index.name
        ));
        return None;
    }

    let preferred = index.name.to_pascal_case();
    let candidate = sanitize_rust_identifier(&preferred, "GeneratedIndex");
    let Some(table_type) = maps.table(index.database.as_deref(), &index.table) else {
        warnings.push(format!(
            "index {} belongs to missing table {}; index was not generated",
            index.name, index.table
        ));
        return None;
    };
    let mut columns = Vec::with_capacity(index.columns.len());
    for index_column in &index.columns {
        let mut key_attrs = Vec::new();
        let field = if index_column.is_expression {
            key_attrs.push(format!(
                "expr = \"{}\"",
                rust_literal(&index_column.expression)
            ));
            let Some(field) = maps.first_column(index.database.as_deref(), &index.table) else {
                warnings.push(format!(
                    "functional index {} belongs to a table with no witness column; index was not generated",
                    index.name
                ));
                return None;
            };
            field
        } else {
            let Some(field) = maps.column(
                index.database.as_deref(),
                &index.table,
                &index_column.expression,
            ) else {
                warnings.push(format!(
                    "index {} references missing column {}; index was not generated",
                    index.name, index_column.expression
                ));
                return None;
            };
            if let Some(length) = index_column.length {
                if !maps
                    .supports_prefix(
                        index.database.as_deref(),
                        &index.table,
                        &index_column.expression,
                    )
                    .unwrap_or(false)
                {
                    warnings.push(format!(
                        "index {} uses a prefix length on unsupported column {}; index was not generated",
                        index.name, index_column.expression
                    ));
                    return None;
                }
                key_attrs.push(format!("prefix = {length}"));
            } else if !maps
                .is_indexable(
                    index.database.as_deref(),
                    &index.table,
                    &index_column.expression,
                )
                .unwrap_or(false)
            {
                warnings.push(format!(
                    "index {} references {} which needs an explicit MySQL index prefix; index was not generated",
                    index.name, index_column.expression
                ));
                return None;
            }
            field
        };
        if let Some(ascending) = index_column.ascending {
            key_attrs.push(if ascending { "asc" } else { "desc" }.to_string());
        }
        let column = format!("{table_type}::{field}");
        columns.push(if key_attrs.is_empty() {
            column
        } else {
            format!("#[index({})] {column}", key_attrs.join(", "))
        });
    }

    let type_name = type_names.allocate(&candidate, "GeneratedIndex");

    let mut attrs = Vec::new();
    if type_name.to_snake_case() != index.name.as_ref() {
        attrs.push(format!("NAME = \"{}\"", rust_literal(&index.name)));
    }
    if index.unique {
        attrs.push("unique".to_string());
    }
    if let Some(using) = index.using {
        attrs.push(format!("using = \"{}\"", index_method(using)));
    }
    if let Some(algorithm) = index.algorithm {
        attrs.push(format!("algorithm = \"{}\"", index_algorithm(algorithm)));
    }
    if let Some(lock) = index.lock {
        attrs.push(format!("lock = \"{}\"", index_lock(lock)));
    }

    let mut code = String::new();
    if attrs.is_empty() {
        code.push_str("#[MySQLIndex]\n");
    } else {
        let _ = writeln!(code, "#[MySQLIndex({})]", attrs.join(", "));
    }
    let _ = writeln!(
        code,
        "{}struct {}({});",
        visibility(options.use_pub),
        type_name,
        columns.join(", ")
    );
    Some((type_name, code))
}

fn generate_view_struct(
    view: &View,
    type_name: &str,
    options: &CodegenOptions,
    warnings: &mut Vec<String>,
) -> String {
    let mut attrs = vec![format!("NAME = \"{}\"", rust_literal(&view.name))];
    if let Some(database) = &view.database {
        attrs.push(format!("DATABASE = \"{}\"", rust_literal(database)));
    }

    if view.is_existing {
        attrs.push("EXISTING".to_string());
        if view.definition.is_some() {
            warnings.push(format!(
                "existing view {} also carried a definition; MySQLView can only emit EXISTING, so the definition was omitted",
                view.name
            ));
        }
    } else if let Some(definition) = &view.definition {
        attrs.push(format!("DEFINITION = \"{}\"", rust_literal(definition)));
    } else {
        warnings.push(format!(
            "view {} has no definition and was emitted as EXISTING to keep generated Rust valid",
            view.name
        ));
        attrs.push("EXISTING".to_string());
    }

    if let Some(algorithm) = view.algorithm {
        attrs.push(format!("ALGORITHM = \"{}\"", view_algorithm(algorithm)));
    }
    if let Some(security) = view.sql_security {
        attrs.push(format!("SQL_SECURITY = \"{}\"", view_security(security)));
    }
    if let Some(check_option) = view.check_option {
        attrs.push(format!(
            "CHECK_OPTION = \"{}\"",
            view_check_option(check_option)
        ));
    }
    if view.definer.is_some() {
        warnings.push(format!(
            "view definer on {} cannot be represented by MySQLView",
            view.name
        ));
    }
    if view.charset.is_some() || view.collation.is_some() {
        warnings.push(format!(
            "view charset/collation on {} cannot be represented by MySQLView",
            view.name
        ));
    }

    let mut code = String::new();
    write_view_attribute(&mut code, &attrs);
    let _ = writeln!(
        code,
        "{}struct {type_name} {{}}",
        visibility(options.use_pub)
    );
    code
}

fn generate_schema_struct(
    schema_name: &str,
    tables: &[Table],
    maps: &NameMaps,
    indexes: &[String],
    views: &[String],
    options: &CodegenOptions,
) -> String {
    let mut fields = IdentifierAllocator::default();
    let mut members = Vec::new();
    for table in tables {
        let table_type = maps
            .table(table.database.as_deref(), &table.name)
            .expect("name map contains every table");
        let name = fields.allocate(
            &apply_field_casing(table_type, options.field_casing),
            "table",
        );
        members.push((name, table_type.to_string()));
    }
    for index in indexes {
        let name = fields.allocate(&apply_field_casing(index, options.field_casing), "index");
        members.push((name, index.clone()));
    }
    for view in views {
        let name = fields.allocate(&apply_field_casing(view, options.field_casing), "view");
        members.push((name, view.clone()));
    }

    let mut code = String::new();
    code.push_str("#[derive(MySQLSchema)]\n");
    let _ = writeln!(
        code,
        "{}struct {schema_name} {{",
        visibility(options.use_pub)
    );
    for (name, ty) in members {
        let _ = writeln!(code, "    {}{name}: {ty},", visibility(options.use_pub));
    }
    code.push_str("}\n");
    code
}

fn write_module_header(code: &mut String, options: &CodegenOptions) {
    code.push_str("//! Auto-generated MySQL schema from introspection\n");
    code.push_str("//!\n");
    if let Some(doc) = &options.module_doc {
        for line in doc.lines() {
            let _ = writeln!(code, "//! {line}");
        }
    }
    code.push('\n');
    code.push_str("use drizzle::mysql::prelude::*;\n\n");
}

fn write_table_attribute(code: &mut String, attrs: &[String]) {
    code.push_str("#[MySQLTable(\n");
    for attr in attrs {
        let _ = writeln!(code, "    {attr},");
    }
    code.push_str(")]\n");
}

fn write_view_attribute(code: &mut String, attrs: &[String]) {
    code.push_str("#[MySQLView(\n");
    for attr in attrs {
        let _ = writeln!(code, "    {attr},");
    }
    code.push_str(")]\n");
}

fn primary_columns(
    entities: &TableEntities<'_>,
    table: &Table,
    warnings: &mut Vec<String>,
) -> HashSet<String> {
    let mut primary = HashSet::new();
    if let Some(key) = entities.primary_key {
        if key.name.as_deref().is_some_and(|name| name != "PRIMARY") {
            warnings.push(format!(
                "primary-key name on {} is normalized by MySQLTable and cannot be preserved",
                table.name
            ));
        }
        for column in &key.columns {
            primary.insert(column.to_string());
        }

        for column in &entities.columns {
            if primary.contains(column.name.as_ref()) && !column.primary_key {
                warnings.push(format!(
                    "primary-key entity on {}.{} is represented by PRIMARY in MySQLTable, which normalizes Column.primary_key to true on the next parser round trip",
                    table.name, column.name
                ));
            }
        }

        let declared_order = entities
            .columns
            .iter()
            .filter(|column| primary.contains(column.name.as_ref()))
            .map(|column| column.name.to_string())
            .collect::<Vec<_>>();
        let key_order = key
            .columns
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        if declared_order != key_order {
            warnings.push(format!(
                "composite primary-key order on {} cannot be represented independently of field declaration order",
                table.name
            ));
        }
    }
    for column in &entities.columns {
        if column.primary_key && primary.insert(column.name.to_string()) {
            warnings.push(format!(
                "column {}.{} had PRIMARY_KEY metadata without a PrimaryKey entity; generated it as PRIMARY",
                table.name, column.name
            ));
        }
    }
    primary
}

fn single_table_unique_columns(entities: &TableEntities<'_>) -> HashSet<String> {
    entities
        .uniques
        .iter()
        .filter(|unique| unique.columns.len() == 1)
        .map(|unique| unique.columns[0].to_string())
        .collect()
}

fn foreign_key_attribute(
    foreign_key: &ForeignKey,
    table: &Table,
    entities: &TableEntities<'_>,
    primary_columns: &HashSet<String>,
    maps: &NameMaps,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let mut source_columns = Vec::with_capacity(foreign_key.columns.len());
    for column in &foreign_key.columns {
        let Some(field) = maps.column(foreign_key.database.as_deref(), &foreign_key.table, column)
        else {
            warnings.push(format!(
                "foreign key {} on {} references missing source column {}; it was not generated",
                foreign_key.name, table.name, column
            ));
            return None;
        };
        source_columns.push(field);
    }
    let Some(target_table) = maps.table(
        foreign_key.foreign_database.as_deref(),
        &foreign_key.foreign_table,
    ) else {
        warnings.push(format!(
            "foreign key {} on {} references missing table {}; it was not generated",
            foreign_key.name, table.name, foreign_key.foreign_table
        ));
        return None;
    };
    let mut target_columns = Vec::with_capacity(foreign_key.foreign_columns.len());
    for column in &foreign_key.foreign_columns {
        let Some(field) = maps.column(
            foreign_key.foreign_database.as_deref(),
            &foreign_key.foreign_table,
            column,
        ) else {
            warnings.push(format!(
                "foreign key {} on {} references missing target column {}.{}; it was not generated",
                foreign_key.name, table.name, foreign_key.foreign_table, column
            ));
            return None;
        };
        target_columns.push(field);
    }
    if source_columns.len() != target_columns.len() || source_columns.is_empty() {
        warnings.push(format!(
            "foreign key {} on {} has invalid column cardinality and was not generated",
            foreign_key.name, table.name
        ));
        return None;
    }

    if matches!(foreign_key.on_delete, Some(ReferentialAction::SetNull))
        || matches!(foreign_key.on_update, Some(ReferentialAction::SetNull))
    {
        let has_required_source = foreign_key.columns.iter().any(|source| {
            primary_columns.contains(source.as_ref())
                || entities
                    .columns
                    .iter()
                    .find(|column| column.name == *source)
                    .is_none_or(|column| column.not_null)
        });
        if has_required_source {
            warnings.push(format!(
                "foreign key {} on {} uses SET NULL for a non-nullable source column; MySQLTable rejects it, so it was not generated",
                foreign_key.name, table.name
            ));
            return None;
        }
    }

    let expected_name = format!("{}_{}_fkey", table.name, foreign_key.columns[0]);
    if foreign_key.name != expected_name {
        warnings.push(format!(
            "foreign-key name {} on {} cannot be preserved; MySQLTable derives `{expected_name}`",
            foreign_key.name, table.name
        ));
    }

    let mut args = vec![
        format!("columns({})", source_columns.join(", ")),
        format!("references({target_table}, {})", target_columns.join(", ")),
    ];
    if let Some(action) = foreign_key.on_delete {
        args.push(format!("on_delete = \"{}\"", referential_action(action)));
    }
    if let Some(action) = foreign_key.on_update {
        args.push(format!("on_update = \"{}\"", referential_action(action)));
    }
    Some(format!("FOREIGN_KEY({})", args.join(", ")))
}

fn unique_attribute(
    unique: &UniqueConstraint,
    table: &Table,
    maps: &NameMaps,
    warnings: &mut Vec<String>,
) -> Option<String> {
    let mut columns = Vec::with_capacity(unique.columns.len());
    for column in &unique.columns {
        let Some(field) = maps.column(unique.database.as_deref(), &unique.table, column) else {
            warnings.push(format!(
                "unique constraint {} on {} references missing column {}; it was not generated",
                unique.name, table.name, column
            ));
            return None;
        };
        let indexable = maps
            .is_indexable(unique.database.as_deref(), &unique.table, column)
            .unwrap_or(false);
        if !indexable {
            warnings.push(format!(
                "unique constraint {} on {} references {} which needs an unsupported MySQL index prefix; it was not generated",
                unique.name, table.name, column
            ));
            return None;
        }
        columns.push(field);
    }
    if columns.is_empty() {
        warnings.push(format!(
            "unique constraint {} on {} has no columns and was not generated",
            unique.name, table.name
        ));
        return None;
    }
    Some(format!(
        "UNIQUE(columns({}), name = \"{}\")",
        columns.join(", "),
        rust_literal(&unique.name)
    ))
}

fn check_attribute(check: &CheckConstraint, table: &Table, warnings: &mut Vec<String>) -> String {
    if check.enforced.is_some() {
        warnings.push(format!(
            "CHECK enforcement state on {}.{} cannot be represented by MySQLTable",
            table.name, check.name
        ));
    }
    format!(
        "CHECK(name = \"{}\", expr = \"{}\")",
        rust_literal(&check.name),
        rust_literal(&check.expression)
    )
}

fn column_type_info(table: &Table, column: &Column, maps: &NameMaps) -> TypeInfo {
    match &column.inline_type {
        Some(InlineType::Enum(values)) => {
            if let Some(enum_type) =
                maps.enum_type(table.database.as_deref(), &table.name, &column.name)
            {
                return TypeInfo {
                    attribute: "ENUM".to_string(),
                    snapshot_sql_type: inline_sql_type(
                        "ENUM",
                        values.values.iter().map(AsRef::as_ref),
                    ),
                    rust_type: enum_type.to_string(),
                    category: MySQLTypeCategory::Enum,
                };
            }
            unreachable!("validate_column_types accepted an ENUM without a generated enum type")
        }
        Some(InlineType::Set(values)) => {
            if inline_set_is_representable(values.values.iter().map(|value| value.as_ref())) {
                let inline_values = values.values.iter().map(AsRef::as_ref).collect::<Vec<_>>();
                let attribute_values = inline_values
                    .iter()
                    .copied()
                    .map(|value| format!("\"{}\"", rust_literal(value)))
                    .collect::<Vec<_>>();
                return TypeInfo {
                    attribute: format!("SET({})", attribute_values.join(", ")),
                    snapshot_sql_type: inline_sql_type("SET", inline_values.into_iter()),
                    rust_type: "String".to_string(),
                    category: MySQLTypeCategory::Set,
                };
            }
            unreachable!("validate_column_types accepted an unrepresentable SET")
        }
        None => parse_standard_type(&column.sql_type)
            .expect("validate_column_types accepted a standard MySQL type"),
    }
}

fn macro_type_is_lossless(column: &Column, maps: &NameMaps) -> bool {
    match &column.inline_type {
        Some(InlineType::Enum(_)) => maps
            .enum_type(column.database.as_deref(), &column.table, &column.name)
            .is_some(),
        Some(InlineType::Set(values)) => {
            inline_set_is_representable(values.values.iter().map(AsRef::as_ref))
        }
        None => parse_standard_type(&column.sql_type).is_some(),
    }
}

fn inline_sql_type<'a>(kind: &str, values: impl Iterator<Item = &'a str>) -> String {
    format!(
        "{kind}({})",
        values
            .map(|value| format!("'{value}'"))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn attribute_sql_type(attribute: &str) -> String {
    let (base, args) = attribute
        .split_once('(')
        .map_or((attribute, ""), |(base, _)| {
            (base, &attribute[base.len()..])
        });
    base.strip_suffix("_UNSIGNED").map_or_else(
        || attribute.to_string(),
        |base| format!("{base}{args} UNSIGNED"),
    )
}

fn parse_standard_type(sql_type: &str) -> Option<TypeInfo> {
    let (base, args, trailing) = split_type(sql_type)?;
    let base = normalize_type_name(&base)?;
    let trailing = normalize_words(&trailing);
    if base == "TINYINT" && args == [1] && trailing.is_empty() {
        return Some(TypeInfo {
            attribute: "BOOLEAN".to_string(),
            snapshot_sql_type: "BOOLEAN".to_string(),
            rust_type: "bool".to_string(),
            category: MySQLTypeCategory::Boolean,
        });
    }
    let (attribute_base, category, rust_type) = match base.as_str() {
        "TINYINT" => integer_type(&trailing, "TINYINT", MySQLTypeCategory::TinyInt, "i8", "u8")?,
        "SMALLINT" => integer_type(
            &trailing,
            "SMALLINT",
            MySQLTypeCategory::SmallInt,
            "i16",
            "u16",
        )?,
        "MEDIUMINT" => integer_type(
            &trailing,
            "MEDIUMINT",
            MySQLTypeCategory::MediumInt,
            "i32",
            "u32",
        )?,
        "INT" => integer_type(&trailing, "INT", MySQLTypeCategory::Int, "i32", "u32")?,
        "BIGINT" => integer_type(&trailing, "BIGINT", MySQLTypeCategory::BigInt, "i64", "u64")?,
        "DECIMAL" => numeric_type(&trailing, "DECIMAL", MySQLTypeCategory::Decimal, "String")?,
        "FLOAT" => numeric_type(&trailing, "FLOAT", MySQLTypeCategory::Float, "f32")?,
        "DOUBLE" => numeric_type(&trailing, "DOUBLE", MySQLTypeCategory::Double, "f64")?,
        "REAL" => numeric_type(&trailing, "REAL", MySQLTypeCategory::Double, "f64")?,
        "BOOLEAN" if trailing.is_empty() => (
            "BOOLEAN".to_string(),
            MySQLTypeCategory::Boolean,
            "bool".to_string(),
        ),
        "BIT" if trailing.is_empty() => (
            "BIT".to_string(),
            MySQLTypeCategory::Bit,
            "Vec<u8>".to_string(),
        ),
        "CHAR" if trailing.is_empty() => (
            "CHAR".to_string(),
            MySQLTypeCategory::Char,
            "String".to_string(),
        ),
        "VARCHAR" if trailing.is_empty() => (
            "VARCHAR".to_string(),
            MySQLTypeCategory::Varchar,
            "String".to_string(),
        ),
        "TINYTEXT" if trailing.is_empty() => (
            "TINYTEXT".to_string(),
            MySQLTypeCategory::TinyText,
            "String".to_string(),
        ),
        "TEXT" if trailing.is_empty() => (
            "TEXT".to_string(),
            MySQLTypeCategory::Text,
            "String".to_string(),
        ),
        "MEDIUMTEXT" if trailing.is_empty() => (
            "MEDIUMTEXT".to_string(),
            MySQLTypeCategory::MediumText,
            "String".to_string(),
        ),
        "LONGTEXT" if trailing.is_empty() => (
            "LONGTEXT".to_string(),
            MySQLTypeCategory::LongText,
            "String".to_string(),
        ),
        "BINARY" if trailing.is_empty() => (
            "BINARY".to_string(),
            MySQLTypeCategory::Binary,
            "Vec<u8>".to_string(),
        ),
        "VARBINARY" if trailing.is_empty() => (
            "VARBINARY".to_string(),
            MySQLTypeCategory::Varbinary,
            "Vec<u8>".to_string(),
        ),
        "TINYBLOB" if trailing.is_empty() => (
            "TINYBLOB".to_string(),
            MySQLTypeCategory::TinyBlob,
            "Vec<u8>".to_string(),
        ),
        "BLOB" if trailing.is_empty() => (
            "BLOB".to_string(),
            MySQLTypeCategory::Blob,
            "Vec<u8>".to_string(),
        ),
        "MEDIUMBLOB" if trailing.is_empty() => (
            "MEDIUMBLOB".to_string(),
            MySQLTypeCategory::MediumBlob,
            "Vec<u8>".to_string(),
        ),
        "LONGBLOB" if trailing.is_empty() => (
            "LONGBLOB".to_string(),
            MySQLTypeCategory::LongBlob,
            "Vec<u8>".to_string(),
        ),
        "JSON" if trailing.is_empty() => (
            "JSON".to_string(),
            MySQLTypeCategory::Json,
            "String".to_string(),
        ),
        "DATE" if trailing.is_empty() => (
            "DATE".to_string(),
            MySQLTypeCategory::Date,
            "String".to_string(),
        ),
        "TIME" if trailing.is_empty() => (
            "TIME".to_string(),
            MySQLTypeCategory::Time,
            "String".to_string(),
        ),
        "DATETIME" if trailing.is_empty() => (
            "DATETIME".to_string(),
            MySQLTypeCategory::DateTime,
            "String".to_string(),
        ),
        "TIMESTAMP" if trailing.is_empty() => (
            "TIMESTAMP".to_string(),
            MySQLTypeCategory::Timestamp,
            "String".to_string(),
        ),
        "YEAR" if trailing.is_empty() => (
            "YEAR".to_string(),
            MySQLTypeCategory::Year,
            "u16".to_string(),
        ),
        _ => return None,
    };

    if !type_arguments_are_valid(&attribute_base, &args) {
        return None;
    }
    let canonical_args = if matches!(
        attribute_base.as_str(),
        "TINYINT"
            | "TINYINT_UNSIGNED"
            | "SMALLINT"
            | "SMALLINT_UNSIGNED"
            | "MEDIUMINT"
            | "MEDIUMINT_UNSIGNED"
            | "INT"
            | "INT_UNSIGNED"
            | "BIGINT"
            | "BIGINT_UNSIGNED"
            | "YEAR"
    ) {
        &[][..]
    } else {
        args.as_slice()
    };
    let attribute = if canonical_args.is_empty() {
        attribute_base
    } else {
        format!(
            "{}({})",
            attribute_base,
            canonical_args
                .iter()
                .map(u16::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )
    };
    Some(TypeInfo {
        snapshot_sql_type: attribute_sql_type(&attribute),
        attribute,
        rust_type,
        category,
    })
}

fn integer_type(
    trailing: &str,
    base: &str,
    category: MySQLTypeCategory,
    signed: &str,
    unsigned: &str,
) -> Option<(String, MySQLTypeCategory, String)> {
    match trailing {
        "" => Some((base.to_string(), category, signed.to_string())),
        "UNSIGNED" => Some((
            format!("{base}_UNSIGNED"),
            unsigned_category(category),
            unsigned.to_string(),
        )),
        _ => None,
    }
}

fn numeric_type(
    trailing: &str,
    base: &str,
    category: MySQLTypeCategory,
    rust_type: &str,
) -> Option<(String, MySQLTypeCategory, String)> {
    match trailing {
        "" => Some((base.to_string(), category, rust_type.to_string())),
        "UNSIGNED" => Some((format!("{base}_UNSIGNED"), category, rust_type.to_string())),
        _ => None,
    }
}

fn unsigned_category(category: MySQLTypeCategory) -> MySQLTypeCategory {
    match category {
        MySQLTypeCategory::TinyInt => MySQLTypeCategory::TinyIntUnsigned,
        MySQLTypeCategory::SmallInt => MySQLTypeCategory::SmallIntUnsigned,
        MySQLTypeCategory::MediumInt => MySQLTypeCategory::MediumIntUnsigned,
        MySQLTypeCategory::Int => MySQLTypeCategory::IntUnsigned,
        MySQLTypeCategory::BigInt => MySQLTypeCategory::BigIntUnsigned,
        _ => category,
    }
}

fn split_type(sql_type: &str) -> Option<(String, Vec<u16>, String)> {
    let sql_type = sql_type.trim();
    if sql_type.is_empty() {
        return None;
    }
    let Some(open) = sql_type.find('(') else {
        let words = sql_type.split_whitespace().collect::<Vec<_>>();
        if words.last().is_some_and(|word| {
            word.eq_ignore_ascii_case("unsigned") || word.eq_ignore_ascii_case("zerofill")
        }) {
            return Some((
                words[..words.len() - 1].join(" "),
                Vec::new(),
                words.last().expect("checked nonempty").to_string(),
            ));
        }
        return Some((sql_type.to_string(), Vec::new(), String::new()));
    };
    let close = sql_type[open + 1..].find(')')? + open + 1;
    if sql_type[close + 1..].contains('(') || sql_type[close + 1..].contains(')') {
        return None;
    }
    let args = sql_type[open + 1..close]
        .split(',')
        .map(str::trim)
        .map(str::parse::<u16>)
        .collect::<Result<Vec<_>, _>>()
        .ok()?;
    Some((
        sql_type[..open].trim().to_string(),
        args,
        sql_type[close + 1..].trim().to_string(),
    ))
}

fn normalize_type_name(base: &str) -> Option<String> {
    match normalize_words(base).as_str() {
        "TINYINT" => Some("TINYINT".to_string()),
        "SMALLINT" => Some("SMALLINT".to_string()),
        "MEDIUMINT" => Some("MEDIUMINT".to_string()),
        "INT" | "INTEGER" => Some("INT".to_string()),
        "BIGINT" => Some("BIGINT".to_string()),
        "DECIMAL" | "NUMERIC" | "DEC" | "FIXED" => Some("DECIMAL".to_string()),
        "FLOAT" => Some("FLOAT".to_string()),
        "DOUBLE" | "DOUBLE PRECISION" => Some("DOUBLE".to_string()),
        "REAL" => Some("REAL".to_string()),
        "BOOLEAN" | "BOOL" => Some("BOOLEAN".to_string()),
        "BIT" => Some("BIT".to_string()),
        "CHAR" | "CHARACTER" => Some("CHAR".to_string()),
        "VARCHAR" | "CHARACTER VARYING" => Some("VARCHAR".to_string()),
        "TINYTEXT" => Some("TINYTEXT".to_string()),
        "TEXT" => Some("TEXT".to_string()),
        "MEDIUMTEXT" => Some("MEDIUMTEXT".to_string()),
        "LONGTEXT" => Some("LONGTEXT".to_string()),
        "BINARY" => Some("BINARY".to_string()),
        "VARBINARY" => Some("VARBINARY".to_string()),
        "TINYBLOB" => Some("TINYBLOB".to_string()),
        "BLOB" => Some("BLOB".to_string()),
        "MEDIUMBLOB" => Some("MEDIUMBLOB".to_string()),
        "LONGBLOB" => Some("LONGBLOB".to_string()),
        "JSON" => Some("JSON".to_string()),
        "DATE" => Some("DATE".to_string()),
        "TIME" => Some("TIME".to_string()),
        "DATETIME" => Some("DATETIME".to_string()),
        "TIMESTAMP" => Some("TIMESTAMP".to_string()),
        "YEAR" => Some("YEAR".to_string()),
        _ => None,
    }
}

fn normalize_words(value: &str) -> String {
    value
        .split_whitespace()
        .map(str::to_ascii_uppercase)
        .collect::<Vec<_>>()
        .join(" ")
}

fn type_arguments_are_valid(attribute: &str, args: &[u16]) -> bool {
    MySQLType::parse_attribute(attribute).is_some_and(|ty| ty.validate_args(args).is_none())
}

pub(super) fn canonical_sql_type(sql_type: &str) -> Option<String> {
    parse_standard_type(sql_type).map(|info| info.snapshot_sql_type)
}

fn inline_enum_is_representable<'a>(mut values: impl Iterator<Item = &'a str>) -> bool {
    let mut seen = HashSet::new();
    let mut count = 0usize;
    let valid = values.all(|value| {
        count += 1;
        enum_variant_identifier(value)
            .is_some_and(|variant| seen.insert(variant.trim_start_matches("r#").to_string()))
    });
    valid && count > 0
}

fn inline_set_is_representable<'a>(values: impl Iterator<Item = &'a str>) -> bool {
    let values = values.collect::<Vec<_>>();
    !values.is_empty()
        && values.len() <= 64
        && values.iter().all(|value| !value.contains(['\'', '\\']))
}

fn enum_variant_identifier(value: &str) -> Option<String> {
    if value.is_empty() || value.contains(['\'', '\\']) || !is_plain_identifier(value) {
        return None;
    }
    if matches!(value, "self" | "Self" | "super" | "crate") {
        return None;
    }
    if is_rust_keyword(value) {
        Some(format!("r#{value}"))
    } else {
        Some(value.to_string())
    }
}

fn default_attribute(default: &str) -> Option<String> {
    default_expression(default).map(|expression| format!("DEFAULT = {expression}"))
}

fn type_supports_on_update(category: &MySQLTypeCategory) -> bool {
    matches!(
        category,
        MySQLTypeCategory::DateTime | MySQLTypeCategory::Timestamp
    )
}

fn type_supports_character_options(category: &MySQLTypeCategory) -> bool {
    matches!(
        category,
        MySQLTypeCategory::Char
            | MySQLTypeCategory::Varchar
            | MySQLTypeCategory::TinyText
            | MySQLTypeCategory::Text
            | MySQLTypeCategory::MediumText
            | MySQLTypeCategory::LongText
            | MySQLTypeCategory::Enum
            | MySQLTypeCategory::Set
    )
}

fn column_is_indexable(column: &Column) -> bool {
    match &column.inline_type {
        Some(InlineType::Enum(values)) => {
            inline_enum_is_representable(values.values.iter().map(AsRef::as_ref))
        }
        Some(InlineType::Set(values)) => {
            inline_set_is_representable(values.values.iter().map(AsRef::as_ref))
        }
        None => parse_standard_type(&column.sql_type)
            .is_some_and(|info| type_is_indexable(&info.category)),
    }
}

fn column_supports_index_prefix(column: &Column) -> bool {
    if column.inline_type.is_some() {
        return false;
    }
    parse_standard_type(&column.sql_type).is_some_and(|info| {
        matches!(
            info.category,
            MySQLTypeCategory::Char
                | MySQLTypeCategory::Varchar
                | MySQLTypeCategory::TinyText
                | MySQLTypeCategory::Text
                | MySQLTypeCategory::MediumText
                | MySQLTypeCategory::LongText
                | MySQLTypeCategory::Binary
                | MySQLTypeCategory::Varbinary
                | MySQLTypeCategory::TinyBlob
                | MySQLTypeCategory::Blob
                | MySQLTypeCategory::MediumBlob
                | MySQLTypeCategory::LongBlob
        )
    })
}

fn type_is_indexable(category: &MySQLTypeCategory) -> bool {
    !matches!(
        category,
        MySQLTypeCategory::TinyText
            | MySQLTypeCategory::Text
            | MySQLTypeCategory::MediumText
            | MySQLTypeCategory::LongText
            | MySQLTypeCategory::TinyBlob
            | MySQLTypeCategory::Blob
            | MySQLTypeCategory::MediumBlob
            | MySQLTypeCategory::LongBlob
            | MySQLTypeCategory::Json
    )
}

fn push_option_attribute(
    attrs: &mut Vec<String>,
    warnings: &mut Vec<String>,
    table: &Table,
    key: &str,
    value: &str,
) {
    if is_mysql_option_identifier(value) {
        attrs.push(format!("{key} = \"{}\"", rust_literal(value)));
    } else {
        warnings.push(format!(
            "{key} value `{value}` on {} cannot be represented by MySQLTable",
            table.name
        ));
    }
}

fn is_mysql_option_identifier(value: &str) -> bool {
    !value.is_empty()
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
}

fn index_method(method: IndexMethod) -> &'static str {
    match method {
        IndexMethod::Btree => "btree",
        IndexMethod::Hash => "hash",
    }
}

fn index_algorithm(algorithm: IndexAlgorithm) -> &'static str {
    match algorithm {
        IndexAlgorithm::Default => "default",
        IndexAlgorithm::Inplace => "inplace",
        IndexAlgorithm::Copy => "copy",
    }
}

fn index_lock(lock: IndexLock) -> &'static str {
    match lock {
        IndexLock::Default => "default",
        IndexLock::None => "none",
        IndexLock::Shared => "shared",
        IndexLock::Exclusive => "exclusive",
    }
}

fn referential_action(action: ReferentialAction) -> &'static str {
    match action {
        ReferentialAction::Cascade => "CASCADE",
        ReferentialAction::SetNull => "SET NULL",
        ReferentialAction::Restrict => "RESTRICT",
        ReferentialAction::NoAction => "NO ACTION",
    }
}

fn view_algorithm(algorithm: ViewAlgorithm) -> &'static str {
    match algorithm {
        ViewAlgorithm::Undefined => "undefined",
        ViewAlgorithm::Merge => "merge",
        ViewAlgorithm::Temptable => "temptable",
    }
}

fn view_security(security: ViewSqlSecurity) -> &'static str {
    match security {
        ViewSqlSecurity::Definer => "definer",
        ViewSqlSecurity::Invoker => "invoker",
    }
}

fn view_check_option(option: ViewCheckOption) -> &'static str {
    match option {
        ViewCheckOption::Cascaded => "cascaded",
        ViewCheckOption::Local => "local",
    }
}

fn nullable_type(base: String, not_null: bool) -> String {
    if not_null {
        base
    } else {
        format!("Option<{base}>")
    }
}

fn visibility(use_pub: bool) -> &'static str {
    if use_pub { "pub " } else { "" }
}

fn rust_literal(value: &str) -> String {
    escape_for_rust_literal(value)
}

fn table_key(database: Option<&str>, table: &str) -> TableKey {
    (database.map(str::to_string), table.to_string())
}

fn column_key(database: Option<&str>, table: &str, column: &str) -> ColumnKey {
    (
        database.map(str::to_string),
        table.to_string(),
        column.to_string(),
    )
}

fn apply_field_casing(name: &str, casing: FieldCasing) -> String {
    match casing {
        FieldCasing::Snake => name.to_snake_case(),
        FieldCasing::Camel => name.to_lower_camel_case(),
        FieldCasing::Preserve => name.to_string(),
    }
}

fn sanitize_rust_identifier(name: &str, fallback: &str) -> String {
    let mut out = String::with_capacity(name.len());
    for (index, ch) in name.chars().enumerate() {
        let valid = if index == 0 {
            ch == '_' || ch.is_ascii_alphabetic()
        } else {
            ch == '_' || ch.is_ascii_alphanumeric()
        };
        out.push(if valid { ch } else { '_' });
    }
    if out.is_empty() || out == "_" {
        out = fallback.to_string();
    }
    if is_rust_keyword(&out) {
        out.push('_');
    }
    out
}

fn is_plain_identifier(value: &str) -> bool {
    value.chars().enumerate().all(|(index, ch)| {
        if index == 0 {
            ch == '_' || ch.is_ascii_alphabetic()
        } else {
            ch == '_' || ch.is_ascii_alphanumeric()
        }
    })
}

fn is_rust_keyword(value: &str) -> bool {
    matches!(
        value,
        "as" | "async"
            | "await"
            | "break"
            | "const"
            | "continue"
            | "crate"
            | "dyn"
            | "else"
            | "enum"
            | "extern"
            | "false"
            | "fn"
            | "for"
            | "if"
            | "impl"
            | "in"
            | "let"
            | "loop"
            | "match"
            | "mod"
            | "move"
            | "mut"
            | "pub"
            | "ref"
            | "return"
            | "self"
            | "Self"
            | "static"
            | "struct"
            | "super"
            | "trait"
            | "true"
            | "try"
            | "type"
            | "union"
            | "unsafe"
            | "use"
            | "where"
            | "while"
            | "yield"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_macro_representable_type_declarations() {
        let cases = [
            (
                "BIGINT UNSIGNED",
                "BIGINT_UNSIGNED",
                "u64",
                "BIGINT UNSIGNED",
            ),
            ("varchar(255)", "VARCHAR(255)", "String", "VARCHAR(255)"),
            (
                "DECIMAL(20, 8)",
                "DECIMAL(20, 8)",
                "String",
                "DECIMAL(20, 8)",
            ),
            (
                "DECIMAL(20, 8) UNSIGNED",
                "DECIMAL_UNSIGNED(20, 8)",
                "String",
                "DECIMAL(20, 8) UNSIGNED",
            ),
            (
                "FLOAT(10, 2) UNSIGNED",
                "FLOAT_UNSIGNED(10, 2)",
                "f32",
                "FLOAT(10, 2) UNSIGNED",
            ),
            (
                "DOUBLE(10, 2) UNSIGNED",
                "DOUBLE_UNSIGNED(10, 2)",
                "f64",
                "DOUBLE(10, 2) UNSIGNED",
            ),
            ("REAL(10, 2)", "REAL(10, 2)", "f64", "REAL(10, 2)"),
            (
                "REAL(10, 2) UNSIGNED",
                "REAL_UNSIGNED(10, 2)",
                "f64",
                "REAL(10, 2) UNSIGNED",
            ),
            ("DATETIME(6)", "DATETIME(6)", "String", "DATETIME(6)"),
        ];
        for (input, attribute, rust_type, snapshot_sql_type) in cases {
            let info = parse_standard_type(input).expect("representable type");
            assert_eq!(info.attribute, attribute);
            assert_eq!(info.rust_type, rust_type);
            assert_eq!(info.snapshot_sql_type, snapshot_sql_type);
        }
        let int = parse_standard_type("INT(11)").expect("display width is non-structural");
        assert_eq!(int.attribute, "INT");
        assert_eq!(int.snapshot_sql_type, "INT");
        let boolean = parse_standard_type("TINYINT(1)").expect("BOOLEAN catalog spelling");
        assert_eq!(boolean.attribute, "BOOLEAN");
        assert!(parse_standard_type("INT ZEROFILL").is_none());
        assert!(parse_standard_type("DECIMAL(66) UNSIGNED").is_none());
        assert!(parse_standard_type("FLOAT(24)").is_some());
        assert!(parse_standard_type("FLOAT(25)").is_none());
        assert!(parse_standard_type("FLOAT(255, 30)").is_some());
        assert!(parse_standard_type("FLOAT(256, 30)").is_none());
        assert!(parse_standard_type("REAL(10, 11)").is_none());
        assert!(parse_standard_type("DOUBLE(10)").is_none());
        assert!(parse_standard_type("REAL(10)").is_none());
    }

    #[test]
    fn enum_labels_must_be_lossless_rust_identifiers() {
        assert_eq!(enum_variant_identifier("draft"), Some("draft".to_string()));
        assert_eq!(enum_variant_identifier("type"), Some("r#type".to_string()));
        assert!(enum_variant_identifier("in progress").is_none());
        assert!(enum_variant_identifier("can't").is_none());
    }
}
