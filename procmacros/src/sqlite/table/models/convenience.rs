//! Convenience method generation for model types.
//!
//! Generates `with_*` methods for Insert, Update, and PartialSelect models.

use super::super::context::{MacroContext, ModelType};
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
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
    let field_index = ctx.field_infos.iter().position(|f| f.ident == field.ident);

    let Some(field_index) = field_index else {
        return quote! { compile_error!("internal error: field missing from macro context"); };
    };

    match model_type {
        ModelType::Insert => generate_insert_convenience_method(field, ctx, field_index),
        ModelType::Update => {
            generate_update_convenience_method(field, base_type, &method_name, ctx)
        }
        ModelType::PartialSelect => {
            generate_partial_select_convenience_method(field, base_type, &method_name)
        }
        ModelType::Select => {
            unreachable!("Select models do not have convenience methods")
        }
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

    // Get paths for fully-qualified types
    let _sql = core_paths::sql();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_insert_value = sqlite_paths::sqlite_insert_value();
    let _value_wrapper = sqlite_paths::value_wrapper();
    let _expression = sqlite_paths::expressions();

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
                        V: ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, #insert_value_type>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        }
        TypeCategory::ArrayString | TypeCategory::ArrayVec | TypeCategory::Enum => {
            // These use the base type directly
            quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, #base_type>>
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
                        V: ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, ::std::string::String>>
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
                        V: ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, ::std::vec::Vec<u8>>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        }
        // All other types (Integer, Real, Bool, DateTime, Unknown, ByteArray) use base type directly
        _ => {
            quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, #base_type>>
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
    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_insert_value = sqlite_paths::sqlite_insert_value();
    let value_wrapper = sqlite_paths::value_wrapper();
    let expression = sqlite_paths::expressions();

    let json_wrapper = match field.column_type {
        SQLiteType::Text => quote! {
            {
                let json_str = ::serde_json::to_string(&value)
                    .expect("failed to serialize JSON value for SQLite TEXT column");
                #sqlite_insert_value::Value(
                    #value_wrapper {
                        value: #expression::json(
                            #sql::param(
                                #sqlite_value::Text(
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
                let json_bytes = ::serde_json::to_vec(&value)
                    .expect("failed to serialize JSON value for SQLite BLOB column");
                #sqlite_insert_value::Value(
                    #value_wrapper {
                        value: #expression::jsonb(
                            #sql::param(
                                #sqlite_value::Blob(
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
// PartialSelect Model Convenience Methods
// =============================================================================

fn generate_partial_select_convenience_method(
    field: &FieldInfo,
    base_type: &syn::Type,
    method_name: &syn::Ident,
) -> TokenStream {
    let field_name = field.ident;

    // PartialSelect methods are simple Option<T> setters, placed inside a shared impl block
    quote! {
        pub fn #method_name(mut self, value: #base_type) -> Self {
            self.#field_name = Some(value);
            self
        }
    }
}

// =============================================================================
// Update Model Convenience Methods
// =============================================================================

fn generate_update_convenience_method(
    field: &FieldInfo,
    base_type: &syn::Type,
    method_name: &syn::Ident,
    ctx: &MacroContext,
) -> TokenStream {
    let field_name = field.ident;
    let update_model = &ctx.update_model_ident;
    let non_empty_marker = core_paths::non_empty_marker();
    let sqlite_update_value = sqlite_paths::sqlite_update_value();
    let sqlite_value = sqlite_paths::sqlite_value();
    let category = field.type_category();

    // Determine the inner type for the UpdateValue wrapper
    let inner_type = match category {
        TypeCategory::String => quote!(::std::string::String),
        TypeCategory::Blob => quote!(::std::vec::Vec<u8>),
        _ => quote!(#base_type),
    };

    // Generate field assignments: the target field gets the new value, others are moved
    let field_assignments: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|f| {
            let fname = f.ident;
            if fname == field_name {
                quote! { #fname: value.into() }
            } else {
                quote! { #fname: self.#fname }
            }
        })
        .collect();

    // Each method in its own impl<'a, S> block so 'a is declared and used
    // within the same quote! invocation (matching the Insert pattern).
    // Accepts any state S, always returns NonEmpty.
    quote! {
        impl<'a, S> #update_model<'a, S> {
            pub fn #method_name<V: ::std::convert::Into<#sqlite_update_value<'a, #sqlite_value<'a>, #inner_type>>>(self, value: V) -> #update_model<'a, #non_empty_marker> {
                #update_model {
                    #(#field_assignments,)*
                    _state: ::std::marker::PhantomData,
                }
            }
        }
    }
}
