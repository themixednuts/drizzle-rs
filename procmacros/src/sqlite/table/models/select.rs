use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use proc_macro2::TokenStream;
use quote::quote;

/// Generates the Select model and its partial variant
pub fn generate_select_model(ctx: &MacroContext) -> TokenStream {
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
    let mut select_field_names = Vec::new();
    let mut select_types = Vec::new();
    let mut partial_types = Vec::new();
    let mut tuple_indices = Vec::new();
    let mut partial_convenience_methods = Vec::new();

    for (i, info) in field_infos.iter().enumerate() {
        let name = info.ident;
        let select_type = MacroContext::get_field_type_for_model(info, ModelType::Select);
        let partial_type = MacroContext::get_field_type_for_model(info, ModelType::PartialSelect);

        select_fields.push(quote! { pub #name: #select_type });
        partial_select_fields.push(quote! { pub #name: #partial_type });
        partial_types.push(partial_type);
        select_types.push(select_type);
        tuple_indices.push(syn::Index::from(i));
        select_field_names.push(name);

        // Generate convenience methods for partial select
        partial_convenience_methods.push(generate_convenience_method(
            info,
            ModelType::PartialSelect,
            ctx,
        ));
    }
    // Debug/Clone/PartialEq/Default exist when every field type has them;
    // nothing is required of user types (JSON payloads, custom columns).
    let select_std_impls = crate::common::generators::model_std_impls(
        select_model_ident,
        &select_field_names
            .iter()
            .copied()
            .zip(select_types.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let partial_std_impls = crate::common::generators::model_std_impls(
        select_model_partial_ident,
        &select_field_names
            .iter()
            .copied()
            .zip(partial_types.iter().cloned())
            .collect::<Vec<_>>(),
    );
    let partial_impl = quote! {
            // Partial Select Model - all fields are optional for selective querying
            #struct_vis struct #select_model_partial_ident { #(#partial_select_fields,)* }
            #partial_std_impls

            impl #select_model_partial_ident {
                // Convenience methods for setting fields
                #(#partial_convenience_methods)*
            }
    };

    quote! {
        // Select Model
        #struct_vis struct #select_model_ident { #(#select_fields,)* }
        #select_std_impls

        impl From<(#(#select_types,)*)> for #select_model_ident {
            fn from(tuple: (#(#select_types,)*)) -> Self {
                Self {
                    #(#select_field_names: tuple.#tuple_indices,)*
                }
            }
        }

        #partial_impl
    }
}
