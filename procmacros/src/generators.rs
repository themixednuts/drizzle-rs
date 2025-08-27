use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate SQLColumnInfo trait implementation
pub fn generate_sql_column_info(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::drizzle::core::SQLColumnInfo for #struct_ident {
            #body
        }
    }
}

/// Generate SQLTableInfo trait implementation
pub fn generate_sql_table_info(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl ::drizzle::core::SQLTableInfo for #struct_ident {
            #body
        }
    }
}

/// Generate basic struct with common derives
pub fn generate_struct(
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

/// Generate basic impl block
pub fn generate_impl(struct_ident: &Ident, body: TokenStream) -> TokenStream {
    quote! {
        impl #struct_ident {
            #body
        }
    }
}