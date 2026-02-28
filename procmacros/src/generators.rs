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
            fn name(&self) -> &'static str {
                #name
            }
            fn r#type(&self) -> &'static str {
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
            fn table(&self) -> &'static dyn #sql_table_info {
                #table
            }
            fn foreign_key(&self) -> ::std::option::Option<&'static dyn #sql_column_info> {
                #foreign_key
            }
        }
    }
}

/// Configuration for generating a `DrizzleTable` trait implementation.
pub struct DrizzleTableConfig<'a> {
    pub struct_ident: &'a Ident,
    pub name: TokenStream,
    pub qualified_name: TokenStream,
    pub schema: TokenStream,
    pub dependency_names: TokenStream,
    pub columns: TokenStream,
    pub primary_key: TokenStream,
    pub foreign_keys: TokenStream,
    pub constraints: TokenStream,
    pub dependencies: TokenStream,
}

/// Generate DrizzleTable trait implementation (blanket provides SQLTableInfo).
pub fn generate_drizzle_table(config: DrizzleTableConfig<'_>) -> TokenStream {
    let drizzle_table = core_paths::drizzle_table();
    let sql_column_info = core_paths::sql_column_info();
    let sql_primary_key_info = core_paths::sql_primary_key_info();
    let sql_foreign_key_info = core_paths::sql_foreign_key_info();
    let sql_constraint_info = core_paths::sql_constraint_info();
    let sql_table_info = core_paths::sql_table_info();

    let DrizzleTableConfig {
        struct_ident,
        name,
        qualified_name,
        schema,
        dependency_names,
        columns,
        primary_key,
        foreign_keys,
        constraints,
        dependencies,
    } = config;

    quote! {
        impl #drizzle_table for #struct_ident {
            const NAME: &'static str = #name;
            const QUALIFIED_NAME: &'static str = #qualified_name;
            const SCHEMA: ::std::option::Option<&'static str> = #schema;
            const DEPENDENCY_NAMES: &'static [&'static str] = #dependency_names;

            fn columns_list() -> &'static [&'static dyn #sql_column_info] {
                #columns
            }

            fn primary_key_info() -> ::std::option::Option<&'static dyn #sql_primary_key_info> {
                #primary_key
            }

            fn foreign_keys_list() -> &'static [&'static dyn #sql_foreign_key_info] {
                #foreign_keys
            }

            fn constraints_list() -> &'static [&'static dyn #sql_constraint_info] {
                #constraints
            }

            fn dependencies_list() -> &'static [&'static dyn #sql_table_info] {
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
