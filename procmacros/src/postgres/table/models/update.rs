use super::super::context::MacroContext;
use crate::postgres::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Generate UPDATE model struct
pub(crate) fn generate_update_model(ctx: &MacroContext) -> Result<TokenStream> {
    let update_ident = &ctx.update_model_ident;
    let struct_vis = ctx.struct_vis;

    let mut update_fields = Vec::new();

    for field_info in ctx.field_infos {
        let field_name = &field_info.ident;
        let field_type = get_update_field_type(field_info);

        update_fields.push(quote! {
            pub #field_name: #field_type,
        });
    }

    Ok(quote! {
        #[derive(Debug, Clone, Default)]
        #struct_vis struct #update_ident {
            #(#update_fields)*
        }
    })
}

/// Determine the appropriate field type for UPDATE operations
fn get_update_field_type(field_info: &FieldInfo) -> TokenStream {
    let base_type = &field_info.ty;

    // For UPDATE operations, all fields are optional since you might only want to update some
    quote! {
        Option<#base_type>
    }
}
