//! `PostgreSQL` const DDL generation
//!
//! This module generates compile-time DDL entity definitions for `PostgreSQL` tables.

use super::context::MacroContext;
use crate::paths::ddl::postgres as ddl_paths;
use crate::paths::{core as core_paths, postgres as postgres_paths};
use crate::postgres::field::{FieldInfo, PostgreSQLDefault};
use drizzle_types::postgres::ddl::{Column, PrimaryKey, Table, UniqueConstraint, sql::TableSql};
use proc_macro2::TokenStream;
use quote::quote;
use std::borrow::Cow;
use std::fmt::Write;
use syn::Ident;

/// Generate the CREATE TABLE SQL string from raw parameters.
///
/// This is the core implementation that doesn't require a full `MacroContext`.
/// Use this before context is fully constructed.
pub fn generate_create_table_sql_from_params(
    schema_name: &str,
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
) -> String {
    // Build Table (owned schema name because DDL runtime types use Cow<'static, str>)
    let table = Table::new(schema_name.to_string(), table_name.to_string());

    // Build Columns
    let columns: Vec<Column> = field_infos
        .iter()
        .map(|field| build_column(schema_name, table_name, field, is_composite_pk))
        .collect();

    // Build PrimaryKey
    let pk_columns: Vec<String> = field_infos
        .iter()
        .filter(|f| f.is_primary)
        .map(|f| f.column_name.clone())
        .collect();

    let primary_key = if pk_columns.is_empty() {
        None
    } else {
        let pk_name = format!("{table_name}_pkey");
        Some(PrimaryKey::from_strings(
            schema_name.to_string(),
            table_name.to_string(),
            pk_name,
            pk_columns,
        ))
    };

    // Build UniqueConstraints (single-column only, non-primary)
    let unique_constraints: Vec<UniqueConstraint> = field_infos
        .iter()
        .filter(|f| f.is_unique && !f.is_primary)
        .map(|field| {
            UniqueConstraint::from_strings(
                schema_name.to_string(),
                table_name.to_string(),
                format!("{}_{}_unique", table_name, field.column_name),
                vec![field.column_name.clone()],
            )
        })
        .collect();

    // Generate SQL (no foreign keys for compile-time generation)
    TableSql::new(&table)
        .columns(&columns)
        .primary_key(primary_key.as_ref())
        .foreign_keys(&[])
        .unique_constraints(&unique_constraints)
        .create_table_sql()
}

/// Generate the CREATE TABLE SQL string at proc-macro time.
///
/// This is used for tables WITHOUT foreign keys, where all information
/// is known at macro expansion time. Uses the same DDL types as runtime
/// generation for consistency.
#[allow(dead_code)]
pub fn generate_create_table_sql(ctx: &MacroContext) -> String {
    let schema_name = ctx.attrs.schema.as_deref().unwrap_or("public");
    generate_create_table_sql_from_params(
        schema_name,
        &ctx.table_name,
        ctx.field_infos,
        ctx.is_composite_pk,
    )
}

/// Build a Column from `FieldInfo`
fn build_column(
    schema_name: &str,
    table_name: &str,
    field: &FieldInfo,
    _is_composite_pk: bool,
) -> Column {
    let mut col = Column::new(
        Cow::Owned(schema_name.to_string()),
        Cow::Owned(table_name.to_string()),
        Cow::Owned(field.column_name.clone()),
        Cow::Owned(field.column_type.to_sql_type().to_string()),
    );

    col.not_null = !field.is_nullable;

    // Note: Serial columns use the SERIAL pseudo-type which handles auto-increment
    // via a sequence + DEFAULT nextval(...). We do NOT set identity for serial columns.
    // Identity columns (GENERATED AS IDENTITY) are only for explicit identity(always/by_default).

    // Note: Primary key is handled at the table level via PrimaryKey constraint
    // Not inline on the column for PostgreSQL DDL generation

    if field.is_unique && !field.is_primary {
        // Unique constraints are handled at the table level
        // but we could mark the column if needed
    }

    // Handle default value (skip if serial - SERIAL type has implicit DEFAULT)
    if !field.is_serial
        && let Some(ref default) = field.default
    {
        let default_str = match default {
            PostgreSQLDefault::Literal(s) | PostgreSQLDefault::Function(s) => s.clone(),
            PostgreSQLDefault::Expression(ts) => ts.to_string(),
        };
        col.default = Some(Cow::Owned(default_str));
    }

    col
}

