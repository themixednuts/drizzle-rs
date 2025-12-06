//! Shared enum implementation generation for SQLite drivers.
//!
//! This module centralizes the generation of enum-related implementations
//! (From<Enum> for SQLiteValue, ToSQL, driver-specific traits) to avoid
//! code duplication across column_definitions.rs and driver modules.

use crate::sqlite::field::{FieldInfo, SQLiteType};
use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;

use super::errors;

/// Generate all enum implementations for a field.
///
/// This includes:
/// - `From<EnumType>` for `SQLiteValue<'a>` (owned)
/// - `From<&EnumType>` for `SQLiteValue<'a>` (reference)
/// - `ToSQL` implementation
/// - Driver-specific implementations (rusqlite, turso, libsql)
pub(crate) fn generate_enum_impls_for_field(info: &FieldInfo) -> Result<TokenStream> {
    if !info.is_enum {
        return Ok(quote! {});
    }

    let value_type = info.base_type;

    // Generate SQLiteValue conversion based on column type
    let (conversion, reference_conversion) = match info.column_type {
        SQLiteType::Integer => (
            quote! {
                let integer: i64 = value.into();
                SQLiteValue::Integer(integer)
            },
            quote! {
                let integer: i64 = value.into();
                SQLiteValue::Integer(integer)
            },
        ),
        SQLiteType::Text => (
            quote! {
                let text: &str = value.into();
                SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
            },
            quote! {
                let text: &str = value.into();
                SQLiteValue::Text(::std::borrow::Cow::Borrowed(text))
            },
        ),
        _ => {
            return Err(syn::Error::new_spanned(
                info.ident,
                errors::enums::INVALID_COLUMN_TYPE,
            ));
        }
    };

    // Generate driver-specific implementations
    #[cfg(feature = "rusqlite")]
    let rusqlite_impl = super::rusqlite::generate_enum_impls(info)?;
    #[cfg(not(feature = "rusqlite"))]
    let rusqlite_impl = quote! {};

    #[cfg(feature = "turso")]
    let turso_impl = super::turso::generate_enum_impls(info)?;
    #[cfg(not(feature = "turso"))]
    let turso_impl = quote! {};

    #[cfg(feature = "libsql")]
    let libsql_impl = super::libsql::generate_enum_impls(info)?;
    #[cfg(not(feature = "libsql"))]
    let libsql_impl = quote! {};

    Ok(quote! {
        // From<Enum> for SQLiteValue (owned)
        impl<'a> ::std::convert::From<#value_type> for SQLiteValue<'a> {
            fn from(value: #value_type) -> Self {
                #conversion
            }
        }

        // From<&Enum> for SQLiteValue (reference)
        impl<'a> ::std::convert::From<&'a #value_type> for SQLiteValue<'a> {
            fn from(value: &'a #value_type) -> Self {
                #reference_conversion
            }
        }

        // ToSQL implementation
        impl<'a> ToSQL<'a, SQLiteValue<'a>> for #value_type {
            fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
                let value = self;
                #conversion.into()
            }
        }

        // Driver-specific implementations
        #rusqlite_impl
        #turso_impl
        #libsql_impl
    })
}
