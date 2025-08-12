use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

/// Generates a schema type name based on the types
pub fn get_schema_name(types: &[Type]) -> String {
    if types.is_empty() {
        "EmptySchema".to_string()
    } else {
        // Create a schema name from the type names
        let combined_names = types
            .iter()
            .map(|ty| {
                if let Type::Path(path) = ty {
                    if let Some(segment) = path.path.segments.last() {
                        return segment.ident.to_string();
                    }
                }
                "Unknown".to_string()
            })
            .collect::<Vec<_>>()
            .join("");

        format!("{}Schema", combined_names)
    }
}

/// Generates a schema type for the given tables
pub(crate) fn generate_schema(input: TokenStream) -> syn::Result<TokenStream> {
    // Parse the input as an optional array of table types
    let types = parse_schema_input(input)?;
    // Generate a schema type name based on the types
    let schema_name = get_schema_name(&types);
    let schema_ident = quote::format_ident!("{}", schema_name);

    // Generate output with schema type definition and implementations
    let output = if types.is_empty() {
        // If no types, just create an empty schema type
        quote! {
            #[derive(Clone, Debug)]
            pub struct EmptySchema;
        }
    } else {
        // Add IsInSchema implementations for each type
        let is_in_schema_impls = types.iter().map(|ty| {
            quote! {
                impl ::drizzle_rs::core::IsInSchema<#schema_ident> for #ty {}
            }
        });
        // Define the schema type and implementations
        quote! {
            #[derive(Clone, Debug)]
            #[allow(non_camel_case_types)]
            pub struct #schema_ident;

            #(#is_in_schema_impls)*
        }
    };
    Ok(output)
}

/// Parse the input as an optional array of table types
pub(crate) fn parse_schema_input(input: TokenStream) -> syn::Result<Vec<Type>> {
    // If input is empty, return an empty vec
    if input.is_empty() {
        return Ok(Vec::new());
    }

    // Try to parse as an array
    if let Ok(array) = syn::parse2::<syn::ExprArray>(input.clone()) {
        let mut types = Vec::new();

        for expr in array.elems {
            if let syn::Expr::Path(path) = expr {
                let type_path = syn::TypePath {
                    qself: None,
                    path: path.path,
                };
                types.push(syn::Type::Path(type_path));
            } else {
                return Err(syn::Error::new_spanned(expr, "Expected a type name"));
            }
        }

        return Ok(types);
    }

    // Try to parse as a single type
    match syn::parse2::<Type>(input) {
        Ok(ty) => Ok(vec![ty]),
        Err(err) => Err(err),
    }
}
