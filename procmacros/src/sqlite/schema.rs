use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{DeriveInput, Fields, Ident, Type};

pub(crate) fn schema_macro(input: DeriveInput) -> syn::Result<TokenStream> {
    let struct_name = &input.ident;

    // Extract the table types from the struct fields
    let table_types = match input.data {
        syn::Data::Struct(ref data) => {
            match &data.fields {
                Fields::Unnamed(fields) => {
                    // For tuple structs like Schema(Users, Followers)
                    fields
                        .unnamed
                        .iter()
                        .map(|field| match &field.ty {
                            Type::Path(type_path) => {
                                if let Some(segment) = type_path.path.segments.last() {
                                    Ok(segment.ident.clone())
                                } else {
                                    Err(syn::Error::new_spanned(field, "Invalid table type"))
                                }
                            }
                            _ => Err(syn::Error::new_spanned(field, "Invalid table type")),
                        })
                        .collect::<Result<Vec<_>, _>>()?
                }
                _ => {
                    return Err(syn::Error::new_spanned(
                        &input,
                        "Schema derive macro only supports tuple structs like Schema(Users, Followers)",
                    ));
                }
            }
        }
        _ => {
            return Err(syn::Error::new_spanned(
                &input,
                "Schema derive macro only supports structs",
            ));
        }
    };

    if table_types.is_empty() {
        return Err(syn::Error::new_spanned(
            &input,
            "Schema must contain at least one table type",
        ));
    }

    // Generate field definitions - lowercase variables for each table
    let field_defs = table_types.iter().map(|table_type| {
        let field_name = format_ident!("{}", table_type.to_string().to_lowercase());

        quote! {
            pub #field_name: &'static #table_type
        }
    });

    // Generate initialization code with LazyLock instances
    let init_fields = table_types.iter().map(|table_type| {
        let field_name = format_ident!("{}", table_type.to_string().to_lowercase());
        let table_var = format_ident!("_TABLE_{}", table_type.to_string().to_uppercase());

        quote! {
            #field_name: &#table_var
        }
    });

    // Generate static table instances
    let table_statics = table_types.iter().map(|table_type| {
        let table_var = format_ident!("_TABLE_{}", table_type.to_string().to_uppercase());

        quote! {
            static #table_var: ::std::sync::LazyLock<#table_type> =
                ::std::sync::LazyLock::new(|| #table_type::default());
        }
    });

    // Generate the Schema struct
    let expanded = quote! {
        #[derive(Debug, Clone)]
        pub struct #struct_name {
            #(#field_defs),*
        }

        impl #struct_name {
            pub fn new() -> Self {
                // Create static instances for each table
                #(#table_statics)*

                Self {
                    #(#init_fields),*
                }
            }

            // Get a table by type
            pub fn table<T: Table + 'static>(&self) -> Option<&'static T> {
                // Implementation would need to compare type ids and return the appropriate table
                // This is a placeholder implementation
                None
            }
        }
    };

    Ok(expanded)
}

fn generate_relationship_methods(table_types: &[Ident]) -> TokenStream {
    // For each pair of tables, generate methods to help with relationships
    // This is just a skeleton - the actual implementation would analyze the tables
    // to determine the relationship types

    let mut methods = Vec::new();

    for (i, table1) in table_types.iter().enumerate() {
        for (j, table2) in table_types.iter().enumerate() {
            if i != j {
                let method_name = format_ident!(
                    "with_{}_{}",
                    table1.to_string().to_lowercase(),
                    table2.to_string().to_lowercase()
                );

                methods.push(quote! {
                    pub fn #method_name(&self) -> (&#table1, &#table2) {
                        (self.#table1, self.#table2)
                    }
                });
            }
        }
    }

    quote! {
        #(#methods)*
    }
}
