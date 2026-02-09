use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Generates the Select model and its partial variant
pub(crate) fn generate_select_model(ctx: &MacroContext) -> Result<TokenStream> {
    #[allow(unused_variables)]
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
        let select_type = ctx.get_field_type_for_model(info, ModelType::Select);
        let partial_type = ctx.get_field_type_for_model(info, ModelType::PartialSelect);
        let column_name = &info.column_name;

        select_fields.push(quote! { pub #name: #select_type });
        partial_select_fields.push(quote! { pub #name: #partial_type });
        select_column_names.push(quote! { #column_name });
        select_field_names.push(name);

        // Generate convenience methods for partial select
        partial_convenience_methods.push(generate_convenience_method(
            info,
            ModelType::PartialSelect,
            ctx,
        ));
    }
    let partial_impl = quote! {
            // Partial Select Model - all fields are optional for selective querying
            #[derive(Debug, Clone, PartialEq, Default)]
            #struct_vis struct #select_model_partial_ident { #(#partial_select_fields,)* }

            impl #select_model_partial_ident {
                // Convenience methods for setting fields
                #(#partial_convenience_methods)*
            }
    };

    Ok(quote! {
        // Select Model
        #[derive(Debug, Clone, PartialEq, Default)]
        #struct_vis struct #select_model_ident { #(#select_fields,)* }

        #partial_impl
    })
}
