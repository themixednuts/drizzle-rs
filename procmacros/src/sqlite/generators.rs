//! Code generation helper functions for `SQLite` table macros.
//!
//! These functions provide reusable trait implementation generators using
//! fully-qualified paths from the paths module.
//!
//! Note: Generic generators (`ToSQL`, `SQLColumn`, `SQLTable`, `SQLSchema`) are implemented
//! in `common::generators` and re-exported here for API stability. The functions
//! in this module delegate to the common implementations with SQLite-specific types.

use crate::common::SqliteDialect;
use crate::common::generators as common_gen;
use crate::paths::sqlite as sqlite_paths;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate `SQLite` `ToSQL` trait implementation.
///
/// Delegates to the common generator with `SQLite` dialect.
pub fn generate_to_sql(struct_ident: &Ident, body: &TokenStream) -> TokenStream {
    common_gen::generate_to_sql::<SqliteDialect>(struct_ident, body)
}

/// Generate `SQLite` `SQLColumn` trait implementation.
///
/// Delegates to the common generator with `SQLite` dialect.
#[allow(clippy::too_many_arguments)]
pub fn generate_sql_column(
    struct_ident: &Ident,
    table: &TokenStream,
    table_type: &TokenStream,
    foreign_keys: &TokenStream,
    r#type: &TokenStream,
    primary_key: &TokenStream,
    not_null: &TokenStream,
    unique: &TokenStream,
    default: &TokenStream,
    default_fn: &TokenStream,
) -> TokenStream {
    common_gen::generate_sql_column::<SqliteDialect>(
        struct_ident,
        table,
        table_type,
        foreign_keys,
        r#type,
        primary_key,
        not_null,
        unique,
        default,
        default_fn,
    )
}

/// Generate `SQLite` `SQLiteColumn` trait implementation
pub fn generate_sqlite_column(struct_ident: &Ident, is_autoincrement: &TokenStream) -> TokenStream {
    let sqlite_column = sqlite_paths::sqlite_column();

    quote! {
        impl<'a> #sqlite_column<'a> for #struct_ident {
            const AUTOINCREMENT: bool = #is_autoincrement;
        }
    }
}

/// Generate `SQLite` `SQLiteTable` trait implementation
pub fn generate_sqlite_table(
    struct_ident: &Ident,
    without_rowid: &TokenStream,
    strict: &TokenStream,
) -> TokenStream {
    let sqlite_table = sqlite_paths::sqlite_table();

    quote! {
        impl<'a> #sqlite_table<'a> for #struct_ident {
            const WITHOUT_ROWID: bool = #without_rowid;
            const STRICT: bool = #strict;
        }
    }
}

/// Generate `SQLite` `SQLTable` trait implementation.
///
/// Delegates to the common generator with `SQLite` dialect.
pub use common_gen::SQLTableConfig;

pub fn generate_sql_table(config: SQLTableConfig<'_>) -> TokenStream {
    common_gen::generate_sql_table::<SqliteDialect>(config)
}

/// Generate `SQLite` `SQLSchema` trait implementation.
///
/// Delegates to the common generator with `SQLite` dialect.
pub fn generate_sql_schema(
    struct_ident: &Ident,
    name: &TokenStream,
    r#type: &TokenStream,
    const_sql: &TokenStream,
) -> TokenStream {
    common_gen::generate_sql_schema::<SqliteDialect>(struct_ident, name, r#type, const_sql)
}

/// Generate `SQLite` `SQLSchema` for fields trait implementation.
///
/// Delegates to the common generator with `SQLite` dialect.
pub fn generate_sql_schema_field(
    struct_ident: &Ident,
    name: &TokenStream,
    r#type: &TokenStream,
    sql: &TokenStream,
) -> TokenStream {
    common_gen::generate_sql_schema_field::<SqliteDialect>(struct_ident, name, r#type, sql)
}
