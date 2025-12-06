//! Insert model generation.
//!
//! Generates the InsertModel struct with type-safe field tracking using marker types.

use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use crate::postgres::field::{FieldInfo, TypeCategory};
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
        let name = &info.ident;
        let field_type = get_field_type_for_model(info, ModelType::Insert);
        let is_optional = is_field_optional_in_insert(info);

        insert_fields.push(quote! { #name: #field_type });
        insert_default_fields.push(get_insert_default_value(info));
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

        impl<'a, T> ToSQL<'a, PostgresValue<'a>> for #insert_model<'a, T> {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                SQLModel::values(self)
            }
        }

        impl<'a, T> SQLModel<'a, PostgresValue<'a>> for #insert_model<'a, T> {
            type Columns = Box<[&'static dyn SQLColumnInfo]>;

            fn columns(&self) -> Self::Columns {
                static TABLE: #struct_ident = #struct_ident::new();
                let all_columns = SQLTableInfo::columns(&TABLE);
                let mut result_columns = Vec::new();

                #(
                    match &self.#insert_field_names {
                        PostgresInsertValue::Omit => {}
                        _ => {
                            result_columns.push(all_columns[#insert_field_indices]);
                        }
                    }
                )*

                result_columns.into_boxed_slice()
            }

            fn values(&self) -> SQL<'a, PostgresValue<'a>> {
                let mut sql_parts = Vec::new();

                #(
                    match &self.#insert_field_names {
                        PostgresInsertValue::Omit => {}
                        PostgresInsertValue::Null => {
                            sql_parts.push(SQL::param(PostgresValue::Null));
                        }
                        PostgresInsertValue::Value(wrapper) => {
                            sql_parts.push(wrapper.value.clone());
                        }
                    }
                )*

                SQL::join(sql_parts, Token::COMMA)
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

/// Determine the appropriate field type for INSERT operations
fn get_field_type_for_model(field_info: &FieldInfo, model_type: ModelType) -> TokenStream {
    let base_type = field_info.base_type();
    match model_type {
        ModelType::Insert => {
            // For insert fields, use the base type (inner type for Option<T>)
            // since InsertValue handles the three-state (Omit, Null, Value) logic
            quote!(PostgresInsertValue<'a, PostgresValue<'a>, #base_type>)
        }
        _ => quote!(#base_type),
    }
}

/// Determines if a field should be optional in the Insert model
fn is_field_optional_in_insert(field: &FieldInfo) -> bool {
    // Nullable fields are always optional
    if field.is_nullable {
        return true;
    }

    // Fields with explicit defaults (SQL or runtime) are optional
    if field.has_default || field.default_fn.is_some() {
        return true;
    }

    // SERIAL fields are auto-generated and optional
    field.is_serial
}

/// Gets the default value expression for insert model
fn get_insert_default_value(field: &FieldInfo) -> TokenStream {
    let name = &field.ident;

    // Handle runtime function defaults (default_fn)
    if let Some(f) = &field.default_fn {
        return quote! { #name: ((#f)()).into() };
    }

    // Handle compile-time PostgreSQL defaults (SQL defaults - let database handle)
    if field.default.is_some() {
        return quote! { #name: PostgresInsertValue::Omit };
    }

    // Default to Omit so database can handle defaults
    quote! { #name: PostgresInsertValue::Omit }
}

/// Generate constructor parameter and assignment based on field type category.
fn generate_constructor_param(info: &FieldInfo) -> (TokenStream, TokenStream) {
    let field_name = &info.ident;
    let base_type = info.base_type();
    let category = info.type_category();

    match category {
        TypeCategory::String => (
            quote! { #field_name: impl Into<PostgresInsertValue<'a, PostgresValue<'a>, ::std::string::String>> },
            quote! { #field_name: #field_name.into() },
        ),
        TypeCategory::Blob => (
            quote! { #field_name: impl Into<PostgresInsertValue<'a, PostgresValue<'a>, ::std::vec::Vec<u8>>> },
            quote! { #field_name: #field_name.into() },
        ),
        // ArrayString, ArrayVec, Uuid, Json, Enum, Primitive use base type directly
        TypeCategory::ArrayString
        | TypeCategory::ArrayVec
        | TypeCategory::Uuid
        | TypeCategory::Json
        | TypeCategory::Enum
        | TypeCategory::Primitive => (
            quote! { #field_name: impl Into<PostgresInsertValue<'a, PostgresValue<'a>, #base_type>> },
            quote! { #field_name: #field_name.into() },
        ),
    }
}
