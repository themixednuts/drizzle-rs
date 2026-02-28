//! Const DDL generation for SQLite tables.
//!
//! This module generates compile-time DDL entity definitions that can be used
//! for migrations, introspection, and schema comparison.

use super::context::MacroContext;
use crate::paths::ddl::sqlite as ddl_paths;
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use crate::sqlite::field::FieldInfo;
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Convert a referential action string to the corresponding enum variant token
fn referential_action_token(action: &str, referential_action: &TokenStream) -> TokenStream {
    match action.to_uppercase().as_str() {
        "NO ACTION" => quote! { #referential_action::NoAction },
        "RESTRICT" => quote! { #referential_action::Restrict },
        "CASCADE" => quote! { #referential_action::Cascade },
        "SET NULL" => quote! { #referential_action::SetNull },
        "SET DEFAULT" => quote! { #referential_action::SetDefault },
        _ => quote! { #referential_action::NoAction },
    }
}

/// Generate a compile-time `const SQL: &'static str` value for `SQLSchema`
/// using `concatcp!` so that foreign key REFERENCES can resolve table names
/// via `<OtherTable>::TABLE_NAME` at compile time.
pub(crate) fn generate_schema_sql_const(ctx: &MacroContext) -> TokenStream {
    let table_name = &ctx.table_name;
    let is_composite_pk = ctx.is_composite_pk;
    let strict = ctx.attrs.strict;
    let without_rowid = ctx.attrs.without_rowid;
    let field_infos = ctx.field_infos;

    let has_foreign_keys = field_infos.iter().any(|f| f.foreign_key.is_some())
        || !ctx.attrs.composite_foreign_keys.is_empty();

    if !has_foreign_keys {
        // For tables without FKs, we can build the SQL entirely at proc-macro
        // time as a string literal (no need for concatcp!)
        let sql = build_create_table_sql(
            table_name,
            field_infos,
            is_composite_pk,
            strict,
            without_rowid,
        );
        return quote! { #sql };
    }

    // For tables WITH FKs, we need concatcp! to reference other table's TABLE_NAME
    // Build the SQL pieces that will be concatenated at compile time
    let mut parts: Vec<TokenStream> = Vec::new();

    // CREATE TABLE "table_name" (\n
    let header = format!("CREATE TABLE \"{}\" (\n", table_name);
    parts.push(quote! { #header });

    // Column definitions
    let column_lines: Vec<String> = field_infos
        .iter()
        .map(|field| build_column_sql(table_name, field, is_composite_pk))
        .collect();

    for (i, col_line) in column_lines.iter().enumerate() {
        let line = if i > 0 {
            format!(",\n{}", col_line)
        } else {
            col_line.clone()
        };
        parts.push(quote! { #line });
    }

    // Composite primary key (if needed)
    if is_composite_pk {
        let pk_columns: Vec<&String> = field_infos
            .iter()
            .filter(|f| f.is_primary)
            .map(|f| &f.column_name)
            .collect();
        if !pk_columns.is_empty() {
            let pk_str = format!(
                ",\nPRIMARY KEY({})",
                pk_columns
                    .iter()
                    .map(|c| format!("\"{}\"", c))
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            parts.push(quote! { #pk_str });
        }
    }

    // Unique constraints
    for field in field_infos.iter().filter(|f| f.is_unique && !f.is_primary) {
        let uq_str = format!(",\nUNIQUE(\"{}\")", field.column_name);
        parts.push(quote! { #uq_str });
    }

    // Single-column foreign keys
    for field in field_infos {
        if let Some(ref fk) = field.foreign_key {
            let ref_table_ident = &fk.table_ident;
            let ref_column = fk.column_ident.to_string().to_snake_case();

            // FK prefix: ,\nFOREIGN KEY ("col") REFERENCES "
            let fk_prefix = format!(",\nFOREIGN KEY (\"{}\") REFERENCES \"", field.column_name);
            parts.push(quote! { #fk_prefix });

            // Table name via const reference
            parts.push(quote! { <#ref_table_ident>::TABLE_NAME });

            // FK suffix: " ("ref_col")
            let mut fk_suffix = format!("\" (\"{}\")", ref_column);

            if let Some(ref on_delete) = fk.on_delete {
                fk_suffix.push_str(&format!(" ON DELETE {}", on_delete.to_uppercase()));
            }
            if let Some(ref on_update) = fk.on_update {
                fk_suffix.push_str(&format!(" ON UPDATE {}", on_update.to_uppercase()));
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
                    .find(|f| f.ident == src)
                    .map(|f| f.column_name.clone())
                    .unwrap_or_else(|| src.to_string())
            })
            .collect();
        let target_columns: Vec<String> = fk.target_columns.iter().map(|c| c.to_string()).collect();

        let source_cols_str = source_columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");
        let target_cols_str = target_columns
            .iter()
            .map(|c| format!("\"{}\"", c))
            .collect::<Vec<_>>()
            .join(", ");

        let fk_prefix = format!(",\nFOREIGN KEY ({}) REFERENCES \"", source_cols_str);
        parts.push(quote! { #fk_prefix });

        parts.push(quote! { <#ref_table_ident>::TABLE_NAME });

        let mut fk_suffix = format!("\" ({})", target_cols_str);

        if let Some(ref on_delete) = fk.on_delete {
            fk_suffix.push_str(&format!(" ON DELETE {}", on_delete.to_uppercase()));
        }
        if let Some(ref on_update) = fk.on_update {
            fk_suffix.push_str(&format!(" ON UPDATE {}", on_update.to_uppercase()));
        }

        parts.push(quote! { #fk_suffix });
    }

    // Closing paren with optional modifiers
    let mut closing = "\n)".to_string();
    if strict && without_rowid {
        closing.push_str(" STRICT, WITHOUT ROWID");
    } else if strict {
        closing.push_str(" STRICT");
    } else if without_rowid {
        closing.push_str(" WITHOUT ROWID");
    }
    parts.push(quote! { #closing });

    quote! {
        ::drizzle::const_format::concatcp!(#(#parts),*)
    }
}

/// Build the CREATE TABLE SQL string entirely at proc-macro time.
///
/// Used for tables WITHOUT foreign keys where all data is known statically.
fn build_create_table_sql(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
    strict: bool,
    without_rowid: bool,
) -> String {
    use drizzle_types::sqlite::ddl::{Column, PrimaryKey, Table, UniqueConstraint, sql::TableSql};
    use std::borrow::Cow;

    let mut table = Table::new(table_name.to_string());
    table.strict = strict;
    table.without_rowid = without_rowid;

    let columns: Vec<Column> = field_infos
        .iter()
        .map(|field| {
            let is_single_pk = field.is_primary && !is_composite_pk;

            let mut col = Column::new(
                Cow::Owned(table_name.to_string()),
                Cow::Owned(field.column_name.clone()),
                Cow::Owned(field.column_type.to_sql_type().to_string()),
            );

            col.not_null = !field.is_nullable;

            if is_single_pk {
                col.primary_key = Some(true);
            }

            if field.is_autoincrement {
                col.autoincrement = Some(true);
            }

            if field.is_unique && !field.is_primary {
                col.unique = Some(true);
            }

            if let Some(ref default_expr) = field.default_value
                && let syn::Expr::Lit(expr_lit) = default_expr
            {
                let default_str = match &expr_lit.lit {
                    syn::Lit::Int(i) => i.to_string(),
                    syn::Lit::Float(f) => f.to_string(),
                    syn::Lit::Bool(b) => if b.value() { "1" } else { "0" }.to_string(),
                    syn::Lit::Str(s) => format!("'{}'", s.value().replace('\'', "''")),
                    _ => return col,
                };
                col.default = Some(Cow::Owned(default_str));
            }

            col
        })
        .collect();

    let pk_columns: Vec<String> = field_infos
        .iter()
        .filter(|f| f.is_primary)
        .map(|f| f.column_name.clone())
        .collect();

    let primary_key = if !pk_columns.is_empty() {
        let pk_name = format!("{}_pkey", table_name);
        Some(PrimaryKey::from_strings(
            table_name.to_string(),
            pk_name,
            pk_columns,
        ))
    } else {
        None
    };

    let unique_constraints: Vec<UniqueConstraint> = field_infos
        .iter()
        .filter(|f| f.is_unique && !f.is_primary)
        .map(|field| {
            UniqueConstraint::from_strings(
                table_name.to_string(),
                format!("{}_{}_unique", table_name, field.column_name),
                vec![field.column_name.clone()],
            )
        })
        .collect();

    TableSql::new(&table)
        .columns(&columns)
        .primary_key(primary_key.as_ref())
        .foreign_keys(&[])
        .unique_constraints(&unique_constraints)
        .create_table_sql()
}

/// Build a column SQL fragment for use in concatcp! based generation.
fn build_column_sql(_table_name: &str, field: &FieldInfo, is_composite_pk: bool) -> String {
    let is_single_pk = field.is_primary && !is_composite_pk;

    let mut parts = vec![format!(
        "\"{}\" {}",
        field.column_name,
        field.column_type.to_sql_type()
    )];

    if is_single_pk {
        parts.push("PRIMARY KEY".to_string());
    }
    if field.is_autoincrement {
        parts.push("AUTOINCREMENT".to_string());
    }
    if !field.is_nullable {
        parts.push("NOT NULL".to_string());
    }
    if field.is_unique && !field.is_primary {
        parts.push("UNIQUE".to_string());
    }

    if let Some(ref default_expr) = field.default_value
        && let syn::Expr::Lit(expr_lit) = default_expr
    {
        let default_str = match &expr_lit.lit {
            syn::Lit::Int(i) => format!("DEFAULT {}", i),
            syn::Lit::Float(f) => format!("DEFAULT {}", f),
            syn::Lit::Bool(b) => format!("DEFAULT {}", if b.value() { "1" } else { "0" }),
            syn::Lit::Str(s) => format!("DEFAULT '{}'", s.value().replace('\'', "''")),
            _ => String::new(),
        };
        if !default_str.is_empty() {
            parts.push(default_str);
        }
    }

    // Use tab indent for columns to match the DDL generator format
    format!("\t{}", parts.join(" "))
}

/// Generate const DDL definitions for the table and its columns.
///
/// This generates:
/// - `DDL_TABLE: drizzle_types::sqlite::ddl::TableDef` - Table definition
/// - `DDL_COLUMNS: &'static [drizzle_types::sqlite::ddl::ColumnDef]` - Column definitions
/// - `DDL_PRIMARY_KEY: Option<...>` - Primary key definition
/// - `DDL_FOREIGN_KEYS: &'static [...]` - Foreign key definitions
/// - `DDL_UNIQUE_CONSTRAINTS: &'static [...]` - Unique constraint definitions
pub(crate) fn generate_const_ddl(ctx: &MacroContext) -> Result<TokenStream> {
    let _struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let strict = ctx.attrs.strict;
    let without_rowid = ctx.attrs.without_rowid;

    // Get core type paths for SQLSchema reference
    let sql_schema = core_paths::sql_schema();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_value = sqlite_paths::sqlite_value();

    // Get DDL type paths
    let table_def = ddl_paths::table_def();
    let column_def = ddl_paths::column_def();
    let primary_key_def = ddl_paths::primary_key_def();
    let foreign_key_def = ddl_paths::foreign_key_def();
    let unique_constraint_def = ddl_paths::unique_constraint_def();
    let index_def = ddl_paths::index_def();
    let table_sql = ddl_paths::table_sql();
    let referential_action = ddl_paths::referential_action();

    // Generate table modifiers
    let mut table_modifiers = Vec::new();
    if strict {
        table_modifiers.push(quote! { .strict() });
    }
    if without_rowid {
        table_modifiers.push(quote! { .without_rowid() });
    }

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
            if field.is_primary && !field.is_autoincrement {
                modifiers.push(quote! { .primary_key() });
            }
            if field.is_autoincrement {
                modifiers.push(quote! { .autoincrement() });
            }
            if field.is_unique && !field.is_primary {
                modifiers.push(quote! { .unique() });
            }
            if let Some(syn::Expr::Lit(expr_lit)) = field.default_value.as_ref() {
                // Convert the expression to a string for DDL
                let default_str = match &expr_lit.lit {
                    syn::Lit::Int(i) => i.to_string(),
                    syn::Lit::Float(f) => f.to_string(),
                    syn::Lit::Bool(b) => if b.value() { "1" } else { "0" }.to_string(),
                    syn::Lit::Str(s) => format!("'{}'", s.value().replace('\'', "''")),
                    _ => String::new(),
                };
                if !default_str.is_empty() {
                    modifiers.push(quote! { .default_value(#default_str) });
                }
            }

            quote! {
                #column_def::new(#table_name, #column_name, #sql_type)
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

    let pk_name = format!("{}_pkey", table_name);
    let pk_def = if !pk_columns.is_empty() {
        let pk_col_cows: Vec<TokenStream> = pk_columns
            .iter()
            .map(|col| quote! { ::std::borrow::Cow::Borrowed(#col) })
            .collect();
        quote! {
            /// Primary key definition
            pub const DDL_PRIMARY_KEY: ::std::option::Option<#primary_key_def> = {
                const PK_COLS: &[::std::borrow::Cow<'static, str>] = &[#(#pk_col_cows),*];
                ::std::option::Option::Some(#primary_key_def::new(#table_name, #pk_name).columns(PK_COLS))
            };
        }
    } else {
        quote! {
            /// Primary key definition (none)
            pub const DDL_PRIMARY_KEY: ::std::option::Option<#primary_key_def> =
                ::std::option::Option::None;
        }
    };

    // Build foreign key DDL definitions
    let mut fk_defs: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .filter_map(|field| {
            field.foreign_key.as_ref().map(|fk_ref| {
                let ref_table_ident = &fk_ref.table_ident;
                let ref_column = fk_ref.column_ident.to_string().to_snake_case();
                let fk_name = format!("{}_{}_fkey", table_name, field.column_name);
                let column_name = &field.column_name;

                let mut modifiers = Vec::new();
                if let Some(ref on_delete) = fk_ref.on_delete {
                    let action_token = referential_action_token(on_delete, &referential_action);
                    modifiers.push(
                        quote! { .on_delete(#action_token) },
                    );
                }
                if let Some(ref on_update) = fk_ref.on_update {
                    let action_token = referential_action_token(on_update, &referential_action);
                    modifiers.push(
                        quote! { .on_update(#action_token) },
                    );
                }

                quote! {
                    {
                        const FK_COLS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#column_name)];
                        const FK_REFS: &[::std::borrow::Cow<'static, str>] = &[::std::borrow::Cow::Borrowed(#ref_column)];
                        #foreign_key_def::new(#table_name, #fk_name)
                            .columns(FK_COLS)
                            .references(<#ref_table_ident>::TABLE_NAME, FK_REFS)
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
                    .find(|f| f.ident == src)
                    .map(|f| f.column_name.clone())
                    .unwrap_or_else(|| src.to_string())
            })
            .collect();
        let target_columns: Vec<String> = fk.target_columns.iter().map(|c| c.to_string()).collect();

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
                #foreign_key_def::new(#table_name, #fk_name)
                    .columns(FK_COLS)
                    .references(<#ref_table_ident>::TABLE_NAME, FK_REF_COLS)
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
                    #unique_constraint_def::new(#table_name, #unique_name).columns(UQ_COLS)
                }
            }
        })
        .collect();

    Ok(quote! {
        /// Const DDL table definition for compile-time schema metadata.
        pub const DDL_TABLE: #table_def =
            #table_def::new(#table_name)
            #(#table_modifiers)*;

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

        /// Index definitions (defined via separate #[SQLiteIndex] structs)
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
            <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::SQL
        }
    })
}

#[cfg(test)]
mod tests {
    use super::build_create_table_sql;
    use crate::sqlite::field::{FieldInfo, SQLiteType};

    #[test]
    fn create_table_sql_escapes_single_quotes_in_default_string_literals() {
        let ident: syn::Ident = syn::parse_str("display_name").expect("valid ident");
        let ty: syn::Type = syn::parse_str("String").expect("valid type");
        let default_expr: syn::Expr = syn::parse_str("\"O'Hara\"").expect("valid expr");

        let field = FieldInfo {
            ident: &ident,
            field_type: &ty,
            base_type: &ty,
            column_name: "display_name".to_string(),
            sql_definition: String::new(),
            is_nullable: false,
            has_default: true,
            is_primary: false,
            is_autoincrement: false,
            is_unique: false,
            is_json: false,
            is_enum: false,
            is_uuid: false,
            column_type: SQLiteType::Text,
            foreign_key: None,
            default_value: Some(default_expr),
            default_fn: None,
            marker_exprs: Vec::new(),
            select_type: None,
            update_type: None,
        };

        let sql = build_create_table_sql("users", &[field], false, false, false);
        assert!(
            sql.contains("DEFAULT 'O''Hara'"),
            "expected escaped default string, got: {sql}"
        );
    }
}
