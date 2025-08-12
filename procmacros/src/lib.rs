extern crate proc_macro;

mod drizzle;
mod qb;
mod schema;
mod utils;

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "rusqlite")]
mod rusqlite;

use drizzle::DrizzleInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Process the drizzle! proc macro
/// This macro creates a Drizzle instance from a connection and schema
#[proc_macro]
pub fn drizzle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DrizzleInput);

    match drizzle::drizzle_impl(input) {
        Ok(output) => output.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro]
pub fn qb(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match qb::qb_impl(input) {
        Ok(qb) => qb.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[cfg(feature = "sqlite")]
#[proc_macro_derive(SQLiteEnum)]
pub fn sqlite_enum_derive(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{Data, DeriveInput, parse_macro_input};

    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Check if this is an enum or tuple struct
    match &input.data {
        Data::Enum(data) => {
            // Check if the enum has any variants
            if data.variants.is_empty() {
                return quote! {
                    compile_error!("SQLiteEnum cannot be derived for empty enums");
                }
                .into();
            }

            // Generate implementation for enum
            match crate::sqlite::r#enum::generate_enum_impl(name, data) {
                Ok(ts) => ts.into(),
                Err(e) => return e.to_compile_error().into(),
            }
        }
        _ => {
            return quote! {
                compile_error!("SQLiteEnum can only be derived for enums and tuple structs");
            }
            .into();
        }
    }
}

#[cfg(feature = "sqlite")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn SQLiteTable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_result = syn::parse_macro_input!(attr as crate::sqlite::table::TableAttributes);

    match crate::sqlite::table::table_attr_macro(input, attr_result) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Attribute macro for creating SQLite indexes
#[cfg(feature = "sqlite")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn SQLiteIndex(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_input = syn::parse_macro_input!(attr as crate::sqlite::index::IndexAttributes);

    match crate::sqlite::index::sqlite_index_attr_macro(attr_input, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Automatically implements `TryFrom<&Row<'_>>` for structs using field name-based column access.
///
/// This derive macro generates a `TryFrom` implementation that maps struct fields to rusqlite
/// columns using the field names directly (e.g., `name: row.get("name")?`).
///
/// # Example
///
/// ```ignore
/// #[derive(FromRow, Debug)]
/// struct User {
///     id: i32,
///     name: String,
///     email: Option<String>,
/// }
///
/// // Generated implementation:
/// impl TryFrom<&Row<'_>> for User {
///     type Error = rusqlite::Error;
///     fn try_from(row: &Row<'_>) -> Result<Self, Self::Error> {
///         Ok(Self {
///             id: row.get("id")?,
///             name: row.get("name")?,
///             email: row.get("email")?,
///         })
///     }
/// }
/// ```
#[cfg(feature = "rusqlite")]
#[proc_macro_derive(FromRow)]
pub fn from_row_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::rusqlite::from_row::generate_from_row_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
