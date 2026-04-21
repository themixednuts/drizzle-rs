use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Attribute, Block, FnArg, Ident, ItemFn, Pat, PatIdent, Type, spanned::Spanned};

// ===========================================================================
// Attribute-style entry: `#[drizzle::test]` / `#[drizzle::test(sqlite|postgres)]`
// ===========================================================================

#[derive(Clone, Copy, Debug)]
enum Dialect {
    Sqlite,
    Postgres,
}

#[derive(Clone, Copy)]
enum DialectOverride {
    None,
    Sqlite,
    Postgres,
}

impl syn::parse::Parse for DialectOverride {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        if input.is_empty() {
            return Ok(Self::None);
        }
        let ident: Ident = input.parse()?;
        if !input.is_empty() {
            return Err(input.error("unexpected tokens; expected `sqlite` or `postgres`"));
        }
        match ident.to_string().as_str() {
            "sqlite" => Ok(Self::Sqlite),
            "postgres" => Ok(Self::Postgres),
            other => Err(syn::Error::new(
                ident.span(),
                format!("expected `sqlite` or `postgres`, got `{other}`"),
            )),
        }
    }
}

/// Parsed view of the user-written `fn` that `#[drizzle::test]` was applied to.
struct FnInput {
    outer_attrs: Vec<Attribute>,
    fn_name: Ident,
    db_pat: PatIdent,
    db_ty: Type,
    schema_pat: PatIdent,
    schema_ty: Type,
    body: Block,
}

