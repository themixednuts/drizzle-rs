//! Toasty Turso backend.
//!
//! Creates a file-backed temp database, builds and seeds it with the same
//! drizzle-seed generator, seed and row counts the built-in Turso targets use
//! (see [`crate::seed_sqlite`]), then serves the HTTP contract through toasty's
//! `turso` driver. Seeding completes before `LISTENING` is printed.
//!
//! Toasty is never asked to create the schema (`push_schema()` / `reset_db()`
//! are never called), so it attaches to the already-populated file.
//!
//! Deviations from the canonical query catalog, declared in the spec's
//! `sql_variant`:
//!
//! * No user-reachable JOIN, so `/employee-with-recipient`,
//!   `/product-with-supplier` and `/order-with-details-and-products` load their
//!   related rows with extra round trips.
//! * No `GROUP BY` / `SUM`, so `/orders-with-details` fetches the order page
//!   plus the matching `order_details` id range and aggregates in Rust — the
//!   same shape the built-in Turso targets use.
//! * `ILIKE` is PostgreSQL-only in toasty, so the search routes use `LIKE`,
//!   which SQLite treats as ASCII case-insensitive — matching the built-in
//!   Turso targets' `... LIKE ?1`.

use crate::common::{DynError, QueryParams, configured_pool_size};
use crate::seed_sqlite::{
    SEED_CUSTOMERS, SEED_EMPLOYEES, SEED_ORDERS, SEED_PRODUCTS, SEED_SUPPLIERS,
};
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::Serialize;
use std::sync::Arc;
use toasty::Db;

type HttpResult<T> = Result<Json<T>, StatusCode>;

// ---------------------------------------------------------------------------
// Models (SQLite/Turso stores the Northwind dates as epoch integers)
// ---------------------------------------------------------------------------

#[derive(Debug, toasty::Model)]
#[table = "customers"]
pub struct Customer {
    #[key]
    pub id: i32,
    pub company_name: String,
    pub contact_name: String,
    pub contact_title: String,
    pub address: String,
    pub city: String,
    pub postal_code: Option<String>,
    pub region: Option<String>,
    pub country: String,
    pub phone: String,
    pub fax: Option<String>,
}

#[derive(Debug, toasty::Model)]
#[table = "employees"]
pub struct Employee {
    #[key]
    pub id: i32,
    pub last_name: String,
    pub first_name: Option<String>,
    pub title: String,
    pub title_of_courtesy: String,
    pub birth_date: i64,
    pub hire_date: i64,
    pub address: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub home_phone: String,
    pub extension: i32,
    pub notes: String,
    pub recipient_id: Option<i32>,
}

#[derive(Debug, toasty::Model)]
#[table = "suppliers"]
pub struct Supplier {
    #[key]
    pub id: i32,
    pub company_name: String,
    pub contact_name: String,
    pub contact_title: String,
    pub address: String,
    pub city: String,
    pub region: Option<String>,
    pub postal_code: String,
    pub country: String,
    pub phone: String,
}

#[derive(Debug, toasty::Model)]
#[table = "products"]
pub struct Product {
    #[key]
    pub id: i32,
    pub name: String,
    pub qt_per_unit: String,
    pub unit_price: f64,
    pub units_in_stock: i32,
    pub units_on_order: i32,
    pub reorder_level: i32,
    pub discontinued: i32,
    pub supplier_id: i32,
}

#[derive(Debug, toasty::Model)]
#[table = "orders"]
pub struct Order {
    #[key]
    pub id: i32,
    pub order_date: i64,
    pub required_date: i64,
    pub shipped_date: Option<i64>,
    pub ship_via: i32,
    pub freight: f64,
    pub ship_name: String,
    pub ship_city: String,
    pub ship_region: Option<String>,
    pub ship_postal_code: Option<String>,
    pub ship_country: String,
    pub customer_id: i32,
    pub employee_id: i32,
}

#[derive(Debug, toasty::Model)]
#[table = "order_details"]
#[key(order_id, product_id)]
pub struct OrderDetail {
    pub unit_price: f64,
    pub quantity: i32,
    pub discount: f64,
    pub order_id: i32,
    pub product_id: i32,
}

// ---------------------------------------------------------------------------
// Responses (identical JSON to the built-in Turso targets)
// ---------------------------------------------------------------------------

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

