use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router};
use chrono::NaiveDate;
use diesel::pg::PgConnection;
use diesel::prelude::*;
use diesel::sql_types::{BigInt, Date, Double, Integer, Nullable, Text};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbBackend, Statement,
};
use serde::{Deserialize, Serialize};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::collections::BTreeMap;
use std::io::Write as _;
use std::net::TcpListener;
use std::process::Command;
use std::sync::{Arc, Mutex};
use sysinfo::System;

const SEED_CUSTOMERS: usize = 10_000;
const SEED_EMPLOYEES: usize = 200;
const SEED_ORDERS: usize = 50_000;
const SEED_SUPPLIERS: usize = 1_000;
const SEED_PRODUCTS: usize = 5_000;

type DynError = Box<dyn std::error::Error + Send + Sync>;
type HttpResult = Result<Json<serde_json::Value>, StatusCode>;

#[derive(Clone)]
enum AppState {
    Sqlx(PgPool),
    Diesel(Arc<Mutex<PgConnection>>),
    SeaOrm(Arc<DatabaseConnection>),
}

#[derive(Debug, Deserialize)]
struct QueryParams {
    id: Option<i32>,
    limit: Option<usize>,
    offset: Option<usize>,
    term: Option<String>,
}

impl QueryParams {
    fn limit_or(&self, default: usize) -> i64 {
        self.limit.unwrap_or(default) as i64
    }

    fn offset(&self) -> i64 {
        self.offset.unwrap_or(0) as i64
    }

    fn id_mod(&self, n: i32) -> i32 {
        self.id.map(|i| i.rem_euclid(n).max(1)).unwrap_or(1)
    }
}

#[derive(Debug, Serialize, sqlx::FromRow, diesel::QueryableByName)]
#[serde(rename_all = "camelCase")]
struct CustomerResponse {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    company_name: String,
    #[diesel(sql_type = Text)]
    contact_name: String,
    #[diesel(sql_type = Text)]
    contact_title: String,
    #[diesel(sql_type = Text)]
    address: String,
    #[diesel(sql_type = Text)]
    city: String,
    #[diesel(sql_type = Nullable<Text>)]
    postal_code: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    region: Option<String>,
    #[diesel(sql_type = Text)]
    country: String,
    #[diesel(sql_type = Text)]
    phone: String,
    #[diesel(sql_type = Nullable<Text>)]
    fax: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow, diesel::QueryableByName)]
#[serde(rename_all = "camelCase")]
struct EmployeeResponse {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    last_name: String,
    #[diesel(sql_type = Nullable<Text>)]
    first_name: Option<String>,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    title_of_courtesy: String,
    #[diesel(sql_type = Date)]
    birth_date: NaiveDate,
    #[diesel(sql_type = Date)]
    hire_date: NaiveDate,
    #[diesel(sql_type = Text)]
    address: String,
    #[diesel(sql_type = Text)]
    city: String,
    #[diesel(sql_type = Text)]
    postal_code: String,
    #[diesel(sql_type = Text)]
    country: String,
    #[diesel(sql_type = Text)]
    home_phone: String,
    #[diesel(sql_type = Integer)]
    extension: i32,
    #[diesel(sql_type = Text)]
    notes: String,
    #[diesel(sql_type = Nullable<Integer>)]
    recipient_id: Option<i32>,
}

#[derive(Debug, Serialize, sqlx::FromRow, diesel::QueryableByName)]
#[serde(rename_all = "camelCase")]
struct EmployeeWithRecipientResponse {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    last_name: String,
    #[diesel(sql_type = Nullable<Text>)]
    first_name: Option<String>,
    #[diesel(sql_type = Text)]
    title: String,
    #[diesel(sql_type = Text)]
    title_of_courtesy: String,
    #[diesel(sql_type = Date)]
    birth_date: NaiveDate,
    #[diesel(sql_type = Date)]
    hire_date: NaiveDate,
    #[diesel(sql_type = Text)]
    address: String,
    #[diesel(sql_type = Text)]
    city: String,
    #[diesel(sql_type = Text)]
    postal_code: String,
    #[diesel(sql_type = Text)]
    country: String,
    #[diesel(sql_type = Text)]
    home_phone: String,
    #[diesel(sql_type = Integer)]
    extension: i32,
    #[diesel(sql_type = Text)]
    notes: String,
    #[diesel(sql_type = Nullable<Integer>)]
    recipient_id: Option<i32>,
    #[diesel(sql_type = Nullable<Text>)]
    recipient_last_name: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    recipient_first_name: Option<String>,
}

