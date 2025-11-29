use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};
use syn::Result;

/// Generates the Insert model with convenience methods and constructor
pub(crate) fn generate_insert_model(
    ctx: &MacroContext,
    required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let insert_model = &ctx.insert_model_ident;
    let struct_ident = &ctx.struct_ident;

    // Convert bool slice to tuple literal for required fields pattern
    let required_fields_pattern_literal = {
        let pattern_values: Vec<_> = required_fields_pattern
            .iter()
            .enumerate()
            .map(|(i, &b)| {
                let field_pascal_case = ctx.field_infos[i].ident.to_string().to_upper_camel_case();
                if b {
                    format_ident!("{}{}Set", ctx.struct_ident, field_pascal_case)
                } else {
                    format_ident!("{}{}NotSet", ctx.struct_ident, field_pascal_case)
                }
            })
            .collect();
        quote! { (#(#pattern_values),*) }
    };

    // Generate tuple type with NotSet for each field
    let empty_pattern_elements: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|info| {
            let field_pascal_case = info.ident.to_string().to_upper_camel_case();
            format_ident!("{}{}NotSet", ctx.struct_ident, field_pascal_case)
        })
        .collect();
    let empty_pattern_tuple = quote! { (#(#empty_pattern_elements),*) };

    let mut insert_fields = Vec::new();
    let mut insert_default_fields = Vec::new();
    let mut insert_field_conversions = Vec::new();
    let mut insert_column_names = Vec::new();
    let mut insert_field_names = Vec::new();
    let mut insert_field_indices = Vec::new();
    let mut insert_convenience_methods = Vec::new();

    // Separate required and optional fields for constructor
    let mut required_constructor_params = Vec::new();
    let mut required_constructor_assignments = Vec::new();

    for (field_index, info) in ctx.field_infos.iter().enumerate() {
        let name = info.ident;
        let field_type = ctx.get_field_type_for_model(info, ModelType::Insert);
        let is_optional = ctx.is_field_optional_in_insert(info);

        // Generate field definition (private fields to enforce encapsulation)
        insert_fields.push(quote! { #name: #field_type });

        // Generate default value
        insert_default_fields.push(ctx.get_insert_default_value(info));

        // Generate field conversion for ToSQL
        let column_name = &info.column_name;
        insert_column_names.push(quote! { #column_name });
        insert_field_names.push(name);
        insert_field_indices.push(quote! { #field_index });
        insert_field_conversions.push(ctx.get_insert_field_conversion(info));

        insert_convenience_methods.push(generate_convenience_method(info, ModelType::Insert, ctx));

        // Generate constructor parameters only for required fields
        if !is_optional {
            let field_name = info.ident;
            let base_type = info.base_type;
            let type_string = base_type.to_token_stream().to_string();

            // Use the same flexible parameter types as convenience methods
            let (param, assignment) = match (info.is_json, info.is_uuid, type_string.as_str()) {
                (true, _, _) => {
                    // Handle JSON fields - wrap in json() or jsonb() based on column type
                    let json_assignment = match info.column_type {
                        crate::sqlite::field::SQLiteType::Text => quote! {
                            #field_name: {
                                let json_str = serde_json::to_string(&#field_name)
                                    .unwrap_or_else(|_| "null".to_string());
                                ::drizzle_sqlite::values::SQLiteInsertValue::Value(
                                    ::drizzle_sqlite::values::ValueWrapper {
                                        value: ::drizzle_sqlite::expression::json(
                                            ::drizzle_core::SQL::param(
                                                ::drizzle_sqlite::values::SQLiteValue::Text(
                                                    ::std::borrow::Cow::Owned(json_str)
                                                )
                                            )),
                                        _phantom: ::std::marker::PhantomData,
                                    }
                                )
                            }
                        },
                        crate::sqlite::field::SQLiteType::Blob => quote! {
                            #field_name: {
                                let json_bytes = serde_json::to_vec(&#field_name)
                                    .unwrap_or_else(|_| "null".as_bytes().to_vec());
                                ::drizzle_sqlite::values::SQLiteInsertValue::Value(
                                    ::drizzle_sqlite::values::ValueWrapper {
                                        value: ::drizzle_sqlite::expression::jsonb(
                                            ::drizzle_core::SQL::param(
                                                ::drizzle_sqlite::values::SQLiteValue::Blob(
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
                (_, true, _) => {
                    // Use String for TEXT columns, Uuid for BLOB columns
                    let insert_value_type = match info.column_type {
                        crate::sqlite::field::SQLiteType::Text => quote! { ::std::string::String },
                        _ => quote! { ::uuid::Uuid },
                    };
                    (
                        quote! { #field_name: impl Into<::drizzle_sqlite::values::SQLiteInsertValue<'a, ::drizzle_sqlite::values::SQLiteValue<'a>, #insert_value_type>> },
                        quote! { #field_name: #field_name.into() },
                    )
                }
                // ArrayString and ArrayVec must be checked BEFORE String/Vec to avoid false matches
                (_, _, s) if s.contains("ArrayString") || s.contains("ArrayVec") => (
                    quote! { #field_name: impl Into<::drizzle_sqlite::values::SQLiteInsertValue<'a, ::drizzle_sqlite::values::SQLiteValue<'a>, #base_type>> },
                    quote! { #field_name: #field_name.into() },
                ),
                (_, _, s) if s.contains("String") => (
                    quote! { #field_name: impl Into<::drizzle_sqlite::values::SQLiteInsertValue<'a, ::drizzle_sqlite::values::SQLiteValue<'a>, ::std::string::String>> },
                    quote! { #field_name: #field_name.into() },
                ),
                (_, _, s) if s.contains("Vec") && s.contains("u8") => (
                    quote! { #field_name: impl Into<::drizzle_sqlite::values::SQLiteInsertValue<'a, ::drizzle_sqlite::values::SQLiteValue<'a>, ::std::vec::Vec<u8>>> },
                    quote! { #field_name: #field_name.into() },
                ),
                (_, _, _) => (
                    quote! { #field_name: impl Into<::drizzle_sqlite::values::SQLiteInsertValue<'a, ::drizzle_sqlite::values::SQLiteValue<'a>, #base_type>> },
                    quote! { #field_name: #field_name.into() },
                ),
            };

            required_constructor_params.push(param);
            required_constructor_assignments.push(assignment);
        }
    }

    // No longer need bit constants with array approach

    // Generate marker types for each field (e.g., UserNameSet, UserNameNotSet)
    let field_marker_types: Vec<_> = ctx
        .field_infos
        .iter()
        .map(|info| {
            let field_pascal_case = info.ident.to_string().to_upper_camel_case();
            let set_marker = format_ident!("{}{}Set", ctx.struct_ident, field_pascal_case);
            let not_set_marker = format_ident!("{}{}NotSet", ctx.struct_ident, field_pascal_case);

            quote! {
                pub struct #set_marker;
                pub struct #not_set_marker;
            }
        })
        .collect();

    // Convenience methods are now generated by convenience.rs with pattern tracking

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
                // For any pattern, default() creates an instance with default field values
                // The pattern type T is preserved but all fields get default values
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

        // Convenience methods for setting fields (generated by convenience.rs)
        #(#insert_convenience_methods)*

        impl<'a, T> ::drizzle_core::ToSQL<'a, ::drizzle_sqlite::values::SQLiteValue<'a>> for #insert_model<'a, T> {
            fn to_sql(&self) -> ::drizzle_core::SQL<'a, ::drizzle_sqlite::values::SQLiteValue<'a>> {
                // For insert models, ToSQL delegates to the values() method
                ::drizzle_core::SQLModel::values(self)
            }
        }

        impl<'a, T> ::drizzle_core::SQLModel<'a, ::drizzle_sqlite::values::SQLiteValue<'a>> for #insert_model<'a, T> {
            fn columns(&self) -> Box<[&'static dyn ::drizzle_core::SQLColumnInfo]> {
                // For insert model, return only non-omitted columns to match values()
                static TABLE: #struct_ident = #struct_ident::new();
                let all_columns = ::drizzle_core::SQLTableInfo::columns(&TABLE);
                let mut result_columns = Vec::new();

                #(
                    match &self.#insert_field_names {
                        ::drizzle_sqlite::values::SQLiteInsertValue::Omit => {
                            // Skip omitted fields
                        }
                        _ => {
                            // Include this column (Value or Null)
                            result_columns.push(all_columns[#insert_field_indices]);
                        }
                    }
                )*

                result_columns.into_boxed_slice()
            }

            fn values(&self) -> ::drizzle_core::SQL<'a, ::drizzle_sqlite::values::SQLiteValue<'a>> {

                let mut sql_parts = Vec::new();

                #(
                    match &self.#insert_field_names {
                        ::drizzle_sqlite::values::SQLiteInsertValue::Omit => {
                            // Skip omitted fields
                        }
                        ::drizzle_sqlite::values::SQLiteInsertValue::Null => {
                            sql_parts.push(::drizzle_core::SQL::param(::drizzle_sqlite::values::SQLiteValue::Null));
                        }
                        ::drizzle_sqlite::values::SQLiteInsertValue::Value(wrapper) => {
                            sql_parts.push(wrapper.value.clone());
                        }
                    }
                )*

                ::drizzle_core::SQL::join(sql_parts, ::drizzle_core::Token::COMMA)
            }
        }
    })
}
