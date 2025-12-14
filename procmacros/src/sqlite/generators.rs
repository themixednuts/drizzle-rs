//! Code generation helper functions for SQLite table macros.
//!
//! These functions provide reusable trait implementation generators using
//! fully-qualified paths from the paths module.

use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate SQLite ToSQL trait implementation
pub fn generate_to_sql(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    let to_sql = core_paths::to_sql();
    let sql = core_paths::sql();
    let sqlite_value = sqlite_paths::sqlite_value();

    quote! {
        impl<'a> #to_sql<'a, #sqlite_value<'a>> for #struct_ident {
            fn to_sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                #body
            }
        }
    }
}

/// Generate SQLite SQLColumn trait implementation
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
    let sqlite_value = sqlite_paths::sqlite_value();

    quote! {
        impl<'a> #sql_column<'a, #sqlite_value<'a>> for #struct_ident {
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

pub fn generate_sqlite_column_info(
    ident: &Ident,
    is_autoincrement: TokenStream,
    table: TokenStream,
    foreign_key: TokenStream,
) -> TokenStream {
    let sqlite_column_info = sqlite_paths::sqlite_column_info();
    let sqlite_table_info = sqlite_paths::sqlite_table_info();

    quote! {
        impl #sqlite_column_info for #ident {
            fn is_autoincrement(&self) -> bool {
                #is_autoincrement
            }

            fn table(&self) -> &dyn #sqlite_table_info {
                #table
            }

            fn foreign_key(&self) -> ::std::option::Option<&'static dyn #sqlite_column_info> {
                #foreign_key
            }
        }
    }
}

/// Generate SQLite SQLiteColumn trait implementation
pub fn generate_sqlite_column(struct_ident: &Ident, is_autoincrement: TokenStream) -> TokenStream {
    let sqlite_column = sqlite_paths::sqlite_column();

    quote! {
        impl<'a> #sqlite_column<'a> for #struct_ident {
            const AUTOINCREMENT: bool = #is_autoincrement;
        }
    }
}

/// Generate SQLite SQLiteTableInfo trait implementation
pub fn generate_sqlite_table_info(
    struct_ident: &Ident,
    r#type: TokenStream,
    strict: TokenStream,
    without_rowid: TokenStream,
    columns: TokenStream,
) -> TokenStream {
    let sqlite_table_info = sqlite_paths::sqlite_table_info();
    let sqlite_column_info = sqlite_paths::sqlite_column_info();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();

    quote! {
        impl #sqlite_table_info for #struct_ident {
            fn r#type(&self) -> &#sqlite_schema_type {
                #r#type
            }

            fn strict(&self) -> bool {
                #strict
            }
            fn without_rowid(&self) -> bool {
                #without_rowid
            }
            fn sqlite_columns(&self) -> &'static [&'static dyn #sqlite_column_info] {
                #columns
            }

            fn sqlite_dependencies(&self) -> ::std::boxed::Box<[&'static dyn #sqlite_table_info]> {
                #sqlite_table_info::sqlite_columns(self)
                    .iter()
                    .filter_map(|col| #sqlite_column_info::foreign_key(*col))
                    .map(|fk_col| #sqlite_column_info::table(fk_col))
                    .collect()
            }
        }
    }
}

/// Generate SQLite SQLiteTable trait implementation
pub fn generate_sqlite_table(
    struct_ident: &Ident,
    without_rowid: TokenStream,
    strict: TokenStream,
) -> TokenStream {
    let sqlite_table = sqlite_paths::sqlite_table();

    quote! {
        impl<'a> #sqlite_table<'a> for #struct_ident {
            const WITHOUT_ROWID: bool = #without_rowid;
            const STRICT: bool = #strict;
        }
    }
}

/// Generate SQLite SQLTable trait implementation
pub fn generate_sql_table(
    struct_ident: &Ident,
    select: TokenStream,
    insert: TokenStream,
    update: TokenStream,
    aliased: TokenStream,
) -> TokenStream {
    let sql_table = core_paths::sql_table();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_value = sqlite_paths::sqlite_value();

    quote! {
        impl<'a> #sql_table<'a, #sqlite_schema_type, #sqlite_value<'a>> for #struct_ident {
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

/// Generate SQLite SQLSchema trait implementation
pub fn generate_sql_schema(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    const_sql: TokenStream,
    runtime_sql: Option<TokenStream>,
) -> TokenStream {
    let sql_schema = core_paths::sql_schema();
    let sql = core_paths::sql();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();
    let sqlite_value = sqlite_paths::sqlite_value();

    let fn_method = runtime_sql
        .map(|v| {
            quote! {
                fn sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                    #v
                }
            }
        })
        .unwrap_or_else(|| quote! {});
    quote! {
        impl<'a> #sql_schema<'a, #sqlite_schema_type, #sqlite_value<'a>> for #struct_ident {
            const NAME: &'a str = #name;
            const TYPE: #sqlite_schema_type = #r#type;
            const SQL: &'static str = #const_sql;
            #fn_method
        }
    }
}

/// Generate SQLite SQLSchema for fields trait implementation
pub fn generate_sql_schema_field(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    sql: TokenStream,
) -> TokenStream {
    let sql_schema = core_paths::sql_schema();
    let sql_type = core_paths::sql();
    let sqlite_value = sqlite_paths::sqlite_value();

    quote! {
        impl<'a> #sql_schema<'a, &'a str, #sqlite_value<'a>> for #struct_ident {
            const NAME: &'a str = #name;
            const TYPE: &'a str = #r#type;
            const SQL: &'static str = "";

            fn sql(&self) -> #sql_type<'a, #sqlite_value<'a>> {
                #sql_type::raw(#sql)
            }
        }
    }
}
