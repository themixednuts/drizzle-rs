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

    let mut update_fields = Vec::new();
    let mut update_field_conversions = Vec::new();
    let mut update_convenience_methods = Vec::new();

    for field_info in ctx.field_infos {
        let field_name = &field_info.ident;
        let field_type = get_update_field_type(field_info);

        update_fields.push(quote! {
            pub #field_name: #field_type,
        });

        // Generate field conversion for ToSQL (column_name, value) pairs
        update_field_conversions.push(get_update_field_conversion(field_info));

        // Generate convenience methods
        update_convenience_methods.push(generate_convenience_method(
            field_info,
            ModelType::Update,
            ctx,
        ));
    }

    Ok(quote! {
        #[derive(Debug, Clone, Default)]
        #struct_vis struct #update_ident {
            #(#update_fields)*
        }

        impl #update_ident {
            // Convenience methods for setting fields
            #(#update_convenience_methods)*
        }

        impl<'a> ToSQL<'a, PostgresValue<'a>> for #update_ident {
            fn to_sql(&self) -> SQL<'a, PostgresValue<'a>> {
                let mut assignments = Vec::new();
                #(#update_field_conversions)*
                SQL::assignments(assignments)
            }
        }
    })
}

/// Determine the appropriate field type for UPDATE operations
fn get_update_field_type(field_info: &FieldInfo) -> TokenStream {
    let base_type = field_info.base_type();

    // For UPDATE operations, all fields are optional since you might only want to update some
    // Use the base type (inner type for Option<T>) to avoid Option<Option<T>>
    quote! {
        Option<#base_type>
    }
}

/// Generate field conversion code for UPDATE assignments
/// Pushes (column_name, value) tuples for use with SQL::assignments()
fn get_update_field_conversion(field_info: &FieldInfo) -> TokenStream {
    let name = &field_info.ident;
    // Column name is the same as the field name (converted to string)
    let column_name = name.to_string();

    // Generate conversion based on field type
    quote! {
        if let Some(val) = &self.#name {
            assignments.push((#column_name, val.clone().try_into().unwrap_or(PostgresValue::Null)));
        }
    }
}