#[derive(Debug, Serialize, sqlx::FromRow, diesel::QueryableByName)]
#[serde(rename_all = "camelCase")]
struct SupplierResponse {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    company_name: String,
    #[diesel(sql_type = Text)]
    contact_name: String,
    #[diesel(sql_type = Text)]
    contact_title: String,
    #[diesel(sql_type = Text)]
    address: String,
    #[diesel(sql_type = Text)]
    city: String,
    #[diesel(sql_type = Nullable<Text>)]
    region: Option<String>,
    #[diesel(sql_type = Text)]
    postal_code: String,
    #[diesel(sql_type = Text)]
    country: String,
    #[diesel(sql_type = Text)]
    phone: String,
}

#[derive(Debug, Serialize, sqlx::FromRow, diesel::QueryableByName)]
#[serde(rename_all = "camelCase")]
struct ProductResponse {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    qt_per_unit: String,
    #[diesel(sql_type = Double)]
    unit_price: f64,
    #[diesel(sql_type = Integer)]
    units_in_stock: i32,
    #[diesel(sql_type = Integer)]
    units_on_order: i32,
    #[diesel(sql_type = Integer)]
    reorder_level: i32,
    #[diesel(sql_type = Integer)]
    discontinued: i32,
    #[diesel(sql_type = Integer)]
    supplier_id: i32,
}

#[derive(Debug, sqlx::FromRow, diesel::QueryableByName)]
struct ProductWithSupplierRow {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    qt_per_unit: String,
    #[diesel(sql_type = Double)]
    unit_price: f64,
    #[diesel(sql_type = Integer)]
    units_in_stock: i32,
    #[diesel(sql_type = Integer)]
    units_on_order: i32,
    #[diesel(sql_type = Integer)]
    reorder_level: i32,
    #[diesel(sql_type = Integer)]
    discontinued: i32,
    #[diesel(sql_type = Integer)]
    supplier_id: i32,
    #[diesel(sql_type = Integer)]
    s_id: i32,
    #[diesel(sql_type = Text)]
    s_company_name: String,
    #[diesel(sql_type = Text)]
    s_contact_name: String,
    #[diesel(sql_type = Text)]
    s_contact_title: String,
    #[diesel(sql_type = Text)]
    s_address: String,
    #[diesel(sql_type = Text)]
    s_city: String,
    #[diesel(sql_type = Nullable<Text>)]
    s_region: Option<String>,
    #[diesel(sql_type = Text)]
    s_postal_code: String,
    #[diesel(sql_type = Text)]
    s_country: String,
    #[diesel(sql_type = Text)]
    s_phone: String,
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

impl From<ProductWithSupplierRow> for ProductWithSupplierResponse {
    fn from(row: ProductWithSupplierRow) -> Self {
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
            supplier: SupplierResponse {
                id: row.s_id,
                company_name: row.s_company_name,
                contact_name: row.s_contact_name,
                contact_title: row.s_contact_title,
                address: row.s_address,
                city: row.s_city,
                region: row.s_region,
                postal_code: row.s_postal_code,
                country: row.s_country,
                phone: row.s_phone,
            },
        }
    }
}

#[derive(Debug, Serialize, sqlx::FromRow, diesel::QueryableByName)]
#[serde(rename_all = "camelCase")]
struct OrderWithDetailsResponse {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Nullable<Date>)]
    shipped_date: Option<NaiveDate>,
    #[diesel(sql_type = Text)]
    ship_name: String,
    #[diesel(sql_type = Text)]
    ship_city: String,
    #[diesel(sql_type = Text)]
    ship_country: String,
    #[diesel(sql_type = BigInt)]
    products_count: i64,
    #[diesel(sql_type = Double)]
    quantity_sum: f64,
    #[diesel(sql_type = Double)]
    total_price: f64,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow, diesel::QueryableByName)]
#[serde(rename_all = "camelCase")]
struct OrderDetailResponse {
    #[diesel(sql_type = Double)]
    unit_price: f64,
    #[diesel(sql_type = Integer)]
    quantity: i32,
    #[diesel(sql_type = Double)]
    discount: f64,
    #[diesel(sql_type = Integer)]
    order_id: i32,
    #[diesel(sql_type = Integer)]
    product_id: i32,
}

