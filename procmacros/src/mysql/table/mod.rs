pub mod alias;
pub mod attributes;
pub mod column_definitions;
pub mod context;
mod ddl;
pub mod models;
pub mod traits;

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
use models::generate_model_definitions;
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::Ident;
use syn::{DeriveInput, Expr, Lit, Result};
use traits::generate_table_impls;

// ============================================================================
// Main Macro Entry Point
// ============================================================================

pub fn table_attr_macro(input: &DeriveInput, attrs: &TableAttributes) -> Result<TokenStream> {
    // -------------------
    // 1. Setup Phase
    // -------------------
    let struct_ident = &input.ident;
    let struct_vis = &input.vis;
    let table_name = table_name_from_attrs(struct_ident, attrs.name.clone());

    let fields = struct_fields(input, "MySQLTable")?;
    let table_comment = doc_comment_from_attrs(&input.attrs);

    let primary_key_count = count_primary_keys(fields, |field| {
        Ok(FieldInfo::from_field(field, false)?.is_primary())
    })?;
    let is_composite_pk = primary_key_count > 1;

    let field_infos = fields
        .iter()
        .map(|field| FieldInfo::from_field(field, is_composite_pk))
        .collect::<Result<Vec<_>>>()?;

    // Generate table metadata JSON for drizzle-kit compatible migrations
    let table_meta_json = generate_table_meta_json(&table_name, &field_infos);

    if field_infos
        .iter()
        .filter(|field| field.is_auto_increment)
        .count()
        > 1
    {
        return Err(syn::Error::new_spanned(
            input,
            "a MySQL table may contain at most one AUTO_INCREMENT column",
        ));
    }
    let has_foreign_keys = field_infos.iter().any(|field| field.foreign_key.is_some())
        || !attrs.composite_foreign_keys.is_empty();
    if attrs.temporary && has_foreign_keys {
        return Err(syn::Error::new_spanned(
            input,
            "MySQL temporary tables cannot declare foreign keys",
        ));
    }
    for field in &field_infos {
        if let Some(reference) = &field.foreign_key
            && (reference.on_delete.as_deref() == Some("SET NULL")
                || reference.on_update.as_deref() == Some("SET NULL"))
            && !field.is_nullable
        {
            return Err(syn::Error::new_spanned(
                &field.ident,
                "SET_NULL requires a nullable MySQL foreign-key column",
            ));
        }
    }
    for foreign_key in &attrs.composite_foreign_keys {
        if foreign_key.on_delete.as_deref() == Some("SET NULL")
            || foreign_key.on_update.as_deref() == Some("SET NULL")
        {
            for source in &foreign_key.source_columns {
                let Some(field) = field_infos.iter().find(|field| field.ident == *source) else {
                    continue;
                };
                if !field.is_nullable {
                    return Err(syn::Error::new_spanned(
                        source,
                        "SET_NULL requires every MySQL foreign-key source column to be nullable",
                    ));
                }
            }
        }
    }
    for field in &field_infos {
        if (field.is_primary() || field.is_unique()) && !field.is_indexable_without_prefix() {
            return Err(syn::Error::new_spanned(
                &field.ident,
                field.direct_index_error(),
            ));
        }
    }
    for unique in &attrs.unique_constraints {
        for column in &unique.columns {
            let Some(field) = field_infos.iter().find(|field| field.ident == *column) else {
                return Err(syn::Error::new_spanned(
                    column,
                    format!("UNIQUE references unknown field `{column}`"),
                ));
            };
            if !field.is_indexable_without_prefix() {
                return Err(syn::Error::new_spanned(column, field.direct_index_error()));
            }
        }
    }

    let ctx = MacroContext {
        struct_ident,
        struct_vis: &input.vis,
        table_name,
        table_comment,
        field_infos: &field_infos,
        select_model_ident: format_ident!("Select{}", struct_ident),
        select_model_partial_ident: format_ident!("PartialSelect{}", struct_ident),
        insert_model_ident: format_ident!("Insert{}", struct_ident),
        update_model_ident: format_ident!("Update{}", struct_ident),
        attrs,
    };

    // Calculate required fields pattern for const generic
    let required_fields_pattern = required_fields_pattern(&field_infos, |info| {
        MacroContext::is_field_optional_in_insert(info)
    });

    // -------------------
    // 2. Generation Phase
    // -------------------
    let (column_definitions, column_zst_idents) = generate_column_definitions(&ctx)?;
    let column_fields = generate_column_fields(&ctx, &column_zst_idents);
    let column_accessors = generate_column_accessors(&ctx, &column_zst_idents);

    let table_impls = generate_table_impls(&ctx, &column_zst_idents, &required_fields_pattern)?;
    let model_definitions =
        generate_model_definitions(&ctx, &column_zst_idents, &required_fields_pattern);
    let alias_definitions = generate_aliased_table(&ctx);

    // Generate table marker const for IDE hover documentation
    let table_marker_const = generate_table_marker_const(struct_ident, &attrs.marker_exprs);

    // Generate const DDL entities
    let const_ddl = generate_const_ddl(&ctx, &column_zst_idents);

    #[cfg(feature = "query")]
    let query_api_impls = generate_query_api_impls(&ctx);
    #[cfg(not(feature = "query"))]
    let query_api_impls = TokenStream::new();

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

        impl<'a> ::core::default::Default for &'a #struct_ident {
            fn default() -> Self {
                static TABLE: #struct_ident = #struct_ident::new();
                &TABLE
            }
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
        #const_ddl
        #query_api_impls
    };

    Ok(expanded)
}

