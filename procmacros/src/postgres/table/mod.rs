pub(crate) mod alias;
pub(crate) mod attributes;
pub(crate) mod column_definitions;
pub(crate) mod context;
mod ddl;
pub(crate) mod drivers;
mod errors;
mod json;
pub(crate) mod models;
pub(crate) mod traits;

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
use ddl::{generate_const_ddl, generate_create_table_sql_from_params};
use models::generate_model_definitions;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;
use syn::{DeriveInput, Result};
use traits::generate_table_impls;

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

    let fields = struct_fields(&input, "PostgresTable")?;

    let primary_key_count = count_primary_keys(fields, |field| {
        Ok(FieldInfo::from_field(field, false)?.is_primary)
    })?;
    let is_composite_pk = primary_key_count > 1;

    let field_infos = fields
        .iter()
        .map(|field| FieldInfo::from_field(field, is_composite_pk))
        .collect::<Result<Vec<_>>>()?;

    // Generate table metadata JSON for drizzle-kit compatible migrations
    let table_meta_json = generate_table_meta_json(&table_name, &field_infos, is_composite_pk);

    // Calculate has_foreign_keys before creating context
    let has_foreign_keys = field_infos.iter().any(|f| f.foreign_key.is_some())
        || !attrs.composite_foreign_keys.is_empty();

    // Generate CREATE TABLE SQL (only for tables without foreign keys)
    let schema_name = attrs.schema.as_deref().unwrap_or("public");

    let create_table_sql = if has_foreign_keys {
        String::new()
    } else {
        generate_create_table_sql_from_params(
            schema_name,
            &table_name,
            &field_infos,
            is_composite_pk,
        )
    };

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
        is_composite_pk,
        attrs: &attrs,
    };

    // Calculate required fields pattern for const generic
    let required_fields_pattern =
        required_fields_pattern(&field_infos, |info| ctx.is_field_optional_in_insert(info));

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

    // Generate TryFrom implementations for all enabled PostgreSQL drivers
    let driver_impls = drivers::generate_all_driver_impls(&ctx)?;

    // Generate TryInto<PostgresValue> implementations for custom JSON types
    let json_impls = json::generate_json_impls(&ctx)?;

    // Generate table marker const for IDE hover documentation
    let table_marker_const = generate_table_marker_const(struct_ident, &attrs.marker_exprs);

    // Generate const DDL entities
    let const_ddl = generate_const_ddl(&ctx, &column_zst_idents)?;

    // Generate query API code (relation ZSTs, accessors, FromJsonValue)
    #[cfg(feature = "query")]
    let query_api_impls = generate_query_api_impls(&ctx)?;
    #[cfg(not(feature = "query"))]
    let query_api_impls = quote!();

    // Get the table name from the context for use in generated code
    let table_name = &ctx.table_name;

    // -------------------
    // 3. Assembly Phase
    // -------------------
    let expanded = quote! {
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
        }

        #column_accessors
        #column_definitions
        #table_impls
        #model_definitions
        #alias_definitions
        #driver_impls
        #json_impls
        #const_ddl
        #query_api_impls
    };

    Ok(expanded)
}

/// Generate query API impls (RelationDef, accessors, FromJsonValue) for PostgreSQL.
///
/// Shared by both `#[PostgresTable]` and `#[PostgresView]`.
#[cfg(feature = "query")]
pub(crate) fn generate_query_api_impls(ctx: &MacroContext) -> Result<TokenStream> {
    use crate::common::query::{EnumStorage, FieldJsonInfo, FkInfo, generate_query_api};
    use crate::common::type_is_uuid;
    use crate::postgres::field::PostgreSQLType;

    let struct_ident = ctx.struct_ident;
    let select_model_ident = &ctx.select_model_ident;
    let partial_select_model_ident = &ctx.select_model_partial_ident;
    let table_name = &ctx.table_name;

    // Collect FK infos
    let fk_infos: Vec<FkInfo> = ctx
        .field_infos
        .iter()
        .filter_map(|f| {
            let fk = f.foreign_key.as_ref()?;
            Some(FkInfo {
                source_column: f.column_name.clone(),
                target_table_ident: fk.table.clone(),
                target_column_ident: fk.column.clone(),
                is_nullable: f.is_nullable,
            })
        })
        .collect();

    // Collect field info for FromJsonValue generation
    let field_json_infos: Vec<FieldJsonInfo> = ctx
        .field_infos
        .iter()
        .map(|f| {
            let enum_storage = if f.is_pgenum {
                // Native PostgreSQL enums are always text-based
                Some(EnumStorage::Text)
            } else if f.is_enum {
                match f.column_type {
                    PostgreSQLType::Integer
                    | PostgreSQLType::Bigint
                    | PostgreSQLType::Smallint
                    | PostgreSQLType::Serial
                    | PostgreSQLType::Smallserial
                    | PostgreSQLType::Bigserial => Some(EnumStorage::Integer),
                    _ => Some(EnumStorage::Text),
                }
            } else {
                None
            };
            FieldJsonInfo {
                ident: f.ident.clone(),
                column_name: f.column_name.clone(),
                is_nullable: f.is_nullable,
                is_uuid: type_is_uuid(&f.base_type),
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
/// This creates hidden const bindings that use the exact tokens from `#[PostgresTable(UNLOGGED)]`,
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
    // (TableMarker for UNLOGGED/TEMPORARY, NameMarker for NAME)
    quote! {
        /// Hidden const that references the original table attribute markers.
        /// This enables IDE hover documentation for `#[PostgresTable(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #marker_const_name: () = {
            #( let _ = #marker_exprs; )*
        };
    }
}
