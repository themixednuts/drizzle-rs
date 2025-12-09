use super::context::MacroContext;
use crate::generators::generate_sql_table_info;
use crate::sqlite::generators::*;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
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

    // Generate SQL implementation based on whether table has foreign keys
    let create_table_sql = &ctx.create_table_sql;

    let (sql_const, sql_method) = if ctx.has_foreign_keys {
        // Use runtime SQL generation for tables with foreign keys
        if let Some(ref runtime_sql) = ctx.create_table_sql_runtime {
            (
                quote! { SQL::empty() }, // Empty const, use runtime method
                Some(quote! {
                    let runtime_sql = #runtime_sql;
                    SQL::raw(runtime_sql)
                }),
            )
        } else {
            // Use static SQL
            (quote! { SQL::raw_const(#create_table_sql) }, None)
        }
    } else {
        // Use static SQL for tables without foreign keys
        (quote! { SQL::raw_const(#create_table_sql) }, None)
    };

    let to_sql_body = quote! {
        static INSTANCE: #struct_ident = #struct_ident::new();
        SQL::table(&INSTANCE)
    };

    let sql_schema_impl = generate_sql_schema(
        struct_ident,
        quote! {#table_name},
        quote! {
            {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #struct_ident = #struct_ident::new();
                SQLiteSchemaType::Table(&TABLE_INSTANCE)
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
        quote! {#update_model},
        quote! {#aliased_table_ident},
    );
    let sql_table_info_impl = generate_sql_table_info(
        struct_ident,
        quote! {
            <Self as SQLSchema<'_, SQLiteSchemaType, SQLiteValue<'_>>>::NAME
        },
        quote! {
            #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            #[allow(non_upper_case_globals)]
            static COLUMNS: [&'static dyn SQLColumnInfo; #columns_len] =
                [#(&#column_zst_idents,)*];
            &COLUMNS
        },
    );
    let sqlite_table_info_impl = generate_sqlite_table_info(
        struct_ident,
        quote! {
            &<Self as SQLSchema<'_, SQLiteSchemaType, SQLiteValue<'_>>>::TYPE
        },
        quote! {#strict},
        quote! {#without_rowid},
        quote! {
            #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
            #[allow(non_upper_case_globals)]
            static SQLITE_COLUMNS: [&'static dyn SQLiteColumnInfo; #columns_len] =
                [#(&#column_zst_idents,)*];
            &SQLITE_COLUMNS
        },
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
