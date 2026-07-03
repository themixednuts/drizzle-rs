//! Insert model generation.
//!
//! Generates the `InsertModel` struct with type-safe field tracking using marker types.

use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use crate::common::model_markers::{
    generate_empty_pattern_tuple, generate_marker_types, generate_pattern_literal,
};
use crate::postgres::field::{FieldInfo, IdentityMode, TypeCategory};
use proc_macro2::TokenStream;
use quote::quote;

/// Generates the Insert model with convenience methods and constructor
pub fn generate_insert_model(ctx: &MacroContext, required_fields_pattern: &[bool]) -> TokenStream {
    let insert_model = &ctx.insert_model_ident;
    let struct_ident = &ctx.struct_ident;

    // Collect &Ident for the shared marker-helpers (avoids re-importing the
    // dialect-specific FieldInfo type into common/).
    let field_idents: Vec<&syn::Ident> = ctx.field_infos.iter().map(|f| &f.ident).collect();

    // Convert bool slice to tuple literal for required fields pattern
    let required_fields_pattern_literal =
        generate_pattern_literal(ctx.struct_ident, &field_idents, required_fields_pattern);

    // Generate tuple type with NotSet for each field
    let empty_pattern_tuple = generate_empty_pattern_tuple(ctx.struct_ident, &field_idents);

    let mut insert_fields = Vec::new();
    let mut insert_default_fields = Vec::new();
    let mut insert_field_names = Vec::new();
    let mut insert_field_indices = Vec::new();
    let mut insert_convenience_methods = Vec::new();
    let mut required_constructor_params = Vec::new();
    let mut required_constructor_assignments = Vec::new();

    for (field_index, info) in ctx.field_infos.iter().enumerate() {
        let name = &info.ident;
        let field_type = MacroContext::get_field_type_for_model(info, ModelType::Insert);
        let is_optional = MacroContext::is_field_optional_in_insert(info);

        insert_fields.push(quote! { #name: #field_type });
        insert_default_fields.push(get_insert_default_value(info));
        insert_field_names.push(name);
        insert_field_indices.push(quote! { #field_index });
        if should_generate_insert_setter(info) {
            insert_convenience_methods.push(generate_convenience_method(
                info,
                ModelType::Insert,
                ctx,
            ));
        }

        // Generate constructor parameters only for required fields
        if !is_optional {
            let (param, assignment) = generate_constructor_param(info);
            required_constructor_params.push(param);
            required_constructor_assignments.push(assignment);
        }
    }

    // Generate marker types for each field
    let field_marker_types = generate_marker_types(ctx.struct_ident, &field_idents);

    quote! {
        // Generate marker types for each field
        #(#field_marker_types)*

        // Insert Model with PhantomData pattern tracking
        #[derive(Debug, Clone)]
        pub struct #insert_model<'a, T = #empty_pattern_tuple> {
            #(#insert_fields,)*
            _pattern: ::std::marker::PhantomData<T>,
        }

        impl<'a, T> Default for #insert_model<'a, T> {
            fn default() -> Self {
                Self {
                    #(#insert_default_fields,)*
                    _pattern: ::std::marker::PhantomData,
                }
            }
        }

        impl<'a> #insert_model<'a, #empty_pattern_tuple> {
            #[allow(clippy::too_many_arguments)]
            pub fn new(#(#required_constructor_params),*) -> #insert_model<'a, #required_fields_pattern_literal> {
                #insert_model {
                    #(#required_constructor_assignments,)*
                    ..Default::default()
                }
            }
        }

        impl<'a, T> #insert_model<'a, T> {
            /// Converts this insert model to an owned version with 'static lifetime
            pub fn into_owned(self) -> #insert_model<'static, T> {
                #insert_model {
                    #(#insert_field_names: self.#insert_field_names.into_owned(),)*
                    _pattern: ::std::marker::PhantomData,
                }
            }
        }

        // Convenience methods for setting fields
        #(#insert_convenience_methods)*

        impl<'a, T> ToSQL<'a, PostgresValue<'a>> for #insert_model<'a, T> {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                SQLModel::values(self)
            }
        }

        impl<'a, T> SQLModel<'a, PostgresValue<'a>> for #insert_model<'a, T> {
            fn columns(&self) -> ::std::borrow::Cow<'static, [drizzle::core::ColumnRef]> {
                let all_columns = <#struct_ident as drizzle::core::DrizzleTable>::TABLE_REF.columns;
                let mut result_columns = Vec::new();

                #(
                    match &self.#insert_field_names {
                        PostgresInsertValue::Omit => {}
                        _ => {
                            result_columns.push(all_columns[#insert_field_indices]);
                        }
                    }
                )*

                ::std::borrow::Cow::Owned(result_columns)
            }

            fn values(&self) -> SQL<'a, PostgresValue<'a>> {
                let mut sql_parts = Vec::new();

                #(
                    match &self.#insert_field_names {
                        PostgresInsertValue::Omit => {}
                        PostgresInsertValue::Null => {
                            sql_parts.push(SQL::param(PostgresValue::Null));
                        }
                        PostgresInsertValue::Value(wrapper) => {
                            sql_parts.push(wrapper.value.clone());
                        }
                    }
                )*

                SQL::join(sql_parts, Token::COMMA)
            }
        }
    }
}

