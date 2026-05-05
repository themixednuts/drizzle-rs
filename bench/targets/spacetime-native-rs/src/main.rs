//! SpacetimeDB native SDK HTTP wrapper for the Northwind benchmark.
//!
//! This target avoids PGWire for measured behavior. It seeds through the
//! module reducer, subscribes to the public benchmark tables over the SDK, then
//! serves the standard HTTP contract from the materialized subscription cache.

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use serde::{Deserialize, Serialize};
use spacetimedb_sdk::{DbContext, Table};
use std::collections::{BTreeMap, HashMap};
use std::net::TcpListener;
use std::sync::Arc;
use std::time::{Duration, Instant};
use sysinfo::System;

mod module_bindings;

use module_bindings::customers_table::CustomersTableAccess;
use module_bindings::employees_table::EmployeesTableAccess;
use module_bindings::order_details_table::OrderDetailsTableAccess;
use module_bindings::orders_table::OrdersTableAccess;
use module_bindings::products_table::ProductsTableAccess;
use module_bindings::seed_reducer::seed;
use module_bindings::suppliers_table::SuppliersTableAccess;
use module_bindings::{Customer, DbConnection, Employee, Order, OrderDetail, Product, Supplier};

const SEED_CUSTOMERS: usize = 10_000;
const SEED_EMPLOYEES: usize = 200;
const SEED_SUPPLIERS: usize = 1_000;
const SEED_PRODUCTS: usize = 5_000;
const SEED_ORDERS: usize = 50_000;
const DETAILS_PER_ORDER: usize = 6;
const SUBSCRIPTION_TIMEOUT: Duration = Duration::from_secs(180);

#[derive(Debug, Deserialize)]
struct QueryParams {
    id: Option<i32>,
    limit: Option<usize>,
    offset: Option<usize>,
    term: Option<String>,
}

impl QueryParams {
    fn limit_or(&self, default: usize) -> usize {
        self.limit.unwrap_or(default)
    }

    fn offset(&self) -> usize {
        self.offset.unwrap_or(0)
    }

    fn user_id(&self, n: i32) -> u32 {
        self.id.map(|i| (i - 1).rem_euclid(n) + 1).unwrap_or(1) as u32
    }
}

#[derive(Clone)]
struct AppState {
    model: Arc<ReadModel>,
}

#[derive(Debug)]
struct ReadModel {
    customers: BTreeMap<u32, Customer>,
    employees: BTreeMap<u32, Employee>,
    suppliers: BTreeMap<u32, Supplier>,
    products: BTreeMap<u32, Product>,
    orders: BTreeMap<u32, Order>,
    order_details_by_order: HashMap<u32, Vec<OrderDetail>>,
}

impl ReadModel {
    fn from_connection(conn: &DbConnection) -> Result<Self, String> {
        let customers = conn
            .db
            .customers()
            .iter()
            .map(|row| (row.id, row))
            .collect::<BTreeMap<_, _>>();
        let employees = conn
            .db
            .employees()
            .iter()
            .map(|row| (row.id, row))
            .collect::<BTreeMap<_, _>>();
        let suppliers = conn
            .db
            .suppliers()
            .iter()
            .map(|row| (row.id, row))
            .collect::<BTreeMap<_, _>>();
        let products = conn
            .db
            .products()
            .iter()
            .map(|row| (row.id, row))
            .collect::<BTreeMap<_, _>>();
        let orders = conn
            .db
            .orders()
            .iter()
            .map(|row| (row.id, row))
            .collect::<BTreeMap<_, _>>();

        let mut order_details_by_order: HashMap<u32, Vec<OrderDetail>> = HashMap::new();
        for row in conn.db.order_details().iter() {
            order_details_by_order
                .entry(row.order_id)
                .or_default()
                .push(row);
        }
        for rows in order_details_by_order.values_mut() {
            rows.sort_by_key(|row| row.id);
        }

        let model = Self {
            customers,
            employees,
            suppliers,
            products,
            orders,
            order_details_by_order,
        };
        model.validate_counts()?;
        Ok(model)
    }

