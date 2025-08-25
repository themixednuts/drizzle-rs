use crate::sqlite::field::FieldInfo;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Visibility;

// Enhanced context struct to hold all the necessary information for generation.
// This provides helper methods to reduce code duplication and improve maintainability.
pub(crate) struct MacroContext<'a> {
    pub(crate) struct_ident: &'a Ident,
    pub(crate) struct_vis: &'a Visibility,
    pub(crate) table_name: String,
    pub(crate) create_table_sql: String,
    pub(crate) create_table_sql_runtime: Option<TokenStream>, // For tables with foreign keys
    pub(crate) field_infos: &'a [FieldInfo<'a>],
    pub(crate) select_model_ident: Ident,
    pub(crate) select_model_partial_ident: Ident,
    pub(crate) insert_model_ident: Ident,
    pub(crate) update_model_ident: Ident,
    pub(crate) without_rowid: bool,
    pub(crate) strict: bool,
    pub(crate) has_foreign_keys: bool,
}

#[derive(Debug, Clone, Copy)]
pub(crate) enum ModelType {
    Insert,
    Update,
    PartialSelect,
}

impl<'a> MacroContext<'a> {
    // ============================================================================
    // Core Field Analysis Methods - Single Source of Truth
    // ============================================================================

    /// Determines if a field can auto-increment (INTEGER PRIMARY KEY in regular tables, excluding enums)
    pub(crate) fn can_field_autoincrement(&self, field: &FieldInfo) -> bool {
        if !field.is_primary || self.without_rowid || field.is_enum {
            return false;
        }

        use crate::sqlite::field::SQLiteType;
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

    /// Gets the appropriate field type for a specific model
    pub(crate) fn get_field_type_for_model(
        &self,
        field: &FieldInfo,
        model_type: ModelType,
    ) -> TokenStream {
        let base_type = field.base_type;
        match model_type {
            ModelType::Insert => {
                // For UUID fields, use String for TEXT columns, Uuid for BLOB columns
                if field.is_uuid {
                    let insert_value_type = match field.column_type {
                        crate::sqlite::field::SQLiteType::Text => quote! { ::std::string::String },
                        _ => quote! { ::uuid::Uuid },
                    };
                    quote!(::drizzle::sqlite::values::InsertValue<'a, ::drizzle::sqlite::values::SQLiteValue<'a>, #insert_value_type>)
                } else {
                    // All other insert fields use InsertValue for three-state handling with owned data
                    quote!(::drizzle::sqlite::values::InsertValue<'a, ::drizzle::sqlite::values::SQLiteValue<'a>, #base_type>)
                }
            }
            ModelType::Update => quote!(Option<#base_type>),
            ModelType::PartialSelect => quote!(Option<#base_type>),
        }
    }

    /// Gets the default value expression for insert model
    pub(crate) fn get_insert_default_value(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;

        // Handle runtime function defaults (default_fn)
        if let Some(f) = &field.default_fn {
            return quote! { #name: ((#f)()).into() };
        }

        // Handle compile-time literal defaults (default = "value")
        if let Some(default_lit) = &field.default_value {
            return quote! { #name: (#default_lit).into() };
        }

        // Handle compile-time SQL defaults or any other case
        // Default to Omit so database can handle defaults
        quote! { #name: ::drizzle::sqlite::values::InsertValue::Omit }
    }

    /// Generates field conversion for insert ToSQL
    pub(crate) fn get_insert_field_conversion(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;

        let value_conversion = if field.is_enum {
            quote! { val.clone().into() }
        } else {
            quote! { val.clone().try_into().unwrap_or(::drizzle::sqlite::values::SQLiteValue::Null) }
        };

        // Handle the three states of InsertValue (Omit, Null, Value)
        if field.default_fn.is_some() {
            // For runtime defaults, we always include the field (either default or user value)
            quote! {
                match &self.#name {
                    ::drizzle::sqlite::values::InsertValue::Omit => {
                        // Use runtime default for omitted values
                        let default_val = self.#name.clone(); // This should never be Omit due to default logic
                        #value_conversion
                    },
                    ::drizzle::sqlite::values::InsertValue::Null => ::drizzle::sqlite::values::SQLiteValue::Null,
                    ::drizzle::sqlite::values::InsertValue::Value(wrapper) => {
                        // Values and placeholders are both handled as SQL
                        continue; // Skip in ToSQL, handled in values() method
                    },
                }
            }
        } else {
            // For compile-time defaults or no defaults, we may omit the field
            quote! {
                match &self.#name {
                    ::drizzle::sqlite::values::InsertValue::Omit => {
                        // This field will be omitted from the column list entirely
                        continue;
                    },
                    ::drizzle::sqlite::values::InsertValue::Null => ::drizzle::sqlite::values::SQLiteValue::Null,
                    ::drizzle::sqlite::values::InsertValue::Value(wrapper) => {
                        // Values and placeholders are both handled as SQL
                        continue; // Skip in ToSQL, handled in values() method
                    },
                }
            }
        }
    }

    /// Generates field conversion for update ToSQL
    pub(crate) fn get_update_field_conversion(&self, field: &FieldInfo) -> TokenStream {
        let name = field.ident;
        let column_name = &field.column_name;

        // Handle UUID fields with field-type-aware conversion
        if field.is_uuid {
            use crate::sqlite::field::SQLiteType;
            let uuid_conversion = match field.column_type {
                SQLiteType::Text => {
                    // Store UUID as TEXT (string format)
                    quote! { ::drizzle::sqlite::values::SQLiteValue::Text(::std::borrow::Cow::Owned(val.to_string())) }
                }
                SQLiteType::Blob => {
                    // Store UUID as BLOB (binary format)
                    quote! { ::drizzle::sqlite::values::SQLiteValue::Blob(::std::borrow::Cow::Owned(val.as_bytes().to_vec())) }
                }
                _ => {
                    // Fallback to generic conversion for other types
                    quote! { val.clone().try_into().unwrap_or(::drizzle::sqlite::values::SQLiteValue::Null) }
                }
            };

            return quote! {
                if let Some(val) = &self.#name {
                    assignments.push((#column_name, #uuid_conversion));
                }
            };
        }

        // Default conversion for all other fields (including enums with generated From implementations)
        let conversion = if field.is_enum {
            quote! { val.clone().into() }
        } else {
            quote! { val.clone().try_into().unwrap_or(::drizzle::sqlite::values::SQLiteValue::Null) }
        };

        quote! {
            if let Some(val) = &self.#name {
                assignments.push((#column_name, #conversion));
            }
        }
    }
}