#[derive(Debug, Serialize, Clone, sqlx::FromRow, diesel::QueryableByName)]
#[serde(rename_all = "camelCase")]
struct OrderDetailProductResponse {
    #[diesel(sql_type = Double)]
    unit_price: f64,
    #[diesel(sql_type = Integer)]
    quantity: i32,
    #[diesel(sql_type = Double)]
    discount: f64,
    #[diesel(sql_type = Integer)]
    order_id: i32,
    #[diesel(sql_type = Integer)]
    product_id: i32,
    #[diesel(sql_type = Text)]
    product_name: String,
}

#[derive(Debug, sqlx::FromRow, diesel::QueryableByName)]
struct OrderBase {
    #[diesel(sql_type = Integer)]
    id: i32,
    #[diesel(sql_type = Date)]
    order_date: NaiveDate,
    #[diesel(sql_type = Date)]
    required_date: NaiveDate,
    #[diesel(sql_type = Nullable<Date>)]
    shipped_date: Option<NaiveDate>,
    #[diesel(sql_type = Integer)]
    ship_via: i32,
    #[diesel(sql_type = Double)]
    freight: f64,
    #[diesel(sql_type = Text)]
    ship_name: String,
    #[diesel(sql_type = Text)]
    ship_city: String,
    #[diesel(sql_type = Nullable<Text>)]
    ship_region: Option<String>,
    #[diesel(sql_type = Nullable<Text>)]
    ship_postal_code: Option<String>,
    #[diesel(sql_type = Text)]
    ship_country: String,
    #[diesel(sql_type = Integer)]
    customer_id: i32,
    #[diesel(sql_type = Integer)]
    employee_id: i32,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct SingleOrderWithDetailsResponse {
    id: i32,
    order_date: NaiveDate,
    required_date: NaiveDate,
    shipped_date: Option<NaiveDate>,
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
    order_date: NaiveDate,
    required_date: NaiveDate,
    shipped_date: Option<NaiveDate>,
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

impl OrderBase {
    fn with_details(self, details: Vec<OrderDetailResponse>) -> SingleOrderWithDetailsResponse {
        SingleOrderWithDetailsResponse {
            id: self.id,
            order_date: self.order_date,
            required_date: self.required_date,
            shipped_date: self.shipped_date,
            ship_via: self.ship_via,
            freight: self.freight,
            ship_name: self.ship_name,
            ship_city: self.ship_city,
            ship_region: self.ship_region,
            ship_postal_code: self.ship_postal_code,
            ship_country: self.ship_country,
            customer_id: self.customer_id,
            employee_id: self.employee_id,
            details,
        }
    }

    fn with_detail_products(
        self,
        details: Vec<OrderDetailProductResponse>,
    ) -> SingleOrderWithDetailsAndProductsResponse {
        SingleOrderWithDetailsAndProductsResponse {
            id: self.id,
            order_date: self.order_date,
            required_date: self.required_date,
            shipped_date: self.shipped_date,
            ship_via: self.ship_via,
            freight: self.freight,
            ship_name: self.ship_name,
            ship_city: self.ship_city,
            ship_region: self.ship_region,
            ship_postal_code: self.ship_postal_code,
            ship_country: self.ship_country,
            customer_id: self.customer_id,
            employee_id: self.employee_id,
            details,
        }
    }
}

const SELECT_CUSTOMERS: &str = "\
SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax
FROM customers ORDER BY id LIMIT $1 OFFSET $2";
const SELECT_CUSTOMER_BY_ID: &str = "\
SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax
FROM customers WHERE id = $1";
const SELECT_EMPLOYEES: &str = "\
SELECT id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id
FROM employees ORDER BY id LIMIT $1 OFFSET $2";
const SELECT_SUPPLIERS: &str = "\
SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone
FROM suppliers ORDER BY id LIMIT $1 OFFSET $2";
const SELECT_SUPPLIER_BY_ID: &str = "\
SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone
FROM suppliers WHERE id = $1";
const SELECT_PRODUCTS: &str = "\
SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id
FROM products ORDER BY id LIMIT $1 OFFSET $2";
const SELECT_EMPLOYEE_WITH_RECIPIENT: &str = "\
SELECT e.id, e.last_name, e.first_name, e.title, e.title_of_courtesy, e.birth_date, e.hire_date,
       e.address, e.city, e.postal_code, e.country, e.home_phone, e.extension, e.notes, e.recipient_id,
       r.last_name AS recipient_last_name, r.first_name AS recipient_first_name
FROM employees e LEFT JOIN employees r ON e.recipient_id = r.id WHERE e.id = $1";
const SELECT_PRODUCT_WITH_SUPPLIER: &str = "\
SELECT p.id, p.name, p.qt_per_unit, p.unit_price, p.units_in_stock, p.units_on_order, p.reorder_level,
       p.discontinued, p.supplier_id, s.id AS s_id, s.company_name AS s_company_name,
       s.contact_name AS s_contact_name, s.contact_title AS s_contact_title, s.address AS s_address,
       s.city AS s_city, s.region AS s_region, s.postal_code AS s_postal_code, s.country AS s_country,
       s.phone AS s_phone
FROM products p INNER JOIN suppliers s ON p.supplier_id = s.id WHERE p.id = $1";
const SELECT_ORDERS_WITH_DETAILS: &str = "\
SELECT o.id, o.shipped_date, o.ship_name, o.ship_city, o.ship_country,
       count(d.product_id) AS products_count,
       COALESCE(sum(d.quantity)::float8, 0) AS quantity_sum,
       COALESCE(sum(d.quantity::float8 * d.unit_price), 0) AS total_price
FROM orders o LEFT JOIN order_details d ON o.id = d.order_id
GROUP BY o.id ORDER BY o.id LIMIT $1 OFFSET $2";
const SELECT_ORDER_BASE: &str = "\
SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city,
       ship_region, ship_postal_code, ship_country, customer_id, employee_id
FROM orders WHERE id = $1";
const SELECT_ORDER_DETAILS: &str = "\
SELECT unit_price, quantity, discount, order_id, product_id
FROM order_details WHERE order_id = $1";
const SELECT_ORDER_DETAIL_PRODUCTS: &str = "\
SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name AS product_name
FROM order_details d LEFT JOIN products p ON d.product_id = p.id WHERE d.order_id = $1";
const SEARCH_CUSTOMERS: &str = "\
SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax
FROM customers WHERE company_name ILIKE $1";
const SEARCH_PRODUCTS: &str = "\
SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id
FROM products WHERE name ILIKE $1";

#[tokio::main]
async fn main() -> Result<(), DynError> {
    let target = std::env::args()
        .nth(1)
        .unwrap_or_else(|| std::env::var("BENCH_TARGET_ID").unwrap_or_default());
    let seed = std::env::var("BENCH_SEED")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .unwrap_or(42);
    let database_url = normalize_database_url();

    seed_postgres(seed)?;

    let state = match target.as_str() {
        "sqlx-pg" => {
            let pool = PgPoolOptions::new()
                .max_connections(8)
                .connect(&database_url)
                .await?;
            AppState::Sqlx(pool)
        }
        "diesel-pg" => {
            let conn = PgConnection::establish(&database_url)?;
            AppState::Diesel(Arc::new(Mutex::new(conn)))
        }
        "seaorm-pg" => {
            let mut options = ConnectOptions::new(database_url);
            options.max_connections(8).min_connections(1);
            let conn = Database::connect(options).await?;
            AppState::SeaOrm(Arc::new(conn))
        }
        other => return Err(format!("unsupported rust-pg-orms target: {other}").into()),
    };

    let app = Router::new()
        .route("/stats", get(stats))
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
        .with_state(state);

    let listener = TcpListener::bind(("127.0.0.1", 0))?;
    listener.set_nonblocking(true)?;
    let port = listener.local_addr()?.port();
    println!("LISTENING port={port}");
    std::io::stdout().flush()?;

    axum::Server::from_tcp(listener)?
        .serve(app.into_make_service())
        .await?;
    Ok(())
}

async fn stats() -> Result<Json<Vec<f64>>, StatusCode> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    let mut out: Vec<f64> = sys
        .cpus()
        .iter()
        .map(|cpu| f64::from(cpu.cpu_usage()))
        .collect();
    if out.is_empty() {
        out.push(0.0);
    }
    Ok(Json(out))
}

async fn customers(State(state): State<AppState>, Query(params): Query<QueryParams>) -> HttpResult {
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, CustomerResponse>(SELECT_CUSTOMERS)
                .bind(params.limit_or(50))
                .bind(params.offset())
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_CUSTOMERS)
                    .bind::<BigInt, _>(params.limit_or(50))
                    .bind::<BigInt, _>(params.offset())
                    .load::<CustomerResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(
                conn,
                SELECT_CUSTOMERS,
                vec![params.limit_or(50).into(), params.offset().into()],
            )
            .await
        }
    }
}