    fn validate_counts(&self) -> Result<(), String> {
        let details = self
            .order_details_by_order
            .values()
            .map(Vec::len)
            .sum::<usize>();
        let expected = [
            ("customers", self.customers.len(), SEED_CUSTOMERS),
            ("employees", self.employees.len(), SEED_EMPLOYEES),
            ("suppliers", self.suppliers.len(), SEED_SUPPLIERS),
            ("products", self.products.len(), SEED_PRODUCTS),
            ("orders", self.orders.len(), SEED_ORDERS),
            ("order_details", details, SEED_ORDERS * DETAILS_PER_ORDER),
        ];
        for (table, actual, expected) in expected {
            if actual != expected {
                return Err(format!(
                    "{table} count mismatch: expected {expected}, got {actual}"
                ));
            }
        }
        Ok(())
    }
}

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

fn as_i32(value: u32) -> i32 {
    value.try_into().unwrap_or(i32::MAX)
}

fn opt_string(value: &str) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn opt_i32(value: i32) -> Option<i32> {
    if value == 0 { None } else { Some(value) }
}

fn opt_i64(value: i64) -> Option<i64> {
    if value == 0 { None } else { Some(value) }
}

fn contains_term(value: &str, term_lower: &str) -> bool {
    term_lower.is_empty() || value.to_ascii_lowercase().contains(term_lower)
}

fn customer_response(row: &Customer) -> CustomerResponse {
    CustomerResponse {
        id: as_i32(row.id),
        company_name: row.company_name.clone(),
        contact_name: row.contact_name.clone(),
        contact_title: row.contact_title.clone(),
        address: row.address.clone(),
        city: row.city.clone(),
        postal_code: opt_string(&row.postal_code),
        region: opt_string(&row.region),
        country: row.country.clone(),
        phone: row.phone.clone(),
        fax: opt_string(&row.fax),
    }
}

fn employee_response(row: &Employee) -> EmployeeResponse {
    EmployeeResponse {
        id: as_i32(row.id),
        last_name: row.last_name.clone(),
        first_name: opt_string(&row.first_name),
        title: row.title.clone(),
        title_of_courtesy: row.title_of_courtesy.clone(),
        birth_date: row.birth_date,
        hire_date: row.hire_date,
        address: row.address.clone(),
        city: row.city.clone(),
        postal_code: row.postal_code.clone(),
        country: row.country.clone(),
        home_phone: row.home_phone.clone(),
        extension: row.extension,
        notes: row.notes.clone(),
        recipient_id: opt_i32(row.recipient_id),
    }
}

fn supplier_response(row: &Supplier) -> SupplierResponse {
    SupplierResponse {
        id: as_i32(row.id),
        company_name: row.company_name.clone(),
        contact_name: row.contact_name.clone(),
        contact_title: row.contact_title.clone(),
        address: row.address.clone(),
        city: row.city.clone(),
        region: opt_string(&row.region),
        postal_code: row.postal_code.clone(),
        country: row.country.clone(),
        phone: row.phone.clone(),
    }
}

fn product_response(row: &Product) -> ProductResponse {
    ProductResponse {
        id: as_i32(row.id),
        name: row.name.clone(),
        qt_per_unit: row.qt_per_unit.clone(),
        unit_price: row.unit_price,
        units_in_stock: row.units_in_stock,
        units_on_order: row.units_on_order,
        reorder_level: row.reorder_level,
        discontinued: row.discontinued,
        supplier_id: as_i32(row.supplier_id),
    }
}

fn detail_response(row: &OrderDetail) -> OrderDetailResponse {
    OrderDetailResponse {
        unit_price: row.unit_price,
        quantity: row.quantity,
        discount: row.discount,
        order_id: as_i32(row.order_id),
        product_id: as_i32(row.product_id),
    }
}

