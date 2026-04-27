//! `#[drizzle::test]` attribute macro.
//!
//! Rewrites `fn body(db: &mut TestDb<Schema>) { ... }` into feature-gated
//! per-driver test modules. The schema type is extracted from the `<Schema>`
//! generic of `db`'s parameter type; a `schema` local is auto-bound in the
//! generated body so user code can destructure or project off it without
//! declaring it in the signature. Two things are handled here so user bodies
//! can be native Rust:
//!
//! 1. **Terminal method rewriting.** A [`BodyVisitor`] walks the body and
//!    rewrites an allowlisted set of method calls (`.execute()`, `.all()`,
//!    `.get()`, `.migrate()`, `.push()`, `.transaction(...)`, `.savepoint(...)`,
//!    and the `(conn, params)` prepared-statement forms) into a block that
//!    captures the rendered SQL, calls the terminal, records on `db`, and
//!    panics with a rich `fail_with_op` report on `Err`. Async drivers get
//!    `.await` injected on the terminal and `async move` on tx/savepoint
//!    closures.
//!
//! 2. **Body-local helpers.** Only two remain, both tiny:
//!    - `result!(expr)` — opt out of rewriting for one expression; returns
//!      the original `Result<T, E>`. Used for `.is_err()` assertions and
//!      query-level rollback inside tx closures.
//!    - `catch!(block)` — expect-panic wrapper; sync uses `catch_unwind`,
//!      async uses `futures_util::future::FutureExt::catch_unwind`.
//!
//! A panic hook is installed at the top of each `fn run()` that appends the
//! captured SQL trail to the panic message, so native `assert_eq!` / `panic!`
//! failures still get the full context.

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, quote};
use syn::{
    Attribute, Block, Expr, ExprClosure, ExprMethodCall, FnArg, Ident, ItemFn, Pat, PatIdent, Type,
    parse_quote,
    spanned::Spanned,
    visit_mut::{self, VisitMut},
};

// ===========================================================================
// Attribute entry
// ===========================================================================

#[derive(Clone, Copy)]
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

struct FnInput {
    outer_attrs: Vec<Attribute>,
    fn_name: Ident,
    db_pat: PatIdent,
    db_ty: Type,
    /// Extracted from the `Schema` generic argument of `db: &mut TestDb<Schema>`.
    /// Used for the `setup_db::<S>()` call and the compile-time
    /// `SQLSchemaImpl` bound assertion. Also bound to the local `schema`
    /// identifier inside the generated body so user code can reference it.
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