async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    let id = params.id_mod(SEED_CUSTOMERS as i32);
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, CustomerResponse>(SELECT_CUSTOMER_BY_ID)
                .bind(id)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_CUSTOMER_BY_ID)
                    .bind::<Integer, _>(id)
                    .load::<CustomerResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(conn, SELECT_CUSTOMER_BY_ID, vec![id.into()]).await
        }
    }
}

async fn employees(State(state): State<AppState>, Query(params): Query<QueryParams>) -> HttpResult {
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, EmployeeResponse>(SELECT_EMPLOYEES)
                .bind(params.limit_or(50))
                .bind(params.offset())
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_EMPLOYEES)
                    .bind::<BigInt, _>(params.limit_or(50))
                    .bind::<BigInt, _>(params.offset())
                    .load::<EmployeeResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(
                conn,
                SELECT_EMPLOYEES,
                vec![params.limit_or(50).into(), params.offset().into()],
            )
            .await
        }
    }
}

async fn suppliers(State(state): State<AppState>, Query(params): Query<QueryParams>) -> HttpResult {
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, SupplierResponse>(SELECT_SUPPLIERS)
                .bind(params.limit_or(50))
                .bind(params.offset())
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_SUPPLIERS)
                    .bind::<BigInt, _>(params.limit_or(50))
                    .bind::<BigInt, _>(params.offset())
                    .load::<SupplierResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(
                conn,
                SELECT_SUPPLIERS,
                vec![params.limit_or(50).into(), params.offset().into()],
            )
            .await
        }
    }
}

