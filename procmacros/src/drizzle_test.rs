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
pub fn drizzle_test_impl(input: TokenStream) -> TokenStream {
    let DriversTestInput {
        test_name,
        schema_type,
        test_body,
    } = parse_macro_input!(input as DriversTestInput);

    let expanded = generate_sqlite_driver_tests(&test_name, &schema_type, &test_body);

    TokenStream::from(expanded)
}

/// Generates test functions for all enabled PostgreSQL drivers
pub fn postgres_test_impl(input: TokenStream) -> TokenStream {
    let DriversTestInput {
        test_name,
        schema_type,
        test_body,
    } = parse_macro_input!(input as DriversTestInput);

    let expanded = generate_postgres_driver_tests(&test_name, &schema_type, &test_body);

    TokenStream::from(expanded)
}

fn generate_sqlite_driver_tests(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
) -> TokenStream2 {
    let rusqlite_test = generate_rusqlite_test(test_name, schema_type, test_body);
    let libsql_test = generate_libsql_test(test_name, schema_type, test_body);
    let turso_test = generate_turso_test(test_name, schema_type, test_body);

    quote! {
        #rusqlite_test
        #libsql_test
        #turso_test
    }
}

fn generate_postgres_driver_tests(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
) -> TokenStream2 {
    let postgres_sync_test = generate_postgres_sync_test(test_name, schema_type, test_body);
    let tokio_postgres_test = generate_tokio_postgres_test(test_name, schema_type, test_body);

    quote! {
        #postgres_sync_test
        #tokio_postgres_test
    }
}

