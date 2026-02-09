//! Insert model generation.
//!
//! Generates the InsertModel struct with type-safe field tracking using marker types.

use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use crate::sqlite::field::{SQLiteType, TypeCategory};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Result;

/// Generates the Insert model with convenience methods and constructor
pub(crate) fn generate_insert_model(
    ctx: &MacroContext,
    required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let insert_model = &ctx.insert_model_ident;
    let struct_ident = &ctx.struct_ident;

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sql_model = core_paths::sql_model();
    let to_sql = core_paths::to_sql();
    let sql_column_info = core_paths::sql_column_info();
    let sql_table_info = core_paths::sql_table_info();
    let token = core_paths::token();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_insert_value = sqlite_paths::sqlite_insert_value();
    let _value_wrapper = sqlite_paths::value_wrapper();
    let _expression = sqlite_paths::expressions();

    // Convert bool slice to tuple literal for required fields pattern
    let required_fields_pattern_literal = generate_pattern_literal(ctx, required_fields_pattern);

    // Generate tuple type with NotSet for each field
    let empty_pattern_tuple = generate_empty_pattern_tuple(ctx);

    let mut insert_fields = Vec::new();
    let mut insert_default_fields = Vec::new();
    let mut insert_field_names = Vec::new();
    let mut insert_field_indices = Vec::new();
    let mut insert_convenience_methods = Vec::new();
    let mut required_constructor_params = Vec::new();
    let mut required_constructor_assignments = Vec::new();

    for (field_index, info) in ctx.field_infos.iter().enumerate() {
        let name = info.ident;
        let field_type = ctx.get_field_type_for_model(info, ModelType::Insert);
        let is_optional = ctx.is_field_optional_in_insert(info);

        insert_fields.push(quote! { #name: #field_type });
        insert_default_fields.push(ctx.get_insert_default_value(info));
        insert_field_names.push(name);
        insert_field_indices.push(quote! { #field_index });
        insert_convenience_methods.push(generate_convenience_method(info, ModelType::Insert, ctx));

        // Generate constructor parameters only for required fields
        if !is_optional {
            let (param, assignment) = generate_constructor_param(info);
            required_constructor_params.push(param);
            required_constructor_assignments.push(assignment);
        }
    }

    // Generate marker types for each field
    let field_marker_types = generate_marker_types(ctx);

    Ok(quote! {
        // Generate marker types for each field
        #(#field_marker_types)*

        // Insert Model with PhantomData pattern tracking
        #[derive(Debug, Clone)]
        pub struct #insert_model<'a, T = #empty_pattern_tuple> {
            #(#insert_fields,)*
            _pattern: ::std::marker::PhantomData<T>,
        }

        impl<'a, T> Default for #insert_model<'a, T> {
            fn default() -> Self {
                Self {
                    #(#insert_default_fields,)*
                    _pattern: ::std::marker::PhantomData,
                }
            }
        }

        impl<'a> #insert_model<'a, #empty_pattern_tuple> {
            pub fn new(#(#required_constructor_params),*) -> #insert_model<'a, #required_fields_pattern_literal> {
                #insert_model {
                    #(#required_constructor_assignments,)*
                    ..Default::default()
                }
            }
        }

        impl<'a, T> #insert_model<'a, T> {
            /// Converts this insert model to an owned version with 'static lifetime
            pub fn into_owned(self) -> #insert_model<'static, T> {
                #insert_model {
                    #(#insert_field_names: self.#insert_field_names.into_owned(),)*
                    _pattern: ::std::marker::PhantomData,
                }
            }
        }

        // Convenience methods for setting fields
        #(#insert_convenience_methods)*

        impl<'a, T> #to_sql<'a, #sqlite_value<'a>> for #insert_model<'a, T> {
            fn to_sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                #sql_model::values(self)
            }
        }

        impl<'a, T> #sql_model<'a, #sqlite_value<'a>> for #insert_model<'a, T> {
            fn columns(&self) -> ::std::borrow::Cow<'static, [&'static dyn #sql_column_info]> {
                static TABLE: #struct_ident = #struct_ident::new();
                let all_columns = #sql_table_info::columns(&TABLE);
                let mut result_columns = ::std::vec::Vec::new();

                #(
                    match &self.#insert_field_names {
                        #sqlite_insert_value::Omit => {}
                        _ => {
                            result_columns.push(all_columns[#insert_field_indices]);
                        }
                    }
                )*

                ::std::borrow::Cow::Owned(result_columns)
            }

            fn values(&self) -> #sql<'a, #sqlite_value<'a>> {
                let mut sql_parts = ::std::vec::Vec::new();

                #(
                    match &self.#insert_field_names {
                        #sqlite_insert_value::Omit => {}
                        #sqlite_insert_value::Null => {
                            sql_parts.push(#sql::param(#sqlite_value::Null));
                        }
                        #sqlite_insert_value::Value(wrapper) => {
                            sql_parts.push(wrapper.value.clone());
                        }
                    }
                )*

                #sql::join(sql_parts, #token::COMMA)
            }
        }
    })
}

