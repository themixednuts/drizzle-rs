use proc_macro2::TokenStream;
use quote::quote;
use syn::{DataEnum, Ident};

// Generate implementation for integer-based enum representation
pub fn generate_integer_enum_impl(name: &Ident, data: &DataEnum) -> syn::Result<TokenStream> {
    let to_integer_variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let discriminant = if let Some((_, ref expr)) = variant.discriminant {
            quote! { #expr }
        } else {
            quote! { Self::#variant_name as i64 }
        };

        quote! {
            Self::#variant_name => #discriminant,
        }
    });

    let from_integer_variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let discriminant = if let Some((_, ref expr)) = variant.discriminant {
            quote! { #expr }
        } else {
            quote! { Self::#variant_name as i64 }
        };

        quote! {
            i if i == #discriminant => Some(Self::#variant_name),
        }
    });

    let display_variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_str = variant_name.to_string();

        quote! {
            Self::#variant_name => write!(f, #variant_str),
        }
    });

    let from_str_variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_str = variant_name.to_string();

        quote! {
            #variant_str => Ok(Self::#variant_name),
        }
    });

    Ok(quote! {
        // Use absolute paths when generating
        impl ::drizzle_rs::sqlite::SQLiteEnum for #name {
            const ENUM_REPR: ::drizzle_rs::sqlite::SQLiteEnumRepr = ::drizzle_rs::sqlite::SQLiteEnumRepr::Integer;

            fn to_integer(&self) -> i64 {
                match self {
                    #(#to_integer_variants)*
                    _ => Self::default().to_integer(),
                }
            }

            fn from_integer(i: i64) -> Option<Self> {
                match i {
                    #(#from_integer_variants)*
                    _ => None,
                }
            }
        }

        // Implement Display for the enum
        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#display_variants)*
                    _ => write!(f, "{}", Self::default()),
                }
            }
        }

        // Implement FromStr for the enum with String as the error type
        impl std::str::FromStr for #name {
            type Err = String;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                match s {
                    #(#from_str_variants)*
                    _ => std::result::Result::Err(format!("Unknown variant: {}", s)),
                }
            }
        }
    })
}

// Generate implementation for text-based enum representation
pub fn generate_text_enum_impl(name: &Ident, data: &DataEnum) -> syn::Result<TokenStream> {
    let display_variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_str = variant_name.to_string();

        quote! {
            Self::#variant_name => write!(f, #variant_str),
        }
    });

    let from_str_variants = data.variants.iter().map(|variant| {
        let variant_name = &variant.ident;
        let variant_str = variant_name.to_string();

        quote! {
            #variant_str => Ok(Self::#variant_name),
        }
    });

    let to_integer_variants = data.variants.iter().enumerate().map(|(i, variant)| {
        let variant_name = &variant.ident;
        let index = i as i64;

        quote! {
            Self::#variant_name => #index,
        }
    });

    let from_integer_variants = data.variants.iter().enumerate().map(|(i, variant)| {
        let variant_name = &variant.ident;
        let index = i as i64;

        quote! {
            i if i == #index => Some(Self::#variant_name),
        }
    });

    Ok(quote! {
        // Use absolute paths when generating
        impl ::drizzle_rs::sqlite::SQLiteEnum for #name {
            const ENUM_REPR: ::drizzle_rs::sqlite::SQLiteEnumRepr = ::drizzle_rs::sqlite::SQLiteEnumRepr::Text;

            fn to_integer(&self) -> i64 {
                match self {
                    #(#to_integer_variants)*
                    _ => Self::default().to_integer(),
                }
            }

            fn from_integer(i: i64) -> Option<Self> {
                match i {
                    #(#from_integer_variants)*
                    _ => None,
                }
            }
        }

        // Implement Display for the enum
        impl std::fmt::Display for #name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self {
                    #(#display_variants)*
                    _ => write!(f, "{}", Self::default()),
                }
            }
        }

        // Implement FromStr for the enum with String as the error type
        impl std::str::FromStr for #name {
            type Err = String;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                match s {
                    #(#from_str_variants)*
                    _ => std::result::Result::Err(format!("Unknown variant: {}", s)),
                }
            }
        }
    })
}
