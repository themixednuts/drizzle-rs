pub(crate) mod convenience;
pub(crate) mod insert;
pub(crate) mod select;
pub(crate) mod update;

use super::context::MacroContext;
use proc_macro2::{Ident, TokenStream};
use quote::quote;
use syn::Result;

// Re-export convenience functions for internal use only
pub(crate) use insert::generate_insert_model;
pub(crate) use select::generate_select_model;
pub(crate) use update::generate_update_model;

/// Generates the `Select`, `Insert`, `Update` model structs and their impls.
pub(crate) fn generate_model_definitions(
    ctx: &MacroContext,
    column_zst_idents: &[Ident],
    required_fields_pattern: &[bool],
) -> Result<TokenStream> {
    let select_model = generate_select_model(ctx)?;
    let insert_model = generate_insert_model(ctx, required_fields_pattern)?;
    let update_model = generate_update_model(ctx)?;
    let model_impls = generate_model_trait_impls(ctx, column_zst_idents)?;

    Ok(quote! {
        #select_model
        #insert_model
        #update_model
        #model_impls
    })
}

/// Generates SQLModel trait implementations for all model types
fn generate_model_trait_impls(
    ctx: &MacroContext,
    _column_zst_idents: &[Ident],
) -> Result<TokenStream> {
    #[allow(unused_variables)]
    let (select_model, select_model_partial, update_model) = (
        &ctx.select_model_ident,
        &ctx.select_model_partial_ident,
        &ctx.update_model_ident,
    );

    let struct_ident = &ctx.struct_ident;

    // Collect field names for update model
    let mut update_field_names = Vec::new();

    for info in ctx.field_infos.iter() {
        let name = info.ident;
        update_field_names.push(name);
    }

    let partial_impl = quote! {
        impl<'a> SQLModel<'a, SQLiteValue<'a>> for #select_model_partial {
            type Columns = &'static [&'static dyn SQLColumnInfo];

            fn columns(&self) -> Self::Columns {
                // For partial select model, return all columns (same as other models)
                static INSTANCE: #struct_ident = #struct_ident::new();
                <#struct_ident as SQLTableInfo>::columns(&INSTANCE)
            }

            fn values(&self) -> SQL<'a, SQLiteValue<'a>> {
                SQL::empty()
            }
        }

        impl<'a> ToSQL<'a, SQLiteValue<'a>> for #select_model_partial {
            fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
                SQLModel::values(self)
            }
        }
    };

    Ok(quote! {
        // SQLModel implementations
        impl<'a> SQLModel<'a, SQLiteValue<'a>> for #select_model {
            type Columns = &'static [&'static dyn SQLColumnInfo];

            fn columns(&self) -> Self::Columns {
                // For select model, return all columns
                static INSTANCE: #struct_ident = #struct_ident::new();
                <#struct_ident as SQLTableInfo>::columns(&INSTANCE)
            }

            fn values(&self) -> SQL<'a, SQLiteValue<'a>> {
                SQL::empty()
            }
        }

        impl<'a> ToSQL<'a, SQLiteValue<'a>> for #select_model {
            fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
                SQLModel::values(self)
            }
        }

        impl<'a> SQLPartial<'a, SQLiteValue<'a>> for #select_model {
            type Partial = #select_model_partial;
        }

        impl<'a> SQLModel<'a, SQLiteValue<'a>> for #update_model {
            type Columns = &'static [&'static dyn SQLColumnInfo];

            fn columns(&self) -> Self::Columns {
                // For update model, return all columns (same as other models)
                static INSTANCE: #struct_ident = #struct_ident::new();
                <#struct_ident as SQLTableInfo>::columns(&INSTANCE)
            }

            fn values(&self) -> SQL<'a, SQLiteValue<'a>> {
                let mut values = Vec::new();
                // For Update model, only include values that are Some()
                #(
                    if let Some(val) = &self.#update_field_names {
                        values.push(val.clone().try_into().unwrap_or(SQLiteValue::Null));
                    }
                )*
                SQL::param_list(values)
            }
        }

        // ToSQL impl for Update model is generated in update.rs using SQL::assignments()

        #partial_impl

    })
}
