pub mod r#enum;
pub mod field;
pub mod generators;
pub mod index;
pub mod schema;
pub mod table;
pub mod view;

pub use schema::generate_mysql_schema_derive_impl;

pub(crate) fn escape_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('\'', "''")
}

pub(crate) fn string_value(value: &syn::MetaNameValue, name: &str) -> syn::Result<String> {
    use syn::spanned::Spanned as _;

    if let syn::Expr::Lit(lit) = &value.value
        && let syn::Lit::Str(literal) = &lit.lit
    {
        Ok(literal.value())
    } else {
        Err(syn::Error::new(
            value.span(),
            format!("{name} requires a string literal"),
        ))
    }
}
