use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use chrono::NaiveDate;
use drizzle::postgres::prelude::*;
use std::sync::Arc;
use tokio_postgres::{Row, Statement, types::ToSql};

#[PostgresTable(name = "customers")]
struct Customer {
    #[column(serial, primary)]
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

#[PostgresTable(name = "employees")]
struct Employee {
    #[column(serial, primary)]
    id: i32,
    last_name: String,
    first_name: Option<String>,
    title: String,
    title_of_courtesy: String,
    birth_date: NaiveDate,
    hire_date: NaiveDate,
    address: String,
    city: String,
    postal_code: String,
    country: String,
    home_phone: String,
    extension: i32,
    notes: String,
    #[column(references = Employee::id)]
    recipient_id: Option<i32>,
}

#[PostgresTable(name = "orders")]
struct Order {
    #[column(serial, primary)]
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
    #[column(references = Customer::id)]
    customer_id: i32,
    #[column(references = Employee::id)]
    employee_id: i32,
}

#[PostgresTable(name = "suppliers")]
struct Supplier {
    #[column(serial, primary)]
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

#[PostgresTable(name = "products")]
struct Product {
    #[column(serial, primary)]
    id: i32,
    name: String,
    qt_per_unit: String,
    unit_price: f64,
    units_in_stock: i32,
    units_on_order: i32,
    reorder_level: i32,
    discontinued: i32,
    #[column(references = Supplier::id)]
    supplier_id: i32,
}

#[PostgresTable(name = "order_details")]
struct Detail {
    unit_price: f64,
    quantity: i32,
    discount: f64,
    #[column(references = Order::id)]
    order_id: i32,
    #[column(references = Product::id)]
    product_id: i32,
}

#[derive(PostgresSchema)]
struct Schema {
    customer: Customer,
    employee: Employee,
    order: Order,
    supplier: Supplier,
    product: Product,
    detail: Detail,
}

// Response types — same as pg_sync
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
    birth_date: NaiveDate,
    hire_date: NaiveDate,
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
    shipped_date: Option<NaiveDate>,
    ship_name: String,
    ship_city: String,
    ship_country: String,
    products_count: i64,
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

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct EmployeeWithRecipientResponse {
    id: i32,
    last_name: String,
    first_name: Option<String>,
    title: String,
    title_of_courtesy: String,
    birth_date: NaiveDate,
    hire_date: NaiveDate,
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

const SQL_CUSTOMERS: &str = "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers ORDER BY id LIMIT $1::bigint OFFSET $2::bigint";
const SQL_CUSTOMER_BY_ID: &str = "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE id = $1";
const SQL_EMPLOYEES: &str = "SELECT id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id FROM employees ORDER BY id LIMIT $1::bigint OFFSET $2::bigint";
const SQL_SUPPLIERS: &str = "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers ORDER BY id LIMIT $1::bigint OFFSET $2::bigint";
const SQL_SUPPLIER_BY_ID: &str = "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers WHERE id = $1";
const SQL_PRODUCTS: &str = "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products ORDER BY id LIMIT $1::bigint OFFSET $2::bigint";
const SQL_EMPLOYEE_WITH_RECIPIENT: &str = "SELECT e.id, e.last_name, e.first_name, e.title, e.title_of_courtesy, e.birth_date, e.hire_date, e.address, e.city, e.postal_code, e.country, e.home_phone, e.extension, e.notes, e.recipient_id, r.last_name, r.first_name FROM employees e LEFT JOIN employees r ON e.recipient_id = r.id WHERE e.id = $1";
const SQL_PRODUCT_WITH_SUPPLIER: &str = "SELECT p.id, p.name, p.qt_per_unit, p.unit_price, p.units_in_stock, p.units_on_order, p.reorder_level, p.discontinued, p.supplier_id, s.id, s.company_name, s.contact_name, s.contact_title, s.address, s.city, s.region, s.postal_code, s.country, s.phone FROM products p INNER JOIN suppliers s ON p.supplier_id = s.id WHERE p.id = $1";
const SQL_ORDERS_WITH_DETAILS: &str = "SELECT o.id, o.shipped_date, o.ship_name, o.ship_city, o.ship_country, count(d.product_id), COALESCE(sum(d.quantity)::float8, 0), COALESCE(sum(d.quantity::float8 * d.unit_price), 0) FROM orders o LEFT JOIN order_details d ON o.id = d.order_id GROUP BY o.id ORDER BY o.id LIMIT $1::bigint OFFSET $2::bigint";
const SQL_ORDER_BY_ID: &str = "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = $1";
const SQL_ORDER_DETAILS_BY_ORDER: &str = "SELECT unit_price, quantity, discount, order_id, product_id FROM order_details WHERE order_id = $1";
const SQL_ORDER_DETAIL_PRODUCTS_BY_ORDER: &str = "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name FROM order_details d LEFT JOIN products p ON d.product_id = p.id WHERE d.order_id = $1";
const SQL_SEARCH_CUSTOMER: &str = "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE company_name ILIKE $1";
const SQL_SEARCH_PRODUCT: &str = "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products WHERE name ILIKE $1";

#[derive(Clone, Copy)]
enum PgQueryMode {
    Prepared,
    Unprepared,
}

struct PgStatements {
    customers: Statement,
    customer_by_id: Statement,
    employees: Statement,
    suppliers: Statement,
    supplier_by_id: Statement,
    products: Statement,
    employee_with_recipient: Statement,
    product_with_supplier: Statement,
    orders_with_details: Statement,
    order_by_id: Statement,
    order_details_by_order: Statement,
    order_detail_products_by_order: Statement,
    search_customer: Statement,
    search_product: Statement,
}

impl PgStatements {
    async fn prepare(client: &tokio_postgres::Client) -> Result<Self, tokio_postgres::Error> {
        Ok(Self {
            customers: client.prepare(SQL_CUSTOMERS).await?,
            customer_by_id: client.prepare(SQL_CUSTOMER_BY_ID).await?,
            employees: client.prepare(SQL_EMPLOYEES).await?,
            suppliers: client.prepare(SQL_SUPPLIERS).await?,
            supplier_by_id: client.prepare(SQL_SUPPLIER_BY_ID).await?,
            products: client.prepare(SQL_PRODUCTS).await?,
            employee_with_recipient: client.prepare(SQL_EMPLOYEE_WITH_RECIPIENT).await?,
            product_with_supplier: client.prepare(SQL_PRODUCT_WITH_SUPPLIER).await?,
            orders_with_details: client.prepare(SQL_ORDERS_WITH_DETAILS).await?,
            order_by_id: client.prepare(SQL_ORDER_BY_ID).await?,
            order_details_by_order: client.prepare(SQL_ORDER_DETAILS_BY_ORDER).await?,
            order_detail_products_by_order: client
                .prepare(SQL_ORDER_DETAIL_PRODUCTS_BY_ORDER)
                .await?,
            search_customer: client.prepare(SQL_SEARCH_CUSTOMER).await?,
            search_product: client.prepare(SQL_SEARCH_PRODUCT).await?,
        })
    }
}

struct PgConn {
    client: tokio_postgres::Client,
    statements: Option<PgStatements>,
}

impl PgConn {
    async fn query(
        &self,
        sql: &'static str,
        statement: fn(&PgStatements) -> &Statement,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, StatusCode> {
        let result = if let Some(statements) = &self.statements {
            self.client.query(statement(statements), params).await
        } else {
            self.client.query(sql, params).await
        };
        result.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
    }
}

#[derive(Clone)]
struct AppState {
    dbs: Arc<Vec<PgConn>>,
    next: Arc<AtomicUsize>,
}

impl AppState {
    fn db(&self) -> &PgConn {
        let idx = self.next.fetch_add(1, Ordering::Relaxed) % self.dbs.len();
        &self.dbs[idx]
    }
}

pub async fn serve_unprepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, PgQueryMode::Unprepared).await
}

pub async fn serve_prepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, PgQueryMode::Prepared).await
}

