use super::context::MacroContext;
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
    } = ctx;

    for info in *field_infos {
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

        // Generate direct From implementations for enum fields
        let enum_impl = if info.is_enum {
            let (conversion, reference_conversion) = match info.column_type {
                crate::sqlite::field::SQLiteType::Integer => (
                    quote! {
                        let integer: i64 = value.into();
                        ::drizzle::sqlite::values::SQLiteValue::Integer(integer)
                    },
                    quote! {
                        let integer: i64 = value.into();
                        ::drizzle::sqlite::values::SQLiteValue::Integer(integer)
                    },
                ),
                crate::sqlite::field::SQLiteType::Text => (
                    quote! {
                        let text: &str = value.into();
                        ::drizzle::sqlite::values::SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
                    },
                    quote! {
                        let text: &str = value.into();
                        ::drizzle::sqlite::values::SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
                    },
                ),
                _ => {
                    return Err(syn::Error::new_spanned(
                        info.ident,
                        "Enum is only supported in text or integer column types",
                    ));
                } // Default to Text for other types
            };

            {
                #[cfg(feature = "rusqlite")]
                let rusqlite_impl = super::rusqlite::generate_enum_impls(info)?;

                #[cfg(not(feature = "rusqlite"))]
                let rusqlite_impl = quote! {};

                #[cfg(feature = "turso")]
                let turso_impl = super::turso::generate_enum_impls(info)?;

                #[cfg(not(feature = "turso"))]
                let turso_impl = quote! {};

                #[cfg(feature = "libsql")]
                let libsql_impl = super::libsql::generate_enum_impls(info)?;

                #[cfg(not(feature = "libsql"))]
                let libsql_impl = quote! {};

                quote! {
                    // Generate From implementations for enum values
                    impl<'a> ::std::convert::From<#value_type> for ::drizzle::sqlite::values::SQLiteValue<'a> {
                        fn from(value: #value_type) -> Self {
                            #conversion
                        }
                    }

                    impl<'a> ::std::convert::From<&'a #value_type> for ::drizzle::sqlite::values::SQLiteValue<'a> {
                        fn from(value: &'a #value_type) -> Self {
                            #reference_conversion
                        }
                    }

                    impl<'a> ::drizzle::core::ToSQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #value_type {
                        fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> {
                            let value = self;
                            #conversion.into()
                        }
                    }


                    // Include driver-specific implementations
                    #rusqlite_impl
                    #turso_impl
                    #libsql_impl
                }
            }
        } else {
            quote! {}
        };

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

        let column_code = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #struct_vis struct #zst_ident;

            impl #zst_ident {
                pub const fn new() -> #zst_ident {
                    #zst_ident
                }
            }

            impl <'a> ::drizzle::core::SQLSchema<'a, &'a str, ::drizzle::sqlite::values::SQLiteValue<'a> > for #zst_ident {
                const NAME: &'a str = #name;
                const TYPE: &'a str = #col_type;
                const SQL: ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> = ::drizzle::core::SQL::text(#sql);
            }

            impl ::drizzle::core::SQLColumnInfo for #zst_ident {
                fn name(&self) -> &str {
                    <Self as ::drizzle::core::SQLSchema<'_, &'static str, ::drizzle::sqlite::values::SQLiteValue<'_>>>::NAME
                }
                fn r#type(&self) -> &str {
                    <Self as ::drizzle::core::SQLSchema<'_, &'static str, ::drizzle::sqlite::values::SQLiteValue<'_>>>::TYPE
                }
                fn is_primary_key(&self) -> bool {
                    <Self as ::drizzle::core::SQLColumn<'_, ::drizzle::sqlite::values::SQLiteValue<'_>>>::PRIMARY_KEY
                }
                fn is_not_null(&self) -> bool {
                    <Self as ::drizzle::core::SQLColumn<'_, ::drizzle::sqlite::values::SQLiteValue<'_>>>::NOT_NULL
                }
                fn is_unique(&self) -> bool {
                    <Self as ::drizzle::core::SQLColumn<'_, ::drizzle::sqlite::values::SQLiteValue<'_>>>::UNIQUE
                }
                fn has_default(&self) -> bool {
                    #has_default
                }
                fn table(&self) -> &dyn ::drizzle::core::SQLTableInfo {
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }
                fn foreign_key(&self) -> Option<&'static dyn ::drizzle::core::SQLColumnInfo> {
                    #foreign_key_impl
                }
            }


            impl ::drizzle::sqlite::traits::SQLiteColumnInfo for #zst_ident {
                fn is_autoincrement(&self) -> bool {
                    <Self as ::drizzle::sqlite::traits::SQLiteColumn<'_>>::AUTOINCREMENT
                }
                fn table(&self) -> &dyn ::drizzle::sqlite::traits::SQLiteTableInfo {
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }
            }

            impl<'a> ::drizzle::core::SQLColumn<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #zst_ident {
                type Table = #struct_ident;
                type Type = #rust_type;

                const PRIMARY_KEY: bool = #is_primary;
                const NOT_NULL: bool = #is_not_null;
                const UNIQUE: bool = #is_unique;
                const DEFAULT: Option<Self::Type> = #default_const;

                fn default_fn(&self) -> Option<impl Fn() -> Self::Type> {
                    #default_fn_body
                }
            }

            impl ::drizzle::sqlite::traits::SQLiteColumn<'_> for #zst_ident {
                const AUTOINCREMENT: bool = #is_autoincrement;
            }

            impl<'a> ::drizzle::core::ToSQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> for #zst_ident
            {
                fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::sqlite::values::SQLiteValue<'a>> {
                    static INSTANCE: #zst_ident = #zst_ident;
                    ::drizzle::core::SQL::column(&INSTANCE)
                }
            }

            impl<'a> ::std::convert::Into<::drizzle::sqlite::values::SQLiteValue<'a>> for #zst_ident {
                fn into(self) -> ::drizzle::sqlite::values::SQLiteValue<'a> {
                    ::drizzle::sqlite::values::SQLiteValue::Text(::std::borrow::Cow::Borrowed(#name))
                }
            }

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
