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

    Ok(quote! {

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
        // Generic implementation for integer types that aren't explicitly covered
        impl TryFrom<isize> for #name {
            type Error = DrizzleError;

            fn try_from(value: isize) -> std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl TryFrom<usize> for #name {
            type Error = DrizzleError;

            fn try_from(value: usize) -> std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        // Generic implementation for integer types that aren't explicitly covered
        impl TryFrom<i32> for #name {
            type Error = DrizzleError;

            fn try_from(value: i32) -> std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl TryFrom<u32> for #name {
            type Error = DrizzleError;

            fn try_from(value: u32) -> std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl TryFrom<i16> for #name {
            type Error = DrizzleError;

            fn try_from(value: i16) -> std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl TryFrom<u16> for #name {
            type Error = DrizzleError;

            fn try_from(value: u16) -> std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl TryFrom<i8> for #name {
            type Error = DrizzleError;

            fn try_from(value: i8) -> std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

        impl TryFrom<u8> for #name {
            type Error = DrizzleError;

            fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
                Self::try_from(value as i64)
            }
        }

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

    })
}
