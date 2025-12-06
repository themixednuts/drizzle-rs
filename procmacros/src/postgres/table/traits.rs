use super::context::MacroContext;
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

    // Generate SQL implementation - always use SQL::empty() for const and provide via fn sql()
    let (sql_const, sql_method) = if ctx.has_foreign_keys {
        // Use runtime SQL generation for tables with foreign keys
        (
            quote! {
                const SQL: SQL<'a, PostgresValue<'a>> = SQL::empty();
            },
            quote! {
                fn sql(&self) -> SQL<'a, PostgresValue<'a>> {
                    let runtime_sql = #create_table_sql;
                    SQL::raw(runtime_sql)
                }
            },
        )
    } else {
        // Use static SQL for tables without foreign keys
        (
            quote! {
                const SQL: SQL<'a, PostgresValue<'a>> = SQL::empty();
            },
            quote! {
                fn sql(&self) -> SQL<'a, PostgresValue<'a>> {
                    SQL::raw(#create_table_sql)
                }
            },
        )
    };

    Ok(quote! {
        impl<'a> SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>> for #struct_ident {
            const NAME: &'a str = #table_name;
            const TYPE: PostgresSchemaType = {
                #[allow(non_upper_case_globals)]
                static TABLE_INSTANCE: #struct_ident = #struct_ident::new();
                PostgresSchemaType::Table(&TABLE_INSTANCE)
            };
            #sql_const
            #sql_method
        }

        impl<'a> SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> for #struct_ident {
            type Select = #select_model;
            type Insert<T> = #insert_model<'a, T>;
            type Update = #update_model;
            type Aliased = #aliased_table_ident;

            fn alias(name: &'static str) -> Self::Aliased {
                #aliased_table_ident::new(name)
            }
        }

        impl SQLTableInfo for #struct_ident {
            fn name(&self) -> &str {
                <Self as SQLSchema<'_, PostgresSchemaType, PostgresValue<'_>>>::NAME
            }
            fn columns(&self) -> &'static [&'static dyn SQLColumnInfo] {
                #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
                #[allow(non_upper_case_globals)]
                static COLUMNS: [&'static dyn SQLColumnInfo; #columns_len] =
                    [#(&#column_zst_idents,)*];
                &COLUMNS
            }

            fn dependencies(&self) -> Box<[&'static dyn SQLTableInfo]> {
                SQLTableInfo::columns(self)
                    .iter()
                    .filter_map(|col| SQLColumnInfo::foreign_key(*col))
                    .map(|fk_col| SQLColumnInfo::table(fk_col))
                    .collect()
            }
        }

        impl PostgresTableInfo for #struct_ident {
            fn r#type(&self) -> &PostgresSchemaType {
                &<Self as SQLSchema<'_, PostgresSchemaType, PostgresValue<'_>>>::TYPE
            }
            fn postgres_columns(&self) -> &'static [&'static dyn PostgresColumnInfo] {
                #(#[allow(non_upper_case_globals)] static #column_zst_idents: #column_zst_idents = #column_zst_idents::new();)*
                #[allow(non_upper_case_globals)]
                static POSTGRES_COLUMNS: [&'static dyn PostgresColumnInfo; #columns_len] =
                    [#(&#column_zst_idents,)*];
                &POSTGRES_COLUMNS
            }

            fn postgres_dependencies(&self) -> Box<[&'static dyn PostgresTableInfo]> {
                PostgresTableInfo::postgres_columns(self)
                    .iter()
                    .filter_map(|col| PostgresColumnInfo::foreign_key(*col))
                    .map(|fk_col| PostgresColumnInfo::table(fk_col))
                    .collect()
            }
        }

        impl<'a> PostgresTable<'a> for #struct_ident {}

        impl<'a> ToSQL<'a, PostgresValue<'a>> for #struct_ident {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                static INSTANCE: #struct_ident = #struct_ident::new();
                SQL::table(&INSTANCE)
            }
        }
    })
}
