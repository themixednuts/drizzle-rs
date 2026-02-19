//! FromRow derive macro implementations for database row conversion.
//!
//! This module generates `TryFrom` implementations for converting database rows
//! to Rust structs for various database drivers.

use crate::common::{extract_struct_fields, parse_column_reference};
use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Expr, ExprPath, Field, Ident, Meta, Result, Visibility};

fn parse_default_from_table(input: &DeriveInput) -> Result<Option<ExprPath>> {
    let mut found: Option<ExprPath> = None;

    for attr in &input.attrs {
        if let Some(ident) = attr.path().get_ident()
            && ident == "from"
            && let Meta::List(meta_list) = &attr.meta
        {
            let expr = syn::parse2::<Expr>(meta_list.tokens.clone())?;
            let Expr::Path(path) = expr else {
                return Err(syn::Error::new_spanned(
                    &attr.meta,
                    "expected #[from(TableType)]",
                ));
            };

            if found.is_some() {
                return Err(syn::Error::new_spanned(
                    &attr.meta,
                    "duplicate #[from(...)] attribute",
                ));
            }

            found = Some(path);
        }
    }

    Ok(found)
}

fn extract_table_from_column_ref(column_ref: &ExprPath) -> Option<syn::Path> {
    let segment_count = column_ref.path.segments.len();
    if segment_count == 0 {
        return None;
    }

    let mut table_path = syn::Path {
        leading_colon: column_ref.path.leading_colon,
        segments: syn::punctuated::Punctuated::new(),
    };

    for segment in column_ref.path.segments.iter().take(segment_count - 1) {
        table_path.segments.push(segment.clone());
    }

    if table_path.segments.is_empty() {
        None
    } else {
        Some(table_path)
    }
}

fn collect_required_tables(
    fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>,
    default_from: Option<&ExprPath>,
) -> Vec<syn::Path> {
    let mut tables: Vec<syn::Path> = Vec::new();

    if let Some(default_table) = default_from
        && !tables.iter().any(|t| t == &default_table.path)
    {
        tables.push(default_table.path.clone());
    }

    for field in fields {
        if let Some(column_ref) = parse_column_reference(field) {
            if let Some(table_path) = extract_table_from_column_ref(&column_ref)
                && !tables.iter().any(|t| t == &table_path)
            {
                tables.push(table_path);
            }
        } else if let Some(default_table) = default_from
            && !tables.iter().any(|t| t == &default_table.path)
        {
            tables.push(default_table.path.clone());
        }
    }

    tables
}

#[cfg(feature = "sqlite")]
fn should_decode_named_fields_by_name(
    fields: &syn::punctuated::Punctuated<Field, syn::token::Comma>,
    is_tuple: bool,
    default_from: Option<&ExprPath>,
) -> bool {
    if is_tuple || default_from.is_some() {
        return false;
    }

    !fields
        .iter()
        .any(|field| parse_column_reference(field).is_some())
}

