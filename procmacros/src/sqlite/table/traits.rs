use super::context::MacroContext;
use crate::generators::generate_sql_table_info;
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use crate::sqlite::generators::*;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::Result;

/// Generates the `SQLSchema` and `SQLTable` implementations.
pub(crate) fn generate_table_impls(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
    _required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let columns_len = column_zst_idents.len();
    let strict = ctx.attrs.strict;
    let without_rowid = ctx.attrs.without_rowid;
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_column_info = core_paths::sql_column_info();
    let sql_table_info = core_paths::sql_table_info();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_column_info = sqlite_paths::sqlite_column_info();
    let sqlite_table_info = sqlite_paths::sqlite_table_info();

    // Generate SQL implementation based on whether table has foreign keys
    let create_table_sql = &ctx.create_table_sql;

    let (sql_const, sql_method) = if ctx.has_foreign_keys {
        // Use runtime SQL generation for tables with foreign keys
        // Call create_table_sql() which includes FK constraints via the DDL definitions
        (
            quote! { "" }, // Empty const, use runtime method
            Some(quote! {
                #sql::raw(Self::create_table_sql())
            }),
        )
    } else {
        // Use static SQL for tables without foreign keys
        (quote! { #create_table_sql }, None)
    };

    let to_sql_body = quote! {
        static INSTANCE: #struct_ident = #struct_ident::new();
        #sql::table(&INSTANCE)
    };

    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        quote! {#table_name},
        quote! {
            {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #struct_ident = #struct_ident::new();
                #sqlite_schema_type::Table(&TABLE_INSTANCE)
            }
        },
        quote! {#sql_const},
        sql_method,
    );
    let aliased_table_ident = format_ident!("Aliased{}", struct_ident);
    let sql_table_impl = generate_sql_table(
        struct_ident,
        quote! {#select_model},
        quote! {#insert_model<'a, T>},
        quote! {#update_model<'a>},
        quote! {#aliased_table_ident},
    );

    let mut dependencies = Vec::new();
    let mut seen_dependencies = HashSet::new();
    for field in ctx.field_infos {
        if let Some(fk) = &field.foreign_key {
            let name = fk.table_ident.to_string();
            if seen_dependencies.insert(name) {
                dependencies.push(fk.table_ident.clone());
            }
        }
    }
    let dependencies_len = dependencies.len();
    let dependency_statics: Vec<_> = dependencies
        .iter()
        .enumerate()
        .map(|(idx, ident)| format_ident!("__DRIZZLE_DEP_{}_{}", idx, ident))
        .collect();
    let sql_dependencies = quote! {
        #(#[allow(non_upper_case_globals)] static #dependency_statics: #dependencies = #dependencies::new(); )*
        #[allow(non_upper_case_globals)]
        static DEPENDENCIES: [&'static dyn #sql_table_info; #dependencies_len] =
            [#(&#dependency_statics,)*];
        &DEPENDENCIES
    };
    let sqlite_dependencies = quote! {
        #(#[allow(non_upper_case_globals)] static #dependency_statics: #dependencies = #dependencies::new(); )*
        #[allow(non_upper_case_globals)]
        static DEPENDENCIES: [&'static dyn #sqlite_table_info; #dependencies_len] =
            [#(&#dependency_statics,)*];
        &DEPENDENCIES
    };
    let sql_table_info_impl = generate_sql_table_info(
        struct_ident,
        quote! {
            <Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::NAME
        },
        quote! {
            #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            #[allow(non_upper_case_globals)]
            static COLUMNS: [&'static dyn #sql_column_info; #columns_len] =
                [#(&#column_zst_idents,)*];
            &COLUMNS
        },
        sql_dependencies,
    );
    let sqlite_table_info_impl = generate_sqlite_table_info(
        struct_ident,
        quote! {
            &<Self as #sql_schema<'_, #sqlite_schema_type, #sqlite_value<'_>>>::TYPE
        },
        quote! {#strict},
        quote! {#without_rowid},
        quote! {
            #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            #[allow(non_upper_case_globals)]
            static SQLITE_COLUMNS: [&'static dyn #sqlite_column_info; #columns_len] =
                [#(&#column_zst_idents,)*];
            &SQLITE_COLUMNS
        },
        sqlite_dependencies,
    );
    let sqlite_table_impl =
        generate_sqlite_table(struct_ident, quote! {#without_rowid}, quote! {#strict});
    let to_sql_impl = generate_to_sql(struct_ident, to_sql_body);

    Ok(quote! {
        #sql_schema_impl
        #sql_table_impl
        #sql_table_info_impl
        #sqlite_table_info_impl
        #sqlite_table_impl
        #to_sql_impl

    })
}