// =============================================================================
// Helper Functions
// =============================================================================

fn generate_pattern_literal(ctx: &MacroContext, required_fields_pattern: &[bool]) -> TokenStream {
    let pattern_values: Vec<_> = required_fields_pattern
        .iter()
        .enumerate()
        .map(|(i, &b)| {
            let pascal = ctx.field_infos[i].ident.to_string().to_upper_camel_case();
            if b {
                format_ident!("{}{}Set", ctx.struct_ident, pascal)
            } else {
                format_ident!("{}{}NotSet", ctx.struct_ident, pascal)
            }
        })
        .collect();
    quote! { (#(#pattern_values),*) }
}

fn generate_empty_pattern_tuple(ctx: &MacroContext) -> TokenStream {
    let elements: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|info| {
            let pascal = info.ident.to_string().to_upper_camel_case();
            format_ident!("{}{}NotSet", ctx.struct_ident, pascal)
        })
        .collect();
    quote! { (#(#elements),*) }
}

fn generate_marker_types(ctx: &MacroContext) -> Vec<TokenStream> {
    ctx.field_infos
        .iter()
        .map(|info| {
            let pascal = info.ident.to_string().to_upper_camel_case();
            let set_marker = format_ident!("{}{}Set", ctx.struct_ident, pascal);
            let not_set_marker = format_ident!("{}{}NotSet", ctx.struct_ident, pascal);

            quote! {
                pub struct #set_marker;
                pub struct #not_set_marker;
            }
        })
        .collect()
}

/// Generate constructor parameter and assignment based on field type category.
fn generate_constructor_param(
    info: &crate::sqlite::field::FieldInfo,
) -> (TokenStream, TokenStream) {
    let field_name = info.ident;
    let base_type = info.base_type;
    let category = info.type_category();

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_insert_value = sqlite_paths::sqlite_insert_value();
    let value_wrapper = sqlite_paths::value_wrapper();
    let expression = sqlite_paths::expressions();

    match category {
        TypeCategory::Json => {
            let json_assignment = match info.column_type {
                SQLiteType::Text => quote! {
                    #field_name: {
                        let json_str = ::serde_json::to_string(&#field_name)
                            .unwrap_or_else(|_| "null".to_string());
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
                    #field_name: {
                        let json_bytes = ::serde_json::to_vec(&#field_name)
                            .unwrap_or_else(|_| "null".as_bytes().to_vec());
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
                _ => quote! { #field_name: #field_name.into() },
            };
            (quote! { #field_name: #base_type }, json_assignment)
        }
        TypeCategory::Uuid => {
            let insert_value_type = info.insert_value_inner_type();
            (
                quote! { #field_name: impl ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, #insert_value_type>> },
                quote! { #field_name: #field_name.into() },
            )
        }
        TypeCategory::String => (
            quote! { #field_name: impl ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, ::std::string::String>> },
            quote! { #field_name: #field_name.into() },
        ),
        TypeCategory::Blob => (
            quote! { #field_name: impl ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, ::std::vec::Vec<u8>>> },
            quote! { #field_name: #field_name.into() },
        ),
        // ArrayString, ArrayVec, Enum use base type directly
        TypeCategory::ArrayString | TypeCategory::ArrayVec | TypeCategory::Enum => (
            quote! { #field_name: impl ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, #base_type>> },
            quote! { #field_name: #field_name.into() },
        ),
        // All other types (Integer, Real, Bool, DateTime, Unknown, ByteArray) use base type directly
        _ => (
            quote! { #field_name: impl ::std::convert::Into<#sqlite_insert_value<'a, #sqlite_value<'a>, #base_type>> },
            quote! { #field_name: #field_name.into() },
        ),
    }
}