impl From<Customer> for CustomerResponse {
    fn from(row: Customer) -> Self {
        Self {
            id: row.id,
            company_name: row.company_name,
            contact_name: row.contact_name,
            contact_title: row.contact_title,
            address: row.address,
            city: row.city,
            postal_code: row.postal_code,
            region: row.region,
            country: row.country,
            phone: row.phone,
            fax: row.fax,
        }
    }
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

impl From<Employee> for EmployeeResponse {
    fn from(row: Employee) -> Self {
        Self {
            id: row.id,
            last_name: row.last_name,
            first_name: row.first_name,
            title: row.title,
            title_of_courtesy: row.title_of_courtesy,
            birth_date: row.birth_date,
            hire_date: row.hire_date,
            address: row.address,
            city: row.city,
            postal_code: row.postal_code,
            country: row.country,
            home_phone: row.home_phone,
            extension: row.extension,
            notes: row.notes,
            recipient_id: row.recipient_id,
        }
    }
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

impl From<Supplier> for SupplierResponse {
    fn from(row: Supplier) -> Self {
        Self {
            id: row.id,
            company_name: row.company_name,
            contact_name: row.contact_name,
            contact_title: row.contact_title,
            address: row.address,
            city: row.city,
            region: row.region,
            postal_code: row.postal_code,
            country: row.country,
            phone: row.phone,
        }
    }
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

impl From<Product> for ProductResponse {
    fn from(row: Product) -> Self {
        Self {
            id: row.id,
            name: row.name,
            qt_per_unit: row.qt_per_unit,
            unit_price: row.unit_price,
            units_in_stock: row.units_in_stock,
            units_on_order: row.units_on_order,
            reorder_level: row.reorder_level,
            discontinued: row.discontinued,
            supplier_id: row.supplier_id,
        }
    }
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

// ---------------------------------------------------------------------------
// Server
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    db: Db,
    /// Kept alive for the process lifetime: dropping it deletes the database
    /// file out from under toasty.
    _tmp: Arc<tempfile::TempDir>,
}

/// `Db` is a cheap `Arc` clone that shares the internal connection pool, but
/// `exec` needs `&mut`, so every handler takes its own handle.
fn handle(state: &AppState) -> Db {
    state.db.clone()
}

fn internal(_err: impl std::fmt::Debug) -> StatusCode {
    StatusCode::INTERNAL_SERVER_ERROR
}

pub async fn serve(seed: u64) -> Result<(), DynError> {
    // The Turso family runs a 4-connection pool; the runner overrides via
    // BENCH_POOL_SIZE.
    let pool_size = configured_pool_size(4);

    let tmp = tempfile::Builder::new()
        .prefix("toasty-bench-turso-")
        .tempdir()?;
    let db_path = tmp.path().join("bench.db");

    crate::seed_sqlite::create_and_seed(&db_path, seed).await?;

    let db = Db::builder()
        .models(toasty::models!(
            crate::turso_backend::Customer,
            crate::turso_backend::Employee,
            crate::turso_backend::Supplier,
            crate::turso_backend::Product,
            crate::turso_backend::Order,
            crate::turso_backend::OrderDetail
        ))
        .max_pool_size(pool_size)
        .build(toasty_driver_turso::Turso::file(&db_path))
        .await?;

    warm_pool(&db, pool_size).await?;

    let app = Router::new()
        .route("/stats", get(crate::stats))
        .route("/customers", get(customers))
        .route("/customer-by-id", get(customer_by_id))
        .route("/employees", get(employees))
        .route("/suppliers", get(suppliers))
        .route("/supplier-by-id", get(supplier_by_id))
        .route("/products", get(products))
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
        .with_state(AppState {
            db,
            _tmp: Arc::new(tmp),
        });

    crate::run_server(app).await
}

/// Open every pooled connection concurrently before announcing readiness.
async fn warm_pool(db: &Db, size: usize) -> Result<(), DynError> {
    let mut handles = Vec::with_capacity(size);
    for _ in 0..size {
        let mut db = db.clone();
        handles.push(tokio::spawn(async move {
            Customer::all().limit(1).exec(&mut db).await
        }));
    }
    for handle in handles {
        handle.await??;
    }
    Ok(())
}

async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<CustomerResponse>> {
    let mut db = handle(&state);
    let rows = Customer::all()
        .order_by(Customer::fields().id().asc())
        .limit(params.limit_or(50) as usize)
        .offset(params.offset() as usize)
        .exec(&mut db)
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<CustomerResponse>> {
    let mut db = handle(&state);
    let id = params.id_mod(SEED_CUSTOMERS as i32);
    let rows = Customer::filter(Customer::fields().id().eq(id))
        .exec(&mut db)
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn employees(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<EmployeeResponse>> {
    let mut db = handle(&state);
    let rows = Employee::all()
        .order_by(Employee::fields().id().asc())
        .limit(params.limit_or(50) as usize)
        .offset(params.offset() as usize)
        .exec(&mut db)
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn suppliers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<SupplierResponse>> {
    let mut db = handle(&state);
    let rows = Supplier::all()
        .order_by(Supplier::fields().id().asc())
        .limit(params.limit_or(50) as usize)
        .offset(params.offset() as usize)
        .exec(&mut db)
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<SupplierResponse>> {
    let mut db = handle(&state);
    let id = params.id_mod(SEED_SUPPLIERS as i32);
    let rows = Supplier::filter(Supplier::fields().id().eq(id))
        .exec(&mut db)
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<ProductResponse>> {
    let mut db = handle(&state);
    let rows = Product::all()
        .order_by(Product::fields().id().asc())
        .limit(params.limit_or(50) as usize)
        .offset(params.offset() as usize)
        .exec(&mut db)
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

/// Toasty has no user-reachable self join, so the recipient is a second round
/// trip rather than a `LEFT JOIN employees r`.
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<EmployeeWithRecipientResponse>> {
    let mut db = handle(&state);
    let id = params.id_mod(SEED_EMPLOYEES as i32);
    let rows = Employee::filter(Employee::fields().id().eq(id))
        .exec(&mut db)
        .await
        .map_err(internal)?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let recipient = match row.recipient_id {
            Some(recipient_id) => Employee::filter(Employee::fields().id().eq(recipient_id))
                .first()
                .exec(&mut db)
                .await
                .map_err(internal)?,
            None => None,
        };
        out.push(EmployeeWithRecipientResponse {
            id: row.id,
            last_name: row.last_name,
            first_name: row.first_name,
            title: row.title,
            title_of_courtesy: row.title_of_courtesy,
            birth_date: row.birth_date,
            hire_date: row.hire_date,
            address: row.address,
            city: row.city,
            postal_code: row.postal_code,
            country: row.country,
            home_phone: row.home_phone,
            extension: row.extension,
            notes: row.notes,
            recipient_id: row.recipient_id,
            recipient_last_name: recipient.as_ref().map(|r| r.last_name.clone()),
            recipient_first_name: recipient.and_then(|r| r.first_name),
        });
    }
    Ok(Json(out))
}

/// Second round trip instead of `INNER JOIN suppliers`.
async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<ProductWithSupplierResponse>> {
    let mut db = handle(&state);
    let id = params.id_mod(SEED_PRODUCTS as i32);
    let rows = Product::filter(Product::fields().id().eq(id))
        .exec(&mut db)
        .await
        .map_err(internal)?;

    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let Some(supplier) = Supplier::filter(Supplier::fields().id().eq(row.supplier_id))
            .first()
            .exec(&mut db)
            .await
            .map_err(internal)?
        else {
            // The canonical query is an INNER JOIN, so a missing supplier drops
            // the row rather than emitting a null nested object.
            continue;
        };
        out.push(ProductWithSupplierResponse {
            id: row.id,
            name: row.name,
            qt_per_unit: row.qt_per_unit,
            unit_price: row.unit_price,
            units_in_stock: row.units_in_stock,
            units_on_order: row.units_on_order,
            reorder_level: row.reorder_level,
            discontinued: row.discontinued,
            supplier_id: row.supplier_id,
            supplier: supplier.into(),
        });
    }
    Ok(Json(out))
}

/// Toasty exposes no `GROUP BY`/`SUM`, so this is the order page plus the
/// matching `order_details` id range, aggregated in Rust.
async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<OrderWithDetailsResponse>> {
    let mut db = handle(&state);
    let orders = Order::all()
        .order_by(Order::fields().id().asc())
        .limit(params.limit_or(50) as usize)
        .offset(params.offset() as usize)
        .exec(&mut db)
        .await
        .map_err(internal)?;

    let details = fetch_details_for_page(&mut db, &orders).await?;

    let mut out = Vec::with_capacity(orders.len());
    for order in orders {
        let mut products_count = 0i32;
        let mut quantity_sum = 0.0f64;
        let mut total_price = 0.0f64;
        for detail in details.iter().filter(|d| d.order_id == order.id) {
            products_count += 1;
            quantity_sum += f64::from(detail.quantity);
            total_price += f64::from(detail.quantity) * detail.unit_price;
        }
        out.push(OrderWithDetailsResponse {
            id: order.id,
            shipped_date: order.shipped_date,
            ship_name: order.ship_name,
            ship_city: order.ship_city,
            ship_country: order.ship_country,
            products_count,
            quantity_sum,
            total_price,
        });
    }
    Ok(Json(out))
}

/// Same id-range detail fetch the built-in Turso targets use: two bound params,
/// so the prepared statement shape stays constant across pages.
async fn fetch_details_for_page(
    db: &mut Db,
    orders: &[Order],
) -> Result<Vec<OrderDetail>, StatusCode> {
    let Some(min) = orders.iter().map(|o| o.id).min() else {
        return Ok(Vec::new());
    };
    let max = orders.iter().map(|o| o.id).max().unwrap_or(min);
    OrderDetail::filter(OrderDetail::fields().order_id().between(min, max))
        .exec(db)
        .await
        .map_err(internal)
}

async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<SingleOrderWithDetailsResponse>> {
    let mut db = handle(&state);
    let id = params.id_mod(SEED_ORDERS as i32);
    let orders = Order::filter(Order::fields().id().eq(id))
        .exec(&mut db)
        .await
        .map_err(internal)?;
    let details: Vec<OrderDetailResponse> =
        OrderDetail::filter(OrderDetail::fields().order_id().eq(id))
            .exec(&mut db)
            .await
            .map_err(internal)?
            .into_iter()
            .map(|d| OrderDetailResponse {
                unit_price: d.unit_price,
                quantity: d.quantity,
                discount: d.discount,
                order_id: d.order_id,
                product_id: d.product_id,
            })
            .collect();

    Ok(Json(
        orders
            .into_iter()
            .map(|order| SingleOrderWithDetailsResponse {
                id: order.id,
                order_date: order.order_date,
                required_date: order.required_date,
                shipped_date: order.shipped_date,
                ship_via: order.ship_via,
                freight: order.freight,
                ship_name: order.ship_name,
                ship_city: order.ship_city,
                ship_region: order.ship_region,
                ship_postal_code: order.ship_postal_code,
                ship_country: order.ship_country,
                customer_id: order.customer_id,
                employee_id: order.employee_id,
                details: details.clone(),
            })
            .collect(),
    ))
}

/// Third round trip for the product names — the canonical query joins
/// `order_details` to `products`, which toasty cannot express.
async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<SingleOrderWithDetailsAndProductsResponse>> {
    let mut db = handle(&state);
    let id = params.id_mod(SEED_ORDERS as i32);
    let orders = Order::filter(Order::fields().id().eq(id))
        .exec(&mut db)
        .await
        .map_err(internal)?;
    let raw_details = OrderDetail::filter(OrderDetail::fields().order_id().eq(id))
        .exec(&mut db)
        .await
        .map_err(internal)?;

    let mut product_ids: Vec<i32> = raw_details.iter().map(|d| d.product_id).collect();
    product_ids.sort_unstable();
    product_ids.dedup();
    let products = if product_ids.is_empty() {
        Vec::new()
    } else {
        Product::filter(Product::fields().id().in_list(product_ids))
            .exec(&mut db)
            .await
            .map_err(internal)?
    };

    let details: Vec<OrderDetailProductResponse> = raw_details
        .into_iter()
        .map(|d| OrderDetailProductResponse {
            unit_price: d.unit_price,
            quantity: d.quantity,
            discount: d.discount,
            order_id: d.order_id,
            product_id: d.product_id,
            product_name: products
                .iter()
                .find(|p| p.id == d.product_id)
                .map(|p| p.name.clone())
                .unwrap_or_default(),
        })
        .collect();

    Ok(Json(
        orders
            .into_iter()
            .map(|order| SingleOrderWithDetailsAndProductsResponse {
                id: order.id,
                order_date: order.order_date,
                required_date: order.required_date,
                shipped_date: order.shipped_date,
                ship_via: order.ship_via,
                freight: order.freight,
                ship_name: order.ship_name,
                ship_city: order.ship_city,
                ship_region: order.ship_region,
                ship_postal_code: order.ship_postal_code,
                ship_country: order.ship_country,
                customer_id: order.customer_id,
                employee_id: order.employee_id,
                details: details.clone(),
            })
            .collect(),
    ))
}

/// SQLite has no `ILIKE`; its `LIKE` is already ASCII case-insensitive, which
/// is exactly what the built-in Turso targets issue.
async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<CustomerResponse>> {
    let mut db = handle(&state);
    let rows = Customer::filter(Customer::fields().company_name().like(params.pattern()))
        .exec(&mut db)
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}

async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult<Vec<ProductResponse>> {
    let mut db = handle(&state);
    let rows = Product::filter(Product::fields().name().like(params.pattern()))
        .exec(&mut db)
        .await
        .map_err(internal)?;
    Ok(Json(rows.into_iter().map(Into::into).collect()))
}
