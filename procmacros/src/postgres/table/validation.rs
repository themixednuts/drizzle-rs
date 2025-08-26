// use crate::postgres::field::{FieldInfo, PostgreSQLDefault};
// use proc_macro2::TokenStream;
// use quote::{ToTokens, quote};
// use syn::Expr;

// /// Generates compile-time validation blocks for default literals
// pub(crate) fn generate_default_validations(field_infos: &[FieldInfo]) -> TokenStream {
//     let validations: Vec<TokenStream> = field_infos
//         .iter()
//         .filter_map(|info| {
//             if let Some(v) = &info.default {
//                 let base_type_tokens = &info.ty; // already a syn::Type
//                 let base_type: proc_macro2::TokenStream =
//                     if base_type_tokens.to_token_stream().to_string() == "String" {
//                         quote! { &str }
//                     } else {
//                         quote! { #base_type_tokens }
//                     };
//                 Some(quote! {
//                     // Compile-time validation: ensure default literal is compatible with field type
//                     const _: () = {
//                         // This will cause a compile error if the literal type doesn't match the field type
//                         // For example: `let _: i32 = "string";` will fail at compile time
//                         //              `let _: String = 42;` will fail at compile time
//                         let _: #base_type = #expr_lit;
//                     };
//                 })
//             } else {
//                 None
//             }
//         })
//         .collect();

//     if validations.is_empty() {
//         quote!() // No validations needed
//     } else {
//         quote! {
//             // Default literal validations - these blocks ensure type compatibility at compile time
//             #(#validations)*
//         }
//     }
// }
