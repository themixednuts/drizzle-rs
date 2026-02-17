//! FromRow derive macro implementations for database row conversion.
//!
//! This module generates `TryFrom` implementations for converting database rows
//! to Rust structs for various database drivers.

use crate::common::{extract_struct_fields, parse_column_reference};
use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DeriveInput, Field, Ident, Result};

#[cfg(feature = "libsql")]
mod libsql;
#[cfg(feature = "postgres")]
mod postgres;
#[cfg(feature = "rusqlite")]
mod rusqlite;
#[cfg(any(feature = "libsql", feature = "turso"))]
mod shared;
#[cfg(feature = "turso")]
mod turso;

// =============================================================================
// Shared Helper Functions
// =============================================================================

/// Generate field assignments for a driver, handling both tuple and named structs.
fn generate_field_assignments<F>(
    fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>,
    is_tuple: bool,
    generator: F,
) -> Result<Vec<TokenStream>>
where
    F: Fn(usize, &Field, Option<&Ident>) -> Result<TokenStream>,
{
    fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            let field_name = if is_tuple { None } else { field.ident.as_ref() };
            generator(idx, field, field_name)
        })
        .collect()
}

/// Generate a TryFrom implementation for a specific driver.
#[cfg(feature = "sqlite")]
fn generate_driver_try_from(
    struct_name: &Ident,
    row_type: TokenStream,
    error_type: TokenStream,
    field_assignments: &[TokenStream],
    is_tuple: bool,
) -> TokenStream {
    let construct = if is_tuple {
        quote! { Self(#(#field_assignments)*) }
    } else {
        quote! { Self { #(#field_assignments)* } }
    };

    quote! {
        impl ::std::convert::TryFrom<&#row_type> for #struct_name {
            type Error = #error_type;

            fn try_from(row: &#row_type) -> ::std::result::Result<Self, Self::Error> {
                ::std::result::Result::Ok(#construct)
            }
        }
    }
}

/// Generate a FromDrizzleRow implementation for a specific driver.
///
/// `impl_generics` should be e.g. `<'__drizzle_r>` for rusqlite, or empty for others.
fn generate_from_drizzle_row_impl(
    struct_name: &Ident,
    impl_generics: TokenStream,
    row_type: TokenStream,
    error_type: TokenStream,
    field_count: usize,
    fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>,
    is_tuple: bool,
) -> TokenStream {
    let from_drizzle_row_fields: Vec<_> = fields
        .iter()
        .enumerate()
        .map(|(idx, field)| {
            if is_tuple {
                quote! {
                    drizzle::core::FromDrizzleRow::from_row_at(row, offset + #idx)?,
                }
            } else {
                let name = field.ident.as_ref().unwrap();
                quote! {
                    #name: drizzle::core::FromDrizzleRow::from_row_at(row, offset + #idx)?,
                }
            }
        })
        .collect();

    let construct = if is_tuple {
        quote! { Self(#(#from_drizzle_row_fields)*) }
    } else {
        quote! { Self { #(#from_drizzle_row_fields)* } }
    };

    // Add where bounds so the impl only exists when all field types support FromDrizzleRow
    let where_bounds: Vec<_> = fields
        .iter()
        .map(|field| {
            let ty = &field.ty;
            quote! { #ty: drizzle::core::FromDrizzleRow<#row_type> }
        })
        .collect();

    quote! {
        impl #impl_generics drizzle::core::FromDrizzleRow<#row_type> for #struct_name
        where
            #(#where_bounds,)*
        {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(row: &#row_type, offset: usize) -> ::std::result::Result<Self, #error_type> {
                ::std::result::Result::Ok(#construct)
            }
        }
    }
}

/// Generate ToSQL implementation for FromRow structs.
///
/// This allows using the struct as a column selector in queries.
fn generate_tosql_impl(
    struct_name: &Ident,
    fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>,
    is_tuple: bool,
    value_type: TokenStream,
) -> TokenStream {
    if is_tuple {
        return quote! {};
    }

    let sql = core_paths::sql();
    let to_sql = core_paths::to_sql();
    let token = core_paths::token();

    let column_specs = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();

            if let Some(column_ref) = parse_column_reference(field) {
                quote! {
                    columns.push(#to_sql::to_sql(&#column_ref).alias(#field_name_str));
                }
            } else {
                quote! {
                    columns.push(#sql::raw(#field_name_str));
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        impl<'a> #to_sql<'a, #value_type<'a>> for #struct_name {
            fn to_sql(&self) -> #sql<'a, #value_type<'a>> {
                let mut columns = ::std::vec::Vec::new();
                #(#column_specs)*
                #sql::join(columns, #token::COMMA)
            }
        }
    }
}

// =============================================================================
// SQLite FromRow Implementation
// =============================================================================

/// Generate SQLite-specific FromRow implementation (rusqlite, libsql, turso)
#[cfg(feature = "sqlite")]
pub(crate) fn generate_sqlite_from_row_impl(input: DeriveInput) -> Result<TokenStream> {
    use crate::paths::sqlite as sqlite_paths;

    let struct_name = &input.ident;
    let (fields, is_tuple) = extract_struct_fields(&input)?;
    #[allow(unused_variables)]
    let drizzle_error = core_paths::drizzle_error();
    let sqlite_value = sqlite_paths::sqlite_value();

    let mut impl_blocks: Vec<TokenStream> = Vec::new();

    let field_count = fields.len();

    // Rusqlite implementation
    #[cfg(feature = "rusqlite")]
    {
        let field_assignments =
            generate_field_assignments(fields, is_tuple, rusqlite::generate_field_assignment)?;

        impl_blocks.push(generate_driver_try_from(
            struct_name,
            quote!(::rusqlite::Row<'_>),
            quote!(#drizzle_error),
            &field_assignments,
            is_tuple,
        ));
        impl_blocks.push(generate_from_drizzle_row_impl(
            struct_name,
            quote!(<'__drizzle_r>),
            quote!(::rusqlite::Row<'__drizzle_r>),
            quote!(#drizzle_error),
            field_count,
            fields,
            is_tuple,
        ));
    }

    // Turso implementation
    #[cfg(feature = "turso")]
    {
        let field_assignments =
            generate_field_assignments(fields, is_tuple, turso::generate_field_assignment)?;

        impl_blocks.push(generate_driver_try_from(
            struct_name,
            quote!(::turso::Row),
            quote!(#drizzle_error),
            &field_assignments,
            is_tuple,
        ));
        impl_blocks.push(generate_from_drizzle_row_impl(
            struct_name,
            quote!(),
            quote!(::turso::Row),
            quote!(#drizzle_error),
            field_count,
            fields,
            is_tuple,
        ));
    }

    // Libsql implementation
    #[cfg(feature = "libsql")]
    {
        let field_assignments =
            generate_field_assignments(fields, is_tuple, libsql::generate_field_assignment)?;

        impl_blocks.push(generate_driver_try_from(
            struct_name,
            quote!(::libsql::Row),
            quote!(#drizzle_error),
            &field_assignments,
            is_tuple,
        ));
        impl_blocks.push(generate_from_drizzle_row_impl(
            struct_name,
            quote!(),
            quote!(::libsql::Row),
            quote!(#drizzle_error),
            field_count,
            fields,
            is_tuple,
        ));
    }

    // Generate ToSQL implementation
    let tosql_impl = generate_tosql_impl(struct_name, fields, is_tuple, sqlite_value);

    let into_select_target = core_paths::into_select_target();
    let select_expr = quote!(drizzle::core::SelectExpr);

    Ok(quote! {
        #(#impl_blocks)*
        #tosql_impl
        impl #into_select_target for #struct_name {
            type Marker = #select_expr;
        }
    })
}

// =============================================================================
// PostgreSQL FromRow Implementation
// =============================================================================

/// Generate PostgreSQL-specific FromRow implementation (postgres-sync, tokio-postgres)
#[cfg(feature = "postgres")]
pub(crate) fn generate_postgres_from_row_impl(input: DeriveInput) -> Result<TokenStream> {
    use crate::paths::postgres as postgres_paths;

    let struct_name = &input.ident;
    let (fields, is_tuple) = extract_struct_fields(&input)?;
    let drizzle_error = core_paths::drizzle_error();
    let postgres_value = postgres_paths::postgres_value();

    let field_assignments =
        generate_field_assignments(fields, is_tuple, postgres::generate_field_assignment)?;

    let struct_construct = if is_tuple {
        quote! {
            ::std::result::Result::Ok(Self(
                #(#field_assignments)*
            ))
        }
    } else {
        quote! {
            ::std::result::Result::Ok(Self {
                #(#field_assignments)*
            })
        }
    };

    // Generate ToSQL implementation
    let tosql_impl = generate_tosql_impl(struct_name, fields, is_tuple, postgres_value);

    // Generate the TryFrom implementations with proper conditional compilation
    // to avoid duplicate implementations (postgres::Row is tokio_postgres::Row)
    //
    // Note: We do NOT generate FromDrizzleRow here because postgres::Row is a concrete
    // type (no lifetime parameter), so where bounds like `Role: FromDrizzleRow<postgres::Row>`
    // are provably unsatisfiable and rejected by the compiler when any field type lacks
    // the impl. The table macro's SelectModel generation handles FromDrizzleRow separately
    // with field-type inspection to skip unsupported cases.
    Ok(quote! {
        // When tokio-postgres is enabled, use tokio_postgres::Row
        // This covers both "tokio-postgres only" and "both features enabled" cases
        #[cfg(feature = "tokio-postgres")]
        impl ::std::convert::TryFrom<&::tokio_postgres::Row> for #struct_name {
            type Error = #drizzle_error;

            fn try_from(row: &::tokio_postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                #struct_construct
            }
        }

        // When only postgres-sync is enabled (without tokio-postgres), use postgres::Row
        #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
        impl ::std::convert::TryFrom<&::postgres::Row> for #struct_name {
            type Error = #drizzle_error;

            fn try_from(row: &::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                #struct_construct
            }
        }

        #tosql_impl
        impl drizzle::core::IntoSelectTarget for #struct_name {
            type Marker = drizzle::core::SelectExpr;
        }
    })
}
