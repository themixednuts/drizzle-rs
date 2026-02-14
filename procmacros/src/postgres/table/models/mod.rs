pub(crate) mod convenience;
pub(crate) mod insert;
pub(crate) mod select;
pub(crate) mod update;

use super::context::MacroContext;
use crate::paths::{core as core_paths, postgres as postgres_paths};
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Result;

// Re-export convenience functions for internal use only
pub(crate) use insert::generate_insert_model;
pub(crate) use select::generate_select_model;
pub(crate) use update::generate_update_model;

/// Generates the `Select`, `Insert`, `Update` model structs and their impls.
pub(crate) fn generate_model_definitions(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
    required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let select_model = generate_select_model(ctx)?;
    let insert_model = generate_insert_model(ctx, required_fields_pattern)?;
    let update_model = generate_update_model(ctx)?;
    let model_impls = generate_model_trait_impls(ctx, column_zst_idents)?;

    Ok(quote! {
        #select_model
        #insert_model
        #update_model
        #model_impls
    })
}

/// Generates SQLModel trait implementations for all model types
fn generate_model_trait_impls(
    ctx: &MacroContext,
    _column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    #[allow(unused_variables)]
    let (select_model, select_model_partial, insert_model, update_model) = (
        &ctx.select_model_ident,
        &ctx.select_model_partial_ident,
        &ctx.insert_model_ident,
        &ctx.update_model_ident,
    );

    let struct_ident = &ctx.struct_ident;

    // Get paths for generated code
    let sql = core_paths::sql();
    let sql_model = core_paths::sql_model();
    let sql_partial = core_paths::sql_partial();
    let sql_column_info = core_paths::sql_column_info();
    let sql_table_info = core_paths::sql_table_info();
    let to_sql = core_paths::to_sql();
    let postgres_value = postgres_paths::postgres_value();
    let non_empty_marker = core_paths::non_empty_marker();

    let partial_impl = quote! {
        impl<'a> #sql_model<'a, #postgres_value<'a>> for #select_model_partial {
            fn columns(&self) -> ::std::borrow::Cow<'static, [&'static dyn #sql_column_info]> {
                // For partial select model, return all columns (same as other models)
                static INSTANCE: #struct_ident = #struct_ident::new();
                ::std::borrow::Cow::Borrowed(<#struct_ident as #sql_table_info>::columns(&INSTANCE))
            }

            fn values(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql::empty()
            }
        }

        impl<'a> #to_sql<'a, #postgres_value<'a>> for #select_model_partial {
            fn to_sql(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql_model::values(self)
            }
        }
    };

    Ok(quote! {
        // SQLModel implementations
        impl<'a> #sql_model<'a, #postgres_value<'a>> for #select_model {
            fn columns(&self) -> ::std::borrow::Cow<'static, [&'static dyn #sql_column_info]> {
                // For select model, return all columns
                static INSTANCE: #struct_ident = #struct_ident::new();
                ::std::borrow::Cow::Borrowed(<#struct_ident as #sql_table_info>::columns(&INSTANCE))
            }

            fn values(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql::empty()
            }
        }

        impl<'a> #to_sql<'a, #postgres_value<'a>> for #select_model {
            fn to_sql(&self) -> #sql<'a, #postgres_value<'a>> {
                #sql_model::values(self)
            }
        }

        impl<'a> #sql_partial<'a, #postgres_value<'a>> for #select_model {
            type Partial = #select_model_partial;
        }

        impl<'a> #sql_model<'a, #postgres_value<'a>> for #update_model<'a, #non_empty_marker> {
            fn columns(&self) -> ::std::borrow::Cow<'static, [&'static dyn #sql_column_info]> {
                // For update model, return all columns (same as other models)
                static INSTANCE: #struct_ident = #struct_ident::new();
                ::std::borrow::Cow::Borrowed(<#struct_ident as #sql_table_info>::columns(&INSTANCE))
            }

            fn values(&self) -> #sql<'a, #postgres_value<'a>> {
                // Update model uses assignments_sql() via ToSQL; values() returns empty
                #sql::empty()
            }
        }

        // ToSQL impl for Update model is generated in update.rs using SQL::assignments_sql()

        #partial_impl

    })
}
