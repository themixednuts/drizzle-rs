use super::attributes::TableAttributes;
use crate::postgres::field::FieldInfo;
use syn::{Ident, Visibility};

/// Represents different model types for code generation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelType {
    Select,
    PartialSelect,
    Insert,
    Update,
}

/// Context object containing all the information needed for PostgreSQL table macro generation
pub(super) struct MacroContext<'a> {
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
    /// Table attributes
    pub attrs: &'a TableAttributes,
}
