use crate::sqlite::field::{FieldInfo, SQLiteType};
use proc_macro2::TokenStream;
use quote::{ToTokens, quote};
use syn::Expr;

pub(crate) fn validate_strict_affinity(field_infos: &[FieldInfo], strict: bool) -> syn::Result<()> {
    let mut errors: Vec<syn::Error> = Vec::new();

    for info in field_infos {
        if strict && !info.column_type.is_strict_allowed() {
            errors.push(syn::Error::new_spanned(
                info.ident,
                format!(
                    "column `{}` uses `{}` affinity, which is not allowed in STRICT tables",
                    info.column_name, info.column_type
                ),
            ));
        }

        if !strict && matches!(info.column_type, SQLiteType::Any) {
            errors.push(syn::Error::new_spanned(
                info.ident,
                format!(
                    "column `{}` uses `ANY`, which is only allowed on STRICT tables; add `#[SQLiteTable(strict)]`",
                    info.column_name
                ),
            ));
        }
    }

    let mut iter = errors.into_iter();
    if let Some(mut first) = iter.next() {
        for err in iter {
            first.combine(err);
        }
        return Err(first);
    }

    Ok(())
}

/// Generates compile-time validation blocks for default literals
pub(crate) fn generate_default_validations(field_infos: &[FieldInfo]) -> TokenStream {
    let validations: Vec<TokenStream> = field_infos
        .iter()
        .filter_map(|info| {
            if let Some(Expr::Lit(expr_lit)) = &info.default_value {
                let base_type_tokens = &info.base_type; // already a syn::Type
                let base_type: proc_macro2::TokenStream =
                    if base_type_tokens.to_token_stream().to_string() == "String" {
                        quote! { &str }
                    } else {
                        quote! { #base_type_tokens }
                    };
                Some(quote! {
                    // Compile-time validation: ensure default literal is compatible with field type
                    const _: () = {
                        // This will cause a compile error if the literal type doesn't match the field type
                        // For example: `let _: i32 = "string";` will fail at compile time
                        //              `let _: String = 42;` will fail at compile time
                        let _: #base_type = #expr_lit;
                    };
                })
            } else {
                None
            }
        })
        .collect();

    if validations.is_empty() {
        quote!() // No validations needed
    } else {
        quote! {
            // Default literal validations - these blocks ensure type compatibility at compile time
            #(#validations)*
        }
    }
}
