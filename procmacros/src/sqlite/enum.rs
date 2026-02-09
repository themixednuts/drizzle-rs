use crate::common::enum_utils::resolve_discriminants;
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Ident};
// Generate implementation for text-based enum representation
pub fn generate_enum_impl(name: &Ident, data: &DataEnum) -> syn::Result<TokenStream> {
    // Get paths for fully-qualified types
    let drizzle_error = core_paths::drizzle_error();
    #[allow(unused_variables)]
    let from_sqlite_value = sqlite_paths::from_sqlite_value();
    let impl_try_from_int = core_paths::impl_try_from_int();

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

    // Resolve discriminants with uniqueness validation
    let resolved = resolve_discriminants(data)?;

    let to_integer_variants: Box<[_]> = resolved
        .iter()
        .map(|(ident, value)| {
            quote! { #name::#ident => #value }
        })
        .collect();

    let to_integer_ref_variants: Box<[_]> = resolved
        .iter()
        .map(|(ident, value)| {
            quote! { &#name::#ident => #value }
        })
        .collect();

    let from_integer_variants: Box<[_]> = resolved
        .iter()
        .map(|(ident, value)| {
            quote! { i if i == #value => #name::#ident }
        })
        .collect();

    let sqlite_value = sqlite_paths::sqlite_value();

    let base_impls = quote! {

        // Implement Expr trait for type-safe comparisons
        // Uses Any type since enums can be stored as TEXT or INTEGER
        // Note: &T impl is handled by blanket impl in drizzle_core
        impl<'a> drizzle::core::expr::Expr<'a, #sqlite_value<'a>> for #name {
            type SQLType = drizzle::core::types::Any;
            type Nullable = drizzle::core::expr::NonNull;
            type Aggregate = drizzle::core::expr::Scalar;
        }

        impl ::std::convert::From<#name> for i64 {
            fn from(value: #name) -> Self {
                match value {
                    #(#to_integer_variants,)*
                }
            }
        }

        impl ::std::convert::From<&#name> for i64 {
            fn from(value: &#name) -> Self {
                match value {
                    #(#to_integer_ref_variants,)*
                }
            }
        }

        impl ::std::convert::TryFrom<i64> for #name {
            type Error = #drizzle_error;

            fn try_from(value: i64) -> ::std::result::Result<Self, Self::Error> {
                ::std::result::Result::Ok(match value {
                    #(#from_integer_variants,)*
                    _ => return ::std::result::Result::Err(#drizzle_error::Mapping(::std::format!("{value}").into())),
                })
            }
        }

        impl ::std::convert::TryFrom<&i64> for #name {
            type Error = #drizzle_error;

            fn try_from(value: &i64) -> ::std::result::Result<Self, Self::Error> {
                let value = *value;
                ::std::result::Result::Ok(match value {
                    #(#from_integer_variants,)*
                    _ => return ::std::result::Result::Err(#drizzle_error::Mapping(::std::format!("{value}").into())),
                })
            }
        }

        // Generic Option implementation - works for any T that can convert to the enum
        impl<T> ::std::convert::TryFrom<::std::option::Option<T>> for #name
        where
            T: ::std::convert::TryInto<#name>,
            T::Error: ::std::convert::Into<#drizzle_error>,
        {
            type Error = #drizzle_error;

            fn try_from(value: ::std::option::Option<T>) -> ::std::result::Result<Self, Self::Error> {
                match value {
                    ::std::option::Option::Some(inner) => inner.try_into().map_err(::std::convert::Into::into),
                    ::std::option::Option::None => ::std::result::Result::Err(#drizzle_error::Mapping("Cannot convert None to enum".into())),
                }
            }
        }
        // Integer type conversions - all delegate to i64 conversion
        // Using macro-generated implementations for cleaner code
        #impl_try_from_int!(#name => isize, usize, i32, u32, i16, u16, i8, u8);

        // Implement Display for the enum
        impl ::std::fmt::Display for #name {
            fn fmt(&self, f: &mut ::std::fmt::Formatter<'_>) -> ::std::fmt::Result {
                match self {
                    #(#display_variants)*
                }
            }
        }

        impl ::std::convert::From<#name> for &str {
            fn from(value: #name) -> Self {
                match value {
                    #(#to_str_variants,)*
                }
            }
        }

        impl ::std::convert::From<&#name> for &str {
            fn from(value: &#name) -> Self {
                match value {
                    #(#to_str_ref_variants,)*
                }
            }
        }

        impl ::std::convert::AsRef<str> for #name {
            fn as_ref(&self) -> &str {
                match self {
                    #(#to_str_variants,)*
                }
            }
        }

        impl ::std::convert::TryFrom<&str> for #name {
            type Error = #drizzle_error;

            fn try_from(value: &str) -> ::std::result::Result<Self, Self::Error> {
                ::std::result::Result::Ok(match value {
                    #(#from_str_variants)*
                    _ => return ::std::result::Result::Err(#drizzle_error::Mapping(::std::format!("{value}").into())),
                })
            }
        }

        impl ::std::convert::TryFrom<&::std::string::String> for #name {
            type Error = #drizzle_error;

            fn try_from(value: &::std::string::String) -> ::std::result::Result<Self, Self::Error> {
                <#name as ::std::str::FromStr>::from_str(value)
            }
        }

        impl ::std::convert::TryFrom<::std::string::String> for #name {
            type Error = #drizzle_error;

            fn try_from(value: ::std::string::String) -> ::std::result::Result<Self, Self::Error> {
                <#name as ::std::str::FromStr>::from_str(&value)
            }
        }


        // Implement FromStr for the enum with String as the error type
        impl ::std::str::FromStr for #name {
            type Err = #drizzle_error;

            fn from_str(s: &str) -> ::std::result::Result<Self, Self::Err> {
                ::std::result::Result::Ok(match s {
                    #(#from_str_variants)*
                    _ => return ::std::result::Result::Err(#drizzle_error::Mapping(::std::format!("{s}").into())),
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
                    _ => ::std::result::Result::Err(::rusqlite::types::FromSqlError::InvalidType),
                }
            }
        }

        // ToSql defaults to TEXT representation (use table macro for INTEGER storage)
        impl ::rusqlite::types::ToSql for #name {
            fn to_sql(&self) -> ::rusqlite::Result<::rusqlite::types::ToSqlOutput<'_>> {
                let val: &str = self.into();
                ::std::result::Result::Ok(::rusqlite::types::ToSqlOutput::Borrowed(
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
        impl #from_sqlite_value for #name {
            fn from_sqlite_integer(value: i64) -> ::std::result::Result<Self, #drizzle_error> {
                Self::try_from(value).map_err(::std::convert::Into::into)
            }

            fn from_sqlite_text(value: &str) -> ::std::result::Result<Self, #drizzle_error> {
                Self::try_from(value).map_err(::std::convert::Into::into)
            }

            fn from_sqlite_real(_value: f64) -> ::std::result::Result<Self, #drizzle_error> {
                ::std::result::Result::Err(#drizzle_error::ConversionError(
                    ::std::format!("cannot convert REAL to {}", stringify!(#name)).into()
                ))
            }

            fn from_sqlite_blob(_value: &[u8]) -> ::std::result::Result<Self, #drizzle_error> {
                ::std::result::Result::Err(#drizzle_error::ConversionError(
                    ::std::format!("cannot convert BLOB to {}", stringify!(#name)).into()
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