fn order_details_for(model: &ReadModel, order_id: u32) -> &[OrderDetail] {
    model
        .order_details_by_order
        .get(&order_id)
        .map(Vec::as_slice)
        .unwrap_or(&[])
}

async fn stats() -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    let cpu = sys
        .cpus()
        .iter()
        .map(|cpu| f64::from(cpu.cpu_usage()))
        .collect::<Vec<_>>();
    Json(if cpu.is_empty() { vec![0.0] } else { cpu })
}

async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let start = params.offset().saturating_add(1) as u32;
    let end = params.offset().saturating_add(params.limit_or(50)) as u32;
    let resp = state
        .model
        .customers
        .range(start..=end)
        .map(|(_, row)| customer_response(row))
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(SEED_CUSTOMERS as i32);
    let resp = state
        .model
        .customers
        .get(&id)
        .map(customer_response)
        .into_iter()
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn employees(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let start = params.offset().saturating_add(1) as u32;
    let end = params.offset().saturating_add(params.limit_or(50)) as u32;
    let resp = state
        .model
        .employees
        .range(start..=end)
        .map(|(_, row)| employee_response(row))
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn suppliers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let start = params.offset().saturating_add(1) as u32;
    let end = params.offset().saturating_add(params.limit_or(50)) as u32;
    let resp = state
        .model
        .suppliers
        .range(start..=end)
        .map(|(_, row)| supplier_response(row))
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(SEED_SUPPLIERS as i32);
    let resp = state
        .model
        .suppliers
        .get(&id)
        .map(supplier_response)
        .into_iter()
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let start = params.offset().saturating_add(1) as u32;
    let end = params.offset().saturating_add(params.limit_or(50)) as u32;
    let resp = state
        .model
        .products
        .range(start..=end)
        .map(|(_, row)| product_response(row))
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(SEED_EMPLOYEES as i32);
    let resp = state
        .model
        .employees
        .get(&id)
        .map(|employee| {
            let recipient = opt_i32(employee.recipient_id)
                .and_then(|id| state.model.employees.get(&(id as u32)));
            EmployeeWithRecipientResponse {
                id: as_i32(employee.id),
                last_name: employee.last_name.clone(),
                first_name: opt_string(&employee.first_name),
                title: employee.title.clone(),
                title_of_courtesy: employee.title_of_courtesy.clone(),
                birth_date: employee.birth_date,
                hire_date: employee.hire_date,
                address: employee.address.clone(),
                city: employee.city.clone(),
                postal_code: employee.postal_code.clone(),
                country: employee.country.clone(),
                home_phone: employee.home_phone.clone(),
                extension: employee.extension,
                notes: employee.notes.clone(),
                recipient_id: opt_i32(employee.recipient_id),
                recipient_last_name: recipient.map(|row| row.last_name.clone()),
                recipient_first_name: recipient.and_then(|row| opt_string(&row.first_name)),
            }
        })
        .into_iter()
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(SEED_PRODUCTS as i32);
    let resp = state
        .model
        .products
        .get(&id)
        .and_then(|product| {
            state
                .model
                .suppliers
                .get(&product.supplier_id)
                .map(|supplier| ProductWithSupplierResponse {
                    id: as_i32(product.id),
                    name: product.name.clone(),
                    qt_per_unit: product.qt_per_unit.clone(),
                    unit_price: product.unit_price,
                    units_in_stock: product.units_in_stock,
                    units_on_order: product.units_on_order,
                    reorder_level: product.reorder_level,
                    discontinued: product.discontinued,
                    supplier_id: as_i32(product.supplier_id),
                    supplier: supplier_response(supplier),
                })
        })
        .into_iter()
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let start = params.offset().saturating_add(1) as u32;
    let end = params.offset().saturating_add(params.limit_or(50)) as u32;
    let resp = state
        .model
        .orders
        .range(start..=end)
        .map(|(_, order)| {
            let details = order_details_for(&state.model, order.id);
            let products_count = details.iter().filter(|row| row.product_id != 0).count() as i32;
            let quantity_sum = details.iter().map(|row| row.quantity as f64).sum::<f64>();
            let total_price = details
                .iter()
                .map(|row| row.quantity as f64 * row.unit_price)
                .sum::<f64>();
            OrderWithDetailsResponse {
                id: as_i32(order.id),
                shipped_date: opt_i64(order.shipped_date),
                ship_name: order.ship_name.clone(),
                ship_city: order.ship_city.clone(),
                ship_country: order.ship_country.clone(),
                products_count,
                quantity_sum,
                total_price,
            }
        })
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(SEED_ORDERS as i32);
    let resp = state
        .model
        .orders
        .get(&id)
        .map(|order| {
            let details = order_details_for(&state.model, order.id)
                .iter()
                .map(detail_response)
                .collect::<Vec<_>>();
            SingleOrderWithDetailsResponse {
                id: as_i32(order.id),
                order_date: order.order_date,
                required_date: order.required_date,
                shipped_date: opt_i64(order.shipped_date),
                ship_via: order.ship_via,
                freight: order.freight,
                ship_name: order.ship_name.clone(),
                ship_city: order.ship_city.clone(),
                ship_region: opt_string(&order.ship_region),
                ship_postal_code: opt_string(&order.ship_postal_code),
                ship_country: order.ship_country.clone(),
                customer_id: as_i32(order.customer_id),
                employee_id: as_i32(order.employee_id),
                details,
            }
        })
        .into_iter()
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(SEED_ORDERS as i32);
    let resp = state
        .model
        .orders
        .get(&id)
        .map(|order| {
            let details = order_details_for(&state.model, order.id)
                .iter()
                .map(|detail| OrderDetailProductResponse {
                    unit_price: detail.unit_price,
                    quantity: detail.quantity,
                    discount: detail.discount,
                    order_id: as_i32(detail.order_id),
                    product_id: as_i32(detail.product_id),
                    product_name: state
                        .model
                        .products
                        .get(&detail.product_id)
                        .map(|row| row.name.clone())
                        .unwrap_or_default(),
                })
                .collect::<Vec<_>>();
            SingleOrderWithDetailsAndProductsResponse {
                id: as_i32(order.id),
                order_date: order.order_date,
                required_date: order.required_date,
                shipped_date: opt_i64(order.shipped_date),
                ship_via: order.ship_via,
                freight: order.freight,
                ship_name: order.ship_name.clone(),
                ship_city: order.ship_city.clone(),
                ship_region: opt_string(&order.ship_region),
                ship_postal_code: opt_string(&order.ship_postal_code),
                ship_country: order.ship_country.clone(),
                customer_id: as_i32(order.customer_id),
                employee_id: as_i32(order.employee_id),
                details,
            }
        })
        .into_iter()
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let term = params.term.as_deref().unwrap_or("").to_ascii_lowercase();
    let resp = state
        .model
        .customers
        .values()
        .filter(|row| contains_term(&row.company_name, &term))
        .map(customer_response)
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let term = params.term.as_deref().unwrap_or("").to_ascii_lowercase();
    let resp = state
        .model
        .products
        .values()
        .filter(|row| contains_term(&row.name, &term))
        .map(product_response)
        .collect::<Vec<_>>();
    Ok(Json(serde_json::to_value(resp).unwrap()))
}

