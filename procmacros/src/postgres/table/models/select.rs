use super::super::context::{MacroContext, ModelType};
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
    let mut select_field_names = Vec::new();
    let mut select_types = Vec::new();
    let mut tuple_indices = Vec::new();

    for (i, field_info) in ctx.field_infos.iter().enumerate() {
        let field_name = &field_info.ident;
        let select_type = ctx.get_field_type_for_model(field_info, ModelType::Select);
        let partial_type = ctx.get_field_type_for_model(field_info, ModelType::PartialSelect);

        select_fields.push(quote! {
            pub #field_name: #select_type,
        });

        partial_select_fields.push(quote! {
            pub #field_name: #partial_type,
        });

        select_field_names.push(field_name);
        select_types.push(select_type);
        tuple_indices.push(syn::Index::from(i));
    }

    Ok(quote! {
        #[derive(Debug, Clone, Default)]
        #struct_vis struct #select_ident {
            #(#select_fields)*
        }

        impl From<(#(#select_types,)*)> for #select_ident {
            fn from(tuple: (#(#select_types,)*)) -> Self {
                Self {
                    #(#select_field_names: tuple.#tuple_indices,)*
                }
            }
        }

        #[derive(Debug, Clone, Default)]
        #struct_vis struct #partial_select_ident {
            #(#partial_select_fields)*
        }
    })
}
