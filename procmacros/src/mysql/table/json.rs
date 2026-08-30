//! Type-owned codecs for custom MySQL JSON fields.

use super::context::MacroContext;
use crate::common::{type_is_json_value, type_is_string_like};
use drizzle_types::mysql::MySQLType;
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use std::collections::BTreeMap;
use syn::Result;

pub fn generate_json_impls(ctx: &MacroContext) -> Result<TokenStream> {
    let fields = ctx.field_infos.iter().filter(|field| {
        field.is_custom_type
            && matches!(field.column_type, MySQLType::Json)
            && !type_is_json_value(&field.base_type)
            && !type_is_string_like(&field.base_type)
    });
    let mut types = BTreeMap::new();
    for field in fields {
        types
            .entry(field.base_type.to_token_stream().to_string())
            .or_insert(field);
    }
    let Some(first) = types.values().next() else {
        return Ok(TokenStream::new());
    };
    if !cfg!(feature = "serde") {
        return Err(syn::Error::new_spanned(
            &first.ident,
            "the `serde` feature is required for custom MySQL JSON fields",
        ));
    }

    let impls = types.values().map(|field| {
        let ty = &field.base_type;
        quote! {
            impl drizzle::mysql::traits::DrizzleMySQLColumn for #ty {
                type SQLType = drizzle::mysql::types::Json;
                const SQL_TYPE: &'static str = "JSON";

                fn decode(
                    value: drizzle::mysql::values::MySQLValue<'_>,
                ) -> ::std::result::Result<Self, drizzle::error::DrizzleError> {
                    match value {
                        drizzle::mysql::values::MySQLValue::Bytes(value) => {
                            ::serde_json::from_slice(value.as_ref()).map_err(Into::into)
                        }
                        _ => Err(drizzle::error::DrizzleError::ConversionError(
                            "expected MySQL JSON bytes".into(),
                        )),
                    }
                }

                fn encode(&self) -> drizzle::mysql::values::MySQLValue<'_> {
                    let value = ::serde_json::to_vec(self)
                        .expect("failed to serialize custom MySQL JSON value");
                    drizzle::mysql::values::MySQLValue::Bytes(
                        ::std::borrow::Cow::Owned(value),
                    )
                }
            }
        }
    });

    Ok(quote!(#(#impls)*))
}
