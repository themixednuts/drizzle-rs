use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Visibility};

use super::attributes::{CompositeForeignKeyAttr, TableAttributes};
use crate::common::rust_type_to_nullability;
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
    /// SQL names of a table-level foreign key's source columns.
    pub(crate) fn composite_foreign_key_columns(&self, foreign_key: &CompositeForeignKeyAttr) -> Vec<String> {
        foreign_key
            .source_columns
            .iter()
            .map(|source| {
                self.field_infos
                    .iter()
                    .find(|field| &field.ident == source)
                    .map_or_else(|| source.to_string(), |field| field.column_name.clone())
            })
            .collect()
    }

    /// Default name of the column-level foreign key on `field`; the schema
    /// parser derives the same name.
    pub(crate) fn column_foreign_key_name(&self, field: &FieldInfo) -> String {
        drizzle_types::mysql::names::foreign_key_name(&self.table_name, &[field.column_name.as_str()])
    }

    /// Default name of the `index`-th table-level foreign key: named after
    /// its first column unless another foreign key on this table would get
    /// the same name, then after all of its columns.
    pub(crate) fn composite_foreign_key_name(&self, index: usize) -> String {
        let columns = self.composite_foreign_key_columns(&self.attrs.composite_foreign_keys[index]);
        let first = columns.first();
        let collides = self
            .field_infos
            .iter()
            .any(|field| field.foreign_key.is_some() && Some(&field.column_name) == first)
            || self
                .attrs
                .composite_foreign_keys
                .iter()
                .enumerate()
                .any(|(other, foreign_key)| {
                    other != index && self.composite_foreign_key_columns(foreign_key).first() == first
                });
        let columns: Vec<&str> = columns.iter().map(String::as_str).collect();
        drizzle_types::mysql::names::foreign_key_name(
            &self.table_name,
            drizzle_types::mysql::names::composite_foreign_key_name_columns(&columns, collides),
        )
    }

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
                let sql_type = field.sql_type_marker();
                let nullable = rust_type_to_nullability(&field.field_type);
                quote!(drizzle::mysql::values::MySQLUpdateValue<'a, drizzle::mysql::values::MySQLValue<'a>, #base_type, #sql_type, #nullable>)
            }
        }
    }
}
