use super::context::MacroContext;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Result;

/// Generates the `SQLSchema` and `SQLTable` implementations.
pub(crate) fn generate_table_impls(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
    _required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let MacroContext {
        strict,
        without_rowid,
        ..
    } = &ctx;
    let struct_ident = ctx.struct_ident;
    let table_name = &ctx.table_name;
    let (select_model, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    // Generate SQL implementation based on whether table has foreign keys

    let (sql_const, sql_method) = if ctx.has_foreign_keys {
        // Use runtime SQL generation for tables with foreign keys
        if let Some(ref runtime_sql) = ctx.create_table_sql_runtime {
            (
                quote! {
                    const SQL: ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> = ::drizzle::core::SQL::text("-- Runtime SQL generation required");
                },
                quote! {
                    fn sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> {
                        let runtime_sql = #runtime_sql;
                        ::drizzle::core::SQL::raw(runtime_sql)
                    }
                },
            )
        } else {
            // Fallback to static SQL
            let create_table_sql = &ctx.create_table_sql;
            (
                quote! {
                    const SQL: ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> = ::drizzle::core::SQL::text(#create_table_sql);
                },
                quote! {},
            )
        }
    } else {
        // Use static SQL for tables without foreign keys
        let create_table_sql = &ctx.create_table_sql;
        (
            quote! {
                const SQL: ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> = ::drizzle::core::SQL::text(#create_table_sql);
            },
            quote! {},
        )
    };

    Ok(quote! {
        impl<'a> ::drizzle::core::SQLSchema<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a> > for #struct_ident {
            const NAME: &'a str = #table_name;
            const TYPE: ::drizzle::sqlite::common::SQLiteSchemaType = {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #struct_ident = #struct_ident::new();
                ::drizzle::sqlite::common::SQLiteSchemaType::Table(&TABLE_INSTANCE)
            };
            #sql_const
            #sql_method
        }


        impl<'a> ::drizzle::core::SQLTable<'a, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            type Select = #select_model;
            type Insert<T> = #insert_model<'a, T>;
            type Update = #update_model;
        }


        impl ::drizzle::core::SQLTableInfo for #struct_ident {
            fn name(&self) -> &str {
                <Self as ::drizzle::core::SQLSchema<'_, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'_>>>::NAME
            }
            fn columns(&self) -> Box<[&'static dyn ::drizzle::core::SQLColumnInfo]> {
                #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*

                Box::new([#(::drizzle::core::AsColumnInfo::as_column(&#column_zst_idents),)*])
            }
        }

        impl ::drizzle::sqlite::traits::SQLiteTableInfo for #struct_ident {
            fn r#type(&self) -> & ::drizzle::sqlite::common::SQLiteSchemaType {
                &<Self as ::drizzle::core::SQLSchema<'_, ::drizzle::sqlite::common::SQLiteSchemaType, ::drizzle::sqlite::values::SQLiteValue<'_>>>::TYPE
            }

            fn strict(&self) -> bool {
                #strict
            }
            fn without_rowid(&self) -> bool {
                #without_rowid
            }
            fn columns(&self) -> Box<[&'static dyn ::drizzle::sqlite::traits::SQLiteColumnInfo]> {
                #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*

                Box::new([#(::drizzle::sqlite::traits::AsColumnInfo::as_column(&#column_zst_idents),)*])
            }

        }

        impl<'a> ::drizzle::sqlite::traits::SQLiteTable<'a> for #struct_ident {
            const WITHOUT_ROWID: bool = #without_rowid;
            const STRICT: bool = #strict;
        }

        impl<'a> ::drizzle::core::ToSQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #struct_ident {
            fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> {
                static INSTANCE: #struct_ident = #struct_ident::new();
                ::drizzle::core::SQL::table(&INSTANCE)
            }
        }
    })
}
