use super::context::MacroContext;
use crate::generators::{generate_impl, generate_sql_column_info};
use crate::sqlite::generators::*;
use heck::ToUpperCamelCase;
use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::Result;

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
        let default_const = quote! { None };

        let default_fn_body = info.default_fn.as_ref().map_or_else(
            || quote! { None::<fn() -> Self::Type> },
            |func| quote! { Some(#func) },
        );

        let sql = &info.sql_definition;

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
                #[allow(non_upper_case_globals)]
                static FK_COLUMN: #fk_zst_ident = #fk_zst_ident::new();
                Some(&FK_COLUMN)
            }
        } else {
            quote! { None }
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
            quote! {<Self as drizzle_sqlite::traits::SQLiteColumn<'_>>::AUTOINCREMENT},
            quote! {
                static TABLE: #struct_ident = #struct_ident::new();
                &TABLE
            },
            quote! {#foreign_key_impl},
        );

        let to_sql_body = quote! {
            static INSTANCE: #zst_ident = #zst_ident;
            drizzle_core::SQL::column(&INSTANCE)
        };

        let into_sqlite_value_impl = quote! {
            impl<'a> ::std::convert::Into<drizzle_sqlite::values::SQLiteValue<'a>> for #zst_ident {
                fn into(self) -> drizzle_sqlite::values::SQLiteValue<'a> {
                    drizzle_sqlite::values::SQLiteValue::Text(::std::borrow::Cow::Borrowed(#name))
                }
            }
        };

        // Use generators for trait implementations
        let sql_schema_field_impl = generate_sql_schema_field(
            &zst_ident,
            quote! {#name},
            quote! {#col_type},
            quote! {drizzle_core::SQL::raw(#sql)},
        );
        let sql_column_info_impl = generate_sql_column_info(
            &zst_ident,
            quote! {
                <Self as drizzle_core::SQLSchema<'_, &'static str, drizzle_sqlite::values::SQLiteValue<'_>>>::NAME
            },
            quote! {
                <Self as drizzle_core::SQLSchema<'_, &'static str, drizzle_sqlite::values::SQLiteValue<'_>>>::TYPE
            },
            quote! {
                <Self as drizzle_core::SQLColumn<'_, drizzle_sqlite::values::SQLiteValue<'_>>>::PRIMARY_KEY
            },
            quote! {
                <Self as drizzle_core::SQLColumn<'_, drizzle_sqlite::values::SQLiteValue<'_>>>::NOT_NULL
            },
            quote! {
                <Self as drizzle_core::SQLColumn<'_, drizzle_sqlite::values::SQLiteValue<'_>>>::UNIQUE
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
        let sql_column_impl = generate_sql_column(
            &zst_ident,
            quote! {#struct_ident},
            quote! {drizzle_sqlite::common::SQLiteSchemaType},
            quote! {#rust_type},
            quote! {#is_primary},
            quote! {#is_not_null},
            quote! {#is_unique},
            quote! {#default_const},
            quote! {#default_fn_body},
        );
        let sqlite_column_impl = generate_sqlite_column(&zst_ident, quote! { #is_autoincrement });
        let to_sql_impl = generate_to_sql(&zst_ident, to_sql_body);

        let column_code = quote! {
            #struct_def
            #impl_new
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
