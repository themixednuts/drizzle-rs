use super::attributes::{CompositeForeignKeyAttr, TableAttributes};
use crate::common::rust_type_to_nullability;
use crate::paths::postgres as pg_paths;
use crate::postgres::field::{FieldInfo, PostgreSQLReference};
use proc_macro2::TokenStream;
use quote::quote;
use syn::{Ident, Visibility};

// Re-export ModelType from common for convenience
pub use crate::common::ModelType;

/// Context object containing all the information needed for `PostgreSQL` table macro generation
pub struct MacroContext<'a> {
    /// Original struct identifier
    pub struct_ident: &'a Ident,
    /// Struct visibility
    pub struct_vis: &'a Visibility,
    /// Table name (can be customized via attributes)
    pub table_name: String,
    /// SQL table comment extracted from doc comments.
    pub table_comment: Option<String>,
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
    /// Whether the table has a composite primary key
    #[allow(dead_code)]
    pub is_composite_pk: bool,
    /// Table attributes
    pub attrs: &'a TableAttributes,
}

impl MacroContext<'_> {
    /// SQL names of a table-level foreign key's source columns.
    pub(crate) fn composite_foreign_key_columns(
        &self,
        foreign_key: &CompositeForeignKeyAttr,
    ) -> Vec<String> {
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

    /// Name of `field`'s foreign key `reference`: its explicit name, else
    /// `{table}_{column}_fkey`.
    pub(crate) fn column_foreign_key_name(
        &self,
        field: &FieldInfo,
        reference: &PostgreSQLReference,
    ) -> String {
        reference.name.clone().unwrap_or_else(|| {
            drizzle_types::postgres::names::foreign_key_name(
                &self.table_name,
                &[field.column_name.as_str()],
            )
        })
    }

    /// Name of the `index`-th table-level foreign key: its explicit name,
    /// else named after its first column unless another foreign key on this
    /// table starts with the same column, then after all of its columns.
    pub(crate) fn composite_foreign_key_name(&self, index: usize) -> String {
        let foreign_key = &self.attrs.composite_foreign_keys[index];
        if let Some(name) = &foreign_key.name {
            return name.clone();
        }
        let columns = self.composite_foreign_key_columns(foreign_key);
        let first = columns.first();
        let collides =
            self.field_infos
                .iter()
                .any(|field| field.foreign_key.is_some() && Some(&field.column_name) == first)
                || self.attrs.composite_foreign_keys.iter().enumerate().any(
                    |(other, foreign_key)| {
                        other != index
                            && self.composite_foreign_key_columns(foreign_key).first() == first
                    },
                );
        let columns: Vec<&str> = columns.iter().map(String::as_str).collect();
        drizzle_types::postgres::names::foreign_key_name(
            &self.table_name,
            drizzle_types::postgres::names::composite_foreign_key_name_columns(&columns, collides),
        )
    }

    /// Determines if a field should be optional in the Insert model.
    /// A field is optional when it is nullable, has a database or runtime default,
    /// or is auto-generated (serial/bigserial, identity, generated column).
    pub(crate) const fn is_field_optional_in_insert(field: &FieldInfo) -> bool {
        field.is_nullable
            || field.has_default
            || field.default_fn.is_some()
            || field.is_serial
            || field.is_generated_identity
            || field.generated_column.is_some()
    }

    /// Gets the appropriate field type for a specific model.
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
                let sql_type = field.sql_type_marker();
                let nullable = rust_type_to_nullability(&field.field_type);
                quote!(#postgres_update_value<'a, #postgres_value<'a>, #base_type, #sql_type, #nullable>)
            }
        }
    }
}
