mod alias;
mod attributes;
mod column_definitions;
mod context;
mod models;
mod sql_generation;
mod traits;
mod validation;

// #[cfg(feature = "sqlx-postgres")]
// mod sqlx;

use super::field::FieldInfo;
use alias::generate_aliased_table;
pub use attributes::TableAttributes;
use column_definitions::{
    generate_column_accessors, generate_column_definitions, generate_column_fields,
};
use context::MacroContext;
use heck::ToSnakeCase;
use models::generate_model_definitions;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use sql_generation::generate_create_table_sql;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Result};
use traits::generate_table_impls;
// use validation::generate_default_validations;

// ============================================================================
// Main Macro Entry Point
// ============================================================================

pub fn table_attr_macro(input: DeriveInput, attrs: TableAttributes) -> Result<TokenStream> {
    // -------------------
    // 1. Setup Phase
    // -------------------
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;
    let table_name = attrs
        .name
        .clone()
        .unwrap_or_else(|| struct_ident.to_string().to_snake_case());

    let fields = if let Data::Struct(data) = &input.data {
        &data.fields
    } else {
        return Err(syn::Error::new(
            input.span(),
            "The #[PostgresTable] attribute can only be applied to struct definitions.\n",
        ));
    };

    let primary_key_count = fields
        .iter()
        .filter(|f| FieldInfo::from_field(f, false).is_ok_and(|f| f.is_primary))
        .count();
    let is_composite_pk = primary_key_count > 1;

    let field_infos = fields
        .iter()
        .map(|field| FieldInfo::from_field(field, is_composite_pk))
        .collect::<Result<Vec<_>>>()?;

    // Check if any field has foreign keys
    let has_foreign_keys = field_infos.iter().any(|info| info.foreign_key.is_some());

    let create_table_sql =
        generate_create_table_sql(&table_name, &field_infos, is_composite_pk, &attrs);

    let ctx = MacroContext {
        struct_ident,
        struct_vis: &input.vis,
        table_name,
        create_table_sql,
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        has_foreign_keys,
        attrs: &attrs,
    };

    // Calculate required fields pattern for const generic
    let required_fields_pattern: Vec<bool> = field_infos
        .iter()
        .map(|info| !ctx.is_field_optional_in_insert(info))
        .collect();

    // -------------------
    // 2. Generation Phase
    // -------------------
    let (column_definitions, column_zst_idents) = generate_column_definitions(&ctx)?;
    let column_fields = generate_column_fields(&ctx, &column_zst_idents)?;
    let column_accessors = generate_column_accessors(&ctx, &column_zst_idents)?;
    let table_impls = generate_table_impls(&ctx, &column_zst_idents, &required_fields_pattern)?;
    let model_definitions =
        generate_model_definitions(&ctx, &column_zst_idents, &required_fields_pattern)?;
    let alias_definitions = generate_aliased_table(&ctx)?;

    // Generate fields for new() method
    let new_method_fields = field_infos
        .iter()
        .zip(&column_zst_idents)
        .map(|(info, zst_ident)| {
            let field_name = &info.ident;
            quote! { #field_name: #zst_ident }
        });
    // Generate compile-time validation for default literals
    // let default_validations = generate_default_validations(&field_infos);

    // #[cfg(feature = "sqlx-postgres")]
    // let sqlx_impls = sqlx::generate_sqlx_impls(&ctx)?;

    // #[cfg(not(feature = "sqlx-postgres"))]
    // let sqlx_impls = quote!();

    // -------------------
    // 3. Assembly Phase
    // -------------------
    let expanded = quote! {
        // Compile-time validation for default literals
        // #default_validations

        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
         #struct_vis struct #struct_ident {
         #column_fields
        }

        #column_accessors
        #column_definitions
        #table_impls
        #model_definitions
        #alias_definitions
        // #json_impls

        // Database-specific implementations
        // #sqlx_impls
    };

    Ok(expanded)
}
