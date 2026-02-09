use super::context::MacroContext;
use crate::common::{
    generate_arithmetic_ops, generate_expr_impl, is_numeric_sql_type, rust_type_to_nullability,
    rust_type_to_sql_type,
};
use crate::paths::postgres as postgres_paths;
use crate::postgres::field::{FieldInfo, PostgreSQLType};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

/// Generate a const that references the original marker tokens from the attribute.
///
/// This creates a hidden const that uses the exact tokens from `#[column(PRIMARY, UNIQUE)]`,
/// enabling rust-analyzer to resolve them and provide hover documentation.
fn generate_marker_const(info: &FieldInfo, _zst_ident: &Ident) -> TokenStream {
    if info.marker_exprs.is_empty() {
        return TokenStream::new();
    }

    let field_name = info.ident.to_string().to_uppercase();
    let marker_const_name = format_ident!("_ATTR_MARKERS_{}", field_name);
    let marker_count = info.marker_exprs.len();
    let markers = &info.marker_exprs;

    quote! {
        /// Hidden const that references the original attribute markers.
        /// This enables IDE hover documentation for `#[column(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #marker_const_name: [ColumnMarker; #marker_count] = [#(#markers),*];
    }
}

/// Generate column type definitions and zero-sized types for each column
pub(crate) fn generate_column_definitions(ctx: &MacroContext) -> Result<(TokenStream, Vec<Ident>)> {
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

        let rust_type = &field_info.field_type;
        let (
            _is_primary,
            _is_not_null,
            _is_unique,
            _is_serial,
            _is_bigserial,
            is_generated_identity,
            has_default,
        ) = (
            field_info.is_primary,
            !field_info.is_nullable,
            field_info.is_unique,
            matches!(field_info.column_type, PostgreSQLType::Serial),
            matches!(field_info.column_type, PostgreSQLType::Bigserial),
            field_info.is_generated_identity,
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
                        PostgresValue::Smallint(smallint)
                    },
                    quote! {
                        let smallint: i16 = value.into();
                        PostgresValue::Smallint(smallint)
                    },
                ),
                PostgreSQLType::Integer => (
                    quote! {
                        let integer: i32 = value.into();
                        PostgresValue::Integer(integer)
                    },
                    quote! {
                        let integer: i32 = value.into();
                        PostgresValue::Integer(integer)
                    },
                ),
                PostgreSQLType::Bigint => (
                    quote! {
                        let bigint: i64 = value.into();
                        PostgresValue::Bigint(bigint)
                    },
                    quote! {
                        let bigint: i64 = value.into();
                        PostgresValue::Bigint(bigint)
                    },
                ),
                PostgreSQLType::Serial => (
                    quote! {
                        let integer: i32 = value.into();
                        PostgresValue::Integer(integer)
                    },
                    quote! {
                        let integer: i32 = value.into();
                        PostgresValue::Integer(integer)
                    },
                ),
                PostgreSQLType::Bigserial => (
                    quote! {
                        let bigint: i64 = value.into();
                        PostgresValue::Bigint(bigint)
                    },
                    quote! {
                        let bigint: i64 = value.into();
                        PostgresValue::Bigint(bigint)
                    },
                ),
                PostgreSQLType::Text | PostgreSQLType::Varchar | PostgreSQLType::Char => (
                    quote! {
                        let text: &str = value.into();
                        PostgresValue::Text(::std::borrow::Cow::Borrowed(text))
                    },
                    quote! {
                        let text: &str = value.into();
                        PostgresValue::Text(::std::borrow::Cow::Borrowed(text))
                    },
                ),
                PostgreSQLType::Enum(_) => (
                    quote! {
                        PostgresValue::Enum(Box::new(value))
                    },
                    quote! {
                        PostgresValue::Enum(Box::new((*value).clone()))
                    },
                ),
                _ => {
                    return Err(syn::Error::new_spanned(
                        &field_info.ident,
                        "Enum is only supported in text, integer, or native enum column types",
                    ));
                }
            };

            let value_type = &field_info.field_type;

            quote! {
                // Generate From implementations for enum values
                impl<'a> ::std::convert::From<#value_type> for PostgresValue<'a> {
                    fn from(value: #value_type) -> Self {
                        #conversion
                    }
                }

                impl<'a> ::std::convert::From<&'a #value_type> for PostgresValue<'a> {
                    fn from(value: &'a #value_type) -> Self {
                        #reference_conversion
                    }
                }

                impl<'a> ToSQL<'a, PostgresValue<'a>> for #value_type {
                    fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
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
                // Const validation that the FK column exists and implements the right traits
                const _: () = { let _ = &#table_ident::#column_ident; };
                #[allow(non_upper_case_globals)]
                static FK_COLUMN: #fk_zst_ident = #fk_zst_ident::new();
                Some(&FK_COLUMN)
            }
        } else {
            quote! { None }
        };

        // Direct const expressions - no runtime builder types needed
        let is_primary = field_info.is_primary;
        let is_not_null = !field_info.is_nullable;
        let is_unique = field_info.is_unique;

        // Compute SQL type and nullability markers for type-safe expressions
        let sql_type_marker = rust_type_to_sql_type(rust_type);
        let sql_nullable_marker = rust_type_to_nullability(rust_type);

        // Only Serial/Bigserial columns have is_serial/is_bigserial fields - others are always false
        let (is_serial_expr, is_bigserial_expr) = match &field_info.column_type {
            PostgreSQLType::Serial => (
                quote! { true },  // Serial is always serial
                quote! { false }, // Serial is not bigserial
            ),
            PostgreSQLType::Bigserial => (
                quote! { true }, // Bigserial is also serial (semantically)
                quote! { true }, // Bigserial is bigserial
            ),
            _ => (
                quote! { false }, // Other types are not serial
                quote! { false }, // Other types are not bigserial
            ),
        };

        // Generate marker const using original tokens for IDE documentation
        let marker_const = generate_marker_const(field_info, &zst_ident);

        // Generate Expr trait implementation for type-safe expressions
        let postgres_value = postgres_paths::postgres_value();
        let expr_impl = generate_expr_impl(
            &zst_ident,
            postgres_value.clone(),
            sql_type_marker.clone(),
            sql_nullable_marker.clone(),
        );

        // Generate arithmetic operators for numeric columns
        let arithmetic_ops = if is_numeric_sql_type(rust_type) {
            generate_arithmetic_ops(
                &zst_ident,
                postgres_value,
                sql_type_marker,
                sql_nullable_marker,
            )
        } else {
            quote! {}
        };

        let column_code = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #struct_vis struct #zst_ident;

            impl #zst_ident {
                pub const fn new() -> #zst_ident {
                    #zst_ident
                }

                #marker_const
            }

            impl<'a> SQLSchema<'a, &'a str, PostgresValue<'a>> for #zst_ident {
                const NAME: &'a str = #name;
                const TYPE: &'a str = #col_type;
                const SQL: &'static str = "";

                fn sql(&self) -> SQL<'a, PostgresValue<'a>> {
                    SQL::raw(#sql)
                }
            }

            impl SQLColumnInfo for #zst_ident {
                fn name(&self) -> &str {
                    <Self as SQLSchema<'_, &'static str, PostgresValue<'_>>>::NAME
                }
                fn r#type(&self) -> &str {
                    <Self as SQLSchema<'_, &'static str, PostgresValue<'_>>>::TYPE
                }
                fn is_primary_key(&self) -> bool {
                    <Self as SQLColumn<'_, PostgresValue<'_>>>::PRIMARY_KEY
                }
                fn is_not_null(&self) -> bool {
                    <Self as SQLColumn<'_, PostgresValue<'_>>>::NOT_NULL
                }
                fn is_unique(&self) -> bool {
                    <Self as SQLColumn<'_, PostgresValue<'_>>>::UNIQUE
                }
                fn has_default(&self) -> bool {
                    #has_default
                }
                fn table(&self) -> &dyn SQLTableInfo {
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }
                fn foreign_key(&self) -> Option<&'static dyn SQLColumnInfo> {
                    #foreign_key_impl
                }
            }

            impl PostgresColumnInfo for #zst_ident {
                fn table(&self) -> &dyn PostgresTableInfo {
                    static TABLE: #struct_ident = #struct_ident::new();
                    &TABLE
                }

                fn is_serial(&self) -> bool {
                    <Self as PostgresColumn<'_>>::SERIAL
                }
                fn is_bigserial(&self) -> bool {
                    <Self as PostgresColumn<'_>>::BIGSERIAL
                }
                fn is_generated_identity(&self) -> bool {
                    <Self as PostgresColumn<'_>>::GENERATED_IDENTITY
                }
                fn postgres_type(&self) -> &'static str {
                    #col_type
                }
            }

            impl<'a> SQLColumn<'a, PostgresValue<'a>> for #zst_ident {
                type Table = #struct_ident;
                type TableType = PostgresSchemaType;
                type Type = #rust_type;

                const PRIMARY_KEY: bool = #is_primary;
                const NOT_NULL: bool = #is_not_null || #is_primary;
                const UNIQUE: bool = #is_unique;
                const DEFAULT: Option<Self::Type> = #default_const;

                fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
                    #default_fn_body
                }
            }

            impl PostgresColumn<'_> for #zst_ident {
                const SERIAL: bool = #is_serial_expr;
                const BIGSERIAL: bool = #is_bigserial_expr;
                const GENERATED_IDENTITY: bool = #is_generated_identity;
            }


            impl<'a> ToSQL<'a, PostgresValue<'a>> for #zst_ident {
                fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                    static INSTANCE: #zst_ident = #zst_ident;
                    SQL::column(&INSTANCE)
                }
            }

            // Expr trait implementation for type-safe expressions
            #expr_impl

            // Arithmetic operators for numeric columns
            #arithmetic_ops

            // Include enum implementation if this is an enum field
            #enum_impl
        };
        all_column_code.extend(column_code);
    }
    Ok((all_column_code, column_zst_idents))
}

/// Generate column field definitions for the main struct
pub(crate) fn generate_column_fields(
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
pub(crate) fn generate_column_accessors(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    let struct_ident = ctx.struct_ident;
    let accessor_impls: Vec<TokenStream> = Vec::new();

    // Table accessor methods
    let _field_accessors: Vec<_> = ctx
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
