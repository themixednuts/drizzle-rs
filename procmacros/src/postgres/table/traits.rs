use super::context::MacroContext;
use crate::generators::generate_sql_table_info;
use crate::postgres::generators::*;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

/// Generate trait implementations for the PostgreSQL table
pub(super) fn generate_table_impls(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
    _required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let columns_len = column_zst_idents.len();
    let struct_ident = ctx.struct_ident;
    let aliased_table_ident = format_ident!("Aliased{}", struct_ident);
    let table_name = &ctx.table_name;
    let create_table_sql = &ctx.create_table_sql;
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    // Generate SQL implementation - always use empty string for const and provide via fn sql()
    let (sql_const, sql_method) = if ctx.has_foreign_keys {
        // Use runtime SQL generation for tables with foreign keys
        (
            quote! { "" },
            Some(quote! {
                SQL::raw(#create_table_sql)
            }),
        )
    } else {
        // Use static SQL for tables without foreign keys
        (quote! { #create_table_sql }, None)
    };

    // Generate ToSQL body
    let to_sql_body = quote! {
        static INSTANCE: #struct_ident = #struct_ident::new();
        SQL::table(&INSTANCE)
    };

    // Use generator functions for consistent pattern with SQLite
    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        quote! { #table_name },
        quote! {
            {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #struct_ident = #struct_ident::new();
                PostgresSchemaType::Table(&TABLE_INSTANCE)
            }
        },
        sql_const,
        sql_method,
    );

    let sql_table_impl = generate_sql_table(
        struct_ident,
        quote! { #select_model },
        quote! { #insert_model<'a, T> },
        quote! { #update_model },
        quote! { #aliased_table_ident },
    );

    let sql_table_info_impl = generate_sql_table_info(
        struct_ident,
        quote! {
            <Self as SQLSchema<'_, PostgresSchemaType, PostgresValue<'_>>>::NAME
        },
        quote! {
            #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            #[allow(non_upper_case_globals)]
            static COLUMNS: [&'static dyn SQLColumnInfo; #columns_len] =
                [#(&#column_zst_idents,)*];
            &COLUMNS
        },
    );

    let postgres_table_info_impl = generate_postgres_table_info(
        struct_ident,
        quote! {
            &<Self as SQLSchema<'_, PostgresSchemaType, PostgresValue<'_>>>::TYPE
        },
        quote! {
            #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            #[allow(non_upper_case_globals)]
            static POSTGRES_COLUMNS: [&'static dyn PostgresColumnInfo; #columns_len] =
                [#(&#column_zst_idents,)*];
            &POSTGRES_COLUMNS
        },
    );

    let postgres_table_impl = generate_postgres_table(struct_ident);
    let to_sql_impl = generate_to_sql(struct_ident, to_sql_body);

    Ok(quote! {
        #sql_schema_impl
        #sql_table_impl
        #sql_table_info_impl
        #postgres_table_info_impl
        #postgres_table_impl
        #to_sql_impl
    })
}
