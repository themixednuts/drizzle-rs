use super::super::context::{MacroContext, ModelType};
use super::convenience::generate_convenience_method;
use crate::paths::{core as core_paths, sqlite as sqlite_paths};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

/// Generates the Update model with convenience methods
pub(crate) fn generate_update_model(ctx: &MacroContext) -> Result<TokenStream> {
    let update_model = &ctx.update_model_ident;
    let empty_marker = core_paths::empty_marker();
    let non_empty_marker = core_paths::non_empty_marker();

    // Get paths for fully-qualified types
    let sql = core_paths::sql();
    let to_sql = core_paths::to_sql();
    let sqlite_value = sqlite_paths::sqlite_value();
    let sqlite_update_value = sqlite_paths::sqlite_update_value();

    let mut field_names: Vec<&syn::Ident> = Vec::new();
    let mut field_base_types: Vec<&syn::Type> = Vec::new();
    let mut update_field_conversions = Vec::new();
    let mut update_convenience_methods = Vec::new();

    for info in ctx.field_infos {
        field_names.push(info.ident);
        field_base_types.push(info.base_type);

        // Generate field conversion for ToSQL
        update_field_conversions.push(ctx.get_update_field_conversion(info));

        // Generate convenience methods (each as standalone impl<'a, S> block)
        update_convenience_methods.push(generate_convenience_method(info, ModelType::Update, ctx));
    }

    // Clone field_names for repeated use in quote repetitions
    let field_names2 = field_names.clone();

    Ok(quote! {
        // Update Model — all 'a tokens generated within this single quote! block
        // S = Empty means no fields set yet; S = NonEmpty means at least one field was set.
        #[derive(Debug, Clone)]
        pub struct #update_model<'a, S = #empty_marker> {
            #(pub(crate) #field_names: #sqlite_update_value<'a, #sqlite_value<'a>, #field_base_types>,)*
            pub(crate) _state: ::std::marker::PhantomData<S>,
        }

        impl<'a> ::std::default::Default for #update_model<'a> {
            fn default() -> Self {
                Self {
                    #(#field_names2: #sqlite_update_value::Skip,)*
                    _state: ::std::marker::PhantomData,
                }
            }
        }

        // Convenience methods — each in its own impl<'a, S> block
        #(#update_convenience_methods)*

        // ToSQL is only implemented for NonEmpty state
        impl<'a> #to_sql<'a, #sqlite_value<'a>> for #update_model<'a, #non_empty_marker> {
            fn to_sql(&self) -> #sql<'a, #sqlite_value<'a>> {
                let mut assignments = ::std::vec::Vec::new();
                #(#update_field_conversions)*
                #sql::assignments_sql(assignments)
            }
        }
    })
}
