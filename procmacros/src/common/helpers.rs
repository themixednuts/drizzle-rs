//! Shared helper functions for procedural macro code generation.
//!
//! These utilities are used across both SQLite and PostgreSQL macro implementations
//! to reduce code duplication and ensure consistent behavior.

use proc_macro2::TokenStream;
use quote::quote;
use syn::{Data, DeriveInput, Error, Expr, ExprPath, Field, Fields, Meta, Result};

/// Create an ExprPath with an UPPERCASE ident but preserving the original span.
///
/// This is used to preserve IDE hover documentation while normalizing attribute names.
///
/// # Example
///
/// ```ignore
/// // Original: #[column(primary)]
/// // Creates ExprPath for "PRIMARY" with the span of "primary"
/// let path = make_uppercase_path(&ident, "PRIMARY");
/// ```
pub(crate) fn make_uppercase_path(original_ident: &syn::Ident, uppercase_name: &str) -> ExprPath {
    let new_ident = syn::Ident::new(uppercase_name, original_ident.span());
    ExprPath {
        attrs: vec![],
        qself: None,
        path: new_ident.into(),
    }
}

/// Parse column reference from field attributes, looking for `#[column(Table::field)]`.
///
/// This is used by FromRow derives to map struct fields to specific table columns,
/// which is especially useful for JOIN queries where multiple tables may have
/// columns with the same name.
///
/// # Example
///
/// ```ignore
/// #[derive(SQLiteFromRow)]
/// struct UserPost {
///     #[column(Users::id)]
///     user_id: i32,
///     #[column(Posts::id)]
///     post_id: i32,
/// }
/// ```
pub(crate) fn parse_column_reference(field: &Field) -> Option<ExprPath> {
    for attr in &field.attrs {
        if let Some(ident) = attr.path().get_ident()
            && ident == "column"
            && let Meta::List(meta_list) = &attr.meta
            && let Ok(Expr::Path(expr_path)) = syn::parse2::<Expr>(meta_list.tokens.clone())
        {
            return Some(expr_path);
        }
    }
    None
}

/// Extract struct fields from a DeriveInput, returning the fields and whether it's a tuple struct.
///
/// # Returns
///
/// - `Ok((fields, is_tuple))` - The punctuated fields and a boolean indicating if it's a tuple struct
/// - `Err(_)` - If the input is not a struct or is a unit struct
///
/// # Errors
///
/// Returns an error if:
/// - The input is a unit struct (no fields)
/// - The input is not a struct (enum or union)
pub(crate) fn extract_struct_fields(
    input: &DeriveInput,
) -> Result<(&syn::punctuated::Punctuated<Field, syn::token::Comma>, bool)> {
    let struct_name = &input.ident;
    match &input.data {
        Data::Struct(data_struct) => match &data_struct.fields {
            Fields::Named(fields) => Ok((&fields.named, false)),
            Fields::Unnamed(fields) => Ok((&fields.unnamed, true)),
            Fields::Unit => Err(Error::new_spanned(
                struct_name,
                "FromRow cannot be derived for unit structs",
            )),
        },
        _ => Err(Error::new_spanned(
            struct_name,
            "FromRow can only be derived for structs",
        )),
    }
}

/// Generate a `TryFrom` implementation for converting database rows to structs.
///
/// This helper reduces duplication across the multiple driver implementations
/// (rusqlite, libsql, turso, postgres, tokio-postgres) by providing a single
/// function that generates the common `TryFrom` impl structure.
///
/// # Arguments
///
/// * `struct_name` - The name of the struct to implement `TryFrom` for
/// * `row_type` - The fully-qualified row type path (e.g., `::rusqlite::Row<'_>`)
/// * `error_type` - The error type for the `TryFrom` implementation
/// * `field_assignments` - The generated field assignment expressions
/// * `is_tuple` - Whether the struct is a tuple struct
///
/// # Example
///
/// ```ignore
/// let impl_block = generate_try_from_impl(
///     &struct_name,
///     quote!(::rusqlite::Row<'_>),
///     quote!(::rusqlite::Error),
///     &field_assignments,
///     false, // named struct
/// );
/// ```
pub(crate) fn generate_try_from_impl(
    struct_name: &syn::Ident,
    row_type: TokenStream,
    error_type: TokenStream,
    field_assignments: &[TokenStream],
    is_tuple: bool,
) -> TokenStream {
    let construct = if is_tuple {
        quote! { Self(#(#field_assignments)*) }
    } else {
        quote! { Self { #(#field_assignments)* } }
    };

    quote! {
        impl ::std::convert::TryFrom<&#row_type> for #struct_name {
            type Error = #error_type;

            fn try_from(row: &#row_type) -> ::std::result::Result<Self, Self::Error> {
                ::std::result::Result::Ok(#construct)
            }
        }
    }
}

/// Check if a field has a specific attribute by name.
///
/// # Example
///
/// ```ignore
/// if has_attribute(field, "json") {
///     // Handle JSON field
/// }
/// ```
#[allow(dead_code)]
pub(crate) fn has_attribute(field: &Field, attr_name: &str) -> bool {
    field.attrs.iter().any(|attr| {
        attr.path()
            .get_ident()
            .is_some_and(|ident| ident == attr_name)
    })
}

/// Check if a type is an Option type.
///
/// This is useful for determining nullability of fields.
pub(crate) fn is_option_type(ty: &syn::Type) -> bool {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
    {
        return segment.ident == "Option";
    }
    false
}

/// Extract the inner type from an Option<T> type.
///
/// Returns the original type if it's not an Option.
pub(crate) fn extract_option_inner(ty: &syn::Type) -> &syn::Type {
    if let syn::Type::Path(type_path) = ty
        && let Some(segment) = type_path.path.segments.last()
        && segment.ident == "Option"
        && let syn::PathArguments::AngleBracketed(args) = &segment.arguments
        && let Some(syn::GenericArgument::Type(inner)) = args.args.first()
    {
        return inner;
    }
    ty
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_is_option_type() {
        let option_type: syn::Type = parse_quote!(Option<String>);
        assert!(is_option_type(&option_type));

        let non_option: syn::Type = parse_quote!(String);
        assert!(!is_option_type(&non_option));

        let nested_option: syn::Type = parse_quote!(Option<Option<i32>>);
        assert!(is_option_type(&nested_option));
    }

    #[test]
    fn test_extract_option_inner() {
        let option_type: syn::Type = parse_quote!(Option<String>);
        let inner = extract_option_inner(&option_type);
        let expected: syn::Type = parse_quote!(String);
        assert_eq!(quote!(#inner).to_string(), quote!(#expected).to_string());

        let non_option: syn::Type = parse_quote!(String);
        let result = extract_option_inner(&non_option);
        assert_eq!(quote!(#result).to_string(), quote!(#non_option).to_string());
    }
}
