use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Visibility};

use super::attributes::TableAttributes;
use crate::mysql::field::FieldInfo;

pub use crate::common::ModelType;

pub struct MacroContext<'a> {
    pub struct_ident: &'a Ident,
    pub struct_vis: &'a Visibility,
    pub table_name: String,
    pub table_comment: Option<String>,
    pub field_infos: &'a [FieldInfo],
    pub select_model_ident: Ident,
    pub select_model_partial_ident: Ident,
    pub insert_model_ident: Ident,
    pub update_model_ident: Ident,
    pub attrs: &'a TableAttributes,
}

impl MacroContext<'_> {
    pub(crate) const fn is_field_optional_in_insert(field: &FieldInfo) -> bool {
        field.is_nullable
            || field.has_default
            || field.default_fn.is_some()
            || field.is_auto_increment
            || field.generated_column.is_some()
    }

    pub(crate) fn get_field_type_for_model(
        field: &FieldInfo,
        model_type: ModelType,
    ) -> TokenStream {
        let base_type = &field.base_type;
        match model_type {
            ModelType::Select => {
                let ty = &field.field_type;
                quote!(#ty)
            }
            ModelType::PartialSelect => quote!(::std::option::Option<#base_type>),
            ModelType::Insert => {
                quote!(drizzle::mysql::values::MySQLInsertValue<'a, drizzle::mysql::values::MySQLValue<'a>, #base_type>)
            }
            ModelType::Update => {
                quote!(drizzle::mysql::values::MySQLUpdateValue<'a, drizzle::mysql::values::MySQLValue<'a>, #base_type>)
            }
        }
    }
}
