use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Generates the Update model with convenience methods
pub(crate) fn generate_update_model(ctx: &MacroContext) -> Result<TokenStream> {
    let update_model = &ctx.update_model_ident;

    let mut update_fields = Vec::new();
    let mut update_field_conversions = Vec::new();
    let mut update_column_names = Vec::new();
    let mut update_field_names = Vec::new();
    let mut update_convenience_methods = Vec::new();

    for info in ctx.field_infos {
        let name = info.ident;
        let update_type = info.get_update_type();
        let column_name = &info.column_name;

        // Generate field definition
        update_fields.push(quote! { pub #name: #update_type });

        // Generate field conversion for ToSQL
        update_column_names.push(quote! { #column_name });
        update_field_names.push(name);
        update_field_conversions.push(ctx.get_update_field_conversion(info));

        // Generate convenience methods
        update_convenience_methods.push(generate_convenience_method(
            info,
            ModelType::Update,
            ctx,
        ));
    }

    Ok(quote! {
        // Update Model
        #[derive(Debug, Clone, PartialEq, Default)]
        pub struct #update_model {
            #(#update_fields,)*
        }

        impl #update_model {
            // Convenience methods for setting fields
            #(#update_convenience_methods)*
        }

        impl<'a> ::drizzle_rs::core::ToSQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> for #update_model {
            fn to_sql(&self) -> ::drizzle_rs::core::SQL<'a, ::drizzle_rs::sqlite::SQLiteValue<'a>> {
                let mut assignments = Vec::new();
                #(#update_field_conversions)*
                ::drizzle_rs::core::SQL::assignments(assignments)
            }
        }
    })
}