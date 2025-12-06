//! Shared driver infrastructure for PostgreSQL row conversion.
//!
//! This module provides a unified approach to generating TryFrom implementations
//! for different PostgreSQL drivers (postgres, tokio-postgres), reducing code duplication
//! and ensuring consistent behavior.
//!
//! This mirrors the pattern used by SQLite's driver infrastructure.

use super::context::MacroContext;
use crate::postgres::field::{FieldInfo, PostgreSQLType, TypeCategory};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Result;

/// Check if a PostgreSQL column type is integer-based
fn is_integer_column(col_type: &PostgreSQLType) -> bool {
    matches!(
        col_type,
        PostgreSQLType::Integer
            | PostgreSQLType::Bigint
            | PostgreSQLType::Smallint
            | PostgreSQLType::Serial
            | PostgreSQLType::Bigserial
    )
}

/// Generate field conversion for SELECT model (non-partial).
///
/// The field type in the Select model matches the original table definition.
fn generate_select_field_conversion(info: &FieldInfo) -> TokenStream {
    let name = &info.ident;
    let name_str = name.to_string();
    let base_type = info.base_type();
    let base_type_str = base_type.to_token_stream().to_string();
    let type_category = TypeCategory::from_type_string(&base_type_str);

    // Determine if we need special handling via FromPostgresValue
    let needs_from_postgres_value = matches!(
        type_category,
        TypeCategory::ArrayString | TypeCategory::ArrayVec
    );

    // Handle enums - check column type for storage format
    // - is_enum with INTEGER column: stored as integer, use try_into()
    // - is_enum with TEXT column: stored as text, use parse()
    // - is_pgenum: native PostgreSQL enum, transmitted as text
    if info.is_enum || info.is_pgenum {
        let is_integer_enum = info.is_enum && is_integer_column(&info.column_type);

        if is_integer_enum {
            // Integer-stored enum: read as i32/i64 and use TryFrom
            if info.is_nullable {
                quote! {
                    #name: {
                        let v: Option<i32> = row.get::<_, Option<i32>>(#name_str);
                        match v {
                            Some(v) => Some(<#base_type as TryFrom<i32>>::try_from(v).map_err(|_| DrizzleError::ConversionError(format!("Failed to convert {} to enum", v).into()))?),
                            None => None,
                        }
                    },
                }
            } else {
                quote! {
                    #name: {
                        let v: i32 = row.get::<_, i32>(#name_str);
                        <#base_type as TryFrom<i32>>::try_from(v).map_err(|_| DrizzleError::ConversionError(format!("Failed to convert {} to enum", v).into()))?
                    },
                }
            }
        } else {
            // Text-stored enum or native pg enum: read as String and use parse()
            if info.is_nullable {
                quote! {
                    #name: {
                        let s: Option<String> = row.get::<_, Option<String>>(#name_str);
                        match s {
                            Some(s) => Some(s.parse::<#base_type>().map_err(|_| DrizzleError::ConversionError(format!("Failed to parse enum from '{}'", s).into()))?),
                            None => None,
                        }
                    },
                }
            } else {
                quote! {
                    #name: {
                        let s: String = row.get::<_, String>(#name_str);
                        s.parse::<#base_type>().map_err(|_| DrizzleError::ConversionError(format!("Failed to parse enum from '{}'", s).into()))?
                    },
                }
            }
        }
    } else if needs_from_postgres_value {
        // Use DrizzleRow::get_column_by_name with FromPostgresValue
        if info.is_nullable {
            quote! {
                #name: {
                    use ::drizzle_postgres::DrizzleRow;
                    DrizzleRow::get_column_by_name::<Option<#base_type>>(row, #name_str)?
                },
            }
        } else {
            quote! {
                #name: {
                    use ::drizzle_postgres::DrizzleRow;
                    DrizzleRow::get_column_by_name::<#base_type>(row, #name_str)?
                },
            }
        }
    } else {
        // Standard types: use native driver's get
        let ty = &info.ty;
        quote! {
            #name: row.get::<_, #ty>(#name_str),
        }
    }
}

