//! Code generation helper functions for SQLite table macros.
//!
//! These functions provide reusable trait implementation generators using
//! fully-qualified paths from the paths module.
//!
//! Note: Generic generators (ToSQL, SQLColumn, SQLTable, SQLSchema) are implemented
//! in `common::generators` and re-exported here for API stability. The functions
//! in this module delegate to the common implementations with SQLite-specific types.

use crate::common::SqliteDialect;
use crate::common::generators as common_gen;
use crate::paths::sqlite as sqlite_paths;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate SQLite ToSQL trait implementation.
///
/// Delegates to the common generator with SQLite dialect.
pub fn generate_to_sql(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    common_gen::generate_to_sql::<SqliteDialect>(struct_ident, body)
}

/// Generate SQLite SQLColumn trait implementation.
///
/// Delegates to the common generator with SQLite dialect.
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
    common_gen::generate_sql_column::<SqliteDialect>(
        struct_ident,
        table,
        table_type,
        r#type,
        primary_key,
        not_null,
        unique,
        default,
        default_fn,
    )
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
    dependencies: TokenStream,
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

            fn sqlite_dependencies(&self) -> &'static [&'static dyn #sqlite_table_info] {
                #dependencies
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

/// Generate SQLite SQLTable trait implementation.
///
/// Delegates to the common generator with SQLite dialect.
pub fn generate_sql_table(
    struct_ident: &Ident,
    select: TokenStream,
    insert: TokenStream,
    update: TokenStream,
    aliased: TokenStream,
) -> TokenStream {
    common_gen::generate_sql_table::<SqliteDialect>(struct_ident, select, insert, update, aliased)
}

/// Generate SQLite SQLSchema trait implementation.
///
/// Delegates to the common generator with SQLite dialect.
pub fn generate_sql_schema(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    const_sql: TokenStream,
    runtime_sql: Option<TokenStream>,
) -> TokenStream {
    common_gen::generate_sql_schema::<SqliteDialect>(
        struct_ident,
        name,
        r#type,
        const_sql,
        runtime_sql,
    )
}

/// Generate SQLite SQLSchema for fields trait implementation.
///
/// Delegates to the common generator with SQLite dialect.
pub fn generate_sql_schema_field(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    sql: TokenStream,
) -> TokenStream {
    common_gen::generate_sql_schema_field::<SqliteDialect>(struct_ident, name, r#type, sql)
}