async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    let id = params.id_mod(SEED_SUPPLIERS as i32);
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, SupplierResponse>(SELECT_SUPPLIER_BY_ID)
                .bind(id)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_SUPPLIER_BY_ID)
                    .bind::<Integer, _>(id)
                    .load::<SupplierResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(conn, SELECT_SUPPLIER_BY_ID, vec![id.into()]).await
        }
    }
}

async fn products(State(state): State<AppState>, Query(params): Query<QueryParams>) -> HttpResult {
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, ProductResponse>(SELECT_PRODUCTS)
                .bind(params.limit_or(50))
                .bind(params.offset())
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_PRODUCTS)
                    .bind::<BigInt, _>(params.limit_or(50))
                    .bind::<BigInt, _>(params.offset())
                    .load::<ProductResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(
                conn,
                SELECT_PRODUCTS,
                vec![params.limit_or(50).into(), params.offset().into()],
            )
            .await
        }
    }
}

async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    let id = params.id_mod(SEED_EMPLOYEES as i32);
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, EmployeeWithRecipientResponse>(SELECT_EMPLOYEE_WITH_RECIPIENT)
                .bind(id)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_EMPLOYEE_WITH_RECIPIENT)
                    .bind::<Integer, _>(id)
                    .load::<EmployeeWithRecipientResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(conn, SELECT_EMPLOYEE_WITH_RECIPIENT, vec![id.into()]).await
        }
    }
}

async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    let id = params.id_mod(SEED_PRODUCTS as i32);
    match state {
        AppState::Sqlx(pool) => {
            let rows = sqlx::query_as::<_, ProductWithSupplierRow>(SELECT_PRODUCT_WITH_SUPPLIER)
                .bind(id)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            json(
                rows.into_iter()
                    .map(ProductWithSupplierResponse::from)
                    .collect::<Vec<_>>(),
            )
        }
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_PRODUCT_WITH_SUPPLIER)
                    .bind::<Integer, _>(id)
                    .load::<ProductWithSupplierRow>(conn)
                    .map(|rows| {
                        rows.into_iter()
                            .map(ProductWithSupplierResponse::from)
                            .collect::<Vec<_>>()
                    })
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(conn, SELECT_PRODUCT_WITH_SUPPLIER, vec![id.into()]).await
        }
    }
}

async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, OrderWithDetailsResponse>(SELECT_ORDERS_WITH_DETAILS)
                .bind(params.limit_or(50))
                .bind(params.offset())
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SELECT_ORDERS_WITH_DETAILS)
                    .bind::<BigInt, _>(params.limit_or(50))
                    .bind::<BigInt, _>(params.offset())
                    .load::<OrderWithDetailsResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(
                conn,
                SELECT_ORDERS_WITH_DETAILS,
                vec![params.limit_or(50).into(), params.offset().into()],
            )
            .await
        }
    }
}

