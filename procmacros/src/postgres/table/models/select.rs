use super::super::context::MacroContext;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Generate SELECT model struct
pub(crate) fn generate_select_model(ctx: &MacroContext) -> Result<TokenStream> {
    let select_ident = &ctx.select_model_ident;
    let partial_select_ident = &ctx.select_model_partial_ident;
    let struct_vis = ctx.struct_vis;

    let mut select_fields = Vec::new();
    let mut partial_select_fields = Vec::new();

    for field_info in ctx.field_infos {
        let field_name = &field_info.ident;
        let field_type = &field_info.field_type;

        // For SELECT models, all fields are Option<T> to handle partial selects
        select_fields.push(quote! {
            pub #field_name: #field_type,
        });

        partial_select_fields.push(quote! {
            pub #field_name: Option<#field_type>,
        });
    }

    Ok(quote! {
        #[derive(Debug, Clone, Default)]
        #struct_vis struct #select_ident {
            #(#select_fields)*
        }

        #[derive(Debug, Clone, Default)]
        #struct_vis struct #partial_select_ident {
            #(#partial_select_fields)*
        }
    })
}
