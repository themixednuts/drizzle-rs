//! Code generation helper functions for PostgreSQL table macros.
//!
//! These functions provide reusable trait implementation generators using
//! fully-qualified paths from the paths module.
//!
//! Note: Generic generators (ToSQL, SQLColumn, SQLTable, SQLSchema) are implemented
//! in `common::generators` and re-exported here for API stability. The functions
//! in this module delegate to the common implementations with PostgreSQL-specific types.

use crate::common::generators as common_gen;
use crate::common::PostgresDialect;
use crate::paths::postgres as postgres_paths;
use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate PostgreSQL ToSQL trait implementation.
///
/// Delegates to the common generator with PostgreSQL dialect.
pub fn generate_to_sql(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    common_gen::generate_to_sql::<PostgresDialect>(struct_ident, body)
}

/// Generate PostgreSQL SQLColumn trait implementation.
///
/// Delegates to the common generator with PostgreSQL dialect.
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
    common_gen::generate_sql_column::<PostgresDialect>(
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

// =============================================================================
// PostgreSQL-Specific Generators
// =============================================================================

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

/// Generate PostgreSQL SQLTable trait implementation.
///
/// Delegates to the common generator with PostgreSQL dialect.
pub fn generate_sql_table(
    struct_ident: &Ident,
    select: TokenStream,
    insert: TokenStream,
    update: TokenStream,
    aliased: TokenStream,
) -> TokenStream {
    common_gen::generate_sql_table::<PostgresDialect>(struct_ident, select, insert, update, aliased)
}

/// Generate PostgreSQL SQLSchema trait implementation.
///
/// Delegates to the common generator with PostgreSQL dialect.
pub fn generate_sql_schema(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    const_sql: TokenStream,
    runtime_sql: Option<TokenStream>,
) -> TokenStream {
    common_gen::generate_sql_schema::<PostgresDialect>(struct_ident, name, r#type, const_sql, runtime_sql)
}

/// Generate PostgreSQL SQLSchema for fields trait implementation.
///
/// Delegates to the common generator with PostgreSQL dialect.
pub fn generate_sql_schema_field(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    sql: TokenStream,
) -> TokenStream {
    common_gen::generate_sql_schema_field::<PostgresDialect>(struct_ident, name, r#type, sql)
}
