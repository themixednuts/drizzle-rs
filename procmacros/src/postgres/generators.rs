//! Code generation helper functions for `PostgreSQL` table macros.
//!
//! These functions provide reusable trait implementation generators using
//! fully-qualified paths from the paths module.
//!
//! Note: Generic generators (`ToSQL`, `SQLColumn`, `SQLTable`, `SQLSchema`) are implemented
//! in `common::generators` and re-exported here for API stability. The functions
//! in this module delegate to the common implementations with PostgreSQL-specific types.

#![allow(dead_code)]

use crate::common::PostgresDialect;
use crate::common::generators as common_gen;
use crate::paths::postgres as postgres_paths;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate `PostgreSQL` `ToSQL` trait implementation.
///
/// Delegates to the common generator with `PostgreSQL` dialect.
pub fn generate_to_sql(struct_ident: &Ident, body: &TokenStream) -> TokenStream {
    common_gen::generate_to_sql::<PostgresDialect>(struct_ident, body)
}

/// Generate `PostgreSQL` `SQLColumn` trait implementation.
///
/// Delegates to the common generator with `PostgreSQL` dialect.
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
    common_gen::generate_sql_column::<PostgresDialect>(
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

// =============================================================================
// PostgreSQL-Specific Generators
// =============================================================================

/// Generate `PostgresColumn` trait implementation
pub fn generate_postgres_column(struct_ident: &Ident, is_serial: &TokenStream) -> TokenStream {
    let postgres_column = postgres_paths::postgres_column();

    quote! {
        impl<'a> #postgres_column<'a> for #struct_ident {
            const SERIAL: bool = #is_serial;
        }
    }
}

/// Generate `PostgresTable` trait implementation
pub fn generate_postgres_table(struct_ident: &Ident) -> TokenStream {
    let postgres_table = postgres_paths::postgres_table();

    quote! {
        impl<'a> #postgres_table<'a> for #struct_ident {}
    }
}

/// Generate `PostgreSQL` `SQLTable` trait implementation.
///
/// Delegates to the common generator with `PostgreSQL` dialect.
pub use common_gen::SQLTableConfig;

pub fn generate_sql_table(config: SQLTableConfig<'_>) -> TokenStream {
    common_gen::generate_sql_table::<PostgresDialect>(config)
}

/// Generate `PostgreSQL` `SQLSchema` trait implementation.
///
/// Delegates to the common generator with `PostgreSQL` dialect.
pub fn generate_sql_schema(
    struct_ident: &Ident,
    name: &TokenStream,
    r#type: &TokenStream,
    const_sql: &TokenStream,
) -> TokenStream {
    common_gen::generate_sql_schema::<PostgresDialect>(struct_ident, name, r#type, const_sql)
}

/// Generate `PostgreSQL` `SQLSchema` for fields trait implementation.
///
/// Delegates to the common generator with `PostgreSQL` dialect.
pub fn generate_sql_schema_field(
    struct_ident: &Ident,
    name: &TokenStream,
    r#type: &TokenStream,
    sql: &TokenStream,
) -> TokenStream {
    common_gen::generate_sql_schema_field::<PostgresDialect>(struct_ident, name, r#type, sql)
}