async fn serve_with_mode(seed: u64, mode: PgQueryMode) -> Result<ServerHandle, Fail> {
    let database_url = pg_url();
    tokio::task::spawn_blocking(move || super::pg_sync::seed_database_url(&database_url, seed))
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("pg_tokio seed panicked: {err}")))?
        .map_err(|msg| Fail::new(Code::RunFail, msg))?;

    let mut dbs = Vec::with_capacity(super::POSTGRES_POOL_SIZE);
    for _ in 0..super::POSTGRES_POOL_SIZE {
        let (client, driver) = ::tokio_postgres::connect(&pg_url(), ::tokio_postgres::NoTls)
            .await
            .map_err(|err| Fail::new(Code::RunFail, format!("postgres connect failed: {err}")))?;
        tokio::spawn(async move {
            let _ = driver.await;
        });
        let statements = if matches!(mode, PgQueryMode::Prepared) {
            Some(PgStatements::prepare(&client).await.map_err(|err| {
                Fail::new(Code::RunFail, format!("postgres prepare failed: {err}"))
            })?)
        } else {
            None
        };
        dbs.push(PgConn { client, statements });
    }

    let router = Router::new()
        .route("/stats", get(stats))
        .route("/customers", get(customers_handler))
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
        .with_state(AppState {
            dbs: Arc::new(dbs),
            next: Arc::new(AtomicUsize::new(0)),
        });
    spawn_server(router).await
}

