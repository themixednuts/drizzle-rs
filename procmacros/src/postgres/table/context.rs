use super::attributes::TableAttributes;
use crate::paths::postgres as pg_paths;
use crate::postgres::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Visibility};

// Re-export ModelType from common for convenience
pub(crate) use crate::common::ModelType;

/// Context object containing all the information needed for PostgreSQL table macro generation
pub(crate) struct MacroContext<'a> {
    /// Original struct identifier
    pub struct_ident: &'a Ident,
    /// Struct visibility
    pub struct_vis: &'a Visibility,
    /// Table name (can be customized via attributes)
    pub table_name: String,
    /// Generated CREATE TABLE SQL statement
    pub create_table_sql: String,
    /// Parsed field information
    pub field_infos: &'a [FieldInfo],
    /// Generated SELECT model identifier
    pub select_model_ident: Ident,
    /// Generated partial SELECT model identifier
    pub select_model_partial_ident: Ident,
    /// Generated INSERT model identifier
    pub insert_model_ident: Ident,
    /// Generated UPDATE model identifier
    pub update_model_ident: Ident,
    /// Whether any field has foreign keys
    pub has_foreign_keys: bool,
    /// Whether the table has a composite primary key
    #[allow(dead_code)]
    pub is_composite_pk: bool,
    /// Table attributes
    pub attrs: &'a TableAttributes,
}

impl<'a> MacroContext<'a> {
    /// Determines if a field should be optional in the Insert model.
    /// A field is optional when it is nullable, has a database or runtime default,
    /// or is auto-generated (serial/bigserial).
    pub(crate) fn is_field_optional_in_insert(&self, field: &FieldInfo) -> bool {
        field.is_nullable || field.has_default || field.default_fn.is_some() || field.is_serial
    }

    /// Gets the appropriate field type for a specific model.
    pub(crate) fn get_field_type_for_model(
        &self,
        field: &FieldInfo,
        model_type: ModelType,
    ) -> TokenStream {
        let base_type = &field.base_type;

        match model_type {
            ModelType::Select => {
                let ty = &field.field_type;
                quote!(#ty)
            }
            ModelType::PartialSelect => {
                quote!(::std::option::Option<#base_type>)
            }
            ModelType::Insert => {
                let postgres_insert_value = pg_paths::postgres_insert_value();
                let postgres_value = pg_paths::postgres_value();
                quote!(#postgres_insert_value<'a, #postgres_value<'a>, #base_type>)
            }
            ModelType::Update => {
                let postgres_update_value = pg_paths::postgres_update_value();
                let postgres_value = pg_paths::postgres_value();
                quote!(#postgres_update_value<'a, #postgres_value<'a>, #base_type>)
            }
        }
    }
}
