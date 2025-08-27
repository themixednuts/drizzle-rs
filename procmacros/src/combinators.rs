use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generic combinator for SQLColumnInfo trait implementations
pub fn sql_column_info_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::drizzle::core::SQLColumnInfo for #struct_ident {
            #body
        }
    }
}

/// Generic combinator for SQLTableInfo trait implementations
pub fn sql_table_info_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::drizzle::core::SQLTableInfo for #struct_ident {
            #body
        }
    }
}

/// Generic combinator for basic struct definitions with common derives
pub fn basic_struct_def(
    struct_vis: &syn::Visibility, 
    struct_ident: &Ident, 
    fields: TokenStream,
    allows: &[&str]
) -> TokenStream {
    let allow_attrs = if !allows.is_empty() {
        let attrs = allows.iter().map(|a| quote! { #a }).collect::<Vec<_>>();
        quote! { #[allow(#(#attrs),*)] }
    } else {
        quote! {}
    };
    
    quote! {
        #allow_attrs
        #[derive(Debug, Clone, Copy, Default, PartialOrd, Ord, Eq, PartialEq, Hash)]
        #struct_vis struct #struct_ident {
            #fields
        }
    }
}

/// Generic combinator for impl blocks with common patterns
pub fn basic_impl_block(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl #struct_ident {
            #body
        }
    }
}