#[debug_handler(state = AppState)]
async fn stats(_: State<AppState>) -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    Json(cpu_usage(&sys))
}

#[debug_handler(state = AppState)]
async fn customers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50) as i64;
    let offset = params.offset() as i64;
    let rows = state
        .db()
        .query(SQL_CUSTOMERS, |s| &s.customers, &[&limit, &offset])
        .await?;
    let resp: Vec<CustomerResponse> = rows
        .iter()
        .map(|r| CustomerResponse {
            id: r.get(0),
            company_name: r.get(1),
            contact_name: r.get(2),
            contact_title: r.get(3),
            address: r.get(4),
            city: r.get(5),
            postal_code: r.get(6),
            region: r.get(7),
            country: r.get(8),
            phone: r.get(9),
            fax: r.get(10),
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
    let rows = state
        .db()
        .query(SQL_CUSTOMER_BY_ID, |s| &s.customer_by_id, &[&id])
        .await?;
    let resp: Vec<CustomerResponse> = rows
        .iter()
        .map(|r| CustomerResponse {
            id: r.get(0),
            company_name: r.get(1),
            contact_name: r.get(2),
            contact_title: r.get(3),
            address: r.get(4),
            city: r.get(5),
            postal_code: r.get(6),
            region: r.get(7),
            country: r.get(8),
            phone: r.get(9),
            fax: r.get(10),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn employees_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50) as i64;
    let offset = params.offset() as i64;
    let rows = state
        .db()
        .query(SQL_EMPLOYEES, |s| &s.employees, &[&limit, &offset])
        .await?;
    let resp: Vec<EmployeeResponse> = rows
        .iter()
        .map(|r| EmployeeResponse {
            id: r.get(0),
            last_name: r.get(1),
            first_name: r.get(2),
            title: r.get(3),
            title_of_courtesy: r.get(4),
            birth_date: r.get(5),
            hire_date: r.get(6),
            address: r.get(7),
            city: r.get(8),
            postal_code: r.get(9),
            country: r.get(10),
            home_phone: r.get(11),
            extension: r.get(12),
            notes: r.get(13),
            recipient_id: r.get(14),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn suppliers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50) as i64;
    let offset = params.offset() as i64;
    let rows = state
        .db()
        .query(SQL_SUPPLIERS, |s| &s.suppliers, &[&limit, &offset])
        .await?;
    let resp: Vec<SupplierResponse> = rows
        .iter()
        .map(|r| SupplierResponse {
            id: r.get(0),
            company_name: r.get(1),
            contact_name: r.get(2),
            contact_title: r.get(3),
            address: r.get(4),
            city: r.get(5),
            region: r.get(6),
            postal_code: r.get(7),
            country: r.get(8),
            phone: r.get(9),
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
    let rows = state
        .db()
        .query(SQL_SUPPLIER_BY_ID, |s| &s.supplier_by_id, &[&id])
        .await?;
    let resp: Vec<SupplierResponse> = rows
        .iter()
        .map(|r| SupplierResponse {
            id: r.get(0),
            company_name: r.get(1),
            contact_name: r.get(2),
            contact_title: r.get(3),
            address: r.get(4),
            city: r.get(5),
            region: r.get(6),
            postal_code: r.get(7),
            country: r.get(8),
            phone: r.get(9),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn products_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let limit = params.limit_or(50) as i64;
    let offset = params.offset() as i64;
    let rows = state
        .db()
        .query(SQL_PRODUCTS, |s| &s.products, &[&limit, &offset])
        .await?;
    let resp: Vec<ProductResponse> = rows
        .iter()
        .map(|r| ProductResponse {
            id: r.get(0),
            name: r.get(1),
            qt_per_unit: r.get(2),
            unit_price: r.get(3),
            units_in_stock: r.get(4),
            units_on_order: r.get(5),
            reorder_level: r.get(6),
            discontinued: r.get(7),
            supplier_id: r.get(8),
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
    let rows = state
        .db()
        .query(
            SQL_EMPLOYEE_WITH_RECIPIENT,
            |s| &s.employee_with_recipient,
            &[&id],
        )
        .await?;
    let resp: Vec<EmployeeWithRecipientResponse> = rows
        .iter()
        .map(|r| EmployeeWithRecipientResponse {
            id: r.get(0),
            last_name: r.get(1),
            first_name: r.get(2),
            title: r.get(3),
            title_of_courtesy: r.get(4),
            birth_date: r.get(5),
            hire_date: r.get(6),
            address: r.get(7),
            city: r.get(8),
            postal_code: r.get(9),
            country: r.get(10),
            home_phone: r.get(11),
            extension: r.get(12),
            notes: r.get(13),
            recipient_id: r.get(14),
            recipient_last_name: r.get(15),
            recipient_first_name: r.get(16),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(5000);
    let rows = state
        .db()
        .query(
            SQL_PRODUCT_WITH_SUPPLIER,
            |s| &s.product_with_supplier,
            &[&id],
        )
        .await?;
    let resp: Vec<ProductWithSupplierResponse> = rows
        .iter()
        .map(|r| ProductWithSupplierResponse {
            id: r.get(0),
            name: r.get(1),
            qt_per_unit: r.get(2),
            unit_price: r.get(3),
            units_in_stock: r.get(4),
            units_on_order: r.get(5),
            reorder_level: r.get(6),
            discontinued: r.get(7),
            supplier_id: r.get(8),
            supplier: SupplierResponse {
                id: r.get(9),
                company_name: r.get(10),
                contact_name: r.get(11),
                contact_title: r.get(12),
                address: r.get(13),
                city: r.get(14),
                region: r.get(15),
                postal_code: r.get(16),
                country: r.get(17),
                phone: r.get(18),
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
    let limit = params.limit_or(50) as i64;
    let offset = params.offset() as i64;
    let rows = state
        .db()
        .query(
            SQL_ORDERS_WITH_DETAILS,
            |s| &s.orders_with_details,
            &[&limit, &offset],
        )
        .await?;
    let resp: Vec<OrderWithDetailsResponse> = rows
        .iter()
        .map(|r| OrderWithDetailsResponse {
            id: r.get(0),
            shipped_date: r.get(1),
            ship_name: r.get(2),
            ship_city: r.get(3),
            ship_country: r.get(4),
            products_count: r.get(5),
            quantity_sum: r.get(6),
            total_price: r.get(7),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let id = params.user_id(50000);
    let db = state.db();
    let order_rows = db
        .query(SQL_ORDER_BY_ID, |s| &s.order_by_id, &[&id])
        .await?;
    let detail_rows = db
        .query(
            SQL_ORDER_DETAILS_BY_ORDER,
            |s| &s.order_details_by_order,
            &[&id],
        )
        .await?;
    let details: Vec<OrderDetailResponse> = detail_rows
        .iter()
        .map(|r| OrderDetailResponse {
            unit_price: r.get(0),
            quantity: r.get(1),
            discount: r.get(2),
            order_id: r.get(3),
            product_id: r.get(4),
        })
        .collect();
    let resp: Vec<SingleOrderWithDetailsResponse> = order_rows
        .iter()
        .map(|r| SingleOrderWithDetailsResponse {
            id: r.get(0),
            order_date: r.get(1),
            required_date: r.get(2),
            shipped_date: r.get(3),
            ship_via: r.get(4),
            freight: r.get(5),
            ship_name: r.get(6),
            ship_city: r.get(7),
            ship_region: r.get(8),
            ship_postal_code: r.get(9),
            ship_country: r.get(10),
            customer_id: r.get(11),
            employee_id: r.get(12),
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
    let db = state.db();
    let order_rows = db
        .query(SQL_ORDER_BY_ID, |s| &s.order_by_id, &[&id])
        .await?;
    let detail_rows = db
        .query(
            SQL_ORDER_DETAIL_PRODUCTS_BY_ORDER,
            |s| &s.order_detail_products_by_order,
            &[&id],
        )
        .await?;
    let details: Vec<OrderDetailProductResponse> = detail_rows
        .iter()
        .map(|r| OrderDetailProductResponse {
            unit_price: r.get(0),
            quantity: r.get(1),
            discount: r.get(2),
            order_id: r.get(3),
            product_id: r.get(4),
            product_name: r.get::<_, Option<String>>(5).unwrap_or_default(),
        })
        .collect();
    let resp: Vec<SingleOrderWithDetailsAndProductsResponse> = order_rows
        .iter()
        .map(|r| SingleOrderWithDetailsAndProductsResponse {
            id: r.get(0),
            order_date: r.get(1),
            required_date: r.get(2),
            shipped_date: r.get(3),
            ship_via: r.get(4),
            freight: r.get(5),
            ship_name: r.get(6),
            ship_city: r.get(7),
            ship_region: r.get(8),
            ship_postal_code: r.get(9),
            ship_country: r.get(10),
            customer_id: r.get(11),
            employee_id: r.get(12),
            details: details.clone(),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let term = params.term.as_deref().unwrap_or("");
    let pattern = format!("%{term}%");
    let rows = state
        .db()
        .query(SQL_SEARCH_CUSTOMER, |s| &s.search_customer, &[&pattern])
        .await?;
    let resp: Vec<CustomerResponse> = rows
        .iter()
        .map(|r| CustomerResponse {
            id: r.get(0),
            company_name: r.get(1),
            contact_name: r.get(2),
            contact_title: r.get(3),
            address: r.get(4),
            city: r.get(5),
            postal_code: r.get(6),
            region: r.get(7),
            country: r.get(8),
            phone: r.get(9),
            fax: r.get(10),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let term = params.term.as_deref().unwrap_or("");
    let pattern = format!("%{term}%");
    let rows = state
        .db()
        .query(SQL_SEARCH_PRODUCT, |s| &s.search_product, &[&pattern])
        .await?;
    let resp: Vec<ProductResponse> = rows
        .iter()
        .map(|r| ProductResponse {
            id: r.get(0),
            name: r.get(1),
            qt_per_unit: r.get(2),
            unit_price: r.get(3),
            units_in_stock: r.get(4),
            units_on_order: r.get(5),
            reorder_level: r.get(6),
            discontinued: r.get(7),
            supplier_id: r.get(8),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}