// =============================================================================
// Helper Functions
// =============================================================================

/// Gets the default value expression for insert model
fn get_insert_default_value(field: &FieldInfo) -> TokenStream {
    let name = &field.ident;

    // Handle runtime function defaults (default_fn)
    if let Some(f) = &field.default_fn {
        return quote! { #name: ((#f)()).into() };
    }

    // Handle compile-time PostgreSQL defaults (SQL defaults - let database handle)
    if field.default.is_some() {
        return quote! { #name: PostgresInsertValue::Omit };
    }

    // Default to Omit so database can handle defaults
    quote! { #name: PostgresInsertValue::Omit }
}

fn should_generate_insert_setter(field: &FieldInfo) -> bool {
    !matches!(field.identity_mode, Some(IdentityMode::Always)) && field.generated_column.is_none()
}

/// Generate constructor parameter and assignment based on field type category.
fn generate_constructor_param(info: &FieldInfo) -> (TokenStream, TokenStream) {
    let field_name = &info.ident;
    let base_type = &info.base_type;
    let category = info.type_category();

    match category {
        TypeCategory::String => (
            quote! { #field_name: impl Into<PostgresInsertValue<'a, PostgresValue<'a>, ::std::string::String>> },
            quote! { #field_name: #field_name.into() },
        ),
        TypeCategory::Blob => (
            quote! { #field_name: impl Into<PostgresInsertValue<'a, PostgresValue<'a>, ::std::vec::Vec<u8>>> },
            quote! { #field_name: #field_name.into() },
        ),
        // ArrayString, ArrayVec, Uuid, Json, Enum, and primitives use base type directly
        // Note: Custom JSON types now have TryInto<PostgresValue> impls generated by json.rs
        _ => (
            quote! { #field_name: impl Into<PostgresInsertValue<'a, PostgresValue<'a>, #base_type>> },
            quote! { #field_name: #field_name.into() },
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::{generate_insert_model, should_generate_insert_setter};
    use crate::postgres::field::{FieldInfo, GeneratedColumn, IdentityMode, PostgreSQLType};
    use crate::postgres::table::{attributes::TableAttributes, context::MacroContext};
    use std::collections::HashSet;

    fn base_field(name: &str) -> FieldInfo {
        let ident: syn::Ident = syn::parse_str(name).expect("valid ident");
        let vis: syn::Visibility = syn::parse_str("pub").expect("valid visibility");
        let field_type: syn::Type = syn::parse_str("i32").expect("valid type");

        FieldInfo {
            ident,
            vis,
            field_type: field_type.clone(),
            base_type: field_type,
            column_name: name.to_string(),
            sql_definition: format!("\"{name}\" INTEGER"),
            column_type: PostgreSQLType::Integer,
            dimensions: None,
            flags: HashSet::new(),
            is_nullable: false,
            is_enum: false,
            is_pgenum: false,
            is_json: false,
            is_jsonb: false,
            is_serial: false,
            is_custom_type: false,
            is_generated_identity: false,
            identity_mode: None,
            generated_column: None,
            default: None,
            default_fn: None,
            check_constraint: None,
            foreign_key: None,
            has_default: false,
            marker_exprs: Vec::new(),
            constraint: crate::common::Constraint::None,
            collate: None,
            comment: None,
        }
    }

    #[test]
    fn generated_and_identity_fields_are_optional_for_insert_constructor() {
        let mut identity_always = base_field("identity_always");
        identity_always.is_generated_identity = true;
        identity_always.identity_mode = Some(IdentityMode::Always);

        let mut identity_by_default = base_field("identity_by_default");
        identity_by_default.is_generated_identity = true;
        identity_by_default.identity_mode = Some(IdentityMode::ByDefault);

        let mut generated = base_field("computed_value");
        generated.generated_column = Some(GeneratedColumn {
            expression: "identity_by_default + 1".to_string(),
            stored: true,
        });

        let required = base_field("required_value");

        assert!(MacroContext::is_field_optional_in_insert(&identity_always));
        assert!(MacroContext::is_field_optional_in_insert(
            &identity_by_default
        ));
        assert!(MacroContext::is_field_optional_in_insert(&generated));
        assert!(!MacroContext::is_field_optional_in_insert(&required));
    }

    #[test]
    fn insert_setters_are_suppressed_only_for_unsettable_generated_fields() {
        let mut identity_always = base_field("identity_always");
        identity_always.is_generated_identity = true;
        identity_always.identity_mode = Some(IdentityMode::Always);

        let mut identity_by_default = base_field("identity_by_default");
        identity_by_default.is_generated_identity = true;
        identity_by_default.identity_mode = Some(IdentityMode::ByDefault);

        let mut generated = base_field("computed_value");
        generated.generated_column = Some(GeneratedColumn {
            expression: "identity_by_default + 1".to_string(),
            stored: true,
        });

        let regular = base_field("regular_value");

        assert!(!should_generate_insert_setter(&identity_always));
        assert!(should_generate_insert_setter(&identity_by_default));
        assert!(!should_generate_insert_setter(&generated));
        assert!(should_generate_insert_setter(&regular));
    }

    #[test]
    fn generated_insert_model_omits_setters_for_unsettable_generated_fields() {
        let struct_ident: syn::Ident = syn::parse_str("GeneratedUsers").expect("valid ident");
        let struct_vis: syn::Visibility = syn::parse_str("pub").expect("valid visibility");

        let mut identity_always = base_field("identity_always");
        identity_always.is_generated_identity = true;
        identity_always.identity_mode = Some(IdentityMode::Always);

        let mut identity_by_default = base_field("identity_by_default");
        identity_by_default.is_generated_identity = true;
        identity_by_default.identity_mode = Some(IdentityMode::ByDefault);

        let mut generated = base_field("computed_value");
        generated.generated_column = Some(GeneratedColumn {
            expression: "identity_by_default + 1".to_string(),
            stored: true,
        });

        let regular = base_field("regular_value");
        let fields = vec![identity_always, identity_by_default, generated, regular];
        let attrs = TableAttributes {
            name: None,
            schema: None,
            unlogged: false,
            temporary: false,
            inherits: None,
            tablespace: None,
            rls: false,
            composite_foreign_keys: Vec::new(),
            unique_constraints: Vec::new(),
            check_constraints: Vec::new(),
            marker_exprs: Vec::new(),
        };

        let ctx = MacroContext {
            struct_ident: &struct_ident,
            struct_vis: &struct_vis,
            table_name: "generated_users".to_string(),
            table_comment: None,
            field_infos: &fields,
            select_model_ident: syn::parse_str("SelectGeneratedUsers").expect("valid ident"),
            select_model_partial_ident: syn::parse_str("SelectGeneratedUsersPartial")
                .expect("valid ident"),
            insert_model_ident: syn::parse_str("InsertGeneratedUsers").expect("valid ident"),
            update_model_ident: syn::parse_str("UpdateGeneratedUsers").expect("valid ident"),
            is_composite_pk: false,
            attrs: &attrs,
        };

        let tokens = generate_insert_model(&ctx, &[false, false, false, true]).to_string();
        assert!(!tokens.contains("with_identity_always"));
        assert!(tokens.contains("with_identity_by_default"));
        assert!(!tokens.contains("with_computed_value"));
        assert!(tokens.contains("with_regular_value"));
        assert!(tokens.contains("regular_value : impl Into"));
        assert!(!tokens.contains("identity_always : impl Into"));
        assert!(!tokens.contains("computed_value : impl Into"));
    }
}
