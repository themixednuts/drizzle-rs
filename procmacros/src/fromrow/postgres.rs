use crate::postgres::field::TypeCategory;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::{Field, Ident, Result};

fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

fn extract_inner_type(ty: &syn::Type) -> &syn::Type {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first()
    {
        return inner_ty;
    }
    ty
}

/// Generate postgres field assignment for FromRow derive
pub(crate) fn generate_field_assignment(
    idx: usize,
    field: &Field,
    field_name: Option<&Ident>,
) -> Result<TokenStream> {
    let type_str = field.ty.to_token_stream().to_string();
    let category = TypeCategory::from_type_string(&type_str);

    let idx_or_name = if let Some(field_name) = field_name {
        let field_name_str = field_name.to_string();
        quote! { #field_name_str }
    } else {
        quote! { #idx }
    };

    let target_type = extract_inner_type(&field.ty);

    let assignment = match category {
        TypeCategory::ArrayString => {
            if is_option_type(&field.ty) {
                quote! {
                    {
                        let temp: Option<String> = row.get(#idx_or_name);
                        match temp {
                            Some(s) => {
                                let converted = <#target_type>::from(s.as_str()).map_err(|_| ::drizzle_core::error::DrizzleError::ConversionError("ArrayString capacity exceeded".into()))?;
                                Some(converted)
                            }
                            None => None,
                        }
                    }
                }
            } else {
                quote! {
                    {
                        let temp: String = row.get(#idx_or_name);
                        <#target_type>::from(temp.as_str()).map_err(|_| ::drizzle_core::error::DrizzleError::ConversionError("ArrayString capacity exceeded".into()))?
                    }
                }
            }
        }
        TypeCategory::ArrayVec => {
            if is_option_type(&field.ty) {
                quote! {
                    {
                        let temp: Option<Vec<u8>> = row.get(#idx_or_name);
                        match temp {
                            Some(v) => {
                                let mut av: #target_type = ::arrayvec::ArrayVec::new();
                                av.try_extend_from_slice(&v).map_err(|_| ::drizzle_core::error::DrizzleError::ConversionError("ArrayVec capacity exceeded".into()))?;
                                Some(av)
                            }
                            None => None,
                        }
                    }
                }
            } else {
                quote! {
                    {
                        let temp: Vec<u8> = row.get(#idx_or_name);
                        let mut av: #target_type = ::arrayvec::ArrayVec::new();
                        av.try_extend_from_slice(&temp).map_err(|_| ::drizzle_core::error::DrizzleError::ConversionError("ArrayVec capacity exceeded".into()))?;
                        av
                    }
                }
            }
        }
        _ => {
            quote! {
                row.get(#idx_or_name)
            }
        }
    };

    let name = if let Some(field_name) = field_name {
        quote! {
            #field_name: #assignment,
        }
    } else {
        quote! {
            #assignment,
        }
    };
    Ok(name)
}
