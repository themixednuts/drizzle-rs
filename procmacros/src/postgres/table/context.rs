use super::attributes::TableAttributes;
use crate::postgres::field::FieldInfo;
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
}