/// Generate a compile-time `const SQL: &'static str` value for `SQLSchema`
/// using `concatcp!` so that foreign key REFERENCES can resolve table names
/// via `<OtherTable>::TABLE_NAME` at compile time.
pub fn generate_schema_sql_const(ctx: &MacroContext) -> TokenStream {
    let table_name = &ctx.table_name;
    let schema_name = ctx.attrs.schema.as_deref().unwrap_or("public");
    let is_composite_pk = ctx.is_composite_pk;
    let field_infos = ctx.field_infos;

    let has_foreign_keys = field_infos.iter().any(|f| f.foreign_key.is_some())
        || !ctx.attrs.composite_foreign_keys.is_empty();

    if !has_foreign_keys {
        // For tables without FKs, build the SQL entirely at proc-macro time
        let sql = generate_create_table_sql(ctx);
        return quote! { #sql };
    }

    // For tables WITH FKs, use concatcp! to reference other table's TABLE_NAME
    let mut parts: Vec<TokenStream> = Vec::new();

    // CREATE TABLE ["schema".]"table" (\n
    let header = if schema_name == "public" {
        format!("CREATE TABLE \"{table_name}\" (\n")
    } else {
        format!("CREATE TABLE \"{schema_name}\".\"{table_name}\" (\n")
    };
    parts.push(quote! { #header });

    // Column definitions
    let column_lines: Vec<String> = field_infos
        .iter()
        .map(|field| build_pg_column_sql(field, is_composite_pk))
        .collect();

    for (i, col_line) in column_lines.iter().enumerate() {
        let line = if i > 0 {
            format!(",\n{col_line}")
        } else {
            col_line.clone()
        };
        parts.push(quote! { #line });
    }

    // Primary key constraint
    let pk_columns: Vec<&String> = field_infos
        .iter()
        .filter(|f| f.is_primary)
        .map(|f| &f.column_name)
        .collect();
    if !pk_columns.is_empty() {
        let pk_str = format!(
            ",\n\tPRIMARY KEY({})",
            pk_columns
                .iter()
                .map(|c| format!("\"{c}\""))
                .collect::<Vec<_>>()
                .join(", ")
        );
        parts.push(quote! { #pk_str });
    }

    // Unique constraints
    for field in field_infos.iter().filter(|f| f.is_unique && !f.is_primary) {
        let uq_name = format!("{}_{}_unique", table_name, field.column_name);
        let uq_str = format!(
            ",\n\tCONSTRAINT \"{}\" UNIQUE(\"{}\")",
            uq_name, field.column_name
        );
        parts.push(quote! { #uq_str });
    }

    // Single-column foreign keys
    for field in field_infos {
        if let Some(ref fk) = field.foreign_key {
            let ref_table_ident = &fk.table;
            let ref_column = fk.column.to_string();
            let fk_name = format!("{}_{}_fkey", table_name, field.column_name);

            // FK prefix: ,\n\tCONSTRAINT "name" FOREIGN KEY ("col") REFERENCES "
            let fk_prefix = format!(
                ",\n\tCONSTRAINT \"{}\" FOREIGN KEY (\"{}\") REFERENCES \"",
                fk_name, field.column_name
            );
            parts.push(quote! { #fk_prefix });

            // Table name via const reference
            parts.push(quote! { <#ref_table_ident>::TABLE_NAME });

            // FK suffix: "("ref_col")
            let mut fk_suffix = format!("\"(\"{ref_column}\")");

            if let Some(ref on_delete) = fk.on_delete {
                let action = on_delete.to_uppercase();
                if action != "NO ACTION" {
                    let _ = write!(fk_suffix, " ON DELETE {action}");
                }
            }
            if let Some(ref on_update) = fk.on_update {
                let action = on_update.to_uppercase();
                if action != "NO ACTION" {
                    let _ = write!(fk_suffix, " ON UPDATE {action}");
                }
            }

            parts.push(quote! { #fk_suffix });
        }
    }

    // Composite foreign keys
    for fk in &ctx.attrs.composite_foreign_keys {
        let ref_table_ident = &fk.target_table;
        let source_columns: Vec<String> = fk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| f.ident == *src)
                    .map_or_else(|| src.to_string(), |f| f.column_name.clone())
            })
            .collect();
        let target_columns: Vec<String> = fk
            .target_columns
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let fk_name = format!("{}_{}_fkey", table_name, source_columns.join("_"));
        let source_cols_str = source_columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");
        let target_cols_str = target_columns
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(", ");

        let fk_prefix =
            format!(",\n\tCONSTRAINT \"{fk_name}\" FOREIGN KEY ({source_cols_str}) REFERENCES \"");
        parts.push(quote! { #fk_prefix });

        parts.push(quote! { <#ref_table_ident>::TABLE_NAME });

        let mut fk_suffix = format!("\"({target_cols_str})");

        if let Some(ref on_delete) = fk.on_delete {
            let action = on_delete.to_uppercase();
            if action != "NO ACTION" {
                let _ = write!(fk_suffix, " ON DELETE {action}");
            }
        }
        if let Some(ref on_update) = fk.on_update {
            let action = on_update.to_uppercase();
            if action != "NO ACTION" {
                let _ = write!(fk_suffix, " ON UPDATE {action}");
            }
        }

        parts.push(quote! { #fk_suffix });
    }

    // Check constraints
    for field in field_infos {
        if let Some(ref check) = field.check_constraint {
            let chk_name = format!("{}_{}_check", table_name, field.column_name);
            let chk_str = format!(",\n\tCONSTRAINT \"{chk_name}\" CHECK ({check})");
            parts.push(quote! { #chk_str });
        }
    }

    // Closing
    parts.push(quote! { "\n);" });

    quote! {
        ::drizzle::const_format::concatcp!(#(#parts),*)
    }
}

/// Build a PG column SQL fragment for use in concatcp! based generation.
fn build_pg_column_sql(field: &FieldInfo, _is_composite_pk: bool) -> String {
    let mut parts = vec![format!(
        "\"{}\" {}",
        field.column_name,
        field.column_type.to_sql_type()
    )];

    // Serial types are implicitly NOT NULL, don't add redundant constraint
    if !field.is_nullable && !field.is_serial {
        parts.push("NOT NULL".to_string());
    }

    // Handle default value (skip if serial - SERIAL type has implicit DEFAULT)
    if !field.is_serial
        && let Some(ref default) = field.default
    {
        match default {
            PostgreSQLDefault::Literal(s) | PostgreSQLDefault::Function(s) => {
                parts.push(format!("DEFAULT {s}"));
            }
            PostgreSQLDefault::Expression(ts) => {
                parts.push(format!("DEFAULT {ts}"));
            }
        }
    }

    format!("\t{}", parts.join(" "))
}

/// Convert a referential action string to the corresponding enum variant token
fn referential_action_token(action: &str, referential_action: &TokenStream) -> TokenStream {
    match action.to_uppercase().as_str() {
        "RESTRICT" => quote! { #referential_action::Restrict },
        "CASCADE" => quote! { #referential_action::Cascade },
        "SET NULL" => quote! { #referential_action::SetNull },
        "SET DEFAULT" => quote! { #referential_action::SetDefault },
        // "NO ACTION" and unknown values default to NoAction
        _ => quote! { #referential_action::NoAction },
    }
}

/// Generate const DDL entities for a `PostgreSQL` table
pub fn generate_const_ddl(ctx: &MacroContext, _column_zst_idents: &[Ident]) -> TokenStream {
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let schema_name = ctx.attrs.schema.as_deref().unwrap_or("public");

    // Get core type paths for SQLSchema reference
    let sql_schema = core_paths::sql_schema();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let postgres_value = postgres_paths::postgres_value();

    // Get DDL type paths
    let table_def = ddl_paths::table_def();
    let column_def = ddl_paths::column_def();
    let primary_key_def = ddl_paths::primary_key_def();
    let foreign_key_def = ddl_paths::foreign_key_def();
    let unique_constraint_def = ddl_paths::unique_constraint_def();
    let index_def = ddl_paths::index_def();
    let _identity_def = ddl_paths::identity_def();
    let table_sql = ddl_paths::table_sql();
    let referential_action = ddl_paths::referential_action();

    // Generate column definitions
    let column_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .map(|field| {
            let column_name = &field.column_name;
            let sql_type = field.column_type.to_sql_type();

            let mut modifiers = Vec::new();

            if !field.is_nullable {
                modifiers.push(quote! { .not_null() });
            }
            // Note: Primary key is handled at table level via DDL_PRIMARY_KEY
            // PostgreSQL doesn't use column-level primary_key() in ColumnDef
            if field.is_unique && !field.is_primary {
                // Unique constraints are also handled at table level via DDL_UNIQUE_CONSTRAINTS
            }
            // Note: Serial columns use the SERIAL pseudo-type which handles auto-increment
            // via a sequence + DEFAULT nextval(...). We do NOT set identity for serial columns.
            // The SERIAL type in the column definition is sufficient.
            // Only add default if not a serial column (SERIAL has implicit DEFAULT)
            if !field.is_serial
                && let Some(ref default) = field.default
            {
                let default_str = match default {
                    PostgreSQLDefault::Literal(s) | PostgreSQLDefault::Function(s) => s.clone(),
                    PostgreSQLDefault::Expression(ts) => ts.to_string(),
                };
                modifiers.push(quote! { .default_value(#default_str) });
            }

            quote! {
                #column_def::new(#schema_name, #table_name, #column_name, #sql_type)
                #(#modifiers)*
            }
        })
        .collect();

    // Build primary key DDL if there are primary key columns
    let pk_columns: Vec<&String> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_primary)
        .map(|f| &f.column_name)
        .collect();

    let pk_name = format!("{table_name}_pkey");
    let pk_def = if pk_columns.is_empty() {
        quote! {
            /// Primary key definition (none)
            pub const DDL_PRIMARY_KEY: ::std::option::Option<#primary_key_def> =
                ::std::option::Option::None;
        }
    } else {
        let pk_col_cows: Vec<TokenStream> = pk_columns
            .iter()
            .map(|col| quote! { ::std::borrow::Cow::Borrowed(#col) })
            .collect();
        quote! {
            /// Primary key definition
            pub const DDL_PRIMARY_KEY: ::std::option::Option<#primary_key_def> = {
                const PK_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#pk_col_cows),*];
                ::std::option::Option::Some(#primary_key_def::new(#schema_name, #table_name, #pk_name).columns(PK_COLS))
            };
        }
    };

    // Build foreign key DDL definitions
    let mut fk_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter_map(|field| {
            field.foreign_key.as_ref().map(|fk_ref| {
                let ref_table_ident = &fk_ref.table;
                let ref_column = fk_ref.column.to_string();
                let fk_name = format!(
                    "{}_{}_fkey",
                    table_name, field.column_name
                );
                let column_name = &field.column_name;

                let mut modifiers = Vec::new();
                if let Some(ref on_delete) = fk_ref.on_delete {
                    let action_token = referential_action_token(on_delete.as_str(), &referential_action);
                    modifiers.push(quote! { .on_delete(#action_token) });
                }
                if let Some(ref on_update) = fk_ref.on_update {
                    let action_token = referential_action_token(on_update.as_str(), &referential_action);
                    modifiers.push(quote! { .on_update(#action_token) });
                }

                quote! {
                    {
                        const FK_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#column_name)];
                        const FK_REF_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#ref_column)];
                        #foreign_key_def::new(#schema_name, #table_name, #fk_name)
                            .columns(FK_COLS)
                            .references(<#ref_table_ident>::DDL_TABLE.schema, <#ref_table_ident>::TABLE_NAME, FK_REF_COLS)
                            #(#modifiers)*
                    }
                }
            })
        })
        .collect();

    for fk in &ctx.attrs.composite_foreign_keys {
        let ref_table_ident = &fk.target_table;
        let source_columns: Vec<String> = fk
            .source_columns
            .iter()
            .map(|src| {
                ctx.field_infos
                    .iter()
                    .find(|f| &f.ident == src)
                    .map_or_else(|| src.to_string(), |f| f.column_name.clone())
            })
            .collect();
        let target_columns: Vec<String> = fk
            .target_columns
            .iter()
            .map(std::string::ToString::to_string)
            .collect();

        let fk_name = format!("{}_{}_fkey", table_name, source_columns.join("_"));
        let fk_cols: Vec<TokenStream> = source_columns
            .iter()
            .map(|c| quote! { ::std::borrow::Cow::Borrowed(#c) })
            .collect();
        let fk_ref_cols: Vec<TokenStream> = target_columns
            .iter()
            .map(|c| quote! { ::std::borrow::Cow::Borrowed(#c) })
            .collect();

        let mut modifiers = Vec::new();
        if let Some(ref on_delete) = fk.on_delete {
            let action_token = referential_action_token(on_delete.as_str(), &referential_action);
            modifiers.push(quote! { .on_delete(#action_token) });
        }
        if let Some(ref on_update) = fk.on_update {
            let action_token = referential_action_token(on_update.as_str(), &referential_action);
            modifiers.push(quote! { .on_update(#action_token) });
        }

        fk_defs.push(quote! {
            {
                const FK_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#fk_cols),*];
                const FK_REF_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#fk_ref_cols),*];
                #foreign_key_def::new(#schema_name, #table_name, #fk_name)
                    .columns(FK_COLS)
                    .references(<#ref_table_ident>::DDL_TABLE.schema, <#ref_table_ident>::TABLE_NAME, FK_REF_COLS)
                    #(#modifiers)*
            }
        });
    }

    // Build unique constraint DDL definitions (for non-primary unique columns)
    let unique_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_unique && !f.is_primary)
        .map(|field| {
            let unique_name = format!("{}_{}_unique", table_name, field.column_name);
            let column_name = &field.column_name;

            quote! {
                {
                    const UQ_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#column_name)];
                    #unique_constraint_def::new(#schema_name, #table_name, #unique_name).columns(UQ_COLS)
                }
            }
        })
        .collect();

    quote! {
        impl #struct_ident {
            /// Const DDL table definition for compile-time schema metadata.
            pub const DDL_TABLE: #table_def =
                #table_def::new(#schema_name, #table_name);

            /// Const DDL column definitions for compile-time schema metadata.
            pub const DDL_COLUMNS: &'static [#column_def] = &[
                #(#column_defs),*
            ];

            #pk_def

            /// Foreign key definitions
            pub const DDL_FOREIGN_KEYS: &'static [#foreign_key_def] = &[
                #(#fk_defs),*
            ];

            /// Unique constraint definitions
            pub const DDL_UNIQUE_CONSTRAINTS: &'static [#unique_constraint_def] = &[
                #(#unique_defs),*
            ];

            /// Index definitions (defined via separate #[PostgresIndex] structs)
            pub const DDL_INDEXES: &'static [#index_def] = &[];

            /// Generate the CREATE TABLE SQL using the DDL definitions.
            ///
            /// This is the single source of truth for SQL generation, building
            /// the statement from the const DDL entities above.
            pub fn create_table_sql() -> ::std::string::String {
                let table = Self::DDL_TABLE.into_table();
                let columns: ::std::vec::Vec<_> = Self::DDL_COLUMNS.iter().map(|c| c.into_column()).collect();
                let pk = Self::DDL_PRIMARY_KEY.map(|p| p.into_primary_key());
                let fks: ::std::vec::Vec<_> = Self::DDL_FOREIGN_KEYS.iter().map(|f| f.into_foreign_key()).collect();
                let uniques: ::std::vec::Vec<_> = Self::DDL_UNIQUE_CONSTRAINTS.iter().map(|u| u.into_unique_constraint()).collect();

                #table_sql::new(&table)
                    .columns(&columns)
                    .primary_key(pk.as_ref())
                    .foreign_keys(&fks)
                    .unique_constraints(&uniques)
                    .create_table_sql()
            }

            /// Returns the DDL SQL for creating this table.
            pub fn ddl_sql() -> &'static str {
                <Self as #sql_schema<'_, #postgres_schema_type, #postgres_value<'_>>>::SQL
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::generate_const_ddl;
    use crate::postgres::field::{FieldInfo, PostgreSQLReference, PostgreSQLType};
    use crate::postgres::table::{attributes::TableAttributes, context::MacroContext};
    use std::collections::HashSet;

    #[test]
    fn generated_fk_uses_referenced_table_schema_constant() {
        let struct_ident: syn::Ident = syn::parse_str("Posts").expect("valid ident");
        let struct_vis: syn::Visibility = syn::parse_str("pub").expect("valid visibility");

        let id_ident: syn::Ident = syn::parse_str("user_id").expect("valid ident");
        let id_type: syn::Type = syn::parse_str("i32").expect("valid type");

        let field = FieldInfo {
            ident: id_ident,
            vis: struct_vis.clone(),
            field_type: id_type.clone(),
            base_type: id_type,
            column_name: "user_id".to_string(),
            sql_definition: "\"user_id\" integer".to_string(),
            column_type: PostgreSQLType::Integer,
            flags: HashSet::new(),
            is_primary: false,
            is_unique: false,
            is_nullable: false,
            is_enum: false,
            is_pgenum: false,
            is_json: false,
            is_jsonb: false,
            is_serial: false,
            is_custom_type: false,
            is_generated_identity: false,
            identity_mode: None,
            generated_column: None,
            default: None,
            default_fn: None,
            check_constraint: None,
            foreign_key: Some(PostgreSQLReference {
                table: syn::parse_str("Users").expect("valid ref table"),
                column: syn::parse_str("id").expect("valid ref column"),
                on_delete: None,
                on_update: None,
            }),
            has_default: false,
            marker_exprs: Vec::new(),
        };

        let fields = vec![field];
        let attrs = TableAttributes {
            name: None,
            schema: Some("app".to_string()),
            unlogged: false,
            temporary: false,
            inherits: None,
            tablespace: None,
            composite_foreign_keys: Vec::new(),
            marker_exprs: Vec::new(),
        };

        let ctx = MacroContext {
            struct_ident: &struct_ident,
            struct_vis: &struct_vis,
            table_name: "posts".to_string(),
            field_infos: &fields,
            select_model_ident: syn::parse_str("PostsSelect").expect("valid ident"),
            select_model_partial_ident: syn::parse_str("PostsPartial").expect("valid ident"),
            insert_model_ident: syn::parse_str("PostsInsert").expect("valid ident"),
            update_model_ident: syn::parse_str("PostsUpdate").expect("valid ident"),
            is_composite_pk: false,
            attrs: &attrs,
        };

        let tokens = generate_const_ddl(&ctx, &[]).to_string();
        assert!(
            tokens.contains(":: DDL_TABLE . schema"),
            "expected FK references to use referenced table schema constant, got: {tokens}"
        );
    }
}
