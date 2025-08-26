use super::super::context::{MacroContext, ModelType};
use crate::postgres::field::FieldInfo;
use proc_macro2::TokenStream;
use quote::{ToTokens, format_ident, quote};

/// Generates a convenience method for a field based on its type
pub(crate) fn generate_convenience_method(
    field: &FieldInfo,
    model_type: ModelType,
    _ctx: &MacroContext,
) -> TokenStream {
    let field_name = &field.ident;
    let base_type = &field.ty;
    let method_name = format_ident!("with_{}", field_name);

    let assignment = match model_type {
        ModelType::Insert => quote! { self.#field_name = value.into(); },
        ModelType::Update => quote! { self.#field_name = Some(value); },
        ModelType::PartialSelect => quote! { self.#field_name = Some(value); },
        ModelType::Select => quote! { self.#field_name = value; },
    };

    // Generate convenience methods - PostgreSQL has simpler type handling than SQLite
    match model_type {
        ModelType::Insert => {
            // For insert models, accept any type that implements Into<InsertValue<T>>
            quote! {
                pub fn #method_name<V>(mut self, value: V) -> Self
                where
                    V: Into<::drizzle::postgres::values::InsertValue<'a, ::drizzle::postgres::values::PostgresValue<'a>, #base_type>>
                {
                    #assignment
                    self
                }
            }
        }
        _ => {
            // For other models (Update, Select, PartialSelect), use direct assignment
            quote! {
                pub fn #method_name(mut self, value: #base_type) -> Self {
                    #assignment
                    self
                }
            }
        }
    }
}