pub fn attribute_impl(args: TokenStream, item: TokenStream) -> TokenStream {
    let args_ts: TokenStream2 = args.into();
    let item_ts: TokenStream2 = item.into();
    match attribute_impl_inner(args_ts, item_ts) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

fn attribute_impl_inner(args: TokenStream2, item: TokenStream2) -> syn::Result<TokenStream2> {
    let dialect_override: DialectOverride = syn::parse2(args)?;
    let input_fn: ItemFn = syn::parse2(item)?;
    let fn_input = parse_fn(input_fn)?;
    let dialect = resolve_dialect(dialect_override, &fn_input)?;

    let bindings_for = |setup_call: TokenStream2| -> TokenStream2 {
        let rebinds = rebind_stmts(
            &fn_input.db_pat,
            &fn_input.db_ty,
            &quote! { __raw_db },
            &fn_input.schema_pat,
            &fn_input.schema_ty,
            &quote! { __raw_schema },
        );
        quote! {
            let (mut __raw_db, __raw_schema) = #setup_call;
            #rebinds
        }
    };

    let generated = match dialect {
        Dialect::Sqlite => generate_sqlite_driver_tests_with(
            &fn_input.fn_name,
            &fn_input.schema_ty,
            &fn_input.body,
            &bindings_for,
        ),
        Dialect::Postgres => generate_postgres_driver_tests_with(
            &fn_input.fn_name,
            &fn_input.schema_ty,
            &fn_input.body,
            &bindings_for,
        ),
    };

    Ok(wrap_with_schema_assertion(&fn_input, &generated))
}

fn parse_fn(item_fn: ItemFn) -> syn::Result<FnInput> {
    let sig = &item_fn.sig;
    if let Some(asyncness) = sig.asyncness {
        return Err(syn::Error::new(
            asyncness.span(),
            "`#[drizzle::test]` functions must be synchronous; use the injected `drizzle_exec!` / `drizzle_try!` / `drizzle_tx!` helper macros in the body for async operations",
        ));
    }
    if let Some(constness) = sig.constness {
        return Err(syn::Error::new(constness.span(), "unexpected `const`"));
    }
    if let Some(unsafety) = sig.unsafety {
        return Err(syn::Error::new(unsafety.span(), "unexpected `unsafe`"));
    }
    if !sig.generics.params.is_empty() {
        return Err(syn::Error::new(
            sig.generics.span(),
            "`#[drizzle::test]` functions cannot declare generics",
        ));
    }

    let inputs: Vec<&FnArg> = sig.inputs.iter().collect();
    if inputs.len() != 2 {
        return Err(syn::Error::new(
            sig.inputs.span(),
            "`#[drizzle::test]` expects exactly 2 parameters: `db` then `schema`",
        ));
    }

    let (db_pat, db_ty) = parse_typed_arg(inputs[0])?;
    let (schema_pat, schema_ty) = parse_typed_arg(inputs[1])?;

    if db_pat.ident != "db" {
        return Err(syn::Error::new(
            db_pat.ident.span(),
            "first parameter must be named `db` (the body-local `drizzle_exec!`/`drizzle_try!`/etc. macros reference it by name)",
        ));
    }
    if schema_pat.ident != "schema" {
        return Err(syn::Error::new(
            schema_pat.ident.span(),
            "second parameter must be named `schema`",
        ));
    }

    Ok(FnInput {
        outer_attrs: item_fn.attrs,
        fn_name: sig.ident.clone(),
        db_pat,
        db_ty,
        schema_pat,
        schema_ty,
        body: *item_fn.block,
    })
}

fn parse_typed_arg(arg: &FnArg) -> syn::Result<(PatIdent, Type)> {
    let pt = match arg {
        FnArg::Typed(pt) => pt,
        FnArg::Receiver(r) => {
            return Err(syn::Error::new(
                r.span(),
                "`self` receivers are not allowed on `#[drizzle::test]` functions",
            ));
        }
    };
    let pat_ident = match &*pt.pat {
        Pat::Ident(pi) => pi.clone(),
        other => {
            return Err(syn::Error::new(
                other.span(),
                "parameter pattern must be a simple identifier (e.g. `db` or `schema`)",
            ));
        }
    };
    Ok((pat_ident, (*pt.ty).clone()))
}

fn resolve_dialect(overrid: DialectOverride, fn_input: &FnInput) -> syn::Result<Dialect> {
    match overrid {
        DialectOverride::Sqlite => Ok(Dialect::Sqlite),
        DialectOverride::Postgres => Ok(Dialect::Postgres),
        DialectOverride::None => {
            let file = proc_macro::Span::call_site().file();
            let normalized = file.replace('\\', "/");
            let in_sqlite = normalized.contains("/sqlite/") || normalized.starts_with("sqlite/");
            let in_postgres =
                normalized.contains("/postgres/") || normalized.starts_with("postgres/");
            match (in_sqlite, in_postgres) {
                (true, false) => Ok(Dialect::Sqlite),
                (false, true) => Ok(Dialect::Postgres),
                _ => Err(syn::Error::new(
                    fn_input.fn_name.span(),
                    format!(
                        "could not auto-detect dialect from file path `{file}` — add `#[drizzle::test(sqlite)]` or `#[drizzle::test(postgres)]`"
                    ),
                )),
            }
        }
    }
}

/// Emit the let-bindings that shadow the raw `__raw_db` / `__raw_schema` locals
/// with names and forms matching what the user wrote in the fn signature.
fn rebind_stmts(
    db_pat: &PatIdent,
    db_ty: &Type,
    raw_db: &TokenStream2,
    schema_pat: &PatIdent,
    schema_ty: &Type,
    raw_schema: &TokenStream2,
) -> TokenStream2 {
    let db_stmt = rebind_stmt(db_pat, db_ty, raw_db);
    let schema_stmt = rebind_stmt(schema_pat, schema_ty, raw_schema);
    quote! {
        #db_stmt
        #schema_stmt
    }
}

fn rebind_stmt(pat: &PatIdent, ty: &Type, raw: &TokenStream2) -> TokenStream2 {
    let name = &pat.ident;
    let user_mut = &pat.mutability;
    let rhs = match ty {
        Type::Reference(tr) if tr.mutability.is_some() => quote! { &mut #raw },
        Type::Reference(_) => quote! { & #raw },
        _ => quote! { #raw },
    };
    quote! { #[allow(unused_variables)] let #user_mut #name = #rhs; }
}

fn wrap_with_schema_assertion(fn_input: &FnInput, generated: &TokenStream2) -> TokenStream2 {
    let schema_ty_owned = strip_outer_ref(&fn_input.schema_ty);
    let outer_attrs = &fn_input.outer_attrs;
    quote! {
        #(#outer_attrs)*
        #[allow(non_snake_case, dead_code)]
        const _: () = {
            fn __drizzle_test_assert_schema<
                S: drizzle::core::SQLSchemaImpl + ::core::default::Default + ::core::marker::Copy,
            >() {}
            let _ = __drizzle_test_assert_schema::<#schema_ty_owned>;
        };
        #generated
    }
}

fn strip_outer_ref(ty: &Type) -> Type {
    match ty {
        Type::Reference(tr) => (*tr.elem).clone(),
        other => other.clone(),
    }
}

fn generate_sqlite_driver_tests_with(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
    bindings: &dyn Fn(TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    let rusqlite_test = generate_rusqlite_test(test_name, schema_type, test_body, bindings);
    let libsql_test = generate_libsql_test(test_name, schema_type, test_body, bindings);
    let turso_test = generate_turso_test(test_name, schema_type, test_body, bindings);

    quote! {
        #rusqlite_test
        #libsql_test
        #turso_test
    }
}

fn generate_postgres_driver_tests_with(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
    bindings: &dyn Fn(TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    let postgres_sync_test =
        generate_postgres_sync_test(test_name, schema_type, test_body, bindings);
    let tokio_postgres_test =
        generate_tokio_postgres_test(test_name, schema_type, test_body, bindings);

    quote! {
        #postgres_sync_test
        #tokio_postgres_test
    }
}

/// Assertion macros (`drizzle_assert_eq!`, `drizzle_assert!`, `drizzle_fail!`)
/// that are identical across every driver's generated test scope. All three
/// reference `db` and `__test_name` from the enclosing scope.
fn common_assertion_macros() -> TokenStream2 {
    quote! {
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
    }
}

fn generate_rusqlite_test(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
    bindings: &dyn Fn(TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{test_name}_rusqlite"), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "rusqlite";
    let setup_call = quote! { rusqlite_setup::setup_db::<#schema_type>() };
    let prelude = bindings(setup_call);
    let assertion_macros = common_assertion_macros();
    quote! {
        #[cfg(feature = "rusqlite")]
        mod #test_fn_name {
            use super::*;
            #[test]
            fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::rusqlite_setup;
                #prelude
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
                    ($builder:expr => all_as) => {{
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
                    ($builder:expr => get_as) => {{
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
                        let __params_str = stringify!($params);
                        match $prepared.execute($conn, $params) {
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
                        let __params_str = stringify!($params);
                        match $prepared.all($conn, $params) {
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
                        let __params_str = stringify!($params);
                        match $prepared.get($conn, $params) {
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
                #assertion_macros
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

fn generate_libsql_test(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
    bindings: &dyn Fn(TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{test_name}_libsql"), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "libsql";
    let setup_call = quote! { libsql_setup::setup_db::<#schema_type>().await };
    let prelude = bindings(setup_call);
    let assertion_macros = common_assertion_macros();
    quote! {
        #[cfg(feature = "libsql")]
        mod #test_fn_name {
            use super::*;
            #[tokio::test]
            async fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::libsql_setup;
                #prelude
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
                    ($builder:expr => all_as) => {{
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
                    ($builder:expr => get_as) => {{
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
                        let __params_str = stringify!($params);
                        match $prepared.execute($conn, $params).await {
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
                        let __params_str = stringify!($params);
                        match $prepared.all($conn, $params).await {
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
                        let __params_str = stringify!($params);
                        match $prepared.get($conn, $params).await {
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
                #assertion_macros
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

fn generate_turso_test(
    test_name: &Ident,
    schema_type: &Type,
    test_body: &Block,
    bindings: &dyn Fn(TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{test_name}_turso"), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "turso";
    let setup_call = quote! { turso_setup::setup_db::<#schema_type>().await };
    let prelude = bindings(setup_call);
    let assertion_macros = common_assertion_macros();
    quote! {
        #[cfg(feature = "turso")]
        mod #test_fn_name {
            use super::*;
            #[tokio::test]
            async fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::turso_setup;
                #prelude
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
                    ($builder:expr => all_as) => {{
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
                    ($builder:expr => get_as) => {{
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
                        let __params_str = stringify!($params);
                        match $prepared.execute($conn, $params).await {
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
                        let __params_str = stringify!($params);
                        match $prepared.all($conn, $params).await {
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
                        let __params_str = stringify!($params);
                        match $prepared.get($conn, $params).await {
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
                #assertion_macros
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
    bindings: &dyn Fn(TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{test_name}_postgres_sync"), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "postgres-sync";
    let setup_call = quote! { postgres_sync_setup::setup_db::<#schema_type>() };
    let prelude = bindings(setup_call);
    let assertion_macros = common_assertion_macros();
    quote! {
        #[cfg(feature = "postgres-sync")]
        mod #test_fn_name {
            use super::*;
            #[test]
            fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::postgres_sync_setup;
                #prelude
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
                    ($builder:expr => all_as) => {{
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
                    ($builder:expr => get_as) => {{
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
                        let __params_str = stringify!($params);
                        match $prepared.execute($conn, $params) {
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
                        let __params_str = stringify!($params);
                        match $prepared.all($conn, $params) {
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
                        let __params_str = stringify!($params);
                        match $prepared.get($conn, $params) {
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
                #assertion_macros
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
    bindings: &dyn Fn(TokenStream2) -> TokenStream2,
) -> TokenStream2 {
    let test_fn_name = syn::Ident::new(&format!("{test_name}_tokio_postgres"), test_name.span());
    let test_name_str = test_name.to_string();
    let driver_name = "tokio-postgres";
    let setup_call = quote! { tokio_postgres_setup::setup_db::<#schema_type>().await };
    let prelude = bindings(setup_call);
    let assertion_macros = common_assertion_macros();
    quote! {
        #[cfg(feature = "tokio-postgres")]
        mod #test_fn_name {
            use super::*;
            #[tokio::test]
            async fn run() -> std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::tokio_postgres_setup;
                #prelude
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
                    ($builder:expr => all_as) => {{
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
                    ($builder:expr => get_as) => {{
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
                        let __params_str = stringify!($params);
                        match $prepared.execute($conn, $params).await {
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
                        let __params_str = stringify!($params);
                        match $prepared.all($conn, $params).await {
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
                        let __params_str = stringify!($params);
                        match $prepared.get($conn, $params).await {
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
                #assertion_macros
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