fn spacetime_token() -> Option<String> {
    if let Ok(token) = std::env::var("SPACETIME_TOKEN")
        && !token.trim().is_empty()
    {
        return Some(token);
    }
    None
}

async fn connect_and_seed(seed_value: u64, trial: u32) -> Result<DbConnection, String> {
    let uri = std::env::var("SPACETIME_URI").unwrap_or_else(|_| "ws://127.0.0.1:3000".into());
    let module = std::env::var("SPACETIME_MODULE").unwrap_or_else(|_| "bench-module".into());

    let (connect_tx, connect_rx) = tokio::sync::oneshot::channel();
    let connect_tx = std::sync::Mutex::new(Some(connect_tx));
    let conn = DbConnection::builder()
        .with_uri(&uri)
        .with_database_name(&module)
        .with_token(spacetime_token())
        .on_connect(move |_ctx, _identity, _token| {
            if let Some(tx) = connect_tx.lock().unwrap().take() {
                let _ = tx.send(());
            }
        })
        .on_connect_error(|_ctx, err| {
            eprintln!("spacetime-sdk-rs: connect error: {err}");
        })
        .build()
        .map_err(|err| format!("spacetime sdk build connection: {err}"))?;

    conn.run_threaded();
    connect_rx
        .await
        .map_err(|_| "connection callback never fired".to_string())?;

    let (seed_tx, seed_rx) = tokio::sync::oneshot::channel();
    conn.reducers
        .seed_then(seed_value, trial, move |_ctx, result| {
            let _ = seed_tx.send(result);
        })
        .map_err(|err| format!("seed reducer send failed: {err}"))?;
    let seed_result = seed_rx
        .await
        .map_err(|_| "seed reducer callback never fired".to_string())?;
    match seed_result {
        Ok(Ok(())) => {}
        Ok(Err(err)) => return Err(format!("seed reducer failed: {err}")),
        Err(err) => return Err(format!("seed reducer internal error: {err}")),
    }

    let (sub_tx, sub_rx) = tokio::sync::oneshot::channel();
    let _subscription = conn
        .subscription_builder()
        .on_applied(move |_ctx| {
            let _ = sub_tx.send(());
        })
        .subscribe([
            "SELECT * FROM customers",
            "SELECT * FROM employees",
            "SELECT * FROM suppliers",
            "SELECT * FROM products",
            "SELECT * FROM orders",
            "SELECT * FROM order_details",
        ]);
    tokio::time::timeout(SUBSCRIPTION_TIMEOUT, sub_rx)
        .await
        .map_err(|_| "subscription timed out".to_string())?
        .map_err(|_| "subscription callback never fired".to_string())?;

    let deadline = Instant::now() + SUBSCRIPTION_TIMEOUT;
    loop {
        if let Ok(model) = ReadModel::from_connection(&conn) {
            eprintln!(
                "spacetime-sdk-rs: subscribed {} customers, {} orders, {} detail groups",
                model.customers.len(),
                model.orders.len(),
                model.order_details_by_order.len()
            );
            return Ok(conn);
        }
        if Instant::now() >= deadline {
            return Err("subscription counts did not reach expected seed sizes".to_string());
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let seed_value = std::env::var("BENCH_SEED")
        .ok()
        .and_then(|raw| raw.parse::<u64>().ok())
        .unwrap_or(42);
    let trial = std::env::var("BENCH_TRIAL")
        .ok()
        .and_then(|raw| raw.parse::<u32>().ok())
        .unwrap_or(1);

    let conn = connect_and_seed(seed_value, trial).await?;
    let model = Arc::new(ReadModel::from_connection(&conn)?);
    drop(conn);

    let state = AppState { model };
    let app = Router::new()
        .route("/stats", get(stats))
        .route("/customers", get(customers))
        .route("/customer-by-id", get(customer_by_id))
        .route("/employees", get(employees))
        .route("/employee-with-recipient", get(employee_with_recipient))
        .route("/suppliers", get(suppliers))
        .route("/supplier-by-id", get(supplier_by_id))
        .route("/products", get(products))
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

    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    let port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;

    let server = tokio::spawn(async move {
        axum::Server::from_tcp(listener)
            .expect("server init")
            .serve(app.into_make_service())
            .with_graceful_shutdown(async {
                tokio::signal::ctrl_c().await.ok();
            })
            .await
            .expect("server failed");
    });

    println!("LISTENING port={port}");
    server.await?;
    Ok(())
}
