use super::context::MacroContext;
use crate::generators::{generate_impl, generate_sql_column_info};
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use crate::sqlite::field::FieldInfo;
use crate::sqlite::generators::*;
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Result;

/// Generate a const that references the original marker tokens from the attribute.
///
/// This creates a hidden const that uses the exact tokens from `#[column(primary, unique)]`,
/// enabling rust-analyzer to resolve them and provide hover documentation.
fn generate_marker_const(info: &FieldInfo, _zst_ident: &Ident) -> TokenStream {
    if info.marker_exprs.is_empty() {
        return TokenStream::new();
    }

    let field_name = info.ident.to_string().to_uppercase();
    let marker_const_name = format_ident!("_ATTR_MARKERS_{}", field_name);
    let marker_count = info.marker_exprs.len();
    let markers = &info.marker_exprs;
    let column_marker = sqlite_paths::column_marker();

    quote! {
        /// Hidden const that references the original attribute markers.
        /// This enables IDE hover documentation for `#[column(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #marker_const_name: [#column_marker; #marker_count] = [#(#markers),*];
    }
}

/// Generates the column ZSTs and their `SQLColumn` implementations.
pub(crate) fn generate_column_definitions<'a>(
    ctx: &MacroContext<'a>,
) -> Result<(TokenStream, Vec<Ident>)> {
    let mut all_column_code = TokenStream::new();
    let mut column_zst_idents = Vec::new();
    let MacroContext {
        struct_ident,
        struct_vis,
        field_infos,
        ..
    } = *ctx;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_schema = core_paths::sql_schema();
    let sql_column = core_paths::sql_column();
    let sqlite_column = sqlite_paths::sqlite_column();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_schema_type = sqlite_paths::sqlite_schema_type();

    for info in field_infos {
        let field_pascal_case = info.ident.to_string().to_upper_camel_case();
        let zst_ident = format_ident!("{}{}", ctx.struct_ident, field_pascal_case);
        column_zst_idents.push(zst_ident.clone());

        let (value_type, rust_type) = (&info.base_type, &info.field_type);
        let (is_primary, is_not_null, is_unique, is_autoincrement, has_default) = (
            info.is_primary,
            !info.is_nullable,
            info.is_unique,
            info.is_autoincrement,
            info.has_default || info.default_fn.is_some(),
        );

        // Only use default_fn for Rust DEFAULT constant, not SQL default literals
        let default_const = quote! { ::std::option::Option::None };

        let default_fn_body = info.default_fn.as_ref().map_or_else(
            || quote! { ::std::option::Option::None::<fn() -> Self::Type> },
            |func| quote! { ::std::option::Option::Some(#func) },
        );

        let sql_def = &info.sql_definition;

        let name = &info.column_name;
        let col_type = &info.column_type.to_sql_type();

        // Generate enum implementations using the shared generator
        let enum_impl = super::enum_impls::generate_enum_impls_for_field(info)?;

        // Generate foreign key reference implementation
        let foreign_key_impl = if let Some(ref fk) = info.foreign_key {
            let table_ident = &fk.table_ident;
            let column_ident = &fk.column_ident;
            let column_pascal_case = column_ident.to_string().to_upper_camel_case();
            let fk_zst_ident = format_ident!("{}{}", table_ident, column_pascal_case);
            quote! {
                // Const validation that the FK column exists and implements SQLColumnInfo
                const _: () = { let _ = &#table_ident::#column_ident; };
                #[allow(non_upper_case_globals)]
                static FK_COLUMN: #fk_zst_ident = #fk_zst_ident::new();
                ::std::option::Option::Some(&FK_COLUMN)
            }
        } else {
            quote! { ::std::option::Option::None }
        };

        // Generate individual trait implementations using generators
        let struct_def = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #struct_vis struct #zst_ident;
        };

        let impl_new = generate_impl(
            &zst_ident,
            quote! {
                pub const fn new() -> #zst_ident {
                    #zst_ident
                }
            },
        );

        let sqlite_column_info_impl = generate_sqlite_column_info(
            &zst_ident,
            quote! {<Self as #sqlite_column<'_>>::AUTOINCREMENT},
            quote! {
                static TABLE: #struct_ident = #struct_ident::new();
                &TABLE
            },
            quote! {#foreign_key_impl},
        );

        let to_sql_body = quote! {
            static INSTANCE: #zst_ident = #zst_ident;
            #sql::column(&INSTANCE)
        };

        let into_sqlite_value_impl = quote! {
            impl<'a> ::std::convert::Into<#sqlite_value<'a>> for #zst_ident {
                fn into(self) -> #sqlite_value<'a> {
                    #sqlite_value::Text(::std::borrow::Cow::Borrowed(#name))
                }
            }
        };

        // Use generators for trait implementations
        let sql_schema_field_impl = generate_sql_schema_field(
            &zst_ident,
            quote! {#name},
            quote! {#col_type},
            quote! {#sql_def},
        );
        let sql_column_info_impl = generate_sql_column_info(
            &zst_ident,
            quote! {
                <Self as #sql_schema<'_, &'static str, #sqlite_value<'_>>>::NAME
            },
            quote! {
                <Self as #sql_schema<'_, &'static str, #sqlite_value<'_>>>::TYPE
            },
            quote! {
                <Self as #sql_column<'_, #sqlite_value<'_>>>::PRIMARY_KEY
            },
            quote! {
                <Self as #sql_column<'_, #sqlite_value<'_>>>::NOT_NULL
            },
            quote! {
                <Self as #sql_column<'_, #sqlite_value<'_>>>::UNIQUE
            },
            quote! {
                #has_default
            },
            quote! {
                #foreign_key_impl
            },
            quote! {
                static TABLE: #struct_ident = #struct_ident::new();
                &TABLE
            },
        );

        // Direct const expressions - no runtime builder types needed
        let is_primary = info.is_primary;
        let is_not_null = !info.is_nullable;
        let is_unique = info.is_unique;
        let is_autoincrement = info.is_autoincrement;

        let sql_column_impl = generate_sql_column(
            &zst_ident,
            quote! {#struct_ident},
            quote! {#sqlite_schema_type},
            quote! {#rust_type},
            quote! { #is_primary },
            quote! { #is_not_null || #is_primary },
            quote! { #is_unique },
            quote! {#default_const},
            quote! {#default_fn_body},
        );

        let sqlite_column_impl = generate_sqlite_column(&zst_ident, quote! { #is_autoincrement });
        let to_sql_impl = generate_to_sql(&zst_ident, to_sql_body);

        // Generate marker const using original tokens for IDE documentation
        let marker_const = generate_marker_const(info, &zst_ident);

        let column_code = quote! {
            #struct_def
            #impl_new

            impl #zst_ident {
                #marker_const
            }

            #sql_schema_field_impl
            #sql_column_info_impl
            #sqlite_column_info_impl
            #sql_column_impl
            #sqlite_column_impl
            #to_sql_impl
            #into_sqlite_value_impl

            // Include enum implementation if this is an enum field
            #enum_impl
        };
        all_column_code.extend(column_code);
    }
    Ok((all_column_code, column_zst_idents))
}

/// Generates the `impl` block on the table struct for individual column access.
/// E.g., `impl User { pub const id: UserId = UserId; }`
pub(crate) fn generate_column_accessors(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    let MacroContext {
        struct_ident,
        field_infos,
        ..
    } = &ctx;
    let const_defs = field_infos
        .iter()
        .zip(column_zst_idents.iter())
        .map(|(info, zst_ident)| {
            let const_name = info.ident; // The original field name, e.g., `id`
            quote! {
                pub const #const_name: #zst_ident = #zst_ident;
            }
        });

    let fields = field_infos
        .iter()
        .zip(column_zst_idents.iter())
        .map(|(info, zst)| {
            let name = info.ident;
            quote! {
                #name: #zst
            }
        });

    Ok(quote! {
        #[allow(non_upper_case_globals)]
        impl #struct_ident {
            pub const fn new() -> Self {
                Self {
                    #(#fields,)*
                }
            }
            #(#const_defs)*
        }
    })
}

/// Generates the column fields for the table struct.
pub(crate) fn generate_column_fields(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    let const_defs =
        ctx.field_infos
            .iter()
            .zip(column_zst_idents.iter())
            .map(|(info, zst_ident)| {
                let const_name = info.ident; // The original field name, e.g., `id`
                quote! {
                    pub #const_name: #zst_ident
                }
            });

    Ok(quote! {
        #(#const_defs,)*
    })
}