/// Generate field conversion for PARTIAL SELECT model.
///
/// In partial models, fields are Option<OriginalType>:
/// - String field -> Option<String>
/// - Option<String> field -> Option<Option<String>>
///
/// We use try_get which returns Result<T, Error> and fall back to None on error.
fn generate_partial_field_conversion(info: &FieldInfo) -> TokenStream {
    let name = &info.ident;
    let name_str = name.to_string();
    let base_type = info.base_type();
    let base_type_str = base_type.to_token_stream().to_string();
    let type_category = TypeCategory::from_type_string(&base_type_str);

    // Determine if we need special handling via FromPostgresValue
    let needs_from_postgres_value = matches!(
        type_category,
        TypeCategory::ArrayString | TypeCategory::ArrayVec
    );

    // Handle enums - check column type for storage format
    if info.is_enum || info.is_pgenum {
        let is_integer_enum = info.is_enum && is_integer_column(&info.column_type);

        if is_integer_enum {
            // Integer-stored enum
            if info.is_nullable {
                // Original is Option<EnumType>, partial is Option<Option<EnumType>>
                quote! {
                    #name: {
                        let v: Option<Option<i32>> = row.try_get::<_, Option<i32>>(#name_str).ok();
                        v.map(|opt| opt.and_then(|v| <#base_type as TryFrom<i32>>::try_from(v).ok()))
                    },
                }
            } else {
                // Original is EnumType, partial is Option<EnumType>
                quote! {
                    #name: {
                        let v: Option<i32> = row.try_get::<_, i32>(#name_str).ok();
                        v.and_then(|v| <#base_type as TryFrom<i32>>::try_from(v).ok())
                    },
                }
            }
        } else {
            // Text-stored or native pg enum
            if info.is_nullable {
                // Original is Option<EnumType>, partial is Option<Option<EnumType>>
                quote! {
                    #name: {
                        let s: Option<Option<String>> = row.try_get::<_, Option<String>>(#name_str).ok();
                        s.map(|opt| opt.and_then(|s| s.parse::<#base_type>().ok()))
                    },
                }
            } else {
                // Original is EnumType, partial is Option<EnumType>
                quote! {
                    #name: {
                        let s: Option<String> = row.try_get::<_, String>(#name_str).ok();
                        s.and_then(|s| s.parse::<#base_type>().ok())
                    },
                }
            }
        }
    } else if needs_from_postgres_value {
        // Use DrizzleRow for FromPostgresValue types
        if info.is_nullable {
            // Original is Option<T>, partial is Option<Option<T>>
            quote! {
                #name: {
                    use ::drizzle_postgres::DrizzleRow;
                    Some(DrizzleRow::get_column_by_name::<Option<#base_type>>(row, #name_str).ok().flatten())
                },
            }
        } else {
            // Original is T, partial is Option<T>
            quote! {
                #name: {
                    use ::drizzle_postgres::DrizzleRow;
                    DrizzleRow::get_column_by_name::<#base_type>(row, #name_str).ok()
                },
            }
        }
    } else {
        // For standard types, try to get the original type (including Option wrapper if nullable)
        let ty = &info.ty;
        quote! {
            #name: row.try_get::<_, #ty>(#name_str).ok(),
        }
    }
}

/// Generate field conversion for UPDATE model.
///
/// In update models, ALL fields are Option<BaseType>. We get the base type value
/// from the row and wrap it in Some().
fn generate_update_field_conversion(info: &FieldInfo) -> TokenStream {
    let name = &info.ident;
    let name_str = name.to_string();
    let base_type = info.base_type();
    let base_type_str = base_type.to_token_stream().to_string();
    let type_category = TypeCategory::from_type_string(&base_type_str);

    // Determine if we need special handling via FromPostgresValue
    let needs_from_postgres_value = matches!(
        type_category,
        TypeCategory::ArrayString | TypeCategory::ArrayVec
    );

    // Handle enums - check column type for storage format, then wrap in Some()
    if info.is_enum || info.is_pgenum {
        let is_integer_enum = info.is_enum && is_integer_column(&info.column_type);

        if is_integer_enum {
            // Integer-stored enum
            quote! {
                #name: {
                    let v: i32 = row.get::<_, i32>(#name_str);
                    Some(<#base_type as TryFrom<i32>>::try_from(v).map_err(|_| DrizzleError::ConversionError(format!("Failed to convert {} to enum", v).into()))?)
                },
            }
        } else {
            // Text-stored or native pg enum
            quote! {
                #name: {
                    let s: String = row.get::<_, String>(#name_str);
                    Some(s.parse::<#base_type>().map_err(|_| DrizzleError::ConversionError(format!("Failed to parse enum from '{}'", s).into()))?)
                },
            }
        }
    } else if needs_from_postgres_value {
        // Use DrizzleRow for FromPostgresValue types
        quote! {
            #name: {
                use ::drizzle_postgres::DrizzleRow;
                Some(DrizzleRow::get_column_by_name::<#base_type>(row, #name_str)?)
            },
        }
    } else {
        // Standard types - get base_type (not the original ty which might be Option<T>)
        quote! {
            #name: Some(row.get::<_, #base_type>(#name_str)),
        }
    }
}

// =============================================================================
// Public API
// =============================================================================

/// Generate TryFrom implementations for all enabled PostgreSQL drivers.
///
/// Note: postgres::Row is a re-export of tokio_postgres::Row, so we must be careful
/// not to generate duplicate implementations. When tokio-postgres is enabled, we use
/// tokio_postgres::Row. When only postgres-sync is enabled, we use postgres::Row.
pub(crate) fn generate_all_driver_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let MacroContext {
        field_infos,
        select_model_ident,
        select_model_partial_ident,
        update_model_ident,
        ..
    } = ctx;

    let select_field_inits: Vec<_> = field_infos
        .iter()
        .map(|info| generate_select_field_conversion(info))
        .collect();

    let partial_field_inits: Vec<_> = field_infos
        .iter()
        .map(|info| generate_partial_field_conversion(info))
        .collect();

    let update_field_inits: Vec<_> = field_infos
        .iter()
        .map(|info| generate_update_field_conversion(info))
        .collect();

    // Generate the implementations
    // Note: We use cfg attributes to ensure only one implementation is generated
    // even when both postgres-sync and tokio-postgres are enabled (they share the same Row type)

    Ok(quote! {
        // When tokio-postgres is enabled, use tokio_postgres::Row
        // This covers both "tokio-postgres only" and "both features enabled" cases
        #[cfg(feature = "tokio-postgres")]
        impl ::std::convert::TryFrom<&::tokio_postgres::Row> for #select_model_ident {
            type Error = DrizzleError;

            fn try_from(row: &::tokio_postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select_field_inits)*
                })
            }
        }

        #[cfg(feature = "tokio-postgres")]
        impl ::std::convert::TryFrom<&::tokio_postgres::Row> for #select_model_partial_ident {
            type Error = DrizzleError;

            fn try_from(row: &::tokio_postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial_field_inits)*
                })
            }
        }

        #[cfg(feature = "tokio-postgres")]
        impl ::std::convert::TryFrom<&::tokio_postgres::Row> for #update_model_ident {
            type Error = DrizzleError;

            fn try_from(row: &::tokio_postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#update_field_inits)*
                })
            }
        }

        // When only postgres-sync is enabled (without tokio-postgres), use postgres::Row
        #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
        impl ::std::convert::TryFrom<&::postgres::Row> for #select_model_ident {
            type Error = DrizzleError;

            fn try_from(row: &::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#select_field_inits)*
                })
            }
        }

        #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
        impl ::std::convert::TryFrom<&::postgres::Row> for #select_model_partial_ident {
            type Error = DrizzleError;

            fn try_from(row: &::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#partial_field_inits)*
                })
            }
        }

        #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
        impl ::std::convert::TryFrom<&::postgres::Row> for #update_model_ident {
            type Error = DrizzleError;

            fn try_from(row: &::postgres::Row) -> ::std::result::Result<Self, Self::Error> {
                Ok(Self {
                    #(#update_field_inits)*
                })
            }
        }
    })
}