async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    let id = params.id_mod(SEED_ORDERS as i32);
    match state {
        AppState::Sqlx(pool) => {
            let orders = sqlx::query_as::<_, OrderBase>(SELECT_ORDER_BASE)
                .bind(id)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let details = sqlx::query_as::<_, OrderDetailResponse>(SELECT_ORDER_DETAILS)
                .bind(id)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            json(
                orders
                    .into_iter()
                    .map(|order| order.with_details(details.clone()))
                    .collect::<Vec<_>>(),
            )
        }
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                let orders = diesel::sql_query(SELECT_ORDER_BASE)
                    .bind::<Integer, _>(id)
                    .load::<OrderBase>(conn)?;
                let details = diesel::sql_query(SELECT_ORDER_DETAILS)
                    .bind::<Integer, _>(id)
                    .load::<OrderDetailResponse>(conn)?;
                Ok(orders
                    .into_iter()
                    .map(|order| order.with_details(details.clone()))
                    .collect::<Vec<_>>())
            })
            .await
        }
        AppState::SeaOrm(conn) => sea_order_with_details(conn, id, false).await,
    }
}

async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    let id = params.id_mod(SEED_ORDERS as i32);
    match state {
        AppState::Sqlx(pool) => {
            let orders = sqlx::query_as::<_, OrderBase>(SELECT_ORDER_BASE)
                .bind(id)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            let details =
                sqlx::query_as::<_, OrderDetailProductResponse>(SELECT_ORDER_DETAIL_PRODUCTS)
                    .bind(id)
                    .fetch_all(&pool)
                    .await
                    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
            json(
                orders
                    .into_iter()
                    .map(|order| order.with_detail_products(details.clone()))
                    .collect::<Vec<_>>(),
            )
        }
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                let orders = diesel::sql_query(SELECT_ORDER_BASE)
                    .bind::<Integer, _>(id)
                    .load::<OrderBase>(conn)?;
                let details = diesel::sql_query(SELECT_ORDER_DETAIL_PRODUCTS)
                    .bind::<Integer, _>(id)
                    .load::<OrderDetailProductResponse>(conn)?;
                Ok(orders
                    .into_iter()
                    .map(|order| order.with_detail_products(details.clone()))
                    .collect::<Vec<_>>())
            })
            .await
        }
        AppState::SeaOrm(conn) => sea_order_with_details(conn, id, true).await,
    }
}

async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or_default());
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, CustomerResponse>(SEARCH_CUSTOMERS)
                .bind(&pattern)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SEARCH_CUSTOMERS)
                    .bind::<Text, _>(pattern)
                    .load::<CustomerResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => {
            sea_json_array(conn, SEARCH_CUSTOMERS, vec![pattern.into()]).await
        }
    }
}

async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> HttpResult {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or_default());
    match state {
        AppState::Sqlx(pool) => json(
            sqlx::query_as::<_, ProductResponse>(SEARCH_PRODUCTS)
                .bind(&pattern)
                .fetch_all(&pool)
                .await
                .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        ),
        AppState::Diesel(conn) => {
            diesel_json(conn, move |conn| {
                diesel::sql_query(SEARCH_PRODUCTS)
                    .bind::<Text, _>(pattern)
                    .load::<ProductResponse>(conn)
            })
            .await
        }
        AppState::SeaOrm(conn) => sea_json_array(conn, SEARCH_PRODUCTS, vec![pattern.into()]).await,
    }
}

fn json(value: impl Serialize) -> HttpResult {
    serde_json::to_value(value)
        .map(Json)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

async fn diesel_json<T, F>(conn: Arc<Mutex<PgConnection>>, f: F) -> HttpResult
where
    T: Serialize + Send + 'static,
    F: FnOnce(&mut PgConnection) -> Result<T, diesel::result::Error> + Send + 'static,
{
    let value = tokio::task::spawn_blocking(move || {
        let mut conn = conn
            .lock()
            .map_err(|_| diesel::result::Error::RollbackTransaction)?;
        f(&mut conn)
    })
    .await
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    json(value)
}

async fn sea_json_array(
    conn: Arc<DatabaseConnection>,
    sql: &str,
    values: Vec<sea_orm::Value>,
) -> HttpResult {
    let rows = conn
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            values,
        ))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let value = row_to_json(&row).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        out.push(value);
    }
    Ok(Json(serde_json::Value::Array(out)))
}

