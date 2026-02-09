use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use crate::postgres::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Generate UPDATE model struct with proper ToSQL impl
pub(crate) fn generate_update_model(ctx: &MacroContext) -> Result<TokenStream> {
    let update_ident = &ctx.update_model_ident;
    let struct_vis = ctx.struct_vis;

    let mut field_names: Vec<&syn::Ident> = Vec::new();
    let mut field_types: Vec<TokenStream> = Vec::new();
    let mut update_field_conversions = Vec::new();
    let mut update_convenience_methods = Vec::new();

    for field_info in ctx.field_infos {
        field_names.push(&field_info.ident);
        field_types.push(ctx.get_field_type_for_model(field_info, ModelType::Update));

        // Generate field conversion for ToSQL (column_name, SQL) pairs
        update_field_conversions.push(get_update_field_conversion(field_info));

        // Generate convenience methods (each as standalone impl<'a> block)
        update_convenience_methods.push(generate_convenience_method(
            field_info,
            ModelType::Update,
            ctx,
        ));
    }

    // Clone field_names for repeated use in quote repetitions
    let field_names2 = field_names.clone();

    Ok(quote! {
        // Update Model — all 'a tokens generated within this single quote! block
        #[derive(Debug, Clone)]
        #struct_vis struct #update_ident<'a> {
            #(pub #field_names: #field_types,)*
        }

        impl<'a> ::std::default::Default for #update_ident<'a> {
            fn default() -> Self {
                Self {
                    #(#field_names2: PostgresUpdateValue::Skip,)*
                }
            }
        }

        // Convenience methods — each in its own impl<'a> block
        #(#update_convenience_methods)*

        impl<'a> ToSQL<'a, PostgresValue<'a>> for #update_ident<'a> {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                let mut assignments = Vec::new();
                #(#update_field_conversions)*
                SQL::assignments_sql(assignments)
            }
        }
    })
}

/// Generate field conversion code for UPDATE assignments
/// Matches on `PostgresUpdateValue` variants to produce `(column_name, SQL)` pairs
fn get_update_field_conversion(field_info: &FieldInfo) -> TokenStream {
    let name = &field_info.ident;
    let column_name = &field_info.column_name;

    quote! {
        match &self.#name {
            PostgresUpdateValue::Skip => {},
            PostgresUpdateValue::Null => {
                assignments.push((#column_name, SQL::param(PostgresValue::Null)));
            },
            PostgresUpdateValue::Value(wrapper) => {
                assignments.push((#column_name, wrapper.value.clone()));
            },
        }
    }
}
