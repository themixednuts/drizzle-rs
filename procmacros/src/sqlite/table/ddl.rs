//! Const DDL generation for SQLite tables.
//!
//! This module generates compile-time DDL entity definitions that can be used
//! for migrations, introspection, and schema comparison.

use super::context::MacroContext;
use crate::paths::ddl::sqlite as ddl_paths;
use crate::sqlite::field::FieldInfo;
use drizzle_types::sqlite::ddl::{Column, PrimaryKey, Table, UniqueConstraint, sql::TableSql};
use heck::ToSnakeCase;
use proc_macro2::TokenStream;
use quote::quote;
use std::borrow::Cow;
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

/// Generate the CREATE TABLE SQL string from raw parameters.
///
/// This is the core implementation that doesn't require a full MacroContext.
/// Use this before context is fully constructed.
pub(crate) fn generate_create_table_sql_from_params(
    table_name: &str,
    field_infos: &[FieldInfo],
    is_composite_pk: bool,
    strict: bool,
    without_rowid: bool,
) -> String {
    // Build Table
    let mut table = Table::new(table_name.to_string());
    table.strict = strict;
    table.without_rowid = without_rowid;

    // Build Columns
    let columns: Vec<Column> = field_infos
        .iter()
        .map(|field| build_column(table_name, field, is_composite_pk))
        .collect();

    // Build PrimaryKey
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

    // Build UniqueConstraints (single-column only, non-primary)
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
pub(crate) fn generate_create_table_sql(ctx: &MacroContext) -> String {
    generate_create_table_sql_from_params(
        &ctx.table_name,
        ctx.field_infos,
        ctx.is_composite_pk,
        ctx.attrs.strict,
        ctx.attrs.without_rowid,
    )
}

/// Build UniqueConstraints from field infos (single-column only, non-primary)
#[allow(dead_code)]
fn build_unique_constraints(table_name: &str, field_infos: &[FieldInfo]) -> Vec<UniqueConstraint> {
    field_infos
        .iter()
        .filter(|f| f.is_unique && !f.is_primary)
        .map(|field| {
            UniqueConstraint::from_strings(
                table_name.to_string(),
                format!("{}_{}_unique", table_name, field.column_name),
                vec![field.column_name.clone()],
            )
        })
        .collect()
}

/// Build a Column from FieldInfo
fn build_column(table_name: &str, field: &FieldInfo, is_composite_pk: bool) -> Column {
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

    // Handle default value
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
    let fk_defs: Vec<TokenStream> = ctx
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
    })
}

#[cfg(test)]
mod tests {
    use super::generate_create_table_sql_from_params;
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

        let sql = generate_create_table_sql_from_params("users", &[field], false, false, false);
        assert!(
            sql.contains("DEFAULT 'O''Hara'"),
            "expected escaped default string, got: {sql}"
        );
    }
}
