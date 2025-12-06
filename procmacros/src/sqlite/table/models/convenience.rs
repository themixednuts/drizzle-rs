//! Convenience method generation for model types.
//!
//! Generates `with_*` methods for Insert, Update, and PartialSelect models.

use super::super::context::{MacroContext, ModelType};
use crate::sqlite::field::{FieldInfo, SQLiteType, TypeCategory};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates a convenience method for a field based on its type
pub(crate) fn generate_convenience_method(
    field: &FieldInfo,
    model_type: ModelType,
    ctx: &MacroContext,
) -> TokenStream {
    let field_name = field.ident;
    let base_type = field.base_type;
    let method_name = format_ident!("with_{}", field_name);

    // Find the field index for pattern tracking
    let field_index = ctx
        .field_infos
        .iter()
        .position(|f| f.ident == field.ident)
        .expect("Field should exist in context");

    match model_type {
        ModelType::Insert => generate_insert_convenience_method(field, ctx, field_index),
        _ => generate_update_convenience_method(field, base_type, &method_name),
    }
}

// =============================================================================
// Insert Model Convenience Methods
// =============================================================================

fn generate_insert_convenience_method(
    field: &FieldInfo,
    ctx: &MacroContext,
    field_index: usize,
) -> TokenStream {
    let field_name = field.ident;
    let base_type = field.base_type;
    let method_name = format_ident!("with_{}", field_name);
    let insert_model = &ctx.insert_model_ident;

    // Create generic parameters: field names as markers (UserName, UserEmail)
    let generic_params: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|f| {
            let pascal = f.ident.to_string().to_upper_camel_case();
            format_ident!("{}{}", ctx.struct_ident, pascal)
        })
        .collect();

    // Create return type pattern: this field becomes Set, others stay generic
    let return_pattern_generics: Vec<_> = ctx
        .field_infos
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let pascal = f.ident.to_string().to_upper_camel_case();
            if i == field_index {
                format_ident!("{}{}Set", ctx.struct_ident, pascal)
            } else {
                format_ident!("{}{}", ctx.struct_ident, pascal)
            }
        })
        .collect();

    // Generate field assignments - only update the specific field
    let field_assignments: Vec<_> = ctx
        .field_infos
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let fname = f.ident;
            if i == field_index {
                quote! { #fname: value.into() }
            } else {
                quote! { #fname: self.#fname }
            }
        })
        .collect();

    // Dispatch based on type category
    let category = field.type_category();

    match category {
        TypeCategory::Json => generate_json_insert_method(
            field,
            ctx,
            field_index,
            &method_name,
            base_type,
            insert_model,
            &generic_params,
            &return_pattern_generics,
        ),
        TypeCategory::Uuid => {
            let insert_value_type = field.insert_value_inner_type();
            quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<SQLiteInsertValue<'a, SQLiteValue<'a>, #insert_value_type>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        }
        TypeCategory::ArrayString
        | TypeCategory::ArrayVec
        | TypeCategory::Primitive
        | TypeCategory::Enum => {
            // These use the base type directly
            quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<SQLiteInsertValue<'a, SQLiteValue<'a>, #base_type>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        }
        TypeCategory::String => {
            quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<SQLiteInsertValue<'a, SQLiteValue<'a>, ::std::string::String>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        }
        TypeCategory::Blob => {
            quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<SQLiteInsertValue<'a, SQLiteValue<'a>, ::std::vec::Vec<u8>>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn generate_json_insert_method(
    field: &FieldInfo,
    ctx: &MacroContext,
    field_index: usize,
    method_name: &syn::Ident,
    base_type: &syn::Type,
    insert_model: &syn::Ident,
    generic_params: &[syn::Ident],
    return_pattern_generics: &[syn::Ident],
) -> TokenStream {
    let json_wrapper = match field.column_type {
        SQLiteType::Text => quote! {
            {
                let json_str = serde_json::to_string(&value)
                    .unwrap_or_else(|_| "null".to_string());
                SQLiteInsertValue::Value(
                    ValueWrapper {
                        value: json(
                            SQL::param(
                                SQLiteValue::Text(
                                    ::std::borrow::Cow::Owned(json_str)
                                )
                            )),
                        _phantom: ::std::marker::PhantomData,
                    }
                )
            }
        },
        SQLiteType::Blob => quote! {
            {
                let json_bytes = serde_json::to_vec(&value)
                    .unwrap_or_else(|_| "null".as_bytes().to_vec());
                SQLiteInsertValue::Value(
                    ValueWrapper {
                        value: jsonb(
                            SQL::param(
                                SQLiteValue::Blob(
                                    ::std::borrow::Cow::Owned(json_bytes)
                                )
                            )),
                        _phantom: ::std::marker::PhantomData,
                    }
                )
            }
        },
        _ => return quote! {},
    };

    // Generate field assignments with JSON handling for the target field
    let json_field_assignments: Vec<_> = ctx
        .field_infos
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let fname = f.ident;
            if i == field_index {
                quote! { #fname: #json_wrapper }
            } else {
                quote! { #fname: self.#fname }
            }
        })
        .collect();

    quote! {
        impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
            pub fn #method_name(self, value: #base_type) -> #insert_model<'a, (#(#return_pattern_generics),*)>
            {
                #insert_model {
                    #(#json_field_assignments,)*
                    _pattern: ::std::marker::PhantomData,
                }
            }
        }
    }
}

// =============================================================================
// Update/PartialSelect Model Convenience Methods
// =============================================================================

fn generate_update_convenience_method(
    field: &FieldInfo,
    base_type: &syn::Type,
    method_name: &syn::Ident,
) -> TokenStream {
    let field_name = field.ident;
    let assignment = quote! { self.#field_name = Some(value); };
    let category = field.type_category();

    match category {
        TypeCategory::Uuid => {
            quote! {
                pub fn #method_name<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
                    let value = value.into();
                    #assignment
                    self
                }
            }
        }
        TypeCategory::ArrayString
        | TypeCategory::ArrayVec
        | TypeCategory::Primitive
        | TypeCategory::Enum
        | TypeCategory::Json => {
            // These use the base type directly
            quote! {
                pub fn #method_name(mut self, value: #base_type) -> Self {
                    #assignment
                    self
                }
            }
        }
        TypeCategory::String => {
            quote! {
                pub fn #method_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
                    let value = value.into();
                    #assignment
                    self
                }
            }
        }
        TypeCategory::Blob => {
            quote! {
                pub fn #method_name<T: Into<::std::vec::Vec<u8>>>(mut self, value: T) -> Self {
                    let value = value.into();
                    #assignment
                    self
                }
            }
        }
    }
}
