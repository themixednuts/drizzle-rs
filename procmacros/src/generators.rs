use proc_macro2::{Ident, TokenStream};
use quote::quote;

/// Generate SQLColumnInfo trait implementation
pub fn generate_sql_column_info(
    struct_ident: &Ident,
    name: TokenStream,
    r#type: TokenStream,
    is_primary_key: TokenStream,
    is_not_null: TokenStream,
    is_unique: TokenStream,
    has_default: TokenStream,
    foreign_key: TokenStream,
    table: TokenStream,
) -> TokenStream {
    quote! {
        impl SQLColumnInfo for #struct_ident {
            fn name(&self) -> &str {
                #name
            }
            fn r#type(&self) -> &str {
                #r#type
            }
            fn is_primary_key(&self) -> bool {
                #is_primary_key
            }
            fn is_not_null(&self) -> bool {
                #is_not_null
            }
            fn is_unique(&self) -> bool {
                #is_unique
            }
            fn has_default(&self) -> bool {
                #has_default
            }
            fn table(&self) -> &dyn SQLTableInfo {
                #table
            }
            fn foreign_key(&self) -> Option<&'static dyn SQLColumnInfo> {
                #foreign_key
            }
        }
    }
}

/// Generate SQLTableInfo trait implementation
pub fn generate_sql_table_info(
    struct_ident: &Ident,
    name: TokenStream,
    columns: TokenStream,
) -> TokenStream {
    quote! {
        impl SQLTableInfo for #struct_ident {
            fn name(&self) -> &str {
                #name
            }

            fn columns(&self) -> Box<[&'static dyn SQLColumnInfo]> {
                #columns
            }
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
