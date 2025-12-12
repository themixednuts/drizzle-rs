//! Code generation helper functions for PostgreSQL table macros.
//!
//! These functions provide reusable trait implementation generators,
//! matching the pattern used in the SQLite module for consistency.

use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate PostgreSQL ToSQL trait implementation
pub fn generate_to_sql(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl<'a> ToSQL<'a, PostgresValue<'a>> for #struct_ident {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                #body
            }
        }
    }
}

/// Generate PostgreSQL SQLColumn trait implementation
#[allow(clippy::too_many_arguments)]
pub fn generate_sql_column(
    struct_ident: &Ident,
    table: TokenStream,
    table_type: TokenStream,
    r#type: TokenStream,
    primary_key: TokenStream,
    not_null: TokenStream,
    unique: TokenStream,
    default: TokenStream,
    default_fn: TokenStream,
) -> TokenStream {
    quote! {
        impl<'a> SQLColumn<'a, PostgresValue<'a>> for #struct_ident {
            type Table = #table;
            type TableType = #table_type;
            type Type = #r#type;

            const PRIMARY_KEY: bool = #primary_key;
            const NOT_NULL: bool = #not_null;
            const UNIQUE: bool = #unique;
            const DEFAULT: Option<Self::Type> = #default;

            fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
                #default_fn
            }
        }
    }
}

/// Generate PostgresColumnInfo trait implementation
pub fn generate_postgres_column_info(
    ident: &Ident,
    is_serial: TokenStream,
    table: TokenStream,
    foreign_key: TokenStream,
) -> TokenStream {
    quote! {
        impl PostgresColumnInfo for #ident {
            fn is_serial(&self) -> bool {
                #is_serial
            }

            fn table(&self) -> &dyn PostgresTableInfo {
                #table
            }

            fn foreign_key(&self) -> Option<&'static dyn PostgresColumnInfo> {
                #foreign_key
            }
        }
    }
}

/// Generate PostgresColumn trait implementation
pub fn generate_postgres_column(struct_ident: &Ident, is_serial: TokenStream) -> TokenStream {
    quote! {
        impl<'a> PostgresColumn<'a> for #struct_ident {
            const SERIAL: bool = #is_serial;
        }
    }
}

/// Generate PostgresTableInfo trait implementation
pub fn generate_postgres_table_info(
    struct_ident: &Ident,
    r#type: TokenStream,
    columns: TokenStream,
) -> TokenStream {
    quote! {
        impl PostgresTableInfo for #struct_ident {
            fn r#type(&self) -> &PostgresSchemaType {
                #r#type
            }

            fn postgres_columns(&self) -> &'static [&'static dyn PostgresColumnInfo] {
                #columns
            }

            fn postgres_dependencies(&self) -> Box<[&'static dyn PostgresTableInfo]> {
                PostgresTableInfo::postgres_columns(self)
                    .iter()
                    .filter_map(|col| PostgresColumnInfo::foreign_key(*col))
                    .map(|fk_col| PostgresColumnInfo::table(fk_col))
                    .collect()
            }
        }
    }
}

/// Generate PostgresTable trait implementation
pub fn generate_postgres_table(struct_ident: &Ident) -> TokenStream {
    quote! {
        impl<'a> PostgresTable<'a> for #struct_ident {}
    }
}

/// Generate PostgreSQL SQLTable trait implementation
pub fn generate_sql_table(
    struct_ident: &Ident,
    select: TokenStream,
    insert: TokenStream,
    update: TokenStream,
    aliased: TokenStream,
) -> TokenStream {
    quote! {
        impl<'a> SQLTable<'a, PostgresSchemaType, PostgresValue<'a>> for #struct_ident {
            type Select = #select;
            type Insert<T> = #insert;
            type Update = #update;
            type Aliased = #aliased;

            fn alias(name: &'static str) -> Self::Aliased {
                #aliased::new(name)
            }
        }
    }
}

/// Generate PostgreSQL SQLSchema trait implementation
pub fn generate_sql_schema(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    const_sql: TokenStream,
    runtime_sql: Option<TokenStream>,
) -> TokenStream {
    let fn_method = runtime_sql
        .map(|v| {
            quote! {
                fn sql(&self) -> SQL<'a, PostgresValue<'a>> {
                    #v
                }
            }
        })
        .unwrap_or_else(|| quote! {});
    quote! {
        impl<'a> SQLSchema<'a, PostgresSchemaType, PostgresValue<'a>> for #struct_ident {
            const NAME: &'a str = #name;
            const TYPE: PostgresSchemaType = #r#type;
            const SQL: &'static str = #const_sql;
            #fn_method
        }
    }
}

/// Generate PostgreSQL SQLSchema for fields trait implementation
pub fn generate_sql_schema_field(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    sql: TokenStream,
) -> TokenStream {
    quote! {
        impl<'a> SQLSchema<'a, &'a str, PostgresValue<'a>> for #struct_ident {
            const NAME: &'a str = #name;
            const TYPE: &'a str = #r#type;
            const SQL: &'static str = "";

            fn sql(&self) -> SQL<'a, PostgresValue<'a>> {
                #sql
            }
        }
    }
}
