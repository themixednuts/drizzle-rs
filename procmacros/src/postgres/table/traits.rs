use super::context::MacroContext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Result};

/// Generate trait implementations for the PostgreSQL table
pub(super) fn generate_table_impls(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
    _required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let create_table_sql = &ctx.create_table_sql;
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    // Generate SQL implementation based on whether table has foreign keys
    let (sql_const, sql_method) = if ctx.has_foreign_keys {
        // Use runtime SQL generation for tables with foreign keys
        (
            quote! {
                const SQL: ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> = ::drizzle::core::SQL::text("-- Runtime SQL generation required");
            },
            quote! {
                fn sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> {
                    let runtime_sql = #create_table_sql;
                    ::drizzle::core::SQL::raw(runtime_sql)
                }
            },
        )
    } else {
        // Use static SQL for tables without foreign keys
        (
            quote! {
                const SQL: ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> = ::drizzle::core::SQL::text(#create_table_sql);
            },
            quote! {},
        )
    };

    Ok(quote! {
        impl<'a> ::drizzle::core::SQLSchema<'a, ::drizzle::postgres::common::PostgresSchemaType, ::drizzle::postgres::values::PostgresValue<'a>> for #struct_ident {
            const NAME: &'a str = #table_name;
            const TYPE: ::drizzle::postgres::common::PostgresSchemaType = {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #struct_ident = #struct_ident::new();
                ::drizzle::postgres::common::PostgresSchemaType::Table(&TABLE_INSTANCE)
            };
            #sql_const
            #sql_method
        }

        impl<'a> ::drizzle::core::SQLTable<'a, ::drizzle::postgres::common::PostgresSchemaType, ::drizzle::postgres::values::PostgresValue<'a>> for #struct_ident {
            type Select = #select_model;
            type Insert<T> = #insert_model<'a, T>;
            type Update = #update_model;
        }

        impl ::drizzle::core::SQLTableInfo for #struct_ident {
            fn name(&self) -> &str {
                <Self as ::drizzle::core::SQLSchema<'_, ::drizzle::postgres::common::PostgresSchemaType, ::drizzle::postgres::values::PostgresValue<'_>>>::NAME
            }
            fn columns(&self) -> Box<[&'static dyn ::drizzle::core::SQLColumnInfo]> {
                #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*

                Box::new([#(::drizzle::core::AsColumnInfo::as_column(&#column_zst_idents),)*])
            }
        }

        impl ::drizzle::postgres::traits::PostgresTableInfo for #struct_ident {
            fn r#type(&self) -> & ::drizzle::postgres::common::PostgresSchemaType {
                &<Self as ::drizzle::core::SQLSchema<'_, ::drizzle::postgres::common::PostgresSchemaType, ::drizzle::postgres::values::PostgresValue<'_>>>::TYPE
            }
            fn columns(&self) -> Box<[&'static dyn ::drizzle::postgres::traits::PostgresColumnInfo]> {
                #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*

                Box::new([#(::drizzle::postgres::traits::AsColumnInfo::as_column(&#column_zst_idents),)*])
            }
        }

        impl<'a> ::drizzle::postgres::traits::PostgresTable<'a> for #struct_ident {}

        impl<'a> ::drizzle::core::ToSQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> for #struct_ident {
            fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> {
                static INSTANCE: #struct_ident = #struct_ident::new();
                ::drizzle::core::SQL::table(&INSTANCE)
            }
        }
    })
}
