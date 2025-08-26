use super::super::context::{MacroContext, ModelType};
use crate::postgres::field::FieldInfo;
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

    // Separate required and optional fields for constructor
    let mut required_constructor_params = Vec::new();
    let mut required_constructor_assignments = Vec::new();

    for (field_index, info) in ctx.field_infos.iter().enumerate() {
        let name = &info.ident;
        let field_type = get_field_type_for_model(info, ModelType::Insert);
        let is_optional = is_field_optional_in_insert(info);

        // Generate field definition (private fields to enforce encapsulation)
        insert_fields.push(quote! { #name: #field_type });

        // Generate default value
        insert_default_fields.push(get_insert_default_value(info));

        // Generate field conversion for ToSQL
        let column_name = &info.ident.to_string();
        insert_column_names.push(quote! { #column_name });
        insert_field_names.push(name);
        insert_field_indices.push(quote! { #field_index });
        insert_field_conversions.push(get_insert_field_conversion(info));

        // Generate constructor parameters only for required fields
        if !is_optional {
            let field_name = &info.ident;
            let base_type = &info.ty;
            let type_string = base_type.to_token_stream().to_string();

            // Use flexible parameter types for convenience methods
            let (param, assignment) = match type_string.as_str() {
                s if s.contains("String") => (
                    quote! { #field_name: impl Into<::drizzle::postgres::values::InsertValue<'a, ::drizzle::postgres::values::PostgresValue<'a>, ::std::string::String>> },
                    quote! { #field_name: #field_name.into() },
                ),
                s if s.contains("Vec") && s.contains("u8") => (
                    quote! { #field_name: impl Into<::drizzle::postgres::values::InsertValue<'a, ::drizzle::postgres::values::PostgresValue<'a>, ::std::vec::Vec<u8>>> },
                    quote! { #field_name: #field_name.into() },
                ),
                _ => (
                    quote! { #field_name: impl Into<::drizzle::postgres::values::InsertValue<'a, ::drizzle::postgres::values::PostgresValue<'a>, #base_type>> },
                    quote! { #field_name: #field_name.into() },
                ),
            };

            required_constructor_params.push(param);
            required_constructor_assignments.push(assignment);
        }
    }

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

    // Generate convenience methods using pattern tracking
    let convenience_methods_with_pattern: Vec<_> = ctx.field_infos.iter().enumerate().map(|(field_index, info)| {
        // Create generic parameters: just field names (UserName, UserEmail)
        let generic_params: Vec<_> = ctx.field_infos.iter().map(|field_info| {
            let field_pascal_case = field_info.ident.to_string().to_upper_camel_case();
            format_ident!("{}{}", ctx.struct_ident, field_pascal_case)
        }).collect();

        // Create return type pattern: this field becomes Set, others stay generic
        let return_pattern_generics: Vec<_> = ctx.field_infos.iter().enumerate().map(|(i, field_info)| {
            let field_pascal_case = field_info.ident.to_string().to_upper_camel_case();
            if i == field_index {
                format_ident!("{}{}Set", ctx.struct_ident, field_pascal_case)
            } else {
                format_ident!("{}{}", ctx.struct_ident, field_pascal_case) // Keep generic
            }
        }).collect();

        // Generate field assignments - only update the specific field
        let field_assignments: Vec<_> = ctx.field_infos.iter().enumerate().map(|(i, field_info)| {
            let field_name = &field_info.ident;
            if i == field_index {
                quote! { #field_name: value.into() }
            } else {
                quote! { #field_name: self.#field_name }
            }
        }).collect();

        // Generate convenience method based on field type
        let field_name = &info.ident;
        let base_type = &info.ty;
        let method_name = format_ident!("with_{}", field_name);
        let type_string = base_type.to_token_stream().to_string();

        match type_string.as_str() {
            s if s.contains("String") => quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<::drizzle::postgres::values::InsertValue<'a, ::drizzle::postgres::values::PostgresValue<'a>, ::std::string::String>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            },
            s if s.contains("Vec") && s.contains("u8") => quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<::drizzle::postgres::values::InsertValue<'a, ::drizzle::postgres::values::PostgresValue<'a>, ::std::vec::Vec<u8>>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            },
            _ => quote! {
                impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                    pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                    where
                        V: Into<::drizzle::postgres::values::InsertValue<'a, ::drizzle::postgres::values::PostgresValue<'a>, #base_type>>
                    {
                        #insert_model {
                            #(#field_assignments,)*
                            _pattern: ::std::marker::PhantomData,
                        }
                    }
                }
            },
        }
    }).collect();

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

        // Convenience methods for setting fields with pattern tracking
        #(#convenience_methods_with_pattern)*

        impl<'a, T> ::drizzle::core::ToSQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> for #insert_model<'a, T> {
            fn to_sql(&self) -> ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> {
                // For insert models, ToSQL delegates to the values() method
                // which handles mixed placeholders and values correctly
                ::drizzle::core::SQLModel::values(self)
            }
        }

        impl<'a, T> ::drizzle::core::SQLModel<'a, ::drizzle::postgres::values::PostgresValue<'a>> for #insert_model<'a, T> {
            fn columns(&self) -> Box<[&'static dyn ::drizzle::core::SQLColumnInfo]> {
                // For insert model, return only non-omitted columns to match values()
                static TABLE: #struct_ident = #struct_ident::new();
                let all_columns = ::drizzle::core::SQLTableInfo::columns(&TABLE);
                let mut result_columns = Vec::new();

                #(
                    match &self.#insert_field_names {
                        ::drizzle::postgres::values::InsertValue::Omit => {
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

            fn values(&self) -> ::drizzle::core::SQL<'a, ::drizzle::postgres::values::PostgresValue<'a>> {
                let mut sql_parts = Vec::new();

                #(
                    match &self.#insert_field_names {
                        ::drizzle::postgres::values::InsertValue::Omit => {
                            // Skip omitted fields
                        }
                        ::drizzle::postgres::values::InsertValue::Null => {
                            sql_parts.push(::drizzle::core::SQL::parameter(::drizzle::postgres::values::PostgresValue::Null));
                        }
                        ::drizzle::postgres::values::InsertValue::Value(wrapper) => {
                            sql_parts.push(wrapper.value.clone());
                        }
                    }
                )*

                ::drizzle::core::SQL::join(sql_parts, ", ")
            }
        }
    })
}

/// Determine the appropriate field type for INSERT operations
fn get_field_type_for_model(field_info: &FieldInfo, model_type: ModelType) -> TokenStream {
    let base_type = &field_info.ty;
    match model_type {
        ModelType::Insert => {
            // For insert fields, we need to use the inner type for Option<T> fields
            // since InsertValue handles the three-state (Omit, Null, Value) logic
            let inner_type = if field_info.is_nullable {
                // Extract T from Option<T>
                extract_inner_type_from_option(base_type).unwrap_or(base_type)
            } else {
                base_type
            };
            
            quote!(::drizzle::postgres::values::InsertValue<'a, ::drizzle::postgres::values::PostgresValue<'a>, #inner_type>)
        }
        _ => {
            // For other model types, use the base type or Option<T>
            quote!(#base_type)
        }
    }
}

/// Extract the inner type from Option<T>
fn extract_inner_type_from_option(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if let Some(segment) = type_path.path.segments.last() {
            if segment.ident == "Option" {
                if let syn::PathArguments::AngleBracketed(args) = &segment.arguments {
                    if let Some(syn::GenericArgument::Type(inner_ty)) = args.args.first() {
                        return Some(inner_ty);
                    }
                }
            }
        }
    }
    None
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
    if let Some(_default) = &field.default {
        // All PostgreSQL defaults (literals, functions, expressions) go in the SQL schema
        // Let the database handle them, so use Omit in the insert model
        return quote! { #name: ::drizzle::postgres::values::InsertValue::Omit };
    }

    // Handle compile-time SQL defaults or any other case
    // Default to Omit so database can handle defaults
    quote! { #name: ::drizzle::postgres::values::InsertValue::Omit }
}

/// Generates field conversion for insert ToSQL
fn get_insert_field_conversion(field: &FieldInfo) -> TokenStream {
    let name = &field.ident;

    let value_conversion = if field.is_enum || field.is_pgenum {
        quote! { val.clone().into() }
    } else {
        quote! { val.clone().try_into().unwrap_or(::drizzle::postgres::values::PostgresValue::Null) }
    };

    // Handle the three states of InsertValue (Omit, Null, Value)
    if field.default_fn.is_some() {
        // For runtime defaults, we always include the field (either default or user value)
        quote! {
            match &self.#name {
                ::drizzle::postgres::values::InsertValue::Omit => {
                    // Use runtime default for omitted values
                    let default_val = self.#name.clone(); // This should never be Omit due to default logic
                    #value_conversion
                },
                ::drizzle::postgres::values::InsertValue::Null => ::drizzle::postgres::values::PostgresValue::Null,
                ::drizzle::postgres::values::InsertValue::Value(wrapper) => {
                    wrapper.value.clone()
                }
            }
        }
    } else {
        // For regular fields, handle all three states
        quote! {
            match &self.#name {
                ::drizzle::postgres::values::InsertValue::Omit => {
                    // Field omitted - database handles default
                    ::drizzle::postgres::values::PostgresValue::Null // This shouldn't be used if field is omitted
                },
                ::drizzle::postgres::values::InsertValue::Null => ::drizzle::postgres::values::PostgresValue::Null,
                ::drizzle::postgres::values::InsertValue::Value(wrapper) => {
                    wrapper.value.clone()
                }
            }
        }
    }
}