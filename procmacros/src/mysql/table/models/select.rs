use super::super::context::{MacroContext, ModelType};
use crate::paths::core as core_paths;
use proc_macro2::TokenStream;
use quote::quote;

/// Generate SELECT model struct
pub fn generate_select_model(ctx: &MacroContext) -> TokenStream {
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
        let select_type = MacroContext::get_field_type_for_model(field_info, ModelType::Select);
        let partial_type =
            MacroContext::get_field_type_for_model(field_info, ModelType::PartialSelect);

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
    let select_model_derive = quote! { #[derive(Debug, Clone)] };
    let partial_select_model_derive = quote! { #[derive(Debug, Clone, Default)] };
    let field_count = select_types.len();
    let row_field_inits = select_field_names
        .iter()
        .zip(select_types.iter())
        .enumerate()
        .map(|(index, (field_name, field_type))| {
            quote! {
                #field_name: <#field_type as drizzle::core::FromDrizzleRow<
                    drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>
                >>::from_row_at(row, offset + #index)?,
            }
        })
        .collect::<Vec<_>>();
    let type_set_cons = core_paths::type_set_cons();
    let type_set_nil = core_paths::type_set_nil();
    let column_list = select_types.iter().rev().fold(
        quote!(#type_set_nil),
        |tail, ty| quote!(#type_set_cons<#ty, #tail>),
    );

    quote! {
        #select_model_derive
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

        impl<'__drizzle_row, __DrizzleRow: drizzle::mysql::driver::MySQLRowAccess + ?Sized>
            drizzle::core::FromDrizzleRow<
                drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>
            > for #select_ident
        {
            const COLUMN_COUNT: usize = #field_count;

            fn from_row_at(
                row: &drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>,
                offset: usize,
            ) -> ::std::result::Result<Self, drizzle::error::DrizzleError> {
                ::std::result::Result::Ok(Self {
                    #(#row_field_inits)*
                })
            }
        }

        impl<'__drizzle_row, __DrizzleRow: drizzle::mysql::driver::MySQLRowAccess + ?Sized>
            drizzle::core::NullProbeRow<
                drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>
            > for #select_ident
        {
            fn is_null_at(
                row: &drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>,
                offset: usize,
            ) -> ::std::result::Result<bool, drizzle::error::DrizzleError> {
                row.is_null_at(offset)
            }
        }

        impl<'__drizzle_row, __DrizzleRow: drizzle::mysql::driver::MySQLRowAccess + ?Sized>
            drizzle::core::RowColumnList<
                drizzle::mysql::driver::MySQLRow<'__drizzle_row, __DrizzleRow>
            > for #select_ident
        {
            type Columns = #column_list;
        }

        #partial_select_model_derive
        #struct_vis struct #partial_select_ident {
            #(#partial_select_fields)*
        }
    }
}
