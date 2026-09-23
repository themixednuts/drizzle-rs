//! Convenience method generation for model types.
//!
//! Generates `with_*` methods for Insert, Update, and `PartialSelect` models.

use super::super::context::{MacroContext, ModelType};
use crate::common::column_types::{insert_state_param, set_marker};
use crate::common::rust_type_to_nullability;
use crate::paths::{core as core_paths, postgres as pg_paths};
use crate::postgres::field::{FieldInfo, TypeCategory};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};

/// Generates a convenience method for a field based on its type
pub fn generate_convenience_method(
    field: &FieldInfo,
    model_type: ModelType,
    ctx: &MacroContext,
) -> TokenStream {
    let field_name = &field.ident;
    let base_type = &field.base_type;
    let method_name = format_ident!("with_{}", field_name);

    // Find the field index for pattern tracking
    let field_index = ctx.field_infos.iter().position(|f| f.ident == field.ident);

    let Some(field_index) = field_index else {
        return quote! { compile_error!("field not found on this table — this is a bug in drizzle-macros, please report it"); };
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
    let field_name = &field.ident;
    let base_type = &field.base_type;
    let method_name = format_ident!("with_{}", field_name);
    let insert_model = &ctx.insert_model_ident;

    // One generic parameter per field for its insert state (`__Name`)
    let generic_params: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|f| insert_state_param(ctx.struct_ident, &f.ident))
        .collect();

    // Create return type pattern: this field becomes Set, others stay generic
    let return_pattern_generics: Vec<TokenStream> = ctx
        .field_infos
        .iter()
        .zip(&generic_params)
        .enumerate()
        .map(|(i, (f, param))| {
            if i == field_index {
                set_marker(ctx.struct_ident, &f.ident)
            } else {
                quote!(#param)
            }
        })
        .collect();

    // Generate field assignments - only update the specific field
    let field_assignments: Vec<_> = ctx
        .field_infos
        .iter()
        .enumerate()
        .map(|(i, f)| {
            let fname = &f.ident;
            if i == field_index && field.is_json_payload() {
                let value = field.json_insert_value(&quote!(value));
                quote! { #fname: #value }
            } else if i == field_index {
                quote! { #fname: value.into() }
            } else {
                quote! { #fname: self.#fname }
            }
        })
        .collect();

    // Dispatch based on type category
    let category = field.type_category();

    // JSON payloads take the payload and bind as the column's `json`/`jsonb`.
    if field.is_json_payload() {
        return quote! {
            impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                pub fn #method_name(self, value: #base_type) -> #insert_model<'a, (#(#return_pattern_generics),*)> {
                    #insert_model {
                        #(#field_assignments,)*
                        _pattern: ::std::marker::PhantomData,
                    }
                }
            }
        };
    }

    match category {
        TypeCategory::String => {
            quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<PostgresInsertValue<'a, PostgresValue<'a>, ::std::string::String>>
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
                        V: Into<PostgresInsertValue<'a, PostgresValue<'a>, ::std::vec::Vec<u8>>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            }
        }
        // ArrayString, ArrayVec, Uuid, serde_json::Value, Enum, and primitives
        // use the base type directly.
        _ => {
            quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<PostgresInsertValue<'a, PostgresValue<'a>, #base_type>>
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

// =============================================================================
// PartialSelect Model Convenience Methods
// =============================================================================

fn generate_partial_select_convenience_method(
    field: &FieldInfo,
    base_type: &syn::Type,
    method_name: &syn::Ident,
) -> TokenStream {
    let field_name = &field.ident;

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
    let postgres_update_value = pg_paths::postgres_update_value();
    let postgres_value = pg_paths::postgres_value();
    let field_name = &field.ident;
    let update_model = &ctx.update_model_ident;
    let non_empty_marker = core_paths::non_empty_marker();
    let category = field.type_category();
    let sql_type = field.sql_type_marker();
    let nullable = rust_type_to_nullability(&field.field_type);

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
            let fname = &f.ident;
            if *fname == *field_name && field.is_json_payload() {
                quote! { #fname: drizzle::core::json::JsonColumnArg::into_json_column(value) }
            } else if *fname == *field_name {
                quote! { #fname: value.into() }
            } else {
                quote! { #fname: self.#fname }
            }
        })
        .collect();

    // JSON payloads accept the payload, `Json(payload)`, or an SQL operand
    // (placeholder, expression, EXCLUDED reference, or an update value). The
    // column's SQL type marker selects `json` or `jsonb`.
    if field.is_json_payload() {
        return quote! {
            impl<'a, S> #update_model<'a, S> {
                pub fn #method_name<V, M>(self, value: V) -> #update_model<'a, #non_empty_marker>
                where
                    V: drizzle::core::json::JsonColumnArg<
                        #postgres_update_value<'a, #postgres_value<'a>, #inner_type, #sql_type, #nullable>,
                        M,
                    >,
                {
                    #update_model {
                        #(#field_assignments,)*
                        _state: ::std::marker::PhantomData,
                    }
                }
            }
        };
    }

    // Each method in its own impl<'a, S> block so 'a is declared and used
    // within the same quote! invocation (matching the Insert pattern).
    // Accepts any state S, always returns NonEmpty.
    quote! {
        impl<'a, S> #update_model<'a, S> {
            pub fn #method_name<V: Into<#postgres_update_value<'a, #postgres_value<'a>, #inner_type, #sql_type, #nullable>>>(self, value: V) -> #update_model<'a, #non_empty_marker> {
                #update_model {
                    #(#field_assignments,)*
                    _state: ::std::marker::PhantomData,
                }
            }
        }
    }
}
