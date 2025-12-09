mod alias;
mod attributes;
mod column_definitions;
mod context;
mod drivers;
mod enum_impls;
mod errors;
mod json;
mod models;
mod sql_generation;
mod traits;
mod validation;

#[cfg(feature = "rusqlite")]
pub mod rusqlite;

#[cfg(feature = "turso")]
pub mod turso;

#[cfg(feature = "libsql")]
pub mod libsql;

use super::field::{FieldInfo, generate_table_meta_json};
use alias::generate_aliased_table;
pub use attributes::TableAttributes;
use column_definitions::{
    generate_column_accessors, generate_column_definitions, generate_column_fields,
};
use context::MacroContext;
use heck::ToSnakeCase;
use json::generate_json_impls;
use models::generate_model_definitions;
use sql_generation::{generate_create_table_sql, generate_create_table_sql_runtime};
use traits::generate_table_impls;
use validation::generate_default_validations;

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Result};

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
            "The #[SQLiteTable] attribute can only be applied to struct definitions.\n",
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

    let (create_table_sql, create_table_sql_runtime) = if has_foreign_keys {
        // For tables with foreign keys, generate runtime SQL generation code
        let runtime_sql = generate_create_table_sql_runtime(
            &table_name,
            &field_infos,
            is_composite_pk,
            attrs.strict,
            attrs.without_rowid,
        );
        // Provide a placeholder static SQL for compile-time usage
        (
            "-- Runtime SQL generation required for foreign keys".to_string(),
            Some(runtime_sql),
        )
    } else {
        // For tables without foreign keys, use static SQL generation
        let static_sql = generate_create_table_sql(
            &table_name,
            &field_infos,
            is_composite_pk,
            attrs.strict,
            attrs.without_rowid,
        );
        (static_sql, None)
    };

    // Calculate required fields pattern for const generic
    let required_fields_pattern: Vec<bool> = field_infos
        .iter()
        .map(|info| {
            let is_optional = info.is_nullable
                || info.has_default
                || info.default_fn.is_some()
                || (info.is_primary
                    && !attrs.without_rowid
                    && !info.is_enum
                    && matches!(info.column_type, crate::sqlite::field::SQLiteType::Integer));
            !is_optional
        })
        .collect();

    // Generate table metadata JSON for drizzle-kit compatible migrations
    let table_meta_json = generate_table_meta_json(&table_name, &field_infos, is_composite_pk);

    // Generate table marker const for IDE hover documentation
    let table_marker_const = generate_table_marker_const(struct_ident, &attrs.marker_exprs);

    let ctx = MacroContext {
        struct_ident,
        struct_vis: &input.vis,
        table_name,
        create_table_sql,
        create_table_sql_runtime,
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        attrs: &attrs,
        has_foreign_keys,
        is_composite_pk,
    };

    // -------------------
    // 2. Generation Phase
    // -------------------
    let (column_definitions, column_zst_idents) = generate_column_definitions(&ctx)?;
    let column_fields = generate_column_fields(&ctx, &column_zst_idents)?;
    let column_accessors = generate_column_accessors(&ctx, &column_zst_idents)?;
    let table_impls = generate_table_impls(&ctx, &column_zst_idents, &required_fields_pattern)?;
    let model_definitions =
        generate_model_definitions(&ctx, &column_zst_idents, &required_fields_pattern)?;
    let json_impls = generate_json_impls(&ctx)?;
    let alias_definitions = generate_aliased_table(&ctx)?;

    #[cfg(feature = "rusqlite")]
    let rusqlite_impls = rusqlite::generate_rusqlite_impls(&ctx)?;

    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impls = quote!();

    #[cfg(feature = "turso")]
    let turso_impls = turso::generate_turso_impls(&ctx)?;

    #[cfg(not(feature = "turso"))]
    let turso_impls = quote!();

    #[cfg(feature = "libsql")]
    let libsql_impls = libsql::generate_libsql_impls(&ctx)?;

    #[cfg(not(feature = "libsql"))]
    let libsql_impls = quote!();

    // Generate compile-time validation for default literals
    let default_validations = generate_default_validations(&field_infos);

    let expanded = quote! {
        // Compile-time validation for default literals
        #default_validations

        // Table marker const for IDE hover documentation
        #table_marker_const

        #[derive(Default, Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
         #struct_vis struct #struct_ident {
         #column_fields
        }

        impl #struct_ident {
            /// Table metadata in drizzle-kit compatible JSON format.
            ///
            /// This constant contains the schema metadata for migrations,
            /// matching the format used by drizzle-kit snapshots.
            pub const __DRIZZLE_TABLE_META: &'static str = #table_meta_json;
        }

        #column_accessors
        #column_definitions
        #table_impls
        #model_definitions
        #json_impls
        #alias_definitions
        #rusqlite_impls
        #turso_impls
        #libsql_impls
    };

    Ok(expanded)
}

/// Generate a const that references the original table marker tokens from the attribute.
///
/// This creates hidden const bindings that use the exact tokens from `#[SQLiteTable(STRICT)]`,
/// enabling rust-analyzer to resolve them and provide hover documentation.
fn generate_table_marker_const(
    struct_ident: &Ident,
    marker_exprs: &[syn::ExprPath],
) -> TokenStream {
    if marker_exprs.is_empty() {
        return TokenStream::new();
    }

    let marker_const_name = format_ident!("_TABLE_ATTR_MARKERS_{}", struct_ident);

    // Generate individual let bindings for each marker since they may be different types
    // (TableMarker for STRICT/WITHOUT_ROWID, NameMarker for NAME)
    quote! {
        /// Hidden const that references the original table attribute markers.
        /// This enables IDE hover documentation for `#[SQLiteTable(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #marker_const_name: () = {
            #( let _ = #marker_exprs; )*
        };
    }
}
