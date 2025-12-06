use super::context::MacroContext;
use crate::postgres::field::{FieldInfo, PostgreSQLFlag, PostgreSQLType};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Ident, Result};

/// Generate the builder function name and method chain for a column type.
///
/// Returns (builder_type, builder_fn_call, builder_methods) where:
/// - builder_type: The concrete builder type like `postgres::columns::TextBuilder<Type>`
/// - builder_fn_call: The initial function call like `postgres::columns::text::<Type>()`
/// - builder_methods: The chained method calls like `.primary().not_null()`
fn generate_builder_chain(info: &FieldInfo) -> (TokenStream, TokenStream, TokenStream) {
    let rust_type = &info.ty;

    // Generate the builder type and function call based on column type
    // Uses postgres::columns::* to avoid conflicts with sqlite builders
    let (builder_type, builder_fn) = match &info.column_type {
        PostgreSQLType::Text => (
            quote! { postgres::columns::TextBuilder<#rust_type> },
            quote! { postgres::columns::text::<#rust_type>() },
        ),
        PostgreSQLType::Varchar => (
            quote! { postgres::columns::VarcharBuilder<#rust_type> },
            quote! { postgres::columns::varchar::<#rust_type>() },
        ),
        PostgreSQLType::Char => (
            quote! { postgres::columns::CharBuilder<#rust_type> },
            quote! { postgres::columns::char::<#rust_type>() },
        ),
        PostgreSQLType::Integer => (
            quote! { postgres::columns::IntegerBuilder<#rust_type> },
            quote! { postgres::columns::integer::<#rust_type>() },
        ),
        PostgreSQLType::Bigint => (
            quote! { postgres::columns::BigintBuilder<#rust_type> },
            quote! { postgres::columns::bigint::<#rust_type>() },
        ),
        PostgreSQLType::Smallint => (
            quote! { postgres::columns::SmallintBuilder<#rust_type> },
            quote! { postgres::columns::smallint::<#rust_type>() },
        ),
        PostgreSQLType::Serial => (
            quote! { postgres::columns::SerialBuilder<#rust_type> },
            quote! { postgres::columns::serial::<#rust_type>() },
        ),
        PostgreSQLType::Bigserial => (
            quote! { postgres::columns::BigserialBuilder<#rust_type> },
            quote! { postgres::columns::bigserial::<#rust_type>() },
        ),
        PostgreSQLType::Real => (
            quote! { postgres::columns::RealBuilder<#rust_type> },
            quote! { postgres::columns::real::<#rust_type>() },
        ),
        PostgreSQLType::DoublePrecision => (
            quote! { postgres::columns::DoublePrecisionBuilder<#rust_type> },
            quote! { postgres::columns::double_precision::<#rust_type>() },
        ),
        PostgreSQLType::Numeric => (
            quote! { postgres::columns::NumericBuilder<#rust_type> },
            quote! { postgres::columns::numeric::<#rust_type>() },
        ),
        PostgreSQLType::Boolean => (
            quote! { postgres::columns::BooleanBuilder<#rust_type> },
            quote! { postgres::columns::boolean::<#rust_type>() },
        ),
        PostgreSQLType::Bytea => (
            quote! { postgres::columns::ByteaBuilder<#rust_type> },
            quote! { postgres::columns::bytea::<#rust_type>() },
        ),
        #[cfg(feature = "uuid")]
        PostgreSQLType::Uuid => (
            quote! { postgres::columns::UuidBuilder<#rust_type> },
            quote! { postgres::columns::uuid::<#rust_type>() },
        ),
        #[cfg(feature = "serde")]
        PostgreSQLType::Json => (
            quote! { postgres::columns::JsonbBuilder<#rust_type> },
            quote! { postgres::columns::json::<#rust_type>() },
        ),
        #[cfg(feature = "serde")]
        PostgreSQLType::Jsonb => (
            quote! { postgres::columns::JsonbBuilder<#rust_type> },
            quote! { postgres::columns::jsonb::<#rust_type>() },
        ),
        PostgreSQLType::Timestamp => (
            quote! { postgres::columns::TimestampBuilder<#rust_type> },
            quote! { postgres::columns::timestamp::<#rust_type>() },
        ),
        PostgreSQLType::Timestamptz => (
            quote! { postgres::columns::TimestampBuilder<#rust_type> },
            quote! { postgres::columns::timestamptz::<#rust_type>() },
        ),
        PostgreSQLType::Date => (
            quote! { postgres::columns::DateBuilder<#rust_type> },
            quote! { postgres::columns::date::<#rust_type>() },
        ),
        PostgreSQLType::Time => (
            quote! { postgres::columns::TimeBuilder<#rust_type> },
            quote! { postgres::columns::time::<#rust_type>() },
        ),
        PostgreSQLType::Timetz => (
            quote! { postgres::columns::TimeBuilder<#rust_type> },
            quote! { postgres::columns::timetz::<#rust_type>() },
        ),
        // Fallback for feature-gated and other types - use text as default
        _ => (
            quote! { postgres::columns::TextBuilder<#rust_type> },
            quote! { postgres::columns::text::<#rust_type>() },
        ),
    };

    // Generate method chain based on field flags
    let mut methods = TokenStream::new();

    if info.is_primary {
        methods.extend(quote! { .primary() });
    }
    if info.is_unique {
        methods.extend(quote! { .unique() });
    }
    if !info.is_nullable && !info.is_primary {
        // primary() already sets not_null
        methods.extend(quote! { .not_null() });
    }
    if info.is_enum {
        methods.extend(quote! { .r#enum() });
    }
    if info.default_fn.is_some() {
        methods.extend(quote! { .has_default_fn() });
    }

    (builder_type, builder_fn, methods)
}

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
        let (is_primary, is_not_null, is_unique, is_serial, is_bigserial, has_default) = (
            field_info.is_primary,
            !field_info.is_nullable,
            field_info.is_unique,
            matches!(field_info.column_type, PostgreSQLType::Serial),
            matches!(field_info.column_type, PostgreSQLType::Bigserial),
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

            let value_type = &field_info.ty;

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
                #[allow(non_upper_case_globals)]
                static FK_COLUMN: #fk_zst_ident = #fk_zst_ident::new();
                Some(&FK_COLUMN)
            }
        } else {
            quote! { None }
        };

        // Generate the builder chain for this column
        let (builder_type, builder_fn, builder_methods) = generate_builder_chain(field_info);

        // Context-aware field expressions based on which builders have which fields
        // Builders WITH is_primary/is_unique: Text, Varchar, Char, Integer, Bigint, Smallint, Serial, Bigserial, Bytea, Uuid
        // Builders WITHOUT is_primary/is_unique: Boolean, Date, DoublePrecision, Jsonb, Numeric, Real, Time, Timestamp
        let (is_primary_expr, is_unique_expr, is_not_null_expr) = match &field_info.column_type {
            // Types that have is_primary and is_unique fields
            PostgreSQLType::Text
            | PostgreSQLType::Varchar
            | PostgreSQLType::Char
            | PostgreSQLType::Integer
            | PostgreSQLType::Bigint
            | PostgreSQLType::Smallint
            | PostgreSQLType::Serial
            | PostgreSQLType::Bigserial
            | PostgreSQLType::Bytea => (
                quote! { Self::COLUMN.is_primary },
                quote! { Self::COLUMN.is_unique },
                quote! { Self::COLUMN.is_not_null },
            ),
            #[cfg(feature = "uuid")]
            PostgreSQLType::Uuid => (
                quote! { Self::COLUMN.is_primary },
                quote! { Self::COLUMN.is_unique },
                quote! { Self::COLUMN.is_not_null },
            ),
            // Types without is_primary/is_unique - use hardcoded values based on field flags
            PostgreSQLType::Boolean
            | PostgreSQLType::Date
            | PostgreSQLType::DoublePrecision
            | PostgreSQLType::Numeric
            | PostgreSQLType::Real
            | PostgreSQLType::Time
            | PostgreSQLType::Timetz
            | PostgreSQLType::Timestamp
            | PostgreSQLType::Timestamptz => {
                let is_primary = field_info.is_primary;
                let is_unique = field_info.is_unique;
                (
                    quote! { #is_primary },
                    quote! { #is_unique },
                    quote! { Self::COLUMN.is_not_null },
                )
            }
            #[cfg(feature = "serde")]
            PostgreSQLType::Json | PostgreSQLType::Jsonb => {
                let is_primary = field_info.is_primary;
                let is_unique = field_info.is_unique;
                (
                    quote! { #is_primary },
                    quote! { #is_unique },
                    quote! { Self::COLUMN.is_not_null },
                )
            }
            // Fallback for any other types (enums, etc.) - use field flags
            _ => {
                let is_primary = field_info.is_primary;
                let is_unique = field_info.is_unique;
                (
                    quote! { #is_primary },
                    quote! { #is_unique },
                    quote! { Self::COLUMN.is_not_null },
                )
            }
        };

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

        let column_code = quote! {
            #[allow(non_camel_case_types)]
            #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
            #struct_vis struct #zst_ident;

            impl #zst_ident {
                pub const fn new() -> #zst_ident {
                    #zst_ident
                }

                /// Column configuration created by builder pattern.
                /// Hover over builder methods to see documentation.
                #[allow(dead_code)]
                pub const COLUMN: #builder_type = #builder_fn #builder_methods;
            }

            impl<'a> SQLSchema<'a, &'a str, PostgresValue<'a>> for #zst_ident {
                const NAME: &'a str = #name;
                const TYPE: &'a str = #col_type;
                const SQL: SQL<'a, PostgresValue<'a>> = SQL::empty();

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

                const PRIMARY_KEY: bool = #is_primary_expr;
                const NOT_NULL: bool = #is_not_null_expr || #is_primary_expr;
                const UNIQUE: bool = #is_unique_expr;
                const DEFAULT: Option<Self::Type> = #default_const;

                fn default_fn(&'a self) -> Option<impl Fn() -> Self::Type> {
                    #default_fn_body
                }
            }

            impl PostgresColumn<'_> for #zst_ident {
                const SERIAL: bool = #is_serial_expr;
                const BIGSERIAL: bool = #is_bigserial_expr;
                const GENERATED_IDENTITY: bool = false; // TODO: Add generated identity support
            }


            impl<'a> ToSQL<'a, PostgresValue<'a>> for #zst_ident {
                fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                    static INSTANCE: #zst_ident = #zst_ident;
                    SQL::column(&INSTANCE)
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
