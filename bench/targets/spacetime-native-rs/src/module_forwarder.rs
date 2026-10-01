//! `spacetime-module-rs`: the HTTP contract served by module procedures.
//!
//! Every contract route's query logic lives in the SpacetimeDB module as a
//! `route_*` procedure (`bench/targets/spacetime-module/src/routes.rs`) that
//! returns the finished JSON response body. This process is only a forwarder:
//! it decodes the route's query parameters, makes exactly one
//! `POST /v1/database/<db>/call/<procedure>` request, and copies the returned
//! body through. There is no cache and no query logic here.
//!
//! In-flight calls are capped at `BENCH_POOL_SIZE` (the spec's `pool.max`) and
//! the HTTP client keeps that many keep-alive connections, which is the same
//! bound the PGWire target's connection pool puts on concurrent database work.

use axum::Router;
use axum::body::Body;
use axum::extract::{Query, State};
use axum::http::{Response, StatusCode, header};
use axum::routing::get;
use bytes::Bytes;
use http_body_util::{BodyExt, Full};
use hyper_util::client::legacy::Client;
use hyper_util::client::legacy::connect::HttpConnector;
use hyper_util::rt::TokioExecutor;
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::Semaphore;

const DEFAULT_POOL_SIZE: usize = 4;

#[derive(Debug, Deserialize)]
struct RouteParams {
    id: Option<i32>,
    limit: Option<u32>,
    offset: Option<u32>,
    term: Option<String>,
}

/// How a route's query parameters become the procedure's positional arguments.
#[derive(Clone, Copy)]
enum Args {
    Page,
    Id,
    Term,
}

impl Args {
    fn encode(self, params: &RouteParams) -> Value {
        match self {
            Args::Page => json!([params.offset.unwrap_or(0), params.limit.unwrap_or(50)]),
            Args::Id => json!([params.id.unwrap_or(1)]),
            Args::Term => json!([params.term.as_deref().unwrap_or("")]),
        }
    }
}

/// Contract route → module procedure.
const ROUTES: &[(&str, &str, Args)] = &[
    ("/customers", "route_customers", Args::Page),
    ("/customer-by-id", "route_customer_by_id", Args::Id),
    ("/employees", "route_employees", Args::Page),
    (
        "/employee-with-recipient",
        "route_employee_with_recipient",
        Args::Id,
    ),
    ("/suppliers", "route_suppliers", Args::Page),
    ("/supplier-by-id", "route_supplier_by_id", Args::Id),
    ("/products", "route_products", Args::Page),
    (
        "/product-with-supplier",
        "route_product_with_supplier",
        Args::Id,
    ),
    (
        "/orders-with-details",
        "route_orders_with_details",
        Args::Page,
    ),
    ("/order-with-details", "route_order_with_details", Args::Id),
    (
        "/order-with-details-and-products",
        "route_order_with_details_and_products",
        Args::Id,
    ),
    ("/search-customer", "route_search_customer", Args::Term),
    ("/search-product", "route_search_product", Args::Term),
];

struct ModuleClient {
    http: Client<HttpConnector, Full<Bytes>>,
    base: String,
    authorization: String,
    permits: Semaphore,
}

impl ModuleClient {
    /// One procedure (or reducer) call; returns the raw response body.
    async fn call(&self, name: &str, args: &Value) -> Result<Bytes, String> {
        let _permit = self
            .permits
            .acquire()
            .await
            .map_err(|err| err.to_string())?;
        let request = hyper::Request::post(format!("{}/call/{name}", self.base))
            .header(header::CONTENT_TYPE, "application/json")
            .header(header::AUTHORIZATION, &self.authorization)
            .body(Full::new(Bytes::from(args.to_string())))
            .map_err(|err| err.to_string())?;
        let response = self
            .http
            .request(request)
            .await
            .map_err(|err| err.to_string())?;
        let status = response.status();
        let body = response
            .into_body()
            .collect()
            .await
            .map_err(|err| err.to_string())?
            .to_bytes();
        if !status.is_success() {
            return Err(format!(
                "{name}: HTTP {status}: {}",
                String::from_utf8_lossy(&body)
            ));
        }
        Ok(body)
    }
}

#[derive(Clone)]
struct ForwardState {
    client: Arc<ModuleClient>,
    procedure: &'static str,
    args: Args,
}

/// The procedure returns the response body as a SATS `String`, which the
/// call endpoint encodes as a JSON string literal; unwrapping that literal is
/// the only transformation applied here.
async fn forward(
    State(state): State<ForwardState>,
    Query(params): Query<RouteParams>,
) -> Result<Response<Body>, StatusCode> {
    let raw = state
        .client
        .call(state.procedure, &state.args.encode(&params))
        .await
        .map_err(|err| {
            eprintln!("spacetime-module-rs: {err}");
            StatusCode::INTERNAL_SERVER_ERROR
        })?;
    let body: String =
        serde_json::from_slice(&raw).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Response::builder()
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(body))
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

/// `SPACETIME_URI` is shared with the SDK target and may be a `ws://` URL;
/// the call endpoint is the same host over HTTP.
fn http_base() -> String {
    let uri = std::env::var("SPACETIME_URI").unwrap_or_else(|_| "http://127.0.0.1:3000".into());
    let uri = uri.trim_end_matches('/');
    let uri = uri
        .strip_prefix("ws://")
        .map(|rest| format!("http://{rest}"))
        .unwrap_or_else(|| uri.to_string());
    let module = std::env::var("SPACETIME_MODULE").unwrap_or_else(|_| "bench-module".into());
    format!("{uri}/v1/database/{module}")
}

pub async fn router(seed_value: u64, trial: u32) -> Result<Router, String> {
    let pool = std::env::var("BENCH_POOL_SIZE")
        .ok()
        .and_then(|raw| raw.parse::<usize>().ok())
        .filter(|size| *size > 0)
        .unwrap_or(DEFAULT_POOL_SIZE);
    let mut connector = HttpConnector::new();
    connector.set_nodelay(true);
    let http = Client::builder(TokioExecutor::new())
        .pool_max_idle_per_host(pool)
        .build(connector);
    // Without a token every call would mint a fresh identity server-side.
    let token = super::spacetime_token().ok_or(
        "spacetime-module-rs needs SPACETIME_TOKEN (the CLI's spacetimedb_token)".to_string(),
    )?;
    let client = Arc::new(ModuleClient {
        http,
        base: http_base(),
        authorization: format!("Bearer {token}"),
        permits: Semaphore::new(pool),
    });

    client
        .call("seed", &json!([seed_value, trial]))
        .await
        .map_err(|err| format!("seed reducer failed: {err}"))?;
    eprintln!("spacetime-module-rs: seeded through the module reducer, pool={pool}");

    let mut app = Router::new().route("/stats", get(super::stats));
    for &(path, procedure, args) in ROUTES {
        let state = ForwardState {
            client: Arc::clone(&client),
            procedure,
            args,
        };
        app = app.route(path, get(forward).with_state(state));
    }
    Ok(app)
}