    let specs = match dialect {
        Dialect::Sqlite => sqlite_driver_specs(),
        Dialect::Postgres => postgres_driver_specs(),
    };
    let parts: Vec<TokenStream2> = specs
        .iter()
        .map(|spec| emit_driver_module(&fn_input, spec))
        .collect();
    let generated = quote! { #(#parts)* };

    Ok(wrap_with_schema_assertion(&fn_input, &generated))
}

fn parse_fn(item_fn: ItemFn) -> syn::Result<FnInput> {
    let sig = &item_fn.sig;
    if let Some(asyncness) = sig.asyncness {
        return Err(syn::Error::new(
            asyncness.span(),
            "`#[drizzle::test]` functions must be synchronous; async is injected per-driver by the macro",
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
    if inputs.len() != 1 {
        return Err(syn::Error::new(
            sig.inputs.span(),
            "`#[drizzle::test]` expects exactly 1 parameter: `db: &mut TestDb<Schema>` — `schema` is bound automatically inside the body",
        ));
    }

    let (db_pat, db_ty) = parse_typed_arg(inputs[0])?;
    if db_pat.ident != "db" {
        return Err(syn::Error::new(
            db_pat.ident.span(),
            "parameter must be named `db` (the rewritten body references it by name)",
        ));
    }
    let schema_ty = extract_schema_ty(&db_ty)?;

    Ok(FnInput {
        outer_attrs: item_fn.attrs,
        fn_name: sig.ident.clone(),
        db_pat,
        db_ty,
        schema_ty,
        body: *item_fn.block,
    })
}

/// Extract `Schema` from `&mut TestDb<Schema>` / `&TestDb<Schema>` /
/// `TestDb<Schema>`. The wrapper name isn't matched literally (some tests
/// use aliases) — we just require the outermost path segment to have
/// exactly one `<T>` angle-bracketed generic argument and return it.
fn extract_schema_ty(db_ty: &Type) -> syn::Result<Type> {
    let inner = match db_ty {
        Type::Reference(tr) => &*tr.elem,
        other => other,
    };
    let path = match inner {
        Type::Path(tp) => &tp.path,
        other => {
            return Err(syn::Error::new(
                other.span(),
                "`db` parameter type must be a path like `TestDb<Schema>` (or `&mut TestDb<Schema>`)",
            ));
        }
    };
    let last = path
        .segments
        .last()
        .ok_or_else(|| syn::Error::new(path.span(), "`db` type path is empty"))?;
    let args = match &last.arguments {
        syn::PathArguments::AngleBracketed(ab) => &ab.args,
        _ => {
            return Err(syn::Error::new(
                last.span(),
                format!(
                    "`db` parameter type `{}` must carry a schema generic, e.g. `TestDb<Schema>`",
                    last.ident
                ),
            ));
        }
    };
    let mut type_args = args.iter().filter_map(|a| match a {
        syn::GenericArgument::Type(t) => Some(t.clone()),
        _ => None,
    });
    let schema_ty = type_args.next().ok_or_else(|| {
        syn::Error::new(
            last.span(),
            format!(
                "`db` parameter type `{}<…>` must have at least one type argument (the schema)",
                last.ident
            ),
        )
    })?;
    Ok(schema_ty)
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
                "parameter pattern must be a simple identifier (e.g. `db`)",
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

fn wrap_with_schema_assertion(fn_input: &FnInput, generated: &TokenStream2) -> TokenStream2 {
    let schema_ty = &fn_input.schema_ty;
    let outer_attrs = &fn_input.outer_attrs;
    quote! {
        #(#outer_attrs)*
        #[allow(non_snake_case, dead_code)]
        const _: () = {
            fn __drizzle_test_assert_schema<
                S: drizzle::core::SQLSchemaImpl + ::core::default::Default + ::core::marker::Copy,
            >() {}
            let _ = __drizzle_test_assert_schema::<#schema_ty>;
        };
        #generated
    }
}

// ===========================================================================
// Per-driver emission
// ===========================================================================

struct DriverSpec {
    /// Cargo feature name (also used as the driver_name string).
    feature: &'static str,
    /// Suffix for the generated module (e.g. `foo_libsql`). Also used as
    /// the setup-module identifier (`{mod_suffix}_setup` in `common::helpers`).
    mod_suffix: &'static str,
    async_mode: bool,
    /// Per-driver `drizzle_client!()` expansion (postgres only; empty for sqlite).
    client_expr: TokenStream2,
}

fn sqlite_driver_specs() -> Vec<DriverSpec> {
    vec![
        DriverSpec {
            feature: "rusqlite",
            mod_suffix: "rusqlite",
            async_mode: false,
            client_expr: TokenStream2::new(),
        },
        DriverSpec {
            feature: "libsql",
            mod_suffix: "libsql",
            async_mode: true,
            client_expr: TokenStream2::new(),
        },
        DriverSpec {
            feature: "turso",
            mod_suffix: "turso",
            async_mode: true,
            client_expr: TokenStream2::new(),
        },
    ]
}

fn postgres_driver_specs() -> Vec<DriverSpec> {
    vec![
        DriverSpec {
            feature: "postgres-sync",
            mod_suffix: "postgres_sync",
            async_mode: false,
            // postgres-sync prepared stmts need `&mut Client`
            client_expr: quote!(db.conn_mut()),
        },
        DriverSpec {
            feature: "tokio-postgres",
            mod_suffix: "tokio_postgres",
            async_mode: true,
            // tokio-postgres prepared stmts take `&Client`
            client_expr: quote!(db.conn()),
        },
    ]
}

fn emit_driver_module(fn_input: &FnInput, spec: &DriverSpec) -> TokenStream2 {
    let mod_name = Ident::new(
        &format!("{}_{}", fn_input.fn_name, spec.mod_suffix),
        fn_input.fn_name.span(),
    );
    let test_name_str = fn_input.fn_name.to_string();
    let driver_name = spec.feature;
    let feature_gate = spec.feature;
    let (test_attr, run_sig, await_setup) = if spec.async_mode {
        (
            quote!(#[tokio::test]),
            quote!(async fn run()),
            quote!(.await),
        )
    } else {
        (quote!(#[test]), quote!(fn run()), TokenStream2::new())
    };
    let setup_mod = Ident::new(
        &format!("{}_setup", spec.mod_suffix),
        proc_macro2::Span::call_site(),
    );
    let schema_ty = &fn_input.schema_ty;
    let setup_call = quote!(#setup_mod::setup_db::<#schema_ty>() #await_setup);

    let db_binding = rebind_db_stmt(&fn_input.db_pat, &fn_input.db_ty);

    let body = rewrite_body(fn_input.body.clone(), spec.async_mode);
    let helper_macros = helper_macros(spec.async_mode, &spec.client_expr);
    let panic_hook = install_panic_hook();

    quote! {
        #[cfg(feature = #feature_gate)]
        mod #mod_name {
            use super::*;
            #test_attr
            #run_sig -> ::std::result::Result<(), drizzle::error::DrizzleError> {
                use crate::common::helpers::#setup_mod;
                let (mut __raw_db, __raw_schema) = #setup_call;
                #db_binding
                // `schema` is always injected — user bodies reference it by name
                // (e.g. `let Schema { users } = schema;`) without declaring it
                // in the signature. Schemas are `Copy`, so no move hazards.
                #[allow(unused_variables)]
                let schema = __raw_schema;
                let __test_name: &str = #test_name_str;
                let __driver_name: &str = #driver_name;
                #panic_hook
                #helper_macros

                #body

                ::std::result::Result::Ok(())
            }
        }
    }
}

/// Emit the `let` that binds the user-named `db` (honoring `&mut` / `&` / owned)
/// from the raw setup-returned value.
fn rebind_db_stmt(db_pat: &PatIdent, db_ty: &Type) -> TokenStream2 {
    let name = &db_pat.ident;
    let user_mut = &db_pat.mutability;
    let rhs = match db_ty {
        Type::Reference(tr) if tr.mutability.is_some() => quote!(&mut __raw_db),
        Type::Reference(_) => quote!(&__raw_db),
        _ => quote!(__raw_db),
    };
    quote! { #[allow(unused_variables)] let #user_mut #name = #rhs; }
}

fn install_panic_hook() -> TokenStream2 {
    // Installs the process-global panic hook once, then registers this test's
    // `(name, statements)` pair in a thread-local via an RAII guard so the
    // hook can find it. The guard's Drop restores the previous trail on
    // normal exit or unwinding. See `common::helpers::panic_hook` for why
    // this is safer than per-test `take_hook`/`set_hook`.
    quote! {
        crate::common::helpers::panic_hook::install_once();
        let __trail_guard = crate::common::helpers::panic_hook::TrailGuard::new(
            ::std::borrow::ToOwned::to_owned(__test_name),
            ::std::clone::Clone::clone(&db.statements),
        );
    }
}

fn helper_macros(async_mode: bool, client_expr: &TokenStream2) -> TokenStream2 {
    let client_macro = if client_expr.is_empty() {
        TokenStream2::new()
    } else {
        quote! {
            /// Per-driver client expression: resolves to `db.conn_mut()`
            /// (postgres-sync) or `db.conn()` (tokio-postgres).
            #[allow(unused_macros)]
            macro_rules! drizzle_client {
                () => { #client_expr };
            }
        }
    };

    // `result!` and `catch!` are kept as `macro_rules!` for the trivial case
    // where the body has no tx/savepoint calls inside them. The visitor also
    // descends into their token streams to apply the closure-asyncify rewrite
    // so `result!(tx.savepoint(|tx| { ... }))` works across sync and async.
    let core = if async_mode {
        quote! {
            /// Escape hatch: returns the raw `Result<T, E>` of `$e` (with `.await` for async).
            #[allow(unused_macros)]
            macro_rules! result {
                ($e:expr) => { $e.await };
            }
            /// Expect-panic wrapper: async form uses `FutureExt::catch_unwind`
            /// directly on the passed Future (mirrors the pre-attribute
            /// `drizzle_catch_unwind!` behavior). Do **not** wrap `$e` in an
            /// `async move { }` block — that would force move-captures on
            /// non-`Copy` values the test still uses after the `catch!` call.
            #[allow(unused_macros)]
            macro_rules! catch {
                ($b:block) => {{
                    use ::futures_util::future::FutureExt as _;
                    ::std::panic::AssertUnwindSafe(async move $b)
                        .catch_unwind()
                        .await
                }};
                ($e:expr) => {{
                    use ::futures_util::future::FutureExt as _;
                    ::std::panic::AssertUnwindSafe($e)
                        .catch_unwind()
                        .await
                }};
            }
        }
    } else {
        quote! {
            /// Escape hatch: returns the raw `Result<T, E>` of `$e`.
            #[allow(unused_macros)]
            macro_rules! result {
                ($e:expr) => { $e };
            }
            /// Expect-panic wrapper: sync form uses `std::panic::catch_unwind`.
            #[allow(unused_macros)]
            macro_rules! catch {
                ($b:block) => {
                    ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| $b))
                };
                ($e:expr) => {
                    ::std::panic::catch_unwind(::std::panic::AssertUnwindSafe(|| $e))
                };
            }
        }
    };
    quote! {
        #core
        #client_macro
    }
}

// ===========================================================================
// Body rewriter
// ===========================================================================

fn rewrite_body(mut body: Block, async_mode: bool) -> Block {
    let mut v = BodyVisitor {
        async_mode,
        asyncify_only: false,
    };
    v.visit_block_mut(&mut body);
    body
}

struct BodyVisitor {
    async_mode: bool,
    /// When true, skip panic-on-Err / SQL-capture rewrites and only asyncify
    /// closures under `.transaction(...)` / `.savepoint(...)`. Set while
    /// descending into `result!(...)` / `catch!(...)` so those escape hatches
    /// preserve their `Result` return type while still producing valid
    /// async-closure syntax for async drivers.
    asyncify_only: bool,
}

impl VisitMut for BodyVisitor {
    fn visit_expr_mut(&mut self, expr: &mut Expr) {
        // Intercept result!/catch! before default recursion so we can descend
        // into their token streams in asyncify-only mode.
        if let Expr::Macro(m) = expr {
            let path = &m.mac.path;
            if path.is_ident("result") || path.is_ident("catch") {
                if let Ok(mut inner) = syn::parse2::<Expr>(m.mac.tokens.clone()) {
                    let prev = self.asyncify_only;
                    self.asyncify_only = true;
                    self.visit_expr_mut(&mut inner);
                    self.asyncify_only = prev;
                    m.mac.tokens = inner.to_token_stream();
                } else if let Ok(mut inner) = syn::parse2::<Block>(m.mac.tokens.clone()) {
                    // `catch!({ block })` form.
                    let prev = self.asyncify_only;
                    self.asyncify_only = true;
                    self.visit_block_mut(&mut inner);
                    self.asyncify_only = prev;
                    m.mac.tokens = inner.to_token_stream();
                }
                return;
            }
        }

        // Recurse first so nested calls (inside arguments, closure bodies, etc.)
        // are rewritten before we consider this node.
        visit_mut::visit_expr_mut(self, expr);

        let Expr::MethodCall(mc) = expr else { return };
        let method_name = mc.method.to_string();
        let arg_count = mc.args.len();

        match (method_name.as_str(), arg_count) {
            ("transaction" | "savepoint", _) => {
                if self.async_mode {
                    asyncify_closures(&mut mc.args);
                }
                if !self.asyncify_only {
                    *expr = rewrite_no_capture_terminal(mc, self.async_mode);
                }
            }
            ("execute" | "all" | "get", 0) if !self.asyncify_only => {
                *expr = rewrite_zero_arg_terminal(mc, self.async_mode);
            }
            // Relations-query terminals — unlike `.all()`/`.get()`, their
            // receiver (the `db.query(...)` builder) does not implement
            // `ToSQL`, so fall back to the no-capture form that only reports
            // the operation on `Err`.
            ("find_many" | "find_first", 0) if !self.asyncify_only => {
                *expr = rewrite_no_capture_terminal(mc, self.async_mode);
            }
            ("execute" | "all" | "get", 1)
                if !self.asyncify_only && is_path_receiver(&mc.receiver) =>
            {
                *expr = rewrite_one_arg_terminal(mc, self.async_mode);
            }
            ("execute" | "all" | "get", 2)
                if !self.asyncify_only && is_path_receiver(&mc.receiver) =>
            {
                *expr = rewrite_prepared_terminal(mc, self.async_mode);
            }
            ("migrate" | "push", 0) if !self.asyncify_only => {
                *expr = rewrite_no_capture_terminal(mc, self.async_mode);
            }
            _ => {}
        }
    }
}

fn is_path_receiver(receiver: &Expr) -> bool {
    matches!(receiver, Expr::Path(_))
}

fn asyncify_closures(args: &mut syn::punctuated::Punctuated<Expr, syn::Token![,]>) {
    for arg in args.iter_mut() {
        if let Expr::Closure(c) = arg {
            ensure_async(c);
        }
    }
}

/// Insert `async` in front of a bare closure so it satisfies `AsyncFnOnce`
/// on the async drivers. We intentionally do **not** force `move` here: the
/// transaction/savepoint closures are polled on the stack (the outer
/// `transaction(..).await` outlives the closure), so capture-by-reference is
/// fine and avoids needlessly moving non-`Copy` captures into nested async
/// blocks — which would break patterns like reusing a prepared statement
/// inside both an outer `transaction` closure and an inner `savepoint` one.
fn ensure_async(c: &mut ExprClosure) {
    if c.asyncness.is_none() {
        c.asyncness = Some(syn::Token![async](proc_macro2::Span::call_site()));
    }
}

/// Shared match scaffold for all terminal-rewrites. `prelude` binds any
/// local variables (e.g. `__sql_str`, `__params_str`); `call` is the
/// method-call expression whose `Result<_, _>` is being matched; `capture`
/// toggles the `db.record_sql(...)` calls on both branches (for terminals
/// that have SQL to report) vs a bare pass-through (for `transaction` /
/// `savepoint` / `migrate` / `push` which don't render SQL themselves).
fn build_terminal_block(
    original: TokenStream2,
    prelude: TokenStream2,
    call: TokenStream2,
    await_kw: TokenStream2,
    capture: bool,
) -> Expr {
    let (ok_arm, err_record) = if capture {
        (
            quote! {
                db.record_sql(__op_str, &__sql_str, &__params_str, ::core::option::Option::None);
            },
            quote! {
                db.record_sql(
                    __op_str,
                    &__sql_str,
                    &__params_str,
                    ::core::option::Option::Some(::std::format!("{}", __e)),
                );
            },
        )
    } else {
        (TokenStream2::new(), TokenStream2::new())
    };
    parse_quote!({
        let __op_str: &str = stringify!(#original);
        #prelude
        match #call #await_kw {
            ::core::result::Result::Ok(__v) => {
                #ok_arm
                __v
            }
            ::core::result::Result::Err(__e) => {
                #err_record
                db.fail_with_op(__test_name, &__e, __op_str);
            }
        }
    })
}

/// Rewrites `<receiver>.<method>()` (0 args) with SQL capture + panic-on-Err.
fn rewrite_zero_arg_terminal(mc: &ExprMethodCall, async_mode: bool) -> Expr {
    let receiver = &mc.receiver;
    let method = &mc.method;
    let await_kw = if async_mode { quote!(.await) } else { quote!() };
    let original = mc.to_token_stream();
    let prelude = quote! {
        let __builder = #receiver;
        let __sql_obj = drizzle::core::ToSQL::to_sql(&__builder);
        let __sql_str = __sql_obj.sql().to_string();
        let __params_str = ::std::format!(
            "{:?}",
            __sql_obj.params().collect::<::std::vec::Vec<_>>()
        );
    };
    build_terminal_block(
        original,
        prelude,
        quote!(__builder.#method()),
        await_kw,
        true,
    )
}

/// Rewrites `<receiver>.<method>(query)` (1 arg) with SQL capture + panic-on-Err.
/// Used for `db.all(query)` / `db.get(query)` / `db.execute(query)` — the
/// driver-on-Drizzle form where the query is an already-built SQL-emitting
/// expression. Receiver must be a simple path expression (evaluated once; the
/// query arg is also evaluated once via a binding).
fn rewrite_one_arg_terminal(mc: &ExprMethodCall, async_mode: bool) -> Expr {
    let receiver = &mc.receiver;
    let method = &mc.method;
    let query_arg = mc.args.first().expect("1-arg dispatch guarantees arg[0]");
    let await_kw = if async_mode { quote!(.await) } else { quote!() };
    let original = mc.to_token_stream();
    let prelude = quote! {
        let __query = #query_arg;
        let __sql_obj = drizzle::core::ToSQL::to_sql(&__query);
        let __sql_str = __sql_obj.sql().to_string();
        let __params_str = ::std::format!(
            "{:?}",
            __sql_obj.params().collect::<::std::vec::Vec<_>>()
        );
    };
    build_terminal_block(
        original,
        prelude,
        quote!((#receiver).#method(__query)),
        await_kw,
        true,
    )
}

/// Rewrites `<prep>.execute(conn, params)` and the `all`/`get` variants.
/// Receiver must be a simple path expression (evaluated twice); complex
/// receivers fall through to the default visitor behavior.
fn rewrite_prepared_terminal(mc: &ExprMethodCall, async_mode: bool) -> Expr {
    let receiver = &mc.receiver;
    let method = &mc.method;
    let args = &mc.args;
    let params_arg = args
        .iter()
        .nth(1)
        .expect("2-arg dispatch guarantees arg[1]");
    let await_kw = if async_mode { quote!(.await) } else { quote!() };
    let original = mc.to_token_stream();
    // `__params_str` is a `String` here (via `.to_string()`) so the shared
    // scaffold's `&__params_str` coerces cleanly to `&str`.
    let prelude = quote! {
        let __params_str: ::std::string::String = ::std::string::ToString::to_string(stringify!(#params_arg));
        let __sql_str: ::std::string::String = (#receiver).to_string();
    };
    build_terminal_block(
        original,
        prelude,
        quote!((#receiver).#method(#args)),
        await_kw,
        true,
    )
}

/// Rewrites terminal-ish methods that have no SQL to capture
/// (`transaction`, `savepoint`, `migrate`, `push`). Just panic-on-Err with
/// a rich report.
fn rewrite_no_capture_terminal(mc: &ExprMethodCall, async_mode: bool) -> Expr {
    let await_kw = if async_mode { quote!(.await) } else { quote!() };
    let original = mc.to_token_stream();
    let call = mc.to_token_stream();
    build_terminal_block(original, TokenStream2::new(), call, await_kw, false)
}
