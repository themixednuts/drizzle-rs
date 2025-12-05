use super::context::MacroContext;
use crate::postgres::field::{FieldInfo, PostgreSQLFlag, PostgreSQLType};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

/// Generate column type definitions and zero-sized types for each column
pub(super) fn generate_column_definitions(ctx: &MacroContext) -> Result<(TokenStream, Vec<Ident>)> {
    let mut all_column_code = TokenStream::new();
    let mut column_zst_idents = Vec::new();
    let MacroContext {
        struct_ident,
        struct_vis,
        field_infos,
        ..
    } = ctx;

    for field_info in *field_infos {
        let field_pascal_case = field_info.ident.to_string().to_upper_camel_case();
        let zst_ident = format_ident!("{}{}", struct_ident, field_pascal_case);
        column_zst_idents.push(zst_ident.clone());

        let rust_type = &field_info.ty;
        let (is_primary, is_not_null, is_unique, is_serial, has_default) = (
            field_info.is_primary,
            !field_info.is_nullable,
            field_info.is_unique,
            field_info.is_serial,
            field_info.has_default || field_info.default_fn.is_some(),
        );

        // Only use default_fn for Rust DEFAULT constant, not SQL default literals
        let default_const = quote! { None };

        let default_fn_body = field_info.default_fn.as_ref().map_or_else(
            || quote! { None::<fn() -> Self::Type> },
            |func| quote! { Some(#func) },
        );

        let name = field_info.ident.to_string();
        let col_type = field_info.column_type.to_sql_type();
        let sql = format!("{} {}", name, col_type); // Basic SQL definition

        // Generate direct From implementations for all enum fields
        let enum_impl = if field_info.is_enum || field_info.is_pgenum {
            let (conversion, reference_conversion) = match field_info.column_type {
                PostgreSQLType::Smallint => (
                    quote! {
                        let smallint: i16 = value.into();
                        drizzle_postgres::values::PostgresValue::Smallint(smallint)
                    },
                    quote! {
                        let smallint: i16 = value.into();
                        drizzle_postgres::values::PostgresValue::Smallint(smallint)
                    },
                ),
                PostgreSQLType::Integer => (
                    quote! {
                        let integer: i32 = value.into();
                        drizzle_postgres::values::PostgresValue::Integer(integer)
                    },
                    quote! {
                        let integer: i32 = value.into();
                        drizzle_postgres::values::PostgresValue::Integer(integer)
                    },
                ),
                PostgreSQLType::Bigint => (
                    quote! {
                        let bigint: i64 = value.into();
                        drizzle_postgres::values::PostgresValue::Bigint(bigint)
                    },
                    quote! {
                        let bigint: i64 = value.into();
                        drizzle_postgres::values::PostgresValue::Bigint(bigint)
                    },
                ),
                PostgreSQLType::Serial => (
                    quote! {
                        let integer: i32 = value.into();
                        drizzle_postgres::values::PostgresValue::Integer(integer)
                    },
                    quote! {
                        let integer: i32 = value.into();
                        drizzle_postgres::values::PostgresValue::Integer(integer)
                    },
                ),
                PostgreSQLType::Bigserial => (
                    quote! {
                        let bigint: i64 = value.into();
                        drizzle_postgres::values::PostgresValue::Bigint(bigint)
                    },
                    quote! {
                        let bigint: i64 = value.into();
                        drizzle_postgres::values::PostgresValue::Bigint(bigint)
                    },
                ),
                PostgreSQLType::Text | PostgreSQLType::Varchar | PostgreSQLType::Char => (
                    quote! {
                        let text: &str = value.into();
                        drizzle_postgres::values::PostgresValue::Text(::std::borrow::Cow::Borrowed(text))
                    },
                    quote! {
                        let text: &str = value.into();
                        drizzle_postgres::values::PostgresValue::Text(::std::borrow::Cow::Borrowed(text))
                    },
                ),
                PostgreSQLType::Enum(_) => (
                    quote! {
                        drizzle_postgres::values::PostgresValue::Enum(Box::new(value))
                    },
                    quote! {
                        drizzle_postgres::values::PostgresValue::Enum(Box::new((*value).clone()))
                    },
                ),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &field_info.ident,
                        "Enum is only supported in text, integer, or native enum column types",
                    ));
                }
            };

            let value_type = &field_info.ty;

            quote! {
                // Generate From implementations for enum values
                impl<'a> ::std::convert::From<#value_type> for drizzle_postgres::values::PostgresValue<'a> {
                    fn from(value: #value_type) -> Self {
                        #conversion
                    }
                }

                impl<'a> ::std::convert::From<&'a #value_type> for drizzle_postgres::values::PostgresValue<'a> {
                    fn from(value: &'a #value_type) -> Self {
                        #reference_conversion
                    }
                }

                impl<'a> drizzle_core::ToSQL<'a, drizzle_postgres::values::PostgresValue<'a>> for #value_type {
                    fn to_sql(&self) -> drizzle_core::SQL<'a, drizzle_postgres::values::PostgresValue<'a>> {
                        let value = self.clone();
                        #conversion.into()
                    }
                }
            }
        } else {
            quote! {}
        };

        // Generate foreign key reference implementation
        let foreign_key_impl = if let Some(ref fk) = field_info.foreign_key {
            let table_ident = &fk.table;
            let column_ident = &fk.column;
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

            impl<'a> drizzle_core::SQLSchema<'a, &'a str, drizzle_postgres::values::PostgresValue<'a>> for #zst_ident {
                const NAME: &'a str = #name;
                const TYPE: &'a str = #col_type;
                const SQL: drizzle_core::SQL<'a, drizzle_postgres::values::PostgresValue<'a>> = drizzle_core::SQL::empty();

                fn sql(&self) -> drizzle_core::SQL<'a, drizzle_postgres::values::PostgresValue<'a>> {
                    drizzle_core::SQL::raw(#sql)
                }
            }

            impl drizzle_core::SQLColumnInfo for #zst_ident {
                fn name(&self) -> &str {
                    <Self as drizzle_core::SQLSchema<'_, &'static str, drizzle_postgres::values::PostgresValue<'_>>>::NAME
                }
                fn r#type(&self) -> &str {
                    <Self as drizzle_core::SQLSchema<'_, &'static str, drizzle_postgres::values::PostgresValue<'_>>>::TYPE
                }
                fn is_primary_key(&self) -> bool {
                    <Self as drizzle_core::SQLColumn<'_, drizzle_postgres::values::PostgresValue<'_>>>::PRIMARY_KEY
                }
                fn is_not_null(&self) -> bool {
                    <Self as drizzle_core::SQLColumn<'_, drizzle_postgres::values::PostgresValue<'_>>>::NOT_NULL
                }
                fn is_unique(&self) -> bool {
                    <Self as drizzle_core::SQLColumn<'_, drizzle_postgres::values::PostgresValue<'_>>>::UNIQUE
                }
                fn has_default(&self) -> bool {
                    #has_default
                }
                fn table(&self) -> &dyn drizzle_core::SQLTableInfo {
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }
                fn foreign_key(&self) -> Option<&'static dyn drizzle_core::SQLColumnInfo> {
                    #foreign_key_impl
                }
            }

            impl drizzle_postgres::traits::PostgresColumnInfo for #zst_ident {
                fn table(&self) -> &dyn drizzle_postgres::traits::PostgresTableInfo {
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }

                fn is_serial(&self) -> bool {
                    <Self as drizzle_postgres::traits::PostgresColumn<'_>>::SERIAL
                }
                fn is_bigserial(&self) -> bool {
                    <Self as drizzle_postgres::traits::PostgresColumn<'_>>::BIGSERIAL
                }
                fn is_generated_identity(&self) -> bool {
                    <Self as drizzle_postgres::traits::PostgresColumn<'_>>::GENERATED_IDENTITY
                }
                fn postgres_type(&self) -> &'static str {
                    #col_type
                }
            }

            impl<'a> drizzle_core::SQLColumn<'a, drizzle_postgres::values::PostgresValue<'a>> for #zst_ident {
                type Table = #struct_ident;
                type TableType = drizzle_postgres::common::PostgresSchemaType;
                type Type = #rust_type;

                const PRIMARY_KEY: bool = #is_primary;
                const NOT_NULL: bool = #is_not_null;
                const UNIQUE: bool = #is_unique;
                const DEFAULT: Option<Self::Type> = #default_const;

                fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
                    #default_fn_body
                }
            }

            impl drizzle_postgres::traits::PostgresColumn<'_> for #zst_ident {
                const SERIAL: bool = #is_serial;
                const BIGSERIAL: bool = false; // TODO: Add bigserial support
                const GENERATED_IDENTITY: bool = false; // TODO: Add generated identity support
            }


            impl<'a> drizzle_core::ToSQL<'a, drizzle_postgres::values::PostgresValue<'a>> for #zst_ident {
                fn to_sql(&self) -> drizzle_core::SQL<'a, drizzle_postgres::values::PostgresValue<'a>> {
                    static INSTANCE: #zst_ident = #zst_ident;
                    drizzle_core::SQL::column(&INSTANCE)
                }
            }

            // Include enum implementation if this is an enum field
            #enum_impl
        };
        all_column_code.extend(column_code);
    }
    Ok((all_column_code, column_zst_idents))
}

