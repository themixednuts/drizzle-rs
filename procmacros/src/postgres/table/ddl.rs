//! PostgreSQL const DDL generation
//!
//! This module generates compile-time DDL entity definitions for PostgreSQL tables.

use super::context::MacroContext;
use crate::paths::ddl::postgres as ddl_paths;
use crate::postgres::field::{FieldInfo, PostgreSQLDefault};
use drizzle_types::postgres::ddl::{Column, PrimaryKey, Table, UniqueConstraint, sql::TableSql};
use proc_macro2::TokenStream;
use quote::quote;
use std::borrow::Cow;
use syn::Ident;

/// Generate the CREATE TABLE SQL string at proc-macro time.
///
/// This is used for tables WITHOUT foreign keys, where all information
/// is known at macro expansion time. Uses the same DDL types as runtime
/// generation for consistency.
pub(crate) fn generate_create_table_sql(ctx: &MacroContext) -> String {
    let table_name = &ctx.table_name;
    let schema_name = "public"; // TODO: Add schema attribute support

    // Build Table
    let table = Table::new(schema_name, table_name.clone());

    // Build Columns
    let columns: Vec<Column> = ctx
        .field_infos
        .iter()
        .map(|field| build_column(schema_name, table_name, field, ctx.is_composite_pk))
        .collect();

    // Build PrimaryKey
    let pk_columns: Vec<String> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_primary)
        .map(|f| f.column_name.clone())
        .collect();

    let primary_key = if !pk_columns.is_empty() {
        let pk_name = format!("{}_pkey", table_name);
        Some(PrimaryKey::from_strings(
            schema_name.to_string(),
            table_name.clone(),
            pk_name,
            pk_columns,
        ))
    } else {
        None
    };

    // Build UniqueConstraints (single-column only, non-primary)
    let unique_constraints: Vec<UniqueConstraint> = ctx
        .field_infos
        .iter()
        .filter(|f| f.is_unique && !f.is_primary)
        .map(|field| {
            UniqueConstraint::from_strings(
                schema_name.to_string(),
                table_name.clone(),
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

/// Build a Column from FieldInfo
fn build_column(
    schema_name: &str,
    table_name: &str,
    field: &FieldInfo,
    is_composite_pk: bool,
) -> Column {
    let is_single_pk = field.is_primary && !is_composite_pk;

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
            PostgreSQLDefault::Literal(s) => s.clone(),
            PostgreSQLDefault::Function(s) => s.clone(),
            PostgreSQLDefault::Expression(ts) => ts.to_string(),
        };
        col.default = Some(Cow::Owned(default_str));
    }

    col
}

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

/// Generate const DDL entities for a PostgreSQL table
pub(crate) fn generate_const_ddl(
    ctx: &MacroContext,
    _column_zst_idents: &[Ident],
) -> Result<TokenStream, syn::Error> {
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let schema_name = "public"; // TODO: Add schema attribute support

    // Get DDL type paths
    let table_def = ddl_paths::table_def();
    let column_def = ddl_paths::column_def();
    let primary_key_def = ddl_paths::primary_key_def();
    let foreign_key_def = ddl_paths::foreign_key_def();
    let unique_constraint_def = ddl_paths::unique_constraint_def();
    let index_def = ddl_paths::index_def();
    let identity_def = ddl_paths::identity_def();
    let identity_type = ddl_paths::identity_type();
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
                    PostgreSQLDefault::Literal(s) => s.clone(),
                    PostgreSQLDefault::Function(s) => s.clone(),
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
                ::std::option::Option::Some(#primary_key_def::new(#schema_name, #table_name, #pk_name).columns(PK_COLS))
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
                            .references(#schema_name, <#ref_table_ident>::TABLE_NAME, FK_REF_COLS)
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
                    #unique_constraint_def::new(#schema_name, #table_name, #unique_name).columns(UQ_COLS)
                }
            }
        })
        .collect();

    Ok(quote! {
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
        }
    })
}
