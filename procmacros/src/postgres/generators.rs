//! Code generation helper functions for PostgreSQL table macros.
//!
//! These functions provide reusable trait implementation generators using
//! fully-qualified paths from the paths module.

use crate::paths::{core as core_paths, postgres as postgres_paths};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate PostgreSQL ToSQL trait implementation
pub fn generate_to_sql(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    let to_sql = core_paths::to_sql();
    let sql = core_paths::sql();
    let postgres_value = postgres_paths::postgres_value();

    quote! {
        impl<'a> #to_sql<'a, #postgres_value<'a>> for #struct_ident {
            fn to_sql(&self) -> #sql<'a, #postgres_value<'a>> {
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
    let sql_column = core_paths::sql_column();
    let postgres_value = postgres_paths::postgres_value();

    quote! {
        impl<'a> #sql_column<'a, #postgres_value<'a>> for #struct_ident {
            type Table = #table;
            type TableType = #table_type;
            type Type = #r#type;

            const PRIMARY_KEY: bool = #primary_key;
            const NOT_NULL: bool = #not_null;
            const UNIQUE: bool = #unique;
            const DEFAULT: ::std::option::Option<Self::Type> = #default;

            fn default_fn(&'a self) -> ::std::option::Option<impl Fn() -> Self::Type> {
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
    let postgres_column_info = postgres_paths::postgres_column_info();
    let postgres_table_info = postgres_paths::postgres_table_info();

    quote! {
        impl #postgres_column_info for #ident {
            fn is_serial(&self) -> bool {
                #is_serial
            }

            fn table(&self) -> &dyn #postgres_table_info {
                #table
            }

            fn foreign_key(&self) -> ::std::option::Option<&'static dyn #postgres_column_info> {
                #foreign_key
            }
        }
    }
}

/// Generate PostgresColumn trait implementation
pub fn generate_postgres_column(struct_ident: &Ident, is_serial: TokenStream) -> TokenStream {
    let postgres_column = postgres_paths::postgres_column();

    quote! {
        impl<'a> #postgres_column<'a> for #struct_ident {
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
    let postgres_table_info = postgres_paths::postgres_table_info();
    let postgres_column_info = postgres_paths::postgres_column_info();
    let postgres_schema_type = postgres_paths::postgres_schema_type();

    quote! {
        impl #postgres_table_info for #struct_ident {
            fn r#type(&self) -> &#postgres_schema_type {
                #r#type
            }

            fn postgres_columns(&self) -> &'static [&'static dyn #postgres_column_info] {
                #columns
            }

            fn postgres_dependencies(&self) -> ::std::boxed::Box<[&'static dyn #postgres_table_info]> {
                #postgres_table_info::postgres_columns(self)
                    .iter()
                    .filter_map(|col| #postgres_column_info::foreign_key(*col))
                    .map(|fk_col| #postgres_column_info::table(fk_col))
                    .collect()
            }
        }
    }
}

/// Generate PostgresTable trait implementation
pub fn generate_postgres_table(struct_ident: &Ident) -> TokenStream {
    let postgres_table = postgres_paths::postgres_table();

    quote! {
        impl<'a> #postgres_table<'a> for #struct_ident {}
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
    let sql_table = core_paths::sql_table();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let postgres_value = postgres_paths::postgres_value();

    quote! {
        impl<'a> #sql_table<'a, #postgres_schema_type, #postgres_value<'a>> for #struct_ident {
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
    let sql_schema = core_paths::sql_schema();
    let sql = core_paths::sql();
    let postgres_schema_type = postgres_paths::postgres_schema_type();
    let postgres_value = postgres_paths::postgres_value();

    let fn_method = runtime_sql
        .map(|v| {
            quote! {
                fn sql(&self) -> #sql<'a, #postgres_value<'a>> {
                    #v
                }
            }
        })
        .unwrap_or_else(|| quote! {});
    quote! {
        impl<'a> #sql_schema<'a, #postgres_schema_type, #postgres_value<'a>> for #struct_ident {
            const NAME: &'a str = #name;
            const TYPE: #postgres_schema_type = #r#type;
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
    let sql_schema = core_paths::sql_schema();
    let sql_type = core_paths::sql();
    let postgres_value = postgres_paths::postgres_value();

    quote! {
        impl<'a> #sql_schema<'a, &'a str, #postgres_value<'a>> for #struct_ident {
            const NAME: &'a str = #name;
            const TYPE: &'a str = #r#type;
            const SQL: &'static str = "";

            fn sql(&self) -> #sql_type<'a, #postgres_value<'a>> {
                #sql
            }
        }
    }
}