/// Generate column field definitions for the main struct
pub(super) fn generate_column_fields(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    let mut field_definitions = Vec::new();

    for (field_info, column_ident) in ctx.field_infos.iter().zip(column_zst_idents) {
        let field_name = &field_info.ident;
        let field_vis = &field_info.vis;

        let field_def = quote! {
            #field_vis #field_name: #column_ident,
        };

        field_definitions.push(field_def);
    }

    Ok(quote! {
        #(#field_definitions)*
    })
}

/// Generate column accessor methods and implementations
pub(super) fn generate_column_accessors(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    let struct_ident = ctx.struct_ident;
    let mut accessor_impls: Vec<TokenStream> = Vec::new();

    // Table accessor methods
    let field_accessors: Vec<_> = ctx
        .field_infos
        .iter()
        .zip(column_zst_idents)
        .map(|(field_info, column_ident)| {
            let field_name = &field_info.ident;
            quote! {
                pub #field_name: #column_ident,
            }
        })
        .collect();

    // Fix the field_name references
    let field_names: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|field_info| &field_info.ident)
        .collect();

    // Generate column constant accessors like SQLite version
    let const_defs =
        ctx.field_infos
            .iter()
            .zip(column_zst_idents.iter())
            .map(|(info, zst_ident)| {
                let const_name = &info.ident;
                quote! {
                    pub const #const_name: #zst_ident = #zst_ident;
                }
            });

    let table_impl = quote! {
        #[allow(non_upper_case_globals)]
        impl #struct_ident {
            pub const fn new() -> Self {
                Self {
                    #(#field_names: #column_zst_idents,)*
                }
            }
            #(#const_defs)*
        }
    };

    Ok(quote! {
        #(#accessor_impls)*
        #table_impl
    })
}
