//! Dialect-agnostic code generation traits and utilities.
//!
//! This module provides traits that abstract over the differences between
//! SQLite and PostgreSQL code generation, allowing shared generator functions
//! to work with both dialects.

use proc_macro2::TokenStream;
use quote::quote;
use syn::Ident;

use crate::paths::core as core_paths;

/// Provides dialect-specific type paths for code generation.
///
/// This trait is implemented by both SQLite and PostgreSQL to provide
/// the appropriate fully-qualified paths for their respective types.
#[allow(dead_code)]
pub(crate) trait GeneratorPaths {
    /// Returns the path to the dialect's value type (e.g., `SQLiteValue`, `PostgresValue`)
    fn value_type() -> TokenStream;

    /// Returns the path to the dialect's schema type (e.g., `SQLiteSchemaType`, `PostgresSchemaType`)
    fn schema_type() -> TokenStream;

    /// Returns the path to the dialect's column trait
    fn column_trait() -> TokenStream;

    /// Returns the path to the dialect's table trait
    fn table_trait() -> TokenStream;

    /// Returns the path to the dialect's column info trait
    fn column_info_trait() -> TokenStream;

    /// Returns the path to the dialect's table info trait
    fn table_info_trait() -> TokenStream;
}

/// Type alias for dialect selection
pub(crate) trait Dialect: GeneratorPaths {}

// =============================================================================
// SQLite Dialect Implementation
// =============================================================================

#[cfg(feature = "sqlite")]
mod sqlite_impl {
    use super::*;

    /// Marker struct for SQLite dialect
    pub struct SqliteDialect;

    impl GeneratorPaths for SqliteDialect {
        fn value_type() -> TokenStream {
            crate::paths::sqlite::sqlite_value()
        }

        fn schema_type() -> TokenStream {
            crate::paths::sqlite::sqlite_schema_type()
        }

        fn column_trait() -> TokenStream {
            crate::paths::sqlite::sqlite_column()
        }

        fn table_trait() -> TokenStream {
            crate::paths::sqlite::sqlite_table()
        }

        fn column_info_trait() -> TokenStream {
            crate::paths::sqlite::sqlite_column_info()
        }

        fn table_info_trait() -> TokenStream {
            crate::paths::sqlite::sqlite_table_info()
        }
    }

    impl Dialect for SqliteDialect {}
}

#[cfg(feature = "sqlite")]
pub(crate) use sqlite_impl::SqliteDialect;

// =============================================================================
// PostgreSQL Dialect Implementation
// =============================================================================

#[cfg(feature = "postgres")]
mod postgres_impl {
    use super::*;

    /// Marker struct for PostgreSQL dialect
    pub struct PostgresDialect;

    impl GeneratorPaths for PostgresDialect {
        fn value_type() -> TokenStream {
            crate::paths::postgres::postgres_value()
        }

        fn schema_type() -> TokenStream {
            crate::paths::postgres::postgres_schema_type()
        }

        fn column_trait() -> TokenStream {
            crate::paths::postgres::postgres_column()
        }

        fn table_trait() -> TokenStream {
            crate::paths::postgres::postgres_table()
        }

        fn column_info_trait() -> TokenStream {
            crate::paths::postgres::postgres_column_info()
        }

        fn table_info_trait() -> TokenStream {
            crate::paths::postgres::postgres_table_info()
        }
    }

    impl Dialect for PostgresDialect {}
}

#[cfg(feature = "postgres")]
pub(crate) use postgres_impl::PostgresDialect;

/// Generate ToSQL trait implementation for a given dialect.
///
/// This is a dialect-agnostic version that works with both SQLite and PostgreSQL.
pub(crate) fn generate_to_sql<D: Dialect>(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    let to_sql = core_paths::to_sql();
    let sql = core_paths::sql();
    let value_type = D::value_type();

    quote! {
        impl<'a> #to_sql<'a, #value_type<'a>> for #struct_ident {
            fn to_sql(&self) -> #sql<'a, #value_type<'a>> {
                #body
            }
        }
    }
}

/// Generate SQLColumn trait implementation for a given dialect.
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_sql_column<D: Dialect>(
    struct_ident: &Ident,
    table: TokenStream,
    table_type: TokenStream,
    foreign_keys: TokenStream,
    r#type: TokenStream,
    primary_key: TokenStream,
    not_null: TokenStream,
    unique: TokenStream,
    default: TokenStream,
    default_fn: TokenStream,
) -> TokenStream {
    let sql_column = core_paths::sql_column();
    let value_type = D::value_type();

    quote! {
        impl<'a> #sql_column<'a, #value_type<'a>> for #struct_ident {
            type Table = #table;
            type TableType = #table_type;
            type ForeignKeys = #foreign_keys;
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

/// Generate SQLTable trait implementation for a given dialect.
#[allow(clippy::too_many_arguments)]
pub(crate) fn generate_sql_table<D: Dialect>(
    struct_ident: &Ident,
    select: TokenStream,
    insert: TokenStream,
    update: TokenStream,
    aliased: TokenStream,
    foreign_keys: TokenStream,
    primary_key: TokenStream,
    constraints: TokenStream,
) -> TokenStream {
    let sql_table = core_paths::sql_table();
    let sql_table_meta = core_paths::sql_table_meta();
    let schema_type = D::schema_type();
    let value_type = D::value_type();

    quote! {
        impl<'a> #sql_table<'a, #schema_type, #value_type<'a>> for #struct_ident {
            type Select = #select;
            type Insert<T> = #insert;
            type Update = #update;
            type Aliased = #aliased;
            type ForeignKeys = #foreign_keys;
            type PrimaryKey = #primary_key;
            type Constraints = #constraints;

            fn alias(name: &'static str) -> Self::Aliased {
                #aliased::new(name)
            }
        }

        impl #sql_table_meta for #struct_ident {
            type ForeignKeys = #foreign_keys;
            type PrimaryKey = #primary_key;
            type Constraints = #constraints;
        }
    }
}

/// Generate SQLSchema trait implementation for a given dialect.
pub(crate) fn generate_sql_schema<D: Dialect>(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    const_sql: TokenStream,
    runtime_sql: Option<TokenStream>,
) -> TokenStream {
    let sql_schema = core_paths::sql_schema();
    let sql = core_paths::sql();
    let schema_type = D::schema_type();
    let value_type = D::value_type();

    let fn_method = runtime_sql
        .map(|v| {
            quote! {
                fn sql(&self) -> #sql<'a, #value_type<'a>> {
                    #v
                }
            }
        })
        .unwrap_or_default();

    quote! {
        impl<'a> #sql_schema<'a, #schema_type, #value_type<'a>> for #struct_ident {
            const NAME: &'static str = #name;
            const TYPE: #schema_type = #r#type;
            const SQL: &'static str = #const_sql;
            #fn_method
        }
    }
}

/// Generate SQLSchema for fields trait implementation for a given dialect.
pub(crate) fn generate_sql_schema_field<D: Dialect>(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    sql: TokenStream,
) -> TokenStream {
    let sql_schema = core_paths::sql_schema();
    let sql_type = core_paths::sql();
    let value_type = D::value_type();

    quote! {
        impl<'a> #sql_schema<'a, &'a str, #value_type<'a>> for #struct_ident {
            const NAME: &'static str = #name;
            const TYPE: &'a str = #r#type;
            const SQL: &'static str = "";

            fn sql(&self) -> #sql_type<'a, #value_type<'a>> {
                #sql_type::raw(#sql)
            }
        }
    }
}
