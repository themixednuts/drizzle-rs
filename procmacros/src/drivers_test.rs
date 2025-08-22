use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Block, Ident, Type, parse_macro_input};

/// Parse input for drivers_test macro
struct DriversTestInput {
    test_name: Ident,
    schema_type: Type,
    test_body: Block,
}

impl syn::parse::Parse for DriversTestInput {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let test_name: Ident = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let schema_type: Type = input.parse()?;
        input.parse::<syn::Token![,]>()?;
        let test_body: Block = input.parse()?;

        Ok(DriversTestInput {
            test_name,
            schema_type,
            test_body,
        })
    }
}

/// Generates test functions for all enabled SQLite drivers
pub fn drivers_test_impl(input: TokenStream) -> TokenStream {
    let DriversTestInput {
        test_name,
        schema_type,
        test_body,
    } = parse_macro_input!(input as DriversTestInput);

    let expanded = generate_driver_tests(&test_name, &schema_type, &test_body);

    TokenStream::from(expanded)
}

fn generate_driver_tests(test_name: &Ident, schema_type: &Type, test_body: &Block) -> TokenStream2 {
    let rusqlite_test = generate_rusqlite_test(test_name, schema_type, test_body);
    let libsql_test = generate_libsql_test(test_name, schema_type, test_body);
    let turso_test = generate_turso_test(test_name, schema_type, test_body);

    quote! {
        #rusqlite_test
        #libsql_test
        #turso_test
    }
}

fn generate_rusqlite_test(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{}_rusqlite", test_name), test_name.span());
    let test_name_str = test_name.to_string();

    quote! {
        #[cfg(feature = "rusqlite")]
        #[test]
        fn #test_fn_name() {
            use crate::common::helpers::rusqlite_setup;
            let (db, schema) = rusqlite_setup::setup_db::<#schema_type>();

            // Debug prints
            println!("🔧 RUSQLITE Driver: Test {} starting", #test_name_str);
            println!("   DB type: {:?}", std::any::type_name_of_val(&db));
            println!("   Schema type: {:?}", std::any::type_name_of_val(&schema));

            // Driver-specific macros for rusqlite
            #[allow(unused_macros)]
            macro_rules! drizzle_exec {
                ($operation:expr) => { $operation.unwrap() };
            }
            #[allow(unused_macros)]
            macro_rules! drizzle_try {
                ($operation:expr) => { $operation };
            }

            #test_body

            println!("✅ RUSQLITE Driver: Test {} completed", #test_name_str);
        }
    }
}

fn generate_libsql_test(test_name: &Ident, schema_type: &Type, test_body: &Block) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{}_libsql", test_name), test_name.span());
    let test_name_str = test_name.to_string();

    quote! {
        #[cfg(feature = "libsql")]
        #[tokio::test]
        async fn #test_fn_name() {
            use crate::common::helpers::libsql_setup;
            let (db, schema) = libsql_setup::setup_db::<#schema_type>().await;

            // Debug prints
            println!("🔧 LIBSQL Driver: Test {} starting", #test_name_str);
            println!("   DB type: {:?}", std::any::type_name_of_val(&db));
            println!("   Schema type: {:?}", std::any::type_name_of_val(&schema));

            // Driver-specific macros for libsql
            #[allow(unused_macros)]
            macro_rules! drizzle_exec {
                ($operation:expr) => { $operation.await.unwrap() };
            }
            #[allow(unused_macros)]
            macro_rules! drizzle_try {
                ($operation:expr) => { $operation.await };
            }

            #test_body

            println!("✅ LIBSQL Driver: Test {} completed", #test_name_str);
        }
    }
}

fn generate_turso_test(test_name: &Ident, schema_type: &Type, test_body: &Block) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{}_turso", test_name), test_name.span());
    let test_name_str = test_name.to_string();

    quote! {
        #[cfg(feature = "turso")]
        #[tokio::test]
        async fn #test_fn_name() {
            use crate::common::helpers::turso_setup;
            let (db, schema) = turso_setup::setup_db::<#schema_type>().await;

            // Debug prints
            println!("🔧 TURSO Driver: Test {} starting", #test_name_str);
            println!("   DB type: {:?}", std::any::type_name_of_val(&db));
            println!("   Schema type: {:?}", std::any::type_name_of_val(&schema));

            // Driver-specific macros for turso
            #[allow(unused_macros)]
            macro_rules! drizzle_exec {
                ($operation:expr) => { $operation.await.unwrap() };
            }
            #[allow(unused_macros)]
            macro_rules! drizzle_try {
                ($operation:expr) => { $operation.await };
            }

            #test_body

            println!("✅ TURSO Driver: Test {} completed", #test_name_str);
        }
    }
}
