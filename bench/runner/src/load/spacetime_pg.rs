use super::*;
use crate::workload_terms::{CUSTOMER_SEARCH_TERMS, PRODUCT_SEARCH_TERMS};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use rand::Rng;
use rand::SeedableRng;
use rand::rngs::StdRng;
use std::error::Error;
use std::sync::Arc;
use std::sync::atomic::AtomicUsize;
use tokio::sync::Mutex;
use tokio_postgres::SimpleQueryMessage;

/// SpacetimeDB PGWire target (Northwind schema).
///
/// Uses tokio-postgres `simple_query()` (Simple Query Protocol) because
/// SpacetimeDB's PGWire interface does not support Extended Query Protocol
/// (no parameterized queries / prepared statements).
///
/// Tables are defined by the SpacetimeDB module (bench/targets/spacetime-module),
/// which must be published before running this target. PGWire does not support
/// DDL (CREATE TABLE / DROP TABLE) — only DML (SELECT, INSERT, UPDATE, DELETE).
///
/// Values are safely embedded in SQL strings — all inputs are controlled
/// by the benchmark runner (integer indices, deterministic seed strings).

// Response types (camelCase JSON, matching other drivers)
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CustomerResponse {
    id: i32,
    company_name: String,
    contact_name: String,
    contact_title: String,
    address: String,
    city: String,
    postal_code: Option<String>,
    region: Option<String>,
    country: String,
    phone: String,
    fax: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmployeeResponse {
    id: i32,
    last_name: String,
    first_name: Option<String>,
    title: String,
    title_of_courtesy: String,
    birth_date: i64,
    hire_date: i64,
    address: String,
    city: String,
    postal_code: String,
    country: String,
    home_phone: String,
    extension: i32,
    notes: String,
    recipient_id: Option<i32>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SupplierResponse {
    id: i32,
    company_name: String,
    contact_name: String,
    contact_title: String,
    address: String,
    city: String,
    region: Option<String>,
    postal_code: String,
    country: String,
    phone: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductResponse {
    id: i32,
    name: String,
    qt_per_unit: String,
    unit_price: f64,
    units_in_stock: i32,
    units_on_order: i32,
    reorder_level: i32,
    discontinued: i32,
    supplier_id: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductWithSupplierResponse {
    id: i32,
    name: String,
    qt_per_unit: String,
    unit_price: f64,
    units_in_stock: i32,
    units_on_order: i32,
    reorder_level: i32,
    discontinued: i32,
    supplier_id: i32,
    supplier: SupplierResponse,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderWithDetailsResponse {
    id: i32,
    shipped_date: Option<i64>,
    ship_name: String,
    ship_city: String,
    ship_country: String,
    products_count: i32,
    quantity_sum: f64,
    total_price: f64,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OrderDetailResponse {
    unit_price: f64,
    quantity: i32,
    discount: f64,
    order_id: i32,
    product_id: i32,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct OrderDetailProductResponse {
    unit_price: f64,
    quantity: i32,
    discount: f64,
    order_id: i32,
    product_id: i32,
    product_name: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SingleOrderWithDetailsResponse {
    id: i32,
    order_date: i64,
    required_date: i64,
    shipped_date: Option<i64>,
    ship_via: i32,
    freight: f64,
    ship_name: String,
    ship_city: String,
    ship_region: Option<String>,
    ship_postal_code: Option<String>,
    ship_country: String,
    customer_id: i32,
    employee_id: i32,
    details: Vec<OrderDetailResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SingleOrderWithDetailsAndProductsResponse {
    id: i32,
    order_date: i64,
    required_date: i64,
    shipped_date: Option<i64>,
    ship_via: i32,
    freight: f64,
    ship_name: String,
    ship_city: String,
    ship_region: Option<String>,
    ship_postal_code: Option<String>,
    ship_country: String,
    customer_id: i32,
    employee_id: i32,
    details: Vec<OrderDetailProductResponse>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmployeeWithRecipientResponse {
    id: i32,
    last_name: String,
    first_name: Option<String>,
    title: String,
    title_of_courtesy: String,
    birth_date: i64,
    hire_date: i64,
    address: String,
    city: String,
    postal_code: String,
    country: String,
    home_phone: String,
    extension: i32,
    notes: String,
    recipient_id: Option<i32>,
    recipient_last_name: Option<String>,
    recipient_first_name: Option<String>,
}

#[derive(Clone)]
struct AppState {
    clients: Arc<Vec<Arc<Mutex<tokio_postgres::Client>>>>,
    next: Arc<AtomicUsize>,
}

// SpacetimeDB PGWire has a low maximum SQL string length. Keep batches
// conservative so wide customer/order rows stay under the query limit.
const INSERT_BATCH_ROWS: usize = 100;
const DETAILS_PER_ORDER: usize = 6;
const SPACETIME_PG_POOL_SIZE: usize = 4;

async fn insert_rows(
    client: &tokio_postgres::Client,
    table: &str,
    columns: &str,
    rows: &mut Vec<String>,
) -> Result<(), Fail> {
    if rows.is_empty() {
        return Ok(());
    }

    let values = rows.join(", ");
    let sql = format!("INSERT INTO {table} ({columns}) VALUES {values}");
    let batch_rows = rows.len();
    let sql_len = sql.len();
    client.simple_query(&sql).await.map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!(
                "seed {table} batch rows={batch_rows} sql_bytes={sql_len}: {e} (source: {:?})",
                e.source()
            ),
        )
    })?;
    rows.clear();
    Ok(())
}

pub async fn serve(seed: u64) -> Result<ServerHandle, Fail> {
    let config = spacetime_pg_config();
    eprintln!(
        "spacetime pg: connecting to {}:{}",
        std::env::var("SPACETIME_PG_HOST").unwrap_or_else(|_| "127.0.0.1".into()),
        std::env::var("SPACETIME_PG_PORT").unwrap_or_else(|_| "5433".into())
    );
    let (client, connection) = config.connect(tokio_postgres::NoTls).await.map_err(|e| {
        Fail::new(
            Code::RunFail,
            format!("spacetime pg connect: {e} (source: {:?})", e.source()),
        )
    })?;

    tokio::spawn(async move {
        if let Err(e) = connection.await {
            eprintln!("spacetime pg connection error: {e}");
        }
    });

    // Clear existing data (tables are defined by the SpacetimeDB module)
    for table in &[
        "order_details",
        "orders",
        "products",
        "suppliers",
        "employees",
        "customers",
    ] {
        client
            .simple_query(&format!("DELETE FROM {table}"))
            .await
            .map_err(|e| {
                Fail::new(
                    Code::RunFail,
                    format!("clear {table}: {e} (source: {:?})", e.source()),
                )
            })?;
    }

    // Generate deterministic seed data with the same cardinalities used by every target.
    let mut rng = StdRng::seed_from_u64(seed);

    const NUM_CUSTOMERS: usize = SEED_CUSTOMERS;
    const NUM_EMPLOYEES: usize = SEED_EMPLOYEES;
    const NUM_SUPPLIERS: usize = SEED_SUPPLIERS;
    const NUM_PRODUCTS: usize = SEED_PRODUCTS;
    const NUM_ORDERS: usize = SEED_ORDERS;

    // Seed customers
    let mut customer_rows = Vec::with_capacity(INSERT_BATCH_ROWS);
    for i in 0..NUM_CUSTOMERS {
        let id = i + 1;
        let postal_code = if rng.random_bool(0.8) {
            format!("{:05}", rng.random_range(10000..99999))
        } else {
            String::new()
        };
        let region = if rng.random_bool(0.5) {
            format!("Region-{}", rng.random_range(1..50))
        } else {
            String::new()
        };
        let fax = if rng.random_bool(0.3) {
            format!("555-{:04}", rng.random_range(1000..9999))
        } else {
            String::new()
        };
        let row = format!(
            "({}, '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}')",
            id,
            sql_escape(&format!(
                "C-{}-{i}",
                CUSTOMER_SEARCH_TERMS[i % CUSTOMER_SEARCH_TERMS.len()]
            )),
            sql_escape(&format!("Contact-{i}")),
            sql_escape(&format!("Title-{i}")),
            sql_escape(&format!("{i} Main St")),
            sql_escape(&format!("City-{i}")),
            sql_escape(&postal_code),
            sql_escape(&region),
            sql_escape(&format!("Country-{}", i % 50)),
            sql_escape(&format!("555-{i:04}")),
            sql_escape(&fax),
        );
        customer_rows.push(row);
        if customer_rows.len() >= INSERT_BATCH_ROWS {
            insert_rows(
                &client,
                "customers",
                "id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax",
                &mut customer_rows,
            )
            .await?;
        }
    }
    insert_rows(
        &client,
        "customers",
        "id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax",
        &mut customer_rows,
    )
    .await?;

    // Seed employees
    let mut employee_rows = Vec::with_capacity(INSERT_BATCH_ROWS);
    for i in 0..NUM_EMPLOYEES {
        let id = i + 1;
        let first_name = if rng.random_bool(0.9) {
            format!("First-{i}")
        } else {
            String::new()
        };
        let courtesy = ["Mr.", "Ms.", "Mrs.", "Dr."][i % 4];
        // SpacetimeDB PGWire rejects negative integer literals in INSERT values,
        // so keep date-like timestamps positive for this target.
        let birth_date = rng.random_range(315_360_000..946_080_000_i64);
        let hire_date = rng.random_range(946_684_800..1_672_531_200_i64);
        let extension = rng.random_range(100..9999_i32);
        let recipient_id: i32 = if i > 0 {
            rng.random_range(1..=i as i32)
        } else {
            0
        };
        let row = format!(
            "({}, '{}', '{}', '{}', '{}', {}, {}, '{}', '{}', '{}', '{}', '{}', {}, '{}', {})",
            id,
            sql_escape(&format!("Last-{i}")),
            sql_escape(&first_name),
            sql_escape(&format!("Title-{i}")),
            sql_escape(courtesy),
            birth_date,
            hire_date,
            sql_escape(&format!("{i} Elm St")),
            sql_escape(&format!("City-{i}")),
            sql_escape(&format!("{:05}", rng.random_range(10000..99999))),
            sql_escape(&format!("Country-{}", i % 20)),
            sql_escape(&format!("555-{i:04}")),
            extension,
            sql_escape(&format!("Notes for employee {i}")),
            recipient_id,
        );
        employee_rows.push(row);
        if employee_rows.len() >= INSERT_BATCH_ROWS {
            insert_rows(
                &client,
                "employees",
                "id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id",
                &mut employee_rows,
            )
            .await?;
        }
    }
    insert_rows(
        &client,
        "employees",
        "id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id",
        &mut employee_rows,
    )
    .await?;

    // Seed suppliers
    let mut supplier_rows = Vec::with_capacity(INSERT_BATCH_ROWS);
    for i in 0..NUM_SUPPLIERS {
        let id = i + 1;
        let region = if rng.random_bool(0.5) {
            format!("Region-{}", rng.random_range(1..50))
        } else {
            String::new()
        };
        let row = format!(
            "({}, '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}', '{}')",
            id,
            sql_escape(&format!("Supplier-{i}")),
            sql_escape(&format!("Contact-{i}")),
            sql_escape(&format!("Title-{i}")),
            sql_escape(&format!("{i} Oak Ave")),
            sql_escape(&format!("City-{i}")),
            sql_escape(&region),
            sql_escape(&format!("{:05}", rng.random_range(10000..99999))),
            sql_escape(&format!("Country-{}", i % 20)),
            sql_escape(&format!("555-{i:04}")),
        );
        supplier_rows.push(row);
        if supplier_rows.len() >= INSERT_BATCH_ROWS {
            insert_rows(
                &client,
                "suppliers",
                "id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone",
                &mut supplier_rows,
            )
            .await?;
        }
    }
    insert_rows(
        &client,
        "suppliers",
        "id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone",
        &mut supplier_rows,
    )
    .await?;

    // Seed products
    let mut product_rows = Vec::with_capacity(INSERT_BATCH_ROWS);
    for i in 0..NUM_PRODUCTS {
        let id = i + 1;
        let unit_price = (rng.random_range(1.0..500.0_f64) * 100.0).round() / 100.0;
        let units_in_stock = rng.random_range(0..200_i32);
        let units_on_order = rng.random_range(0..100_i32);
        let reorder_level = rng.random_range(0..50_i32);
        let discontinued: i32 = if rng.random_bool(0.1) { 1 } else { 0 };
        let supplier_id = rng.random_range(1..=NUM_SUPPLIERS as i32);
        let row = format!(
            "({}, '{}', '{}', {}, {}, {}, {}, {}, {})",
            id,
            sql_escape(&format!(
                "P-{}-{i}",
                PRODUCT_SEARCH_TERMS[i % PRODUCT_SEARCH_TERMS.len()]
            )),
            sql_escape(&format!(
                "{} boxes x {} bags",
                rng.random_range(1..20),
                rng.random_range(1..50)
            )),
            unit_price,
            units_in_stock,
            units_on_order,
            reorder_level,
            discontinued,
            supplier_id,
        );
        product_rows.push(row);
        if product_rows.len() >= INSERT_BATCH_ROWS {
            insert_rows(
                &client,
                "products",
                "id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id",
                &mut product_rows,
            )
            .await?;
        }
    }
    insert_rows(
        &client,
        "products",
        "id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id",
        &mut product_rows,
    )
    .await?;

    // Seed orders
    let mut order_rows = Vec::with_capacity(INSERT_BATCH_ROWS);
    for i in 0..NUM_ORDERS {
        let id = i + 1;
        let order_date = rng.random_range(946_684_800..1_672_531_200_i64);
        let required_date = order_date + rng.random_range(604_800..2_592_000);
        let shipped_date: i64 = if rng.random_bool(0.85) {
            order_date + rng.random_range(86_400..1_209_600)
        } else {
            0
        };
        let ship_via = rng.random_range(1..=3_i32);
        let freight = (rng.random_range(0.5..500.0_f64) * 100.0).round() / 100.0;
        let ship_region = if rng.random_bool(0.5) {
            format!("Region-{}", rng.random_range(1..50))
        } else {
            String::new()
        };
        let ship_postal_code = if rng.random_bool(0.8) {
            format!("{:05}", rng.random_range(10000..99999))
        } else {
            String::new()
        };
        let customer_id = rng.random_range(1..=NUM_CUSTOMERS as i32);
        let employee_id = rng.random_range(1..=NUM_EMPLOYEES as i32);
        let row = format!(
            "({}, {}, {}, {}, {}, {}, '{}', '{}', '{}', '{}', '{}', {}, {})",
            id,
            order_date,
            required_date,
            shipped_date,
            ship_via,
            freight,
            sql_escape(&format!("Ship-{i}")),
            sql_escape(&format!("City-{i}")),
            sql_escape(&ship_region),
            sql_escape(&ship_postal_code),
            sql_escape(&format!("Country-{}", i % 50)),
            customer_id,
            employee_id,
        );
        order_rows.push(row);
        if order_rows.len() >= INSERT_BATCH_ROWS {
            insert_rows(
                &client,
                "orders",
                "id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id",
                &mut order_rows,
            )
            .await?;
        }
    }
    insert_rows(
        &client,
        "orders",
        "id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id",
        &mut order_rows,
    )
    .await?;

    // Seed order details with the same fixed details-per-order cardinality as the shared dataset.
    let mut detail_rows = Vec::with_capacity(INSERT_BATCH_ROWS);
    for order_i in 0..NUM_ORDERS {
        let order_id = (order_i + 1) as i32;
        for detail_i in 0..DETAILS_PER_ORDER {
            let id = order_i * DETAILS_PER_ORDER + detail_i + 1;
            let unit_price = (rng.random_range(1.0..200.0_f64) * 100.0).round() / 100.0;
            let quantity = rng.random_range(1..=100_i32);
            let discount = if rng.random_bool(0.3) {
                (rng.random_range(0.05..0.25_f64) * 100.0).round() / 100.0
            } else {
                0.0
            };
            let product_id = rng.random_range(1..=NUM_PRODUCTS as i32);
            let row = format!(
                "({}, {}, {}, {}, {}, {})",
                id, unit_price, quantity, discount, order_id, product_id,
            );
            detail_rows.push(row);
            if detail_rows.len() >= INSERT_BATCH_ROWS {
                insert_rows(
                    &client,
                    "order_details",
                    "id, unit_price, quantity, discount, order_id, product_id",
                    &mut detail_rows,
                )
                .await?;
            }
        }
    }
    insert_rows(
        &client,
        "order_details",
        "id, unit_price, quantity, discount, order_id, product_id",
        &mut detail_rows,
    )
    .await?;

    let pool_size = super::configured_pool_size(SPACETIME_PG_POOL_SIZE);
    let mut clients = Vec::with_capacity(pool_size);
    clients.push(Arc::new(Mutex::new(client)));
    for idx in 1..pool_size {
        let (client, connection) = config.connect(tokio_postgres::NoTls).await.map_err(|e| {
            Fail::new(
                Code::RunFail,
                format!(
                    "spacetime pg pool connect #{idx}: {e} (source: {:?})",
                    e.source()
                ),
            )
        })?;
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("spacetime pg pooled connection #{idx} error: {e}");
            }
        });
        clients.push(Arc::new(Mutex::new(client)));
    }

    let state = AppState {
        clients: Arc::new(clients),
        next: Arc::new(AtomicUsize::new(0)),
    };
    let router = Router::new()
        .route("/stats", get(stats))
        .route("/customers", get(customers))
        .route("/customer-by-id", get(customer_by_id))
        .route("/employees", get(employees_handler))
        .route("/suppliers", get(suppliers_handler))
        .route("/supplier-by-id", get(supplier_by_id))
        .route("/products", get(products_handler))
        .route("/employee-with-recipient", get(employee_with_recipient))
        .route("/product-with-supplier", get(product_with_supplier))
        .route("/orders-with-details", get(orders_with_details))
        .route("/order-with-details", get(order_with_details))
        .route(
            "/order-with-details-and-products",
            get(order_with_details_and_products),
        )
        .route("/search-customer", get(search_customer))
        .route("/search-product", get(search_product))
        .with_state(state);

    spawn_server(router).await
}

/// Extract row data from `SimpleQueryMessage` responses.
fn extract_rows(messages: Vec<SimpleQueryMessage>) -> Vec<Vec<Option<String>>> {
    messages
        .into_iter()
        .filter_map(|m| match m {
            SimpleQueryMessage::Row(row) => {
                let ncols = row.columns().len();
                let vals: Vec<Option<String>> =
                    (0..ncols).map(|i| row.get(i).map(str::to_string)).collect();
                Some(vals)
            }
            _ => None,
        })
        .collect()
}

fn col_str(cols: &[Option<String>], idx: usize) -> String {
    cols.get(idx).and_then(|v| v.clone()).unwrap_or_default()
}

fn col_opt(cols: &[Option<String>], idx: usize) -> Option<String> {
    cols.get(idx)
        .and_then(|v| v.clone())
        .filter(|s| !s.is_empty())
}

fn col_i32(cols: &[Option<String>], idx: usize) -> i32 {
    cols.get(idx)
        .and_then(|v| v.as_deref())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn col_i64(cols: &[Option<String>], idx: usize) -> i64 {
    cols.get(idx)
        .and_then(|v| v.as_deref())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0)
}

fn col_i64_opt(cols: &[Option<String>], idx: usize) -> Option<i64> {
    let v = col_i64(cols, idx);
    if v == 0 { None } else { Some(v) }
}

fn sort_by_i32_col(rows: &mut [Vec<Option<String>>], idx: usize) {
    rows.sort_by_key(|row| col_i32(row, idx));
}

fn col_i32_opt(cols: &[Option<String>], idx: usize) -> Option<i32> {
    let v = col_i32(cols, idx);
    if v == 0 { None } else { Some(v) }
}

fn col_f64(cols: &[Option<String>], idx: usize) -> f64 {
    cols.get(idx)
        .and_then(|v| v.as_deref())
        .and_then(|s| s.parse().ok())
        .unwrap_or(0.0)
}

fn contains_term(value: &str, term_lower: &str) -> bool {
    term_lower.is_empty() || value.to_ascii_lowercase().contains(term_lower)
}

#[debug_handler(state = AppState)]
async fn stats(_: State<AppState>) -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    Json(cpu_usage(&sys))
}

async fn pg_query(state: &AppState, sql: &str) -> Result<Vec<SimpleQueryMessage>, StatusCode> {
    let idx = state.next.fetch_add(1, Ordering::Relaxed) % state.clients.len();
    let client = state.clients[idx].lock().await;
    client.simple_query(sql).await.map_err(|e| {
        eprintln!("spacetime pg query failed: {e} (source: {:?})", e.source());
        StatusCode::INTERNAL_SERVER_ERROR
    })
}

async fn pg_rows(state: &AppState, sql: &str) -> Result<Vec<Vec<Option<String>>>, StatusCode> {
    Ok(extract_rows(pg_query(state, sql).await?))
}

async fn pg_sorted_rows(
    state: &AppState,
    sql: &str,
    sort_col: usize,
) -> Result<Vec<Vec<Option<String>>>, StatusCode> {
    // SpacetimeDB PGWire only supports a subset of PostgreSQL SQL. Keep the
    // HTTP contract intact by doing stable ordering in the target process.
    let mut rows = pg_rows(state, sql).await?;
    sort_by_i32_col(&mut rows, sort_col);
    Ok(rows)
}

fn id_window(offset: usize, limit: usize) -> Option<(usize, usize)> {
    if limit == 0 {
        return None;
    }

    let start = offset.saturating_add(1);
    let end = offset.saturating_add(limit);
    Some((start, end))
}

#[debug_handler(state = AppState)]
async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50);
    let offset = params.offset();
    let rows = if let Some((start, end)) = id_window(offset, limit) {
        let sql = format!(
            "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE id >= {start} AND id <= {end}"
        );
        pg_sorted_rows(&state, &sql, 0).await?
    } else {
        Vec::new()
    };
    let resp: Vec<CustomerResponse> = rows
        .into_iter()
        .map(|c| CustomerResponse {
            id: col_i32(&c, 0),
            company_name: col_str(&c, 1),
            contact_name: col_str(&c, 2),
            contact_title: col_str(&c, 3),
            address: col_str(&c, 4),
            city: col_str(&c, 5),
            postal_code: col_opt(&c, 6),
            region: col_opt(&c, 7),
            country: col_str(&c, 8),
            phone: col_str(&c, 9),
            fax: col_opt(&c, 10),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(10000);
    let sql = format!(
        "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE id = {id}"
    );
    let messages = pg_query(&state, &sql).await?;
    let resp: Vec<CustomerResponse> = extract_rows(messages)
        .into_iter()
        .map(|c| CustomerResponse {
            id: col_i32(&c, 0),
            company_name: col_str(&c, 1),
            contact_name: col_str(&c, 2),
            contact_title: col_str(&c, 3),
            address: col_str(&c, 4),
            city: col_str(&c, 5),
            postal_code: col_opt(&c, 6),
            region: col_opt(&c, 7),
            country: col_str(&c, 8),
            phone: col_str(&c, 9),
            fax: col_opt(&c, 10),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn employees_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50);
    let offset = params.offset();
    let rows = if let Some((start, end)) = id_window(offset, limit) {
        let sql = format!(
            "SELECT id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id FROM employees WHERE id >= {start} AND id <= {end}"
        );
        pg_sorted_rows(&state, &sql, 0).await?
    } else {
        Vec::new()
    };
    let resp: Vec<EmployeeResponse> = rows
        .into_iter()
        .map(|c| EmployeeResponse {
            id: col_i32(&c, 0),
            last_name: col_str(&c, 1),
            first_name: col_opt(&c, 2),
            title: col_str(&c, 3),
            title_of_courtesy: col_str(&c, 4),
            birth_date: col_i64(&c, 5),
            hire_date: col_i64(&c, 6),
            address: col_str(&c, 7),
            city: col_str(&c, 8),
            postal_code: col_str(&c, 9),
            country: col_str(&c, 10),
            home_phone: col_str(&c, 11),
            extension: col_i32(&c, 12),
            notes: col_str(&c, 13),
            recipient_id: col_i32_opt(&c, 14),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn suppliers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50);
    let offset = params.offset();
    let rows = if let Some((start, end)) = id_window(offset, limit) {
        let sql = format!(
            "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers WHERE id >= {start} AND id <= {end}"
        );
        pg_sorted_rows(&state, &sql, 0).await?
    } else {
        Vec::new()
    };
    let resp: Vec<SupplierResponse> = rows
        .into_iter()
        .map(|c| SupplierResponse {
            id: col_i32(&c, 0),
            company_name: col_str(&c, 1),
            contact_name: col_str(&c, 2),
            contact_title: col_str(&c, 3),
            address: col_str(&c, 4),
            city: col_str(&c, 5),
            region: col_opt(&c, 6),
            postal_code: col_str(&c, 7),
            country: col_str(&c, 8),
            phone: col_str(&c, 9),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(1000);
    let sql = format!(
        "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers WHERE id = {id}"
    );
    let messages = pg_query(&state, &sql).await?;
    let resp: Vec<SupplierResponse> = extract_rows(messages)
        .into_iter()
        .map(|c| SupplierResponse {
            id: col_i32(&c, 0),
            company_name: col_str(&c, 1),
            contact_name: col_str(&c, 2),
            contact_title: col_str(&c, 3),
            address: col_str(&c, 4),
            city: col_str(&c, 5),
            region: col_opt(&c, 6),
            postal_code: col_str(&c, 7),
            country: col_str(&c, 8),
            phone: col_str(&c, 9),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn products_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50);
    let offset = params.offset();
    let rows = if let Some((start, end)) = id_window(offset, limit) {
        let sql = format!(
            "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products WHERE id >= {start} AND id <= {end}"
        );
        pg_sorted_rows(&state, &sql, 0).await?
    } else {
        Vec::new()
    };
    let resp: Vec<ProductResponse> = rows
        .into_iter()
        .map(|c| ProductResponse {
            id: col_i32(&c, 0),
            name: col_str(&c, 1),
            qt_per_unit: col_str(&c, 2),
            unit_price: col_f64(&c, 3),
            units_in_stock: col_i32(&c, 4),
            units_on_order: col_i32(&c, 5),
            reorder_level: col_i32(&c, 6),
            discontinued: col_i32(&c, 7),
            supplier_id: col_i32(&c, 8),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(200);
    let employee_sql = format!(
        "SELECT id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, \
         address, city, postal_code, country, home_phone, extension, notes, recipient_id \
         FROM employees WHERE id = {id}"
    );
    let employee_rows = pg_rows(&state, &employee_sql).await?;
    let mut resp = Vec::with_capacity(employee_rows.len());
    for c in employee_rows {
        let recipient_id = col_i32_opt(&c, 14);
        let (recipient_last_name, recipient_first_name) = if let Some(recipient_id) = recipient_id {
            let recipient_sql =
                format!("SELECT last_name, first_name FROM employees WHERE id = {recipient_id}");
            let recipient_rows = pg_rows(&state, &recipient_sql).await?;
            match recipient_rows.first() {
                Some(row) => (col_opt(row, 0), col_opt(row, 1)),
                None => (None, None),
            }
        } else {
            (None, None)
        };

        resp.push(EmployeeWithRecipientResponse {
            id: col_i32(&c, 0),
            last_name: col_str(&c, 1),
            first_name: col_opt(&c, 2),
            title: col_str(&c, 3),
            title_of_courtesy: col_str(&c, 4),
            birth_date: col_i64(&c, 5),
            hire_date: col_i64(&c, 6),
            address: col_str(&c, 7),
            city: col_str(&c, 8),
            postal_code: col_str(&c, 9),
            country: col_str(&c, 10),
            home_phone: col_str(&c, 11),
            extension: col_i32(&c, 12),
            notes: col_str(&c, 13),
            recipient_id,
            recipient_last_name,
            recipient_first_name,
        });
    }
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(5000);
    let sql = format!(
        "SELECT p.id, p.name, p.qt_per_unit, p.unit_price, p.units_in_stock, p.units_on_order, \
         p.reorder_level, p.discontinued, p.supplier_id, \
         s.id AS supplier_pk, s.company_name, s.contact_name, s.contact_title, s.address, s.city, s.region, s.postal_code, s.country, s.phone \
         FROM products p INNER JOIN suppliers s ON p.supplier_id = s.id WHERE p.id = {id}"
    );
    let messages = pg_query(&state, &sql).await?;
    let resp: Vec<ProductWithSupplierResponse> = extract_rows(messages)
        .into_iter()
        .map(|c| ProductWithSupplierResponse {
            id: col_i32(&c, 0),
            name: col_str(&c, 1),
            qt_per_unit: col_str(&c, 2),
            unit_price: col_f64(&c, 3),
            units_in_stock: col_i32(&c, 4),
            units_on_order: col_i32(&c, 5),
            reorder_level: col_i32(&c, 6),
            discontinued: col_i32(&c, 7),
            supplier_id: col_i32(&c, 8),
            supplier: SupplierResponse {
                id: col_i32(&c, 9),
                company_name: col_str(&c, 10),
                contact_name: col_str(&c, 11),
                contact_title: col_str(&c, 12),
                address: col_str(&c, 13),
                city: col_str(&c, 14),
                region: col_opt(&c, 15),
                postal_code: col_str(&c, 16),
                country: col_str(&c, 17),
                phone: col_str(&c, 18),
            },
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50);
    let offset = params.offset();
    let Some((start, end)) = id_window(offset, limit) else {
        return Ok(Json(
            serde_json::to_value(Vec::<OrderWithDetailsResponse>::new()).unwrap(),
        ));
    };
    let order_sql = format!(
        "SELECT id, shipped_date, ship_name, ship_city, ship_country FROM orders WHERE id >= {start} AND id <= {end}"
    );
    let order_rows = pg_sorted_rows(&state, &order_sql, 0).await?;
    let detail_sql = format!(
        "SELECT quantity, unit_price, product_id, order_id FROM order_details WHERE order_id >= {start} AND order_id <= {end}"
    );
    let detail_rows = pg_rows(&state, &detail_sql).await?;

    let mut resp = Vec::with_capacity(order_rows.len());
    for c in order_rows {
        let id = col_i32(&c, 0);
        let products_count = detail_rows
            .iter()
            .filter(|row| col_i32(row, 3) == id && col_i32(row, 2) != 0)
            .count() as i32;
        let quantity_sum = detail_rows
            .iter()
            .filter(|row| col_i32(row, 3) == id)
            .map(|row| col_i32(row, 0) as f64)
            .sum::<f64>();
        let total_price = detail_rows
            .iter()
            .filter(|row| col_i32(row, 3) == id)
            .map(|row| col_i32(row, 0) as f64 * col_f64(row, 1))
            .sum::<f64>();
        resp.push(OrderWithDetailsResponse {
            id,
            shipped_date: col_i64_opt(&c, 1),
            ship_name: col_str(&c, 2),
            ship_city: col_str(&c, 3),
            ship_country: col_str(&c, 4),
            products_count,
            quantity_sum,
            total_price,
        });
    }
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(50000);
    let order_sql = format!(
        "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, \
         ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = {id}"
    );
    let detail_sql = format!(
        "SELECT unit_price, quantity, discount, order_id, product_id FROM order_details WHERE order_id = {id}"
    );
    let order_msgs = pg_query(&state, &order_sql).await?;
    let detail_msgs = pg_query(&state, &detail_sql).await?;
    let details: Vec<OrderDetailResponse> = extract_rows(detail_msgs)
        .into_iter()
        .map(|c| OrderDetailResponse {
            unit_price: col_f64(&c, 0),
            quantity: col_i32(&c, 1),
            discount: col_f64(&c, 2),
            order_id: col_i32(&c, 3),
            product_id: col_i32(&c, 4),
        })
        .collect();
    let resp: Vec<SingleOrderWithDetailsResponse> = extract_rows(order_msgs)
        .into_iter()
        .map(|c| SingleOrderWithDetailsResponse {
            id: col_i32(&c, 0),
            order_date: col_i64(&c, 1),
            required_date: col_i64(&c, 2),
            shipped_date: col_i64_opt(&c, 3),
            ship_via: col_i32(&c, 4),
            freight: col_f64(&c, 5),
            ship_name: col_str(&c, 6),
            ship_city: col_str(&c, 7),
            ship_region: col_opt(&c, 8),
            ship_postal_code: col_opt(&c, 9),
            ship_country: col_str(&c, 10),
            customer_id: col_i32(&c, 11),
            employee_id: col_i32(&c, 12),
            details: details.clone(),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(50000);
    let order_sql = format!(
        "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, \
         ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = {id}"
    );
    let detail_sql = format!(
        "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name \
         FROM order_details d INNER JOIN products p ON d.product_id = p.id WHERE d.order_id = {id}"
    );
    let order_msgs = pg_query(&state, &order_sql).await?;
    let detail_msgs = pg_query(&state, &detail_sql).await?;
    let details: Vec<OrderDetailProductResponse> = extract_rows(detail_msgs)
        .into_iter()
        .map(|c| OrderDetailProductResponse {
            unit_price: col_f64(&c, 0),
            quantity: col_i32(&c, 1),
            discount: col_f64(&c, 2),
            order_id: col_i32(&c, 3),
            product_id: col_i32(&c, 4),
            product_name: col_str(&c, 5),
        })
        .collect();
    let resp: Vec<SingleOrderWithDetailsAndProductsResponse> = extract_rows(order_msgs)
        .into_iter()
        .map(|c| SingleOrderWithDetailsAndProductsResponse {
            id: col_i32(&c, 0),
            order_date: col_i64(&c, 1),
            required_date: col_i64(&c, 2),
            shipped_date: col_i64_opt(&c, 3),
            ship_via: col_i32(&c, 4),
            freight: col_f64(&c, 5),
            ship_name: col_str(&c, 6),
            ship_city: col_str(&c, 7),
            ship_region: col_opt(&c, 8),
            ship_postal_code: col_opt(&c, 9),
            ship_country: col_str(&c, 10),
            customer_id: col_i32(&c, 11),
            employee_id: col_i32(&c, 12),
            details: details.clone(),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

/// Build a `tokio_postgres::Config` for SpacetimeDB PGWire.
fn spacetime_pg_config() -> tokio_postgres::Config {
    let host = std::env::var("SPACETIME_PG_HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port: u16 = std::env::var("SPACETIME_PG_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(5433);
    let dbname = std::env::var("SPACETIME_MODULE").unwrap_or_else(|_| "bench-module".to_string());
    let token = spacetime_token();

    let mut config = tokio_postgres::Config::new();
    config.host(&host);
    config.port(port);
    config.dbname(&dbname);
    config.user(&dbname);
    config.password(&token);
    config
}

/// Read the SpacetimeDB identity token for PGWire auth.
fn spacetime_token() -> String {
    if let Ok(tok) = std::env::var("SPACETIME_TOKEN")
        && !tok.trim().is_empty()
    {
        return tok;
    }

    let home = std::env::var("HOME").unwrap_or_default();
    if home.is_empty() {
        return String::new();
    }
    let path = std::path::Path::new(&home)
        .join(".config")
        .join("spacetime")
        .join("cli.toml");
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return String::new(),
    };

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(val) = trimmed.strip_prefix("spacetimedb_token") {
            let val = val.trim();
            if let Some(val) = val.strip_prefix('=') {
                let val = val.trim().trim_matches('"').trim_matches('\'');
                if !val.is_empty() {
                    return val.to_string();
                }
            }
        }
    }

    String::new()
}

#[debug_handler(state = AppState)]
async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let term_lower = params.term.as_deref().unwrap_or("").to_ascii_lowercase();
    let sql = "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers";
    let resp: Vec<CustomerResponse> = pg_rows(&state, sql)
        .await?
        .into_iter()
        .filter(|c| contains_term(&col_str(c, 1), &term_lower))
        .map(|c| CustomerResponse {
            id: col_i32(&c, 0),
            company_name: col_str(&c, 1),
            contact_name: col_str(&c, 2),
            contact_title: col_str(&c, 3),
            address: col_str(&c, 4),
            city: col_str(&c, 5),
            postal_code: col_opt(&c, 6),
            region: col_opt(&c, 7),
            country: col_str(&c, 8),
            phone: col_str(&c, 9),
            fax: col_opt(&c, 10),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let term_lower = params.term.as_deref().unwrap_or("").to_ascii_lowercase();
    let sql = "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products";
    let resp: Vec<ProductResponse> = pg_rows(&state, sql)
        .await?
        .into_iter()
        .filter(|c| contains_term(&col_str(c, 1), &term_lower))
        .map(|c| ProductResponse {
            id: col_i32(&c, 0),
            name: col_str(&c, 1),
            qt_per_unit: col_str(&c, 2),
            unit_price: col_f64(&c, 3),
            units_in_stock: col_i32(&c, 4),
            units_on_order: col_i32(&c, 5),
            reorder_level: col_i32(&c, 6),
            discontinued: col_i32(&c, 7),
            supplier_id: col_i32(&c, 8),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

/// Escape single quotes for SQL string literals (Simple Query Protocol).
fn sql_escape(s: &str) -> String {
    s.replace('\'', "''")
}
