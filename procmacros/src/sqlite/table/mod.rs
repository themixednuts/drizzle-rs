pub(crate) mod alias;
pub(crate) mod attributes;
pub(crate) mod column_definitions;
pub(crate) mod context;
mod ddl;
#[cfg(feature = "turso")]
mod drivers;
mod enum_impls;
mod errors;
mod json;
pub(crate) mod models;
pub(crate) mod traits;
mod validation;

#[cfg(feature = "rusqlite")]
pub mod rusqlite;

#[cfg(feature = "turso")]
pub mod turso;

#[cfg(feature = "libsql")]
pub mod libsql;

use super::field::{FieldInfo, generate_table_meta_json};
use crate::common::{
    count_primary_keys, required_fields_pattern, struct_fields, table_name_from_attrs,
};
use alias::generate_aliased_table;
pub use attributes::TableAttributes;
use column_definitions::{
    generate_column_accessors, generate_column_definitions, generate_column_fields,
};
use context::MacroContext;
use ddl::generate_const_ddl;
use json::generate_json_impls;
use models::generate_model_definitions;
use traits::generate_table_impls;
use validation::{generate_default_validations, validate_strict_affinity};

use proc_macro2::{Ident, TokenStream};
use quote::{format_ident, quote};
use syn::{DeriveInput, Result};

// ============================================================================
// Main Macro Entry Point
// ============================================================================

pub fn table_attr_macro(input: DeriveInput, attrs: TableAttributes) -> Result<TokenStream> {
    // -------------------
    // 1. Setup Phase
    // -------------------
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;
    let table_name = table_name_from_attrs(struct_ident, attrs.name.clone());

    let fields = struct_fields(&input, "SQLiteTable")?;

    let primary_key_count = count_primary_keys(fields, |field| {
        Ok(FieldInfo::from_field(field, false)?.is_primary)
    })?;
    let is_composite_pk = primary_key_count > 1;

    let field_infos = fields
        .iter()
        .map(|field| FieldInfo::from_field(field, is_composite_pk))
        .collect::<Result<Vec<_>>>()?;

    validate_strict_affinity(&field_infos, attrs.strict)?;

    // Calculate required fields pattern for const generic
    let required_fields_pattern = required_fields_pattern(&field_infos, |info| {
        info.is_nullable
            || info.has_default
            || info.default_fn.is_some()
            || (info.is_primary
                && !attrs.without_rowid
                && !info.is_enum
                && matches!(info.column_type, crate::sqlite::field::SQLiteType::Integer))
    });

    // Generate table metadata JSON for drizzle-kit compatible migrations
    let table_meta_json = generate_table_meta_json(&table_name, &field_infos, is_composite_pk);

    // Generate table marker const for IDE hover documentation
    let table_marker_const = generate_table_marker_const(struct_ident, &attrs.marker_exprs);

    // Calculate has_foreign_keys before creating context
    let has_foreign_keys = field_infos.iter().any(|f| f.foreign_key.is_some())
        || !attrs.composite_foreign_keys.is_empty();

    // Generate CREATE TABLE SQL (only for tables without foreign keys)
    let create_table_sql = if has_foreign_keys {
        String::new()
    } else {
        ddl::generate_create_table_sql_from_params(
            &table_name,
            &field_infos,
            is_composite_pk,
            attrs.strict,
            attrs.without_rowid,
        )
    };

    let ctx = MacroContext {
        struct_ident,
        struct_vis: &input.vis,
        table_name,
        create_table_sql,
        create_table_sql_runtime: None,
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

    // Generate query API code (relation ZSTs, accessors, FromJsonValue)
    #[cfg(feature = "query")]
    let query_api_impls = generate_query_api_impls(&ctx)?;
    #[cfg(not(feature = "query"))]
    let query_api_impls = quote!();

    // Generate compile-time validation for default literals
    let default_validations = generate_default_validations(&field_infos);

    // Generate const DDL definitions
    let const_ddl = generate_const_ddl(&ctx)?;

    let table_name = &ctx.table_name;
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
            /// The table name as used in SQL statements.
            /// This respects the `name = "..."` attribute if specified,
            /// otherwise uses the snake_case version of the struct name.
            pub const TABLE_NAME: &'static str = #table_name;

            /// Table metadata in drizzle-kit compatible JSON format.
            ///
            /// This constant contains the schema metadata for migrations,
            /// matching the format used by drizzle-kit snapshots.
            pub const __DRIZZLE_TABLE_META: &'static str = #table_meta_json;

            #const_ddl
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
        #query_api_impls
    };

    Ok(expanded)
}

/// Generate query API impls (RelationDef, accessors, FromJsonValue) for SQLite.
///
/// Shared by both `#[SQLiteTable]` and `#[SQLiteView]`.
#[cfg(feature = "query")]
pub(crate) fn generate_query_api_impls(ctx: &MacroContext) -> Result<TokenStream> {
    use crate::common::query::{EnumStorage, FieldJsonInfo, FkInfo, generate_query_api};
    use crate::sqlite::field::SQLiteType;

    let struct_ident = ctx.struct_ident;
    let select_model_ident = &ctx.select_model_ident;
    let table_name = &ctx.table_name;

    // Collect FK infos
    let fk_infos: Vec<FkInfo> = ctx
        .field_infos
        .iter()
        .filter_map(|f| {
            let fk = f.foreign_key.as_ref()?;
            Some(FkInfo {
                source_column: f.column_name.clone(),
                target_table_ident: fk.table_ident.clone(),
                target_column_ident: fk.column_ident.clone(),
                is_nullable: f.is_nullable,
            })
        })
        .collect();

    let partial_select_model_ident = &ctx.select_model_partial_ident;

    // Collect field info for FromJsonValue generation
    let field_json_infos: Vec<FieldJsonInfo> = ctx
        .field_infos
        .iter()
        .map(|f| {
            let enum_storage = if f.is_enum {
                match f.column_type {
                    SQLiteType::Integer => Some(EnumStorage::Integer),
                    _ => Some(EnumStorage::Text),
                }
            } else {
                None
            };
            FieldJsonInfo {
                ident: f.ident.clone(),
                column_name: f.column_name.clone(),
                is_nullable: f.is_nullable,
                is_uuid: f.is_uuid,
                enum_storage,
                base_type: f.base_type.clone(),
            }
        })
        .collect();

    // Collect column names
    let column_names: Vec<String> = ctx
        .field_infos
        .iter()
        .map(|f| f.column_name.clone())
        .collect();

    let inner = generate_query_api(
        struct_ident,
        ctx.struct_vis,
        table_name,
        select_model_ident,
        partial_select_model_ident,
        &fk_infos,
        &field_json_infos,
        &column_names,
    )?;

    Ok(inner)
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

    let marker_const_name = format_ident!(
        "_TABLE_ATTR_MARKERS_{}",
        struct_ident.to_string().to_ascii_uppercase()
    );

    // Generate individual let bindings for each marker since they may be different types
    // (TableMarker for STRICT/WITHOUT_ROWID, NameMarker for NAME)
    quote! {
        /// Hidden const that references the original table attribute markers.
        /// This enables IDE hover documentation for `#[SQLiteTable(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code)]
        const #marker_const_name: () = {
            #( let _ = #marker_exprs; )*
        };
    }
}