async fn sea_order_with_details(
    conn: Arc<DatabaseConnection>,
    id: i32,
    with_products: bool,
) -> HttpResult {
    let sql = if with_products {
        r#"
        SELECT json_build_object(
          'id', o.id, 'orderDate', o.order_date, 'requiredDate', o.required_date,
          'shippedDate', o.shipped_date, 'shipVia', o.ship_via, 'freight', o.freight,
          'shipName', o.ship_name, 'shipCity', o.ship_city, 'shipRegion', o.ship_region,
          'shipPostalCode', o.ship_postal_code, 'shipCountry', o.ship_country,
          'customerId', o.customer_id, 'employeeId', o.employee_id,
          'details', COALESCE((
            SELECT json_agg(json_build_object(
              'unitPrice', d.unit_price, 'quantity', d.quantity, 'discount', d.discount,
              'orderId', d.order_id, 'productId', d.product_id, 'productName', p.name
            ))
            FROM order_details d LEFT JOIN products p ON d.product_id = p.id
            WHERE d.order_id = o.id
          ), '[]'::json)
        ) AS value
        FROM orders o WHERE o.id = $1"#
    } else {
        r#"
        SELECT json_build_object(
          'id', o.id, 'orderDate', o.order_date, 'requiredDate', o.required_date,
          'shippedDate', o.shipped_date, 'shipVia', o.ship_via, 'freight', o.freight,
          'shipName', o.ship_name, 'shipCity', o.ship_city, 'shipRegion', o.ship_region,
          'shipPostalCode', o.ship_postal_code, 'shipCountry', o.ship_country,
          'customerId', o.customer_id, 'employeeId', o.employee_id,
          'details', COALESCE((
            SELECT json_agg(json_build_object(
              'unitPrice', d.unit_price, 'quantity', d.quantity, 'discount', d.discount,
              'orderId', d.order_id, 'productId', d.product_id
            ))
            FROM order_details d WHERE d.order_id = o.id
          ), '[]'::json)
        ) AS value
        FROM orders o WHERE o.id = $1"#
    };

    let rows = conn
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            sql,
            vec![id.into()],
        ))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let mut out = Vec::with_capacity(rows.len());
    for row in rows {
        let value: serde_json::Value = row
            .try_get("", "value")
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        out.push(value);
    }
    Ok(Json(serde_json::Value::Array(out)))
}

fn row_to_json(row: &sea_orm::QueryResult) -> Result<serde_json::Value, sea_orm::DbErr> {
    let mut map = serde_json::Map::new();

    for (snake, camel) in [
        ("company_name", "companyName"),
        ("contact_name", "contactName"),
        ("contact_title", "contactTitle"),
        ("address", "address"),
        ("city", "city"),
        ("postal_code", "postalCode"),
        ("region", "region"),
        ("country", "country"),
        ("phone", "phone"),
        ("fax", "fax"),
        ("last_name", "lastName"),
        ("first_name", "firstName"),
        ("title", "title"),
        ("title_of_courtesy", "titleOfCourtesy"),
        ("home_phone", "homePhone"),
        ("notes", "notes"),
        ("recipient_last_name", "recipientLastName"),
        ("recipient_first_name", "recipientFirstName"),
        ("name", "name"),
        ("qt_per_unit", "qtPerUnit"),
        ("ship_name", "shipName"),
        ("ship_city", "shipCity"),
        ("ship_country", "shipCountry"),
    ] {
        insert_sea_opt_string(row, &mut map, snake, camel)?;
    }

    for (snake, camel) in [
        ("id", "id"),
        ("extension", "extension"),
        ("recipient_id", "recipientId"),
        ("units_in_stock", "unitsInStock"),
        ("units_on_order", "unitsOnOrder"),
        ("reorder_level", "reorderLevel"),
        ("discontinued", "discontinued"),
        ("supplier_id", "supplierId"),
    ] {
        insert_sea_i32(row, &mut map, snake, camel)?;
    }

    insert_sea_i64(row, &mut map, "products_count", "productsCount")?;
    insert_sea_f64(row, &mut map, "unit_price", "unitPrice")?;
    insert_sea_f64(row, &mut map, "quantity_sum", "quantitySum")?;
    insert_sea_f64(row, &mut map, "total_price", "totalPrice")?;
    insert_sea_opt_date(row, &mut map, "birth_date", "birthDate")?;
    insert_sea_opt_date(row, &mut map, "hire_date", "hireDate")?;
    insert_sea_opt_date(row, &mut map, "shipped_date", "shippedDate")?;

    if let Ok(s_id) = row.try_get::<i32>("", "s_id") {
        let supplier = serde_json::json!({
            "id": s_id,
            "companyName": sea_opt_string(row, "s_company_name")?,
            "contactName": sea_opt_string(row, "s_contact_name")?,
            "contactTitle": sea_opt_string(row, "s_contact_title")?,
            "address": sea_opt_string(row, "s_address")?,
            "city": sea_opt_string(row, "s_city")?,
            "region": sea_opt_string(row, "s_region")?,
            "postalCode": sea_opt_string(row, "s_postal_code")?,
            "country": sea_opt_string(row, "s_country")?,
            "phone": sea_opt_string(row, "s_phone")?
        });
        map.insert("supplier".to_string(), supplier);
    }

    Ok(serde_json::Value::Object(map))
}

