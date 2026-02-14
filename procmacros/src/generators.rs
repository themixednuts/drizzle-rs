//! Shared code generation helper functions.
//!
//! These functions provide reusable trait implementation generators using
//! fully-qualified paths from the paths module.

use crate::paths::core as core_paths;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

#[cfg(feature = "sqlite")]
#[allow(clippy::too_many_arguments)]
/// Generate SQLColumnInfo trait implementation
pub fn generate_sql_column_info(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    is_primary_key: TokenStream,
    is_not_null: TokenStream,
    is_unique: TokenStream,
    has_default: TokenStream,
    foreign_key: TokenStream,
    table: TokenStream,
) -> TokenStream {
    let sql_column_info = core_paths::sql_column_info();
    let sql_table_info = core_paths::sql_table_info();

    quote! {
        impl #sql_column_info for #struct_ident {
            fn name(&self) -> &str {
                #name
            }
            fn r#type(&self) -> &str {
                #r#type
            }
            fn is_primary_key(&self) -> bool {
                #is_primary_key
            }
            fn is_not_null(&self) -> bool {
                #is_not_null
            }
            fn is_unique(&self) -> bool {
                #is_unique
            }
            fn has_default(&self) -> bool {
                #has_default
            }
            fn table(&self) -> &dyn #sql_table_info {
                #table
            }
            fn foreign_key(&self) -> ::std::option::Option<&'static dyn #sql_column_info> {
                #foreign_key
            }
        }
    }
}

/// Configuration for generating an `SQLTableInfo` trait implementation.
pub struct SQLTableInfoConfig<'a> {
    pub struct_ident: &'a Ident,
    pub name: TokenStream,
    pub schema: TokenStream,
    pub columns: TokenStream,
    pub primary_key: TokenStream,
    pub foreign_keys: TokenStream,
    pub constraints: TokenStream,
    pub dependencies: TokenStream,
}

/// Generate SQLTableInfo trait implementation
pub fn generate_sql_table_info(config: SQLTableInfoConfig<'_>) -> TokenStream {
    let sql_table_info = core_paths::sql_table_info();
    let sql_column_info = core_paths::sql_column_info();
    let sql_primary_key_info = core_paths::sql_primary_key_info();
    let sql_foreign_key_info = core_paths::sql_foreign_key_info();
    let sql_constraint_info = core_paths::sql_constraint_info();

    let SQLTableInfoConfig {
        struct_ident,
        name,
        schema,
        columns,
        primary_key,
        foreign_keys,
        constraints,
        dependencies,
    } = config;

    quote! {
        impl #sql_table_info for #struct_ident {
            fn name(&self) -> &str {
                #name
            }

            fn schema(&self) -> ::std::option::Option<&str> {
                #schema
            }

            fn columns(&self) -> &'static [&'static dyn #sql_column_info] {
                #columns
            }

            fn primary_key(&self) -> ::std::option::Option<&'static dyn #sql_primary_key_info> {
                #primary_key
            }

            fn foreign_keys(&self) -> &'static [&'static dyn #sql_foreign_key_info] {
                #foreign_keys
            }

            fn constraints(&self) -> &'static [&'static dyn #sql_constraint_info] {
                #constraints
            }

            fn dependencies(&self) -> &'static [&'static dyn #sql_table_info] {
                #dependencies
            }
        }
    }
}

/// Generate basic impl block
#[cfg(feature = "sqlite")]
pub fn generate_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl #struct_ident {
            #body
        }
    }
}
