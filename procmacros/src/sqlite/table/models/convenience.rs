use super::super::context::{MacroContext, ModelType};
use crate::sqlite::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::{format_ident, quote, ToTokens};

/// Generates a convenience method for a field based on its type
pub(crate) fn generate_convenience_method(
    field: &FieldInfo,
    model_type: ModelType,
    _ctx: &MacroContext,
) -> TokenStream {
    let field_name = field.ident;
    let base_type = field.base_type;
    let method_name = format_ident!("with_{}", field_name);

    let assignment = match model_type {
        ModelType::Insert => quote! { self.#field_name = value.into(); },
        ModelType::Update => quote! { self.#field_name = Some(value); },
        ModelType::PartialSelect => quote! { self.#field_name = Some(value); },
    };

    // Generate type-specific convenience methods using modern pattern matching
    match model_type {
        ModelType::Insert => {
            // For insert models, accept any type that implements Into<InsertValue<T>>
            // This allows both regular values (String, i32, etc.) and SQL objects to work
            let type_string = base_type.to_token_stream().to_string();
            match (field.is_uuid, type_string.as_str()) {
                (true, _) => quote! {
                    pub fn #method_name<V>(mut self, value: V) -> Self
                    where
                        V: Into<::drizzle_rs::sqlite::InsertValue<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>, ::uuid::Uuid>>
                    {
                        #assignment
                        self
                    }
                },
                (_, s) if s.contains("String") => quote! {
                    pub fn #method_name<V>(mut self, value: V) -> Self
                    where
                        V: Into<::drizzle_rs::sqlite::InsertValue<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>, ::std::string::String>>
                    {
                        #assignment
                        self
                    }
                },
                (_, s) if s.contains("Vec") && s.contains("u8") => quote! {
                    pub fn #method_name<V>(mut self, value: V) -> Self
                    where
                        V: Into<::drizzle_rs::sqlite::InsertValue<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>, ::std::vec::Vec<u8>>>
                    {
                        #assignment
                        self
                    }
                },
                _ => quote! {
                    pub fn #method_name<V>(mut self, value: V) -> Self
                    where
                        V: Into<::drizzle_rs::sqlite::InsertValue<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>, #base_type>>
                    {
                        #assignment
                        self
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

/// Generates constructor parameter and assignment for required fields only
pub(crate) fn generate_required_param_and_assignment(
    field: &FieldInfo,
    _ctx: &MacroContext,
) -> (TokenStream, TokenStream) {
    let field_name = field.ident;
    let base_type = field.base_type;
    let type_string = base_type.to_token_stream().to_string();

    // Required parameters - convert using Into<InsertValue<...>> trait
    match (field.is_uuid, type_string.as_str()) {
        (true, _) => (
            quote! { #field_name: impl Into<::uuid::Uuid> },
            quote! { #field_name: #field_name.into().into() },
        ),
        (false, s) if s.contains("String") => (
            quote! { #field_name: impl Into<::std::string::String> },
            quote! { #field_name: #field_name.into().into() },
        ),
        (false, s) if s.contains("Vec") && s.contains("u8") => (
            quote! { #field_name: impl Into<::std::vec::Vec<u8>> },
            quote! { #field_name: #field_name.into().into() },
        ),
        (false, _) => (
            quote! { #field_name: #base_type },
            quote! { #field_name: #field_name.into() },
        ),
    }
}