fn insert_sea_i32(
    row: &sea_orm::QueryResult,
    map: &mut serde_json::Map<String, serde_json::Value>,
    snake: &str,
    camel: &str,
) -> Result<(), sea_orm::DbErr> {
    if let Ok(value) = row.try_get::<Option<i32>>("", snake) {
        map.insert(camel.to_string(), serde_json::json!(value));
    }
    Ok(())
}

fn insert_sea_i64(
    row: &sea_orm::QueryResult,
    map: &mut serde_json::Map<String, serde_json::Value>,
    snake: &str,
    camel: &str,
) -> Result<(), sea_orm::DbErr> {
    if let Ok(value) = row.try_get::<Option<i64>>("", snake) {
        map.insert(camel.to_string(), serde_json::json!(value));
    }
    Ok(())
}

fn insert_sea_f64(
    row: &sea_orm::QueryResult,
    map: &mut serde_json::Map<String, serde_json::Value>,
    snake: &str,
    camel: &str,
) -> Result<(), sea_orm::DbErr> {
    if let Ok(value) = row.try_get::<Option<f64>>("", snake) {
        map.insert(camel.to_string(), serde_json::json!(value));
    }
    Ok(())
}

fn insert_sea_opt_string(
    row: &sea_orm::QueryResult,
    map: &mut serde_json::Map<String, serde_json::Value>,
    snake: &str,
    camel: &str,
) -> Result<(), sea_orm::DbErr> {
    if let Ok(value) = sea_opt_string(row, snake) {
        map.insert(camel.to_string(), serde_json::json!(value));
    }
    Ok(())
}

fn insert_sea_opt_date(
    row: &sea_orm::QueryResult,
    map: &mut serde_json::Map<String, serde_json::Value>,
    snake: &str,
    camel: &str,
) -> Result<(), sea_orm::DbErr> {
    if let Ok(value) = row.try_get::<Option<NaiveDate>>("", snake) {
        map.insert(camel.to_string(), serde_json::json!(value));
    }
    Ok(())
}

fn sea_opt_string(
    row: &sea_orm::QueryResult,
    column: &str,
) -> Result<Option<String>, sea_orm::DbErr> {
    row.try_get::<Option<String>>("", column)
}

fn normalize_database_url() -> String {
    let raw = std::env::var("DATABASE_URL").unwrap_or_default();
    if raw.trim().is_empty() {
        return "postgres://postgres:postgres@localhost:5432/drizzle_test".to_string();
    }
    if raw.starts_with("postgres://") || raw.starts_with("postgresql://") {
        return raw;
    }

    let mut parts = BTreeMap::new();
    for token in raw.split_whitespace() {
        if let Some((key, value)) = token.split_once('=') {
            parts.insert(key, value);
        }
    }
    let user = parts.get("user").copied().unwrap_or("postgres");
    let password = parts.get("password").copied().unwrap_or("postgres");
    let host = parts.get("host").copied().unwrap_or("localhost");
    let port = parts.get("port").copied().unwrap_or("5432");
    let dbname = parts.get("dbname").copied().unwrap_or("drizzle_test");
    format!("postgres://{user}:{password}@{host}:{port}/{dbname}")
}

fn seed_postgres(seed: u64) -> Result<(), DynError> {
    let status = Command::new("cargo")
        .args([
            "run",
            "-q",
            "-p",
            "bench-runner",
            "--",
            "seed-postgres",
            "--seed",
            &seed.to_string(),
        ])
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("bench-runner seed-postgres exited with {status}").into())
    }
}
