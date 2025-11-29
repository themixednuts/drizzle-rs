use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Expr, ExprLit, ExprUnary, Ident, Lit, UnOp, spanned::Spanned};

fn parse_discriminant(expr: &Expr) -> syn::Result<i64> {
    match expr {
        // Simple positive literal like `3`
        Expr::Lit(ExprLit {
            lit: Lit::Int(i), ..
        }) => i
            .base10_parse::<i64>()
            .map_err(|e| syn::Error::new(i.span(), e)),

        // Negative literal like `-1`
        Expr::Unary(ExprUnary {
            op: UnOp::Neg(_),
            expr,
            ..
        }) => {
            if let Expr::Lit(ExprLit {
                lit: Lit::Int(i), ..
            }) = &**expr
            {
                let val = i
                    .base10_parse::<i64>()
                    .map_err(|e| syn::Error::new(i.span(), e))?;
                Ok(-val)
            } else {
                Err(syn::Error::new(
                    expr.span(),
                    "Expected integer literal after unary minus",
                ))
            }
        }

        other => Err(syn::Error::new(
            other.span(),
            "Expected integer literal or unary minus",
        )),
    }
}
// Generate implementation for text-based enum representation
pub fn generate_enum_impl(name: &Ident, data: &DataEnum) -> syn::Result<TokenStream> {
    let display_variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_str = variant_name.to_string();

        quote! {
            #name::#variant_name => write!(f, #variant_str),
        }
    });

    let to_str_variants: Vec<_> = data
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;

            quote! {
                #name::#ident => stringify!(#ident)
            }
        })
        .collect();

    let to_str_ref_variants: Box<_> = data
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;

            quote! {
                &#name::#ident => stringify!(#ident)
            }
        })
        .collect();

    let from_str_variants: Box<_> = data
        .variants
        .iter()
        .map(|variant| {
            let ident = &variant.ident;

            quote! {
                stringify!(#ident) => #name::#ident,
            }
        })
        .collect();

    // let to_str_variants: Vec<_> = data
    //     .variants
    //     .iter()
    //     .map(|variant| {
    //         let ident = &variant.ident;

    //         quote! {
    //             #name::#ident => ::drizzle::sqlite::values::SQLiteValue::from(#ident).into()
    //         }
    //     })
    //     .collect();

    let to_integer_variants = data
        .variants
        .iter()
        .try_fold((0, Vec::new()), |(val, mut acc), variant| {
            let ident = &variant.ident;
            let value = if let Some((_, expr)) = &variant.discriminant {
                parse_discriminant(expr)?
            } else {
                val
            };

            acc.push(quote! {
                #name::#ident => #value
            });

            Ok::<_, syn::Error>((value + 1, acc))
        })
        .map(|(_, t)| t.into_boxed_slice())?;

    let to_integer_ref_variants = data
        .variants
        .iter()
        .try_fold((0, Vec::new()), |(val, mut acc), variant| {
            let ident = &variant.ident;
            let value = if let Some((_, expr)) = &variant.discriminant {
                parse_discriminant(expr)?
            } else {
                val
            };

            acc.push(quote! {
                &#name::#ident => #value
            });

            Ok::<_, syn::Error>((value + 1, acc))
        })
        .map(|(_, t)| t.into_boxed_slice())?;

    let from_integer_variants = data
        .variants
        .iter()
        .try_fold((0, Vec::new()), |(val, mut acc), variant| {
            let ident = &variant.ident;

            // Determine the numeric value
            let value = if let Some((_, expr)) = &variant.discriminant {
                parse_discriminant(expr)?
            } else {
                val
            };
            acc.push(quote! {
                i if i == #value => #name::#ident
            });
            Ok::<_, syn::Error>((value + 1, acc))
        })
        .map(|(_, t)| t.into_boxed_slice())?;

    let base_impls = quote! {

        impl From<#name> for i64 {
            fn from(value: #name) -> Self {
                match value {
                    #(#to_integer_variants,)*
                }
            }
        }

        impl From<&#name> for i64 {
            fn from(value: &#name) -> Self {
                match value {
                    #(#to_integer_ref_variants,)*
                }
            }
        }

        impl TryFrom<i64> for #name {
            type Error = DrizzleError;

            fn try_from(value: i64) -> std::result::Result<Self, Self::Error> {
                Ok(match value {
                    #(#from_integer_variants,)*
                    _ => return Err(DrizzleError::Mapping(format!("{value}").into())),
                })
            }
        }

        impl TryFrom<&i64> for #name {
            type Error = DrizzleError;

            fn try_from(value: &i64) -> std::result::Result<Self, Self::Error> {
                let value = *value;
                Ok(match value {
                    #(#from_integer_variants,)*
                    _ => return Err(DrizzleError::Mapping(format!("{value}").into())),
                })
            }
        }

        // Generic Option implementation - works for any T that can convert to the enum
        impl<T> TryFrom<Option<T>> for #name
        where
            T: TryInto<#name>,
            T::Error: Into<DrizzleError>,
        {
            type Error = DrizzleError;

            fn try_from(value: Option<T>) -> std::result::Result<Self, Self::Error> {
                match value {
                    Some(inner) => inner.try_into().map_err(Into::into),
                    None => Err(DrizzleError::Mapping("Cannot convert None to enum".into())),
                }
            }
        }
        // Integer type conversions - all delegate to i64 conversion
        // Using macro-generated implementations for cleaner code
        ::drizzle_core::impl_try_from_int!(#name => isize, usize, i32, u32, i16, u16, i8, u8);

        // Implement Display for the enum
        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#display_variants)*
                }
            }
        }

        impl From<#name> for &str {
            fn from(value: #name) -> Self {
                match value {
                    #(#to_str_variants,)*
                }
            }
        }

        impl From<&#name> for &str {
            fn from(value: &#name) -> Self {
                match value {
                    #(#to_str_ref_variants,)*
                }
            }
        }

        impl AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                match self {
                    #(#to_str_variants,)*
                }
            }
        }

        impl TryFrom<&str> for #name {
            type Error = DrizzleError;

            fn try_from(value: &str) -> std::result::Result<Self, Self::Error> {
                Ok(match value {
                    #(#from_str_variants)*
                    _ => return Err(DrizzleError::Mapping(format!("{value}").into())),
                })
            }
        }

        impl TryFrom<&String> for #name {
            type Error = DrizzleError;

            fn try_from(value: &String) -> std::result::Result<Self, Self::Error> {
                <#name as std::str::FromStr>::from_str(value)
            }
        }

        impl TryFrom<String> for #name {
            type Error = DrizzleError;

            fn try_from(value: String) -> std::result::Result<Self, Self::Error> {
                <#name as std::str::FromStr>::from_str(&value)
            }
        }


        // Implement FromStr for the enum with String as the error type
        impl std::str::FromStr for #name {
            type Err = DrizzleError;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                Ok(match s {
                    #(#from_str_variants)*
                    _ => return Err(DrizzleError::Mapping(format!("{s}").into())),
                })
            }
        }

    };

    // Add rusqlite FromSql/ToSql implementations when the feature is enabled
    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = quote! {
        // FromSql implementation that handles both TEXT and INTEGER storage
        impl ::rusqlite::types::FromSql for #name {
            fn column_result(value: ::rusqlite::types::ValueRef<'_>) -> ::rusqlite::types::FromSqlResult<Self> {
                match value {
                    ::rusqlite::types::ValueRef::Integer(i) => {
                        Self::try_from(i).map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)
                    },
                    ::rusqlite::types::ValueRef::Text(s) => {
                        let s_str = ::std::str::from_utf8(s)
                            .map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)?;
                        Self::try_from(s_str).map_err(|_| ::rusqlite::types::FromSqlError::InvalidType)
                    },
                    _ => Err(::rusqlite::types::FromSqlError::InvalidType),
                }
            }
        }

        // ToSql defaults to TEXT representation (use table macro for INTEGER storage)
        impl ::rusqlite::types::ToSql for #name {
            fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                let val: &str = self.into();
                Ok(::rusqlite::types::ToSqlOutput::Borrowed(
                    ::rusqlite::types::ValueRef::Text(val.as_bytes())
                ))
            }
        }
    };

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls = quote! {};

    // Generate FromSQLiteValue implementation for libsql/turso
    // This trait provides a unified interface for value conversion
    #[cfg(any(feature = "libsql", feature = "turso"))]
    let from_sqlite_value_impl = quote! {
        impl ::drizzle_sqlite::traits::FromSQLiteValue for #name {
            fn from_sqlite_integer(value: i64) -> ::std::result::Result<Self, ::drizzle_core::error::DrizzleError> {
                Self::try_from(value).map_err(Into::into)
            }

            fn from_sqlite_text(value: &str) -> ::std::result::Result<Self, ::drizzle_core::error::DrizzleError> {
                Self::try_from(value).map_err(Into::into)
            }

            fn from_sqlite_real(_value: f64) -> ::std::result::Result<Self, ::drizzle_core::error::DrizzleError> {
                Err(::drizzle_core::error::DrizzleError::ConversionError(
                    format!("cannot convert REAL to {}", stringify!(#name)).into()
                ))
            }

            fn from_sqlite_blob(_value: &[u8]) -> ::std::result::Result<Self, ::drizzle_core::error::DrizzleError> {
                Err(::drizzle_core::error::DrizzleError::ConversionError(
                    format!("cannot convert BLOB to {}", stringify!(#name)).into()
                ))
            }
        }
    };

    #[cfg(not(any(feature = "libsql", feature = "turso")))]
    let from_sqlite_value_impl = quote! {};

    Ok(quote! {
        #base_impls
        #rusqlite_impls
        #from_sqlite_value_impl
    })
}