fn generate_rusqlite_test(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{}_rusqlite", test_name), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "rusqlite";
    quote! {
        #[cfg(feature = "rusqlite")]
        mod #test_fn_name {
            use super::*;
            #[test]
            fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::rusqlite_setup;
                let (mut db, schema) = rusqlite_setup::setup_db::<#schema_type>();
                let __test_name = #test_name_str;
                let __driver_name = #driver_name;

                // Driver-specific macros for rusqlite
                #[allow(unused_macros)]
                macro_rules! drizzle_exec {
                    // New pattern: drizzle_exec!(builder => all) - captures actual SQL and params
                    ($builder:expr => all) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.all() {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => get) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.get() {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => execute) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.execute() {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Prepared operation pattern with SQL/param capture
                    ($prepared:ident . execute($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.execute($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.execute($conn, __params_vec.clone()) {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . all($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.all($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.all($conn, __params_vec.clone()) {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . get($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.get($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.get($conn, __params_vec.clone()) {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Old pattern: drizzle_exec!(operation) - uses stringify
                    ($operation:expr) => {{
                        let __op_str = stringify!($operation);
                        match $operation {
                            Ok(v) => {
                                db.record(__op_str, None);
                                v
                            },
                            Err(e) => {
                                db.record(__op_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_try {
                    ($operation:expr) => { $operation };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_tx {
                    ($tx:ident, $body:block) => {
                        |$tx| $body
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert_eq {
                    ($expected:expr, $actual:expr) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!("assertion failed: `(left == right)`"),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                    ($expected:expr, $actual:expr, $($msg:tt)+) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!($($msg)+),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert {
                    ($cond:expr) => {
                        if !$cond {
                            db.fail(__test_name, &format!("assertion failed: `{}`", stringify!($cond)), None, None);
                        }
                    };
                    ($cond:expr, $($msg:tt)+) => {
                        if !$cond {
                            db.fail(__test_name, &format!($($msg)+), None, None);
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_fail {
                    ($($msg:tt)+) => {
                        db.fail(__test_name, &format!($($msg)+), None, None);
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_catch_unwind {
                    ($operation:expr) => {
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $operation))
                    };
                }


                #test_body

                Ok(())
            }
        }
    }
}

fn generate_libsql_test(test_name: &Ident, schema_type: &Type, test_body: &Block) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{}_libsql", test_name), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "libsql";
    quote! {
        #[cfg(feature = "libsql")]
        mod #test_fn_name {
            use super::*;
            #[tokio::test]
            async fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::libsql_setup;
                let (mut db, schema) = libsql_setup::setup_db::<#schema_type>().await;
                let __test_name = #test_name_str;
                let __driver_name = #driver_name;

                // Driver-specific macros for libsql
                #[allow(unused_macros)]
                macro_rules! drizzle_exec {
                    // New pattern: drizzle_exec!(builder => all) - captures actual SQL and params
                    ($builder:expr => all) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.all().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => get) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.get().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => execute) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.execute().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Prepared operation pattern with SQL/param capture
                    ($prepared:ident . execute($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.execute($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.execute($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . all($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.all($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.all($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . get($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.get($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.get($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Old pattern: drizzle_exec!(operation) - uses stringify
                    ($operation:expr) => {{
                        let __op_str = stringify!($operation);
                        match $operation.await {
                            Ok(v) => {
                                db.record(__op_str, None);
                                v
                            },
                            Err(e) => {
                                db.record(__op_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_try {
                    ($operation:expr) => { $operation.await };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_tx {
                    ($tx:ident, $body:block) => {
                        async |$tx| $body
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert_eq {
                    ($expected:expr, $actual:expr) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!("assertion failed: `(left == right)`"),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                    ($expected:expr, $actual:expr, $($msg:tt)+) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!($($msg)+),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert {
                    ($cond:expr) => {
                        if !$cond {
                            db.fail(__test_name, &format!("assertion failed: `{}`", stringify!($cond)), None, None);
                        }
                    };
                    ($cond:expr, $($msg:tt)+) => {
                        if !$cond {
                            db.fail(__test_name, &format!($($msg)+), None, None);
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_fail {
                    ($($msg:tt)+) => {
                        db.fail(__test_name, &format!($($msg)+), None, None);
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_catch_unwind {
                    ($operation:expr) => {
                        futures_util::future::FutureExt::catch_unwind(
                            std::panic::AssertUnwindSafe($operation)
                        ).await
                    };
                }


                #test_body

                Ok(())
            }
        }
    }
}

fn generate_turso_test(test_name: &Ident, schema_type: &Type, test_body: &Block) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{}_turso", test_name), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "turso";
    quote! {
        #[cfg(feature = "turso")]
        mod #test_fn_name {
            use super::*;
            #[tokio::test]
            async fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::turso_setup;
                let (mut db, schema) = turso_setup::setup_db::<#schema_type>().await;
                let __test_name = #test_name_str;
                let __driver_name = #driver_name;

                // Driver-specific macros for turso
                #[allow(unused_macros)]
                macro_rules! drizzle_exec {
                    // New pattern: drizzle_exec!(builder => all) - captures actual SQL and params
                    ($builder:expr => all) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.all().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => get) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.get().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => execute) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.execute().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Prepared operation pattern with SQL/param capture
                    ($prepared:ident . execute($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.execute($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.execute($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . all($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.all($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.all($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . get($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.get($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.get($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Old pattern: drizzle_exec!(operation) - uses stringify
                    ($operation:expr) => {{
                        let __op_str = stringify!($operation);
                        match $operation.await {
                            Ok(v) => {
                                db.record(__op_str, None);
                                v
                            },
                            Err(e) => {
                                db.record(__op_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                }
                /// Execute a query with SQL capture: drizzle_sql!(builder, all) or drizzle_sql!(builder, execute)
                #[allow(unused_macros)]
                macro_rules! drizzle_sql {
                    ($builder:expr, all) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.all().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr, get) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.get().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr, execute) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.execute().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_try {
                    ($operation:expr) => { $operation.await };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_tx {
                    ($tx:ident, $body:block) => {
                        async |$tx| $body
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert_eq {
                    ($expected:expr, $actual:expr) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!("assertion failed: `(left == right)`"),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                    ($expected:expr, $actual:expr, $($msg:tt)+) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!($($msg)+),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert {
                    ($cond:expr) => {
                        if !$cond {
                            db.fail(__test_name, &format!("assertion failed: `{}`", stringify!($cond)), None, None);
                        }
                    };
                    ($cond:expr, $($msg:tt)+) => {
                        if !$cond {
                            db.fail(__test_name, &format!($($msg)+), None, None);
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_fail {
                    ($($msg:tt)+) => {
                        db.fail(__test_name, &format!($($msg)+), None, None);
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_catch_unwind {
                    ($operation:expr) => {
                        futures_util::future::FutureExt::catch_unwind(
                            std::panic::AssertUnwindSafe($operation)
                        ).await
                    };
                }


                #test_body

                Ok(())
            }
        }
    }
}

fn generate_postgres_sync_test(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{}_postgres_sync", test_name), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "postgres-sync";
    quote! {
        #[cfg(feature = "postgres-sync")]
        mod #test_fn_name {
            use super::*;
            #[test]
            fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::postgres_sync_setup;
                let (mut db, schema) = postgres_sync_setup::setup_db::<#schema_type>();
                let __test_name = #test_name_str;
                let __driver_name = #driver_name;

                // Driver-specific macros for postgres-sync
                #[allow(unused_macros)]
                macro_rules! drizzle_exec {
                    // New pattern: drizzle_exec!(builder => all) - captures actual SQL and params
                    ($builder:expr => all) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.all() {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => get) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.get() {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => execute) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.execute() {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Prepared operation pattern with SQL/param capture
                    ($prepared:ident . execute($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.execute($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.execute($conn, __params_vec.clone()) {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . all($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.all($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.all($conn, __params_vec.clone()) {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . get($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.get($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.get($conn, __params_vec.clone()) {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Old pattern: drizzle_exec!(operation) - uses stringify
                    ($operation:expr) => {{
                        let __op_str = stringify!($operation);
                        let __op = $operation;
                        match __op {
                            Ok(v) => {
                                db.record(__op_str, None);
                                v
                            },
                            Err(e) => {
                                db.record(__op_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_try {
                    ($operation:expr) => { $operation };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_tx {
                    ($tx:ident, $body:block) => {
                        |$tx| $body
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert_eq {
                    ($expected:expr, $actual:expr) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!("assertion failed: `(left == right)`"),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                    ($expected:expr, $actual:expr, $($msg:tt)+) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!($($msg)+),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert {
                    ($cond:expr) => {
                        if !$cond {
                            db.fail(__test_name, &format!("assertion failed: `{}`", stringify!($cond)), None, None);
                        }
                    };
                    ($cond:expr, $($msg:tt)+) => {
                        if !$cond {
                            db.fail(__test_name, &format!($($msg)+), None, None);
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_fail {
                    ($($msg:tt)+) => {
                        db.fail(__test_name, &format!($($msg)+), None, None);
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_catch_unwind {
                    ($operation:expr) => {
                        std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| $operation))
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_client {
                    () => { db.conn_mut() };
                }


                #test_body

                Ok(())
            }
        }
    }
}

fn generate_tokio_postgres_test(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{}_tokio_postgres", test_name), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "tokio-postgres";
    quote! {
        #[cfg(feature = "tokio-postgres")]
        mod #test_fn_name {
            use super::*;
            #[tokio::test]
            async fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::tokio_postgres_setup;
                let (mut db, schema) = tokio_postgres_setup::setup_db::<#schema_type>().await;
                let __test_name = #test_name_str;
                let __driver_name = #driver_name;

                // Driver-specific macros for tokio-postgres
                #[allow(unused_macros)]
                macro_rules! drizzle_exec {
                    // New pattern: drizzle_exec!(builder => all) - captures actual SQL and params
                    ($builder:expr => all) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.all().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => get) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.get().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($builder:expr => execute) => {{
                        use drizzle::core::ToSQL;
                        let __op_str = stringify!($builder);
                        let __builder = $builder;
                        let __sql_obj = __builder.to_sql();
                        let __sql_str = __sql_obj.sql().to_string();
                        let __params_str = format!("{:?}", __sql_obj.params().collect::<Vec<_>>());
                        match __builder.execute().await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            },
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Prepared operation pattern with SQL/param capture
                    ($prepared:ident . execute($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.execute($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.execute($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . all($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.all($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.all($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    ($prepared:ident . get($conn:expr, $params:expr)) => {{
                        let __op_str = stringify!($prepared.get($conn, $params));
                        let __sql_str = $prepared.to_string();
                        let __params_vec: Vec<_> = ($params).into_iter().collect();
                        let __params_str = format!("{:?}", __params_vec);
                        match $prepared.get($conn, __params_vec.clone()).await {
                            Ok(v) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, None);
                                v
                            }
                            Err(e) => {
                                db.record_sql(__op_str, &__sql_str, &__params_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                    // Old pattern: drizzle_exec!(operation) - uses stringify
                    ($operation:expr) => {{
                        let __op_str = stringify!($operation);
                        let __op = $operation.await;
                        match __op {
                            Ok(v) => {
                                db.record(__op_str, None);
                                v
                            },
                            Err(e) => {
                                db.record(__op_str, Some(format!("{}", e)));
                                db.fail_with_op(__test_name, &e, __op_str);
                            }
                        }
                    }};
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_try {
                    ($operation:expr) => { $operation.await };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_tx {
                    ($tx:ident, $body:block) => {
                        async |$tx| $body
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert_eq {
                    ($expected:expr, $actual:expr) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!("assertion failed: `(left == right)`"),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                    ($expected:expr, $actual:expr, $($msg:tt)+) => {
                        if $expected != $actual {
                            db.fail(
                                __test_name,
                                &format!($($msg)+),
                                Some(&format!("{:#?}", $expected)),
                                Some(&format!("{:#?}", $actual)),
                            );
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_assert {
                    ($cond:expr) => {
                        if !$cond {
                            db.fail(__test_name, &format!("assertion failed: `{}`", stringify!($cond)), None, None);
                        }
                    };
                    ($cond:expr, $($msg:tt)+) => {
                        if !$cond {
                            db.fail(__test_name, &format!($($msg)+), None, None);
                        }
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_fail {
                    ($($msg:tt)+) => {
                        db.fail(__test_name, &format!($($msg)+), None, None);
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_catch_unwind {
                    ($operation:expr) => {
                        futures_util::future::FutureExt::catch_unwind(
                            std::panic::AssertUnwindSafe($operation)
                        ).await
                    };
                }
                #[allow(unused_macros)]
                macro_rules! drizzle_client {
                    () => { db.conn() };
                }


                #test_body

                Ok(())
            }
        }
    }
}
