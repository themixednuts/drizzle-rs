//! Macro context and helper methods for SQLite table generation.
//!
//! Provides the MacroContext struct which holds all information needed
//! for code generation, along with helper methods that serve as the
//! single source of truth for field analysis decisions.

use super::attributes::TableAttributes;
use crate::paths::sqlite as sqlite_paths;
use crate::sqlite::field::{FieldInfo, SQLiteType};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Visibility;

// Re-export ModelType from common for convenience
pub(crate) use crate::common::ModelType;

/// Context struct holding all information needed for macro generation.
///
/// This provides helper methods to reduce code duplication and improve maintainability
/// by centralizing decisions about field handling.
pub(crate) struct MacroContext<'a> {
    pub(crate) struct_ident: &'a Ident,
    pub(crate) struct_vis: &'a Visibility,
    pub(crate) table_name: String,
    pub(crate) create_table_sql: String,
    pub(crate) create_table_sql_runtime: Option<TokenStream>,
    pub(crate) field_infos: &'a [FieldInfo<'a>],
    pub(crate) select_model_ident: Ident,
    pub(crate) select_model_partial_ident: Ident,
    pub(crate) insert_model_ident: Ident,
    pub(crate) update_model_ident: Ident,
    /// Table attributes (strict, without_rowid, etc.)
    pub(crate) attrs: &'a TableAttributes,
    pub(crate) has_foreign_keys: bool,
    pub(crate) is_composite_pk: bool,
}

impl<'a> MacroContext<'a> {
    // =========================================================================
    // Core Field Analysis Methods - Single Source of Truth
    // =========================================================================

    /// Determines if a field can auto-increment (INTEGER PRIMARY KEY in regular tables, excluding enums)
    pub(crate) fn can_field_autoincrement(&self, field: &FieldInfo) -> bool {
        if !field.is_primary || self.attrs.without_rowid || field.is_enum {
            return false;
        }
        matches!(field.column_type, SQLiteType::Integer)
    }

    /// Determines if a field should be optional in the Insert model
    pub(crate) fn is_field_optional_in_insert(&self, field: &FieldInfo) -> bool {
        // Nullable fields are always optional
        if field.is_nullable {
            return true;
        }

        // Fields with explicit defaults (SQL or runtime) are optional
        if field.has_default || field.default_fn.is_some() {
            return true;
        }

        // Primary key fields that can auto-increment are optional
        self.can_field_autoincrement(field)
    }

    // =========================================================================
    // Type Generation Methods - Using TypeCategory for consistency
    // =========================================================================

    /// Gets the appropriate field type for a specific model.
    ///
    /// Uses TypeCategory for consistent type handling across the codebase.
    pub(crate) fn get_field_type_for_model(
        &self,
        field: &FieldInfo,
        model_type: ModelType,
    ) -> TokenStream {
        let base_type = field.base_type;
        let sqlite_value = sqlite_paths::sqlite_value();
        let sqlite_insert_value = sqlite_paths::sqlite_insert_value();

        match model_type {
            ModelType::Select => {
                // Select model uses the original field type
                let ty = field.field_type;
                quote!(#ty)
            }
            ModelType::Insert => {
                // Use TypeCategory to determine the insert value type
                let insert_value_inner = field.insert_value_inner_type();
                quote!(#sqlite_insert_value<'a, #sqlite_value<'a>, #insert_value_inner>)
            }
            ModelType::Update => {
                let sqlite_update_value = sqlite_paths::sqlite_update_value();
                quote!(#sqlite_update_value<'a, #sqlite_value<'a>, #base_type>)
            }
            ModelType::PartialSelect => {
                quote!(::std::option::Option<#base_type>)
            }
        }
    }

    /// Gets the default value expression for insert model
    pub(crate) fn get_insert_default_value(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;
        let sqlite_insert_value = sqlite_paths::sqlite_insert_value();

        // Handle runtime function defaults (default_fn)
        if let Some(f) = &field.default_fn {
            return quote! { #name: ((#f)()).into() };
        }

        // Handle compile-time literal defaults (default = "value")
        if let Some(default_lit) = &field.default_value {
            return quote! { #name: (#default_lit).into() };
        }

        // Default to Omit so database can handle defaults
        quote! { #name: #sqlite_insert_value::Omit }
    }

    // =========================================================================
    // Field Conversion Methods
    // =========================================================================

    /// Generates field conversion for update ToSQL.
    ///
    /// Matches on `SQLiteUpdateValue` variants to produce `(column_name, SQL)` pairs
    /// for use with `SQL::assignments_sql()`.
    pub(crate) fn get_update_field_conversion(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;
        let column_name = &field.column_name;
        let sqlite_value = sqlite_paths::sqlite_value();
        let sqlite_update_value = sqlite_paths::sqlite_update_value();

        quote! {
            match &self.#name {
                #sqlite_update_value::Skip => {},
                #sqlite_update_value::Null => {
                    assignments.push((#column_name, SQL::param(#sqlite_value::Null)));
                },
                #sqlite_update_value::Value(wrapper) => {
                    assignments.push((#column_name, wrapper.value.clone()));
                },
            }
        }
    }
}
