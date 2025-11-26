use super::super::context::{MacroContext, ModelType};
use crate::sqlite::field::{FieldInfo, SQLiteType};
use heck::ToUpperCamelCase;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};

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

    let assignment = match model_type {
        ModelType::Insert => quote! { self.#field_name = value.into(); },
        ModelType::Update => quote! { self.#field_name = Some(value); },
        ModelType::PartialSelect => quote! { self.#field_name = Some(value); },
    };

    // Generate type-specific convenience methods using modern pattern matching
    match model_type {
        ModelType::Insert => {
            let insert_model = &ctx.insert_model_ident;

            // Create generic parameters: just field names (UserName, UserEmail)
            let generic_params: Vec<_> = ctx
                .field_infos
                .iter()
                .map(|field_info| {
                    let field_pascal_case = field_info.ident.to_string().to_upper_camel_case();
                    format_ident!("{}{}", ctx.struct_ident, field_pascal_case)
                })
                .collect();

            // Create return type pattern: this field becomes Set, others stay generic
            let return_pattern_generics: Vec<_> = ctx
                .field_infos
                .iter()
                .enumerate()
                .map(|(i, field_info)| {
                    let field_pascal_case = field_info.ident.to_string().to_upper_camel_case();
                    if i == field_index {
                        format_ident!("{}{}Set", ctx.struct_ident, field_pascal_case)
                    } else {
                        format_ident!("{}{}", ctx.struct_ident, field_pascal_case) // Keep generic
                    }
                })
                .collect();

            // Generate field assignments - only update the specific field
            let field_assignments: Vec<_> = ctx
                .field_infos
                .iter()
                .enumerate()
                .map(|(i, field_info)| {
                    let field_name = field_info.ident;
                    if i == field_index {
                        quote! { #field_name: value.into() }
                    } else {
                        quote! { #field_name: self.#field_name }
                    }
                })
                .collect();

            // Use the original working convenience method logic but modify the return type
            let type_string = base_type.to_token_stream().to_string();

            match (field.is_json, field.is_uuid, type_string.as_str()) {
                (true, _, _) => {
                    // Handle JSON fields - wrap in json() or jsonb() based on column type
                    let json_wrapper = match field.column_type {
                        SQLiteType::Text => quote! {
                            // For TEXT columns, use json() wrapper
                            {
                                let json_str = serde_json::to_string(&value)
                                    .unwrap_or_else(|_| "null".to_string());
                                InsertValue::Value(
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
                            // For BLOB columns, use jsonb() wrapper
                            {
                                let json_bytes = serde_json::to_vec(&value)
                                    .unwrap_or_else(|_| "null".as_bytes().to_vec());
                                InsertValue::Value(
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
                        _ => return quote! {}, // Skip unsupported column types
                    };

                    // Generate field assignments with JSON handling for the target field
                    let json_field_assignments: Vec<_> = ctx
                        .field_infos
                        .iter()
                        .enumerate()
                        .map(|(i, field_info)| {
                            let field_name = field_info.ident;
                            if i == field_index {
                                quote! { #field_name: #json_wrapper }
                            } else {
                                quote! { #field_name: self.#field_name }
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
                (_, true, _) => {
                    // Use String for TEXT columns, Uuid for BLOB columns
                    let insert_value_type = match field.column_type {
                        crate::sqlite::field::SQLiteType::Text => quote! { ::std::string::String },
                        _ => quote! { ::uuid::Uuid },
                    };
                    quote! {
                        impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                            pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                            where
                                V: Into<InsertValue<'a, SQLiteValue<'a>, #insert_value_type>>
                            {
                                #insert_model {
                                    #(#field_assignments,)*
                                    _pattern: ::std::marker::PhantomData,
                                }
                            }
                        }
                    }
                }
                (_, _, s) if s.contains("String") => quote! {
                    impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                        pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                        where
                            V: Into<InsertValue<'a, SQLiteValue<'a>, ::std::string::String>>
                        {
                            #insert_model {
                                #(#field_assignments,)*
                                _pattern: ::std::marker::PhantomData,
                            }
                        }
                    }
                },
                (_, _, s) if s.contains("Vec") && s.contains("u8") => quote! {
                    impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                        pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                        where
                            V: Into<InsertValue<'a, SQLiteValue<'a>, ::std::vec::Vec<u8>>>
                        {
                            #insert_model {
                                #(#field_assignments,)*
                                _pattern: ::std::marker::PhantomData,
                            }
                        }
                    }
                },
                (_, _, _) => quote! {
                    impl<'a, #(#generic_params),*> #insert_model<'a, (#(#generic_params),*)> {
                        pub fn #method_name<V>(self, value: V) -> #insert_model<'a, (#(#return_pattern_generics),*)>
                        where
                            V: Into<InsertValue<'a, SQLiteValue<'a>, #base_type>>
                        {
                            #insert_model {
                                #(#field_assignments,)*
                                _pattern: ::std::marker::PhantomData,
                            }
                        }
                    }
                },
            }
        }
        _ => {
            // For other models, keep the existing logic
            let type_string = base_type.to_token_stream().to_string();
            match (field.is_uuid, type_string.as_str()) {
                (true, _) => quote! {
                    pub fn #method_name<T: Into<::uuid::Uuid>>(mut self, value: T) -> Self {
                        let value = value.into();
                        #assignment
                        self
                    }
                },
                (_, s) if s.contains("String") => quote! {
                    pub fn #method_name<T: Into<::std::string::String>>(mut self, value: T) -> Self {
                        let value = value.into();
                        #assignment
                        self
                    }
                },
                (_, s) if s.contains("Vec") && s.contains("u8") => quote! {
                    pub fn #method_name<T: Into<::std::vec::Vec<u8>>>(mut self, value: T) -> Self {
                        let value = value.into();
                        #assignment
                        self
                    }
                },
                _ => quote! {
                    pub fn #method_name(mut self, value: #base_type) -> Self {
                        #assignment
                        self
                    }
                },
            }
        }
    }
}