/// Generate relational-query metadata, accessors, and JSON decoders for MySQL.
#[cfg(feature = "query")]
pub(super) fn generate_query_api_impls(ctx: &MacroContext) -> TokenStream {
    use crate::common::query::{
        EnumStorage, FieldJsonInfo, FieldProjectionKind, FieldStorageKind, FkInfo,
        generate_query_api,
    };
    use crate::common::{
        type_is_array_char, type_is_array_u8, type_is_arrayvec_u8, type_is_bool, type_is_uuid,
        type_is_vec_u8,
    };

    let fk_infos = ctx
        .field_infos
        .iter()
        .filter_map(|field| {
            let reference = field.foreign_key.as_ref()?;
            Some(FkInfo {
                source_column: field.column_name.clone(),
                target_table_ident: reference.table.clone(),
                target_column_ident: reference.column.clone(),
                is_nullable: field.is_nullable,
                relation_name: field.relation_name.clone(),
            })
        })
        .collect::<Vec<_>>();

    let field_json_infos = ctx
        .field_infos
        .iter()
        .map(|field| {
            let is_bit = matches!(&field.column_type, drizzle_types::mysql::MySQLType::Bit);
            let is_binary_rust_type = type_is_vec_u8(&field.base_type)
                || type_is_array_u8(&field.base_type)
                || type_is_arrayvec_u8(&field.base_type);
            let is_binary = type_is_uuid(&field.base_type)
                || (is_bit && is_binary_rust_type)
                || matches!(
                    &field.column_type,
                    drizzle_types::mysql::MySQLType::Binary
                        | drizzle_types::mysql::MySQLType::Varbinary
                        | drizzle_types::mysql::MySQLType::Tinyblob
                        | drizzle_types::mysql::MySQLType::Blob
                        | drizzle_types::mysql::MySQLType::Mediumblob
                        | drizzle_types::mysql::MySQLType::Longblob
                );
            let is_text_cast = matches!(
                &field.column_type,
                drizzle_types::mysql::MySQLType::Decimal
                    | drizzle_types::mysql::MySQLType::Date
                    | drizzle_types::mysql::MySQLType::Time
                    | drizzle_types::mysql::MySQLType::Datetime
                    | drizzle_types::mysql::MySQLType::Timestamp
            );
            let storage = if is_binary {
                FieldStorageKind::MySQLBlob
            } else if is_text_cast {
                FieldStorageKind::MySQLText
            } else if type_is_bool(&field.base_type) {
                FieldStorageKind::Bool
            } else if type_is_array_char(&field.base_type) {
                FieldStorageKind::MySQLText
            } else {
                FieldStorageKind::Plain
            };
            let projection = if is_binary {
                FieldProjectionKind::TaggedHex
            } else if is_text_cast {
                FieldProjectionKind::Text
            } else if is_bit {
                FieldProjectionKind::Unsigned
            } else {
                FieldProjectionKind::Native
            };
            FieldJsonInfo {
                ident: field.ident.clone(),
                column_name: field.column_name.clone(),
                is_nullable: field.is_nullable,
                is_json: matches!(&field.column_type, drizzle_types::mysql::MySQLType::Json),
                storage,
                projection,
                enum_storage: field.is_enum.then_some(EnumStorage::Text),
                base_type: field.base_type.clone(),
                select_type: MacroContext::get_field_type_for_model(
                    field,
                    crate::common::ModelType::Select,
                ),
                partial_select_type: MacroContext::get_field_type_for_model(
                    field,
                    crate::common::ModelType::PartialSelect,
                ),
            }
        })
        .collect::<Vec<_>>();

    let column_names = ctx
        .field_infos
        .iter()
        .map(|field| field.column_name.clone())
        .collect::<Vec<_>>();

    generate_query_api(
        ctx.struct_ident,
        ctx.struct_vis,
        ctx.attrs.database.as_deref(),
        &ctx.table_name,
        &ctx.select_model_ident,
        &ctx.select_model_partial_ident,
        &fk_infos,
        &field_json_infos,
        &column_names,
    )
}

fn doc_comment_from_attrs(attrs: &[syn::Attribute]) -> Option<String> {
    let lines = attrs.iter().filter_map(|attr| {
        if !attr.path().is_ident("doc") {
            return None;
        }
        let syn::Meta::NameValue(meta) = &attr.meta else {
            return None;
        };
        let Expr::Lit(expr_lit) = &meta.value else {
            return None;
        };
        let Lit::Str(lit) = &expr_lit.lit else {
            return None;
        };
        let value = lit.value();
        Some(value.strip_prefix(' ').unwrap_or(&value).to_string())
    });
    let comment = lines.collect::<Vec<_>>().join("\n");
    if comment.is_empty() {
        None
    } else {
        Some(comment)
    }
}

/// Generate a const that references the original table marker tokens from the attribute.
///
/// This creates hidden const bindings that use the exact tokens from `#[MySQLTable(UNLOGGED)]`,
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
        /// This enables IDE hover documentation for `#[MySQLTable(...)]` attributes.
        #[doc(hidden)]
        #[allow(dead_code, non_upper_case_globals)]
        const #marker_const_name: () = {
            #( let _ = #marker_exprs; )*
        };
    }
}