fn build_scope_list_type(table_paths: &[syn::Path]) -> TokenStream {
    let type_set_nil = core_paths::type_set_nil();
    let type_set_cons = core_paths::type_set_cons();
    table_paths.iter().rev().fold(
        type_set_nil,
        |acc, table_path| quote!(#type_set_cons<#table_path, #acc>),
    )
}

fn build_column_list_type(field_count: usize) -> TokenStream {
    let type_set_nil = core_paths::type_set_nil();
    let type_set_cons = core_paths::type_set_cons();
    let mut columns = quote!(#type_set_nil);
    for _ in 0..field_count {
        columns = quote!(#type_set_cons<(), #columns>);
    }
    columns
}

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

/// Generate a driver-specific FromDrizzleRow implementation.
fn generate_driver_from_drizzle_row_impl(
    struct_name: &Ident,
    impl_generics: TokenStream,
    row_type: TokenStream,
    error_type: TokenStream,
    field_assignments: &[TokenStream],
    is_tuple: bool,
    field_count: usize,
) -> TokenStream {
    let construct = if is_tuple {
        quote! { Self(#(#field_assignments)*) }
    } else {
        quote! { Self { #(#field_assignments)* } }
    };

    quote! {
        impl #impl_generics drizzle::core::FromDrizzleRow<#row_type> for #struct_name {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(row: &#row_type, offset: usize) -> ::std::result::Result<Self, #error_type> {
                ::std::result::Result::Ok(#construct)
            }
        }
    }
}

fn generate_driver_row_column_list_impl(
    struct_name: &Ident,
    impl_generics: TokenStream,
    row_type: TokenStream,
    field_count: usize,
) -> TokenStream {
    let row_column_list = core_paths::row_column_list();
    let columns = build_column_list_type(field_count);
    quote! {
        impl #impl_generics #row_column_list<#row_type> for #struct_name {
            type Columns = #columns;
        }
    }
}

/// Generate ToSQL implementation for FromRow structs.
///
/// This allows using the struct as a column selector in queries.
fn generate_tosql_impl(
    struct_name: &Ident,
    struct_vis: &Visibility,
    default_from: Option<&ExprPath>,
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
    let selector_ident = format_ident!("__DrizzleSelect{}", struct_name);
    let select_const_ident = format_ident!("Select");

    let column_specs = fields
        .iter()
        .map(|field| {
            let field_name = field.ident.as_ref().unwrap();
            let field_name_str = field_name.to_string();

            if let Some(column_ref) = parse_column_reference(field) {
                quote! {
                    columns.push(#to_sql::to_sql(&#column_ref).alias(#field_name_str));
                }
            } else if let Some(default_table) = default_from {
                quote! {
                    columns.push(#to_sql::to_sql(&#default_table::#field_name).alias(#field_name_str));
                }
            } else {
                quote! {
                    columns.push(#sql::ident(#field_name_str));
                }
            }
        })
        .collect::<Vec<_>>();

    quote! {
        #[doc(hidden)]
        #[derive(Clone, Copy, Debug, Default)]
        #struct_vis struct #selector_ident;

        impl #struct_name {
            #[doc(hidden)]
            #[allow(non_upper_case_globals)]
            #struct_vis const #select_const_ident: #selector_ident = #selector_ident;
        }

        impl<'a> #to_sql<'a, #value_type<'a>> for #struct_name {
            fn to_sql(&self) -> #sql<'a, #value_type<'a>> {
                let mut columns = ::std::vec::Vec::new();
                #(#column_specs)*
                #sql::join(columns, #token::COMMA)
            }
        }

        impl<'a> #to_sql<'a, #value_type<'a>> for #selector_ident {
            fn to_sql(&self) -> #sql<'a, #value_type<'a>> {
                let mut columns = ::std::vec::Vec::new();
                #(#column_specs)*
                #sql::join(columns, #token::COMMA)
            }
        }

        impl drizzle::core::IntoSelectTarget for #selector_ident {
            type Marker = drizzle::core::SelectAs<#struct_name>;
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
    let default_from = parse_default_from_table(&input)?;
    let (fields, is_tuple) = extract_struct_fields(&input)?;
    #[allow(unused_variables)]
    let drizzle_error = core_paths::drizzle_error();
    let sqlite_value = sqlite_paths::sqlite_value();

    let mut impl_blocks: Vec<TokenStream> = Vec::new();
    let field_count = fields.len();
    let decode_named_by_name =
        should_decode_named_fields_by_name(fields, is_tuple, default_from.as_ref());

    // Rusqlite implementation
    #[cfg(feature = "rusqlite")]
    {
        let field_assignments =
            generate_field_assignments(fields, is_tuple, rusqlite::generate_field_assignment)?;
        let from_drizzle_assignments = if decode_named_by_name {
            generate_field_assignments(fields, is_tuple, rusqlite::generate_field_assignment)?
        } else {
            fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    let field_name = if is_tuple { None } else { field.ident.as_ref() };
                    rusqlite::generate_field_assignment_with_index_expr(
                        quote!(offset + #idx),
                        field,
                        field_name,
                    )
                })
                .collect::<Result<Vec<_>>>()?
        };

        impl_blocks.push(generate_driver_try_from(
            struct_name,
            quote!(::rusqlite::Row<'_>),
            quote!(#drizzle_error),
            &field_assignments,
            is_tuple,
        ));
        impl_blocks.push(generate_driver_from_drizzle_row_impl(
            struct_name,
            quote!(<'__drizzle_r>),
            quote!(::rusqlite::Row<'__drizzle_r>),
            quote!(#drizzle_error),
            &from_drizzle_assignments,
            is_tuple,
            field_count,
        ));
        impl_blocks.push(generate_driver_row_column_list_impl(
            struct_name,
            quote!(<'__drizzle_r>),
            quote!(::rusqlite::Row<'__drizzle_r>),
            field_count,
        ));
    }

    // Turso implementation
    #[cfg(feature = "turso")]
    {
        let field_assignments =
            generate_field_assignments(fields, is_tuple, turso::generate_field_assignment)?;
        let from_drizzle_assignments = fields
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                let field_name = if is_tuple { None } else { field.ident.as_ref() };
                turso::generate_field_assignment_with_index_expr(
                    quote!(offset + #idx),
                    field,
                    field_name,
                )
            })
            .collect::<Result<Vec<_>>>()?;

        impl_blocks.push(generate_driver_try_from(
            struct_name,
            quote!(::turso::Row),
            quote!(#drizzle_error),
            &field_assignments,
            is_tuple,
        ));
        impl_blocks.push(generate_driver_from_drizzle_row_impl(
            struct_name,
            quote!(),
            quote!(::turso::Row),
            quote!(#drizzle_error),
            &from_drizzle_assignments,
            is_tuple,
            field_count,
        ));
        impl_blocks.push(generate_driver_row_column_list_impl(
            struct_name,
            quote!(),
            quote!(::turso::Row),
            field_count,
        ));
    }

    // Libsql implementation
    #[cfg(feature = "libsql")]
    {
        let field_assignments =
            generate_field_assignments(fields, is_tuple, libsql::generate_field_assignment)?;
        let from_drizzle_assignments = if decode_named_by_name {
            generate_field_assignments(fields, is_tuple, libsql::generate_field_assignment)?
        } else {
            fields
                .iter()
                .enumerate()
                .map(|(idx, field)| {
                    let field_name = if is_tuple { None } else { field.ident.as_ref() };
                    libsql::generate_field_assignment_with_index_expr(
                        quote!(offset + #idx),
                        field,
                        field_name,
                    )
                })
                .collect::<Result<Vec<_>>>()?
        };

        impl_blocks.push(generate_driver_try_from(
            struct_name,
            quote!(::libsql::Row),
            quote!(#drizzle_error),
            &field_assignments,
            is_tuple,
        ));
        impl_blocks.push(generate_driver_from_drizzle_row_impl(
            struct_name,
            quote!(),
            quote!(::libsql::Row),
            quote!(#drizzle_error),
            &from_drizzle_assignments,
            is_tuple,
            field_count,
        ));
        impl_blocks.push(generate_driver_row_column_list_impl(
            struct_name,
            quote!(),
            quote!(::libsql::Row),
            field_count,
        ));
    }

    // Generate ToSQL implementation
    let tosql_impl = generate_tosql_impl(
        struct_name,
        &input.vis,
        default_from.as_ref(),
        fields,
        is_tuple,
        sqlite_value,
    );

    let into_select_target = core_paths::into_select_target();
    let select_as = quote!(drizzle::core::SelectAs<#struct_name>);
    let select_as_from = quote!(drizzle::core::SelectAsFrom);
    let select_as_from_impl = if let Some(default_table) = default_from.as_ref() {
        quote! {
            impl #select_as_from<#default_table> for #struct_name {}
        }
    } else {
        quote! {
            impl<__Table> #select_as_from<__Table> for #struct_name {}
        }
    };

    let select_required_tables = quote!(drizzle::core::SelectRequiredTables);
    let required_tables = collect_required_tables(fields, default_from.as_ref());
    let required_scope = build_scope_list_type(&required_tables);
    let required_tables_impl = quote! {
        impl #select_required_tables for #struct_name {
            type RequiredTables = #required_scope;
        }
    };

    Ok(quote! {
        #(#impl_blocks)*
        #tosql_impl
        #select_as_from_impl
        #required_tables_impl
        impl #into_select_target for #struct_name {
            type Marker = #select_as;
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
    let default_from = parse_default_from_table(&input)?;
    let (fields, is_tuple) = extract_struct_fields(&input)?;
    let drizzle_error = core_paths::drizzle_error();
    let postgres_value = postgres_paths::postgres_value();

    let field_assignments =
        generate_field_assignments(fields, is_tuple, postgres::generate_field_assignment)?;
    let decode_named_by_name = !is_tuple
        && !fields
            .iter()
            .any(|field| parse_column_reference(field).is_some());

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
    let tosql_impl = generate_tosql_impl(
        struct_name,
        &input.vis,
        default_from.as_ref(),
        fields,
        is_tuple,
        postgres_value,
    );

    // Generate TryFrom + FromDrizzleRow implementations with proper conditional
    // compilation to avoid duplicate implementations
    // (postgres::Row is tokio_postgres::Row).
    let field_count = fields.len();
    let from_drizzle_assignments = if decode_named_by_name {
        fields
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                let field_name = field
                    .ident
                    .as_ref()
                    .ok_or_else(|| syn::Error::new_spanned(field, "expected named struct field"))?;
                postgres::generate_named_field_assignment_with_offset_fallback(
                    idx, field, field_name,
                )
            })
            .collect::<Result<Vec<_>>>()?
    } else {
        fields
            .iter()
            .enumerate()
            .map(|(idx, field)| {
                let field_name = if is_tuple { None } else { field.ident.as_ref() };
                postgres::generate_field_assignment_with_index_expr(
                    quote!(offset + #idx),
                    field,
                    field_name,
                )
            })
            .collect::<Result<Vec<_>>>()?
    };
    let tokio_from_drizzle_impl = generate_driver_from_drizzle_row_impl(
        struct_name,
        quote!(),
        quote!(::tokio_postgres::Row),
        quote!(#drizzle_error),
        &from_drizzle_assignments,
        is_tuple,
        field_count,
    );
    let tokio_row_column_list_impl = generate_driver_row_column_list_impl(
        struct_name,
        quote!(),
        quote!(::tokio_postgres::Row),
        field_count,
    );
    let sync_from_drizzle_impl = generate_driver_from_drizzle_row_impl(
        struct_name,
        quote!(),
        quote!(::postgres::Row),
        quote!(#drizzle_error),
        &from_drizzle_assignments,
        is_tuple,
        field_count,
    );
    let sync_row_column_list_impl = generate_driver_row_column_list_impl(
        struct_name,
        quote!(),
        quote!(::postgres::Row),
        field_count,
    );
    let select_as_from = quote!(drizzle::core::SelectAsFrom);
    let select_as_from_impl = if let Some(default_table) = default_from.as_ref() {
        quote! {
            impl #select_as_from<#default_table> for #struct_name {}
        }
    } else {
        quote! {
            impl<__Table> #select_as_from<__Table> for #struct_name {}
        }
    };

    let select_required_tables = quote!(drizzle::core::SelectRequiredTables);
    let required_tables = collect_required_tables(fields, default_from.as_ref());
    let required_scope = build_scope_list_type(&required_tables);
    let required_tables_impl = quote! {
        impl #select_required_tables for #struct_name {
            type RequiredTables = #required_scope;
        }
    };

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

        #[cfg(feature = "tokio-postgres")]
        #tokio_from_drizzle_impl

        #[cfg(feature = "tokio-postgres")]
        #tokio_row_column_list_impl

        // When only postgres-sync is enabled (without tokio-postgres), use postgres::Row
        #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
        impl ::std::convert::TryFrom<&::postgres::Row> for #struct_name {
            type Error = #drizzle_error;

            fn try_from(row: &::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                #struct_construct
            }
        }

        #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
        #sync_from_drizzle_impl

        #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
        #sync_row_column_list_impl

        #tosql_impl
        #select_as_from_impl
        #required_tables_impl
        impl drizzle::core::IntoSelectTarget for #struct_name {
            type Marker = drizzle::core::SelectAs<#struct_name>;
        }
    })
}
