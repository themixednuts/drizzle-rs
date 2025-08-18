use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Generates the Select model and its partial variant
pub(crate) fn generate_select_model(ctx: &MacroContext) -> Result<TokenStream> {
    let MacroContext {
        select_model_ident,
        select_model_partial_ident,
        struct_vis,
        field_infos,
        ..
    } = ctx;

    let mut select_fields = Vec::new();
    let mut partial_select_fields = Vec::new();
    let mut select_column_names = Vec::new();
    let mut select_field_names = Vec::new();
    let mut partial_convenience_methods = Vec::new();

    for info in *field_infos {
        let name = info.ident;
        let select_type = info.get_select_type();
        let base_type = info.base_type;
        let column_name = &info.column_name;

        select_fields.push(quote! { pub #name: #select_type });
        partial_select_fields.push(quote! { pub #name: Option<#base_type> });
        select_column_names.push(quote! { #column_name });
        select_field_names.push(name);

        // Generate convenience methods for partial select
        partial_convenience_methods.push(generate_convenience_method(
            info,
            ModelType::PartialSelect,
            ctx,
        ));
    }

    // Partial Select Model - feature gated for drivers that support column-based access
    #[cfg(not(any(feature = "libsql", feature = "turso")))]
    let partial_impl = quote! {
            // Partial Select Model - all fields are optional for selective querying
            #[derive(Debug, Clone, PartialEq, Default)]
            #struct_vis struct #select_model_partial_ident { #(#partial_select_fields,)* }

            impl #select_model_partial_ident {
                // Convenience methods for setting fields
                #(#partial_convenience_methods)*
            }

            // Implement SQLPartial trait for SelectModel
            impl<'a> ::drizzle_rs::core::SQLPartial<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model_ident {
                type Partial = #select_model_partial_ident;
            }

            impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model_partial_ident {
                fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                    unimplemented!()
                    // Only include columns that are Some() for selective querying
                    // let mut selected_columns = Vec::new();
                    // #(
                    //     if self.#select_field_names.is_some() {
                    //         selected_columns.push(#select_column_names);
                    //     }
                    // )*

                    // if selected_columns.is_empty() {
                    //         unimplemented!()
                    //         // If no fields selected, default to all columns
                    //         // const ALL_COLUMNS: &'static [&'static str] = &[#(#select_column_names,)*];
                    //         // ::drizzle_rs::core::SQL::join(ALL_COLUMNS, ", ")
                    // } else {
                    //     ::drizzle_rs::core::SQL::join(&selected_columns, ", ")
                    // }
                }
            }

    };
    #[cfg(any(feature = "libsql", feature = "turso"))]
    let partial_impl = quote! {};

    Ok(quote! {
        // Select Model
        #[derive(Debug, Clone, PartialEq, Default)]
        #struct_vis struct #select_model_ident { #(#select_fields,)* }

        // For libsql and turso: partial select models are disabled due to index-based access limitations
        // Use full select model or specific column tuples instead
        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #select_model_ident {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                unimplemented!()
                // Generate column list for SELECT
                // const COLUMN_NAMES: &'static [&'static str] = &[#(#select_column_names,)*];
                // ::drizzle_rs::core::SQL::join(COLUMN_NAMES, ", ")
            }
        }

        #partial_impl
    })
}