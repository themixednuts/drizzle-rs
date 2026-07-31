use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use chrono::NaiveDate;
use drizzle::core::expr::{alias, coalesce, count, eq, sum};
use drizzle::postgres::prelude::*;
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

type PgDb = drizzle::postgres::tokio::Drizzle<Schema>;

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

impl From<SelectCustomer> for CustomerResponse {
    fn from(row: SelectCustomer) -> Self {
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

impl From<SelectEmployee> for EmployeeResponse {
    fn from(row: SelectEmployee) -> Self {
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

impl From<SelectSupplier> for SupplierResponse {
    fn from(row: SelectSupplier) -> Self {
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

impl From<SelectProduct> for ProductResponse {
    fn from(row: SelectProduct) -> Self {
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

impl From<SelectDetail> for OrderDetailResponse {
    fn from(row: SelectDetail) -> Self {
        Self {
            unit_price: row.unit_price,
            quantity: row.quantity,
            discount: row.discount,
            order_id: row.order_id,
            product_id: row.product_id,
        }
    }
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

impl SingleOrderWithDetailsResponse {
    fn new(order: SelectOrder, details: Vec<OrderDetailResponse>) -> Self {
        Self {
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
            details,
        }
    }
}

impl SingleOrderWithDetailsAndProductsResponse {
    fn new(order: SelectOrder, details: Vec<OrderDetailProductResponse>) -> Self {
        Self {
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
            details,
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

type EmployeeWithRecipientRow = (
    i32,
    String,
    Option<String>,
    String,
    String,
    NaiveDate,
    NaiveDate,
    String,
    String,
    String,
    String,
    String,
    i32,
    String,
    Option<i32>,
    Option<String>,
    Option<String>,
);

type ProductWithSupplierRow = (
    i32,
    String,
    String,
    f64,
    i32,
    i32,
    i32,
    i32,
    i32,
    i32,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
);

type OrderAggregateRow = (
    i32,
    Option<NaiveDate>,
    String,
    String,
    String,
    i64,
    i64,
    f64,
);

type DetailWithProductRow = (f64, i32, f64, i32, i32, Option<String>);

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

#[derive(Clone, Copy, Eq, PartialEq)]
enum PgMode {
    /// drizzle-rs typed select builder over tokio-postgres (statements cached
    /// by the driver's own statement cache).
    DrizzleSelect,
    /// drizzle-rs relational query API (`db.query(..).with(..)`).
    DrizzleQuery,
    /// Hand-written SQL through `Client::query` with server-side prepare.
    RawPrepared,
    /// Hand-written SQL through `Client::query` with the one-shot path.
    RawUnprepared,
}

impl PgMode {
    fn is_raw(self) -> bool {
        matches!(self, Self::RawPrepared | Self::RawUnprepared)
    }
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

/// One pooled connection.
///
/// The drizzle handle owns the client, so the raw baselines reach the same
/// connection through `db.conn()` instead of the pool holding two variants.
struct PgConn {
    db: PgDb,
    statements: Option<PgStatements>,
}

impl PgConn {
    async fn raw_query(
        &self,
        sql: &'static str,
        statement: fn(&PgStatements) -> &Statement,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, StatusCode> {
        let client = self.db.conn();
        let result = if let Some(statements) = &self.statements {
            client.query(statement(statements), params).await
        } else {
            client.query(sql, params).await
        };
        result.map_err(db_err)
    }
}

#[derive(Clone)]
struct AppState {
    dbs: super::pool::AsyncResourcePool<PgConn>,
    mode: PgMode,
    schema: Schema,
}

impl AppState {
    async fn acquire(&self) -> Result<super::pool::PooledResource<PgConn>, StatusCode> {
        self.dbs.acquire().await.map_err(db_err)
    }

    async fn raw_query(
        &self,
        sql: &'static str,
        statement: fn(&PgStatements) -> &Statement,
        params: &[&(dyn ToSql + Sync)],
    ) -> Result<Vec<Row>, StatusCode> {
        self.acquire()
            .await?
            .raw_query(sql, statement, params)
            .await
    }
}

pub async fn serve_select(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, PgMode::DrizzleSelect).await
}

pub async fn serve_query(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, PgMode::DrizzleQuery).await
}

pub async fn serve_unprepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, PgMode::RawUnprepared).await
}

pub async fn serve_prepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, PgMode::RawPrepared).await
}

async fn serve_with_mode(seed: u64, mode: PgMode) -> Result<ServerHandle, Fail> {
    let database_url = pg_url();
    tokio::task::spawn_blocking(move || super::pg_sync::seed_database_url(&database_url, seed))
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("pg_tokio seed panicked: {err}")))?
        .map_err(|msg| Fail::new(Code::RunFail, msg))?;

    let pool_size = super::configured_pool_size(super::POSTGRES_POOL_SIZE);
    let mut dbs = Vec::with_capacity(pool_size);
    for _ in 0..pool_size {
        let (client, driver) = ::tokio_postgres::connect(&pg_url(), ::tokio_postgres::NoTls)
            .await
            .map_err(|err| Fail::new(Code::RunFail, format!("postgres connect failed: {err}")))?;
        tokio::spawn(async move {
            let _ = driver.await;
        });
        let statements = if matches!(mode, PgMode::RawPrepared) {
            Some(PgStatements::prepare(&client).await.map_err(|err| {
                Fail::new(Code::RunFail, format!("postgres prepare failed: {err}"))
            })?)
        } else {
            None
        };
        let (db, _) = drizzle::postgres::tokio::Drizzle::new(client, Schema::new());
        dbs.push(PgConn { db, statements });
    }

    let router = Router::new()
        .route("/stats", get(super::stats))
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
            dbs: super::pool::AsyncResourcePool::new(dbs),
            mode,
            schema: Schema::new(),
        });
    spawn_server(router).await
}

#[debug_handler(state = AppState)]
async fn customers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    if state.mode.is_raw() {
        let rows = state
            .raw_query(
                SQL_CUSTOMERS,
                |s| &s.customers,
                &[&(limit as i64), &(offset as i64)],
            )
            .await?;
        return Ok(Json(rows.iter().map(customer_from_row).collect()));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    let rows: Vec<SelectCustomer> = match state.mode {
        PgMode::DrizzleQuery => db
            .db
            .query(schema.customer)
            .order_by(asc(schema.customer.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .await
            .map_err(db_err)?,
        _ => db
            .db
            .select(())
            .from(schema.customer)
            .order_by([asc(schema.customer.id)])
            .limit(limit)
            .offset(offset)
            .all()
            .await
            .map_err(db_err)?,
    };
    Ok(Json(rows.into_iter().map(CustomerResponse::from).collect()))
}

#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_CUSTOMERS as i32);
    if state.mode.is_raw() {
        let rows = state
            .raw_query(SQL_CUSTOMER_BY_ID, |s| &s.customer_by_id, &[&id])
            .await?;
        return Ok(Json(rows.iter().map(customer_from_row).collect()));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    let rows: Vec<SelectCustomer> = match state.mode {
        PgMode::DrizzleQuery => db
            .db
            .query(schema.customer)
            .r#where(eq(schema.customer.id, id))
            .find_many()
            .await
            .map_err(db_err)?,
        _ => db
            .db
            .select(())
            .from(schema.customer)
            .r#where(eq(schema.customer.id, id))
            .all()
            .await
            .map_err(db_err)?,
    };
    Ok(Json(rows.into_iter().map(CustomerResponse::from).collect()))
}

#[debug_handler(state = AppState)]
async fn employees_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    if state.mode.is_raw() {
        let rows = state
            .raw_query(
                SQL_EMPLOYEES,
                |s| &s.employees,
                &[&(limit as i64), &(offset as i64)],
            )
            .await?;
        return Ok(Json(rows.iter().map(employee_from_row).collect()));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    let rows: Vec<SelectEmployee> = match state.mode {
        PgMode::DrizzleQuery => db
            .db
            .query(schema.employee)
            .order_by(asc(schema.employee.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .await
            .map_err(db_err)?,
        _ => db
            .db
            .select(())
            .from(schema.employee)
            .order_by([asc(schema.employee.id)])
            .limit(limit)
            .offset(offset)
            .all()
            .await
            .map_err(db_err)?,
    };
    Ok(Json(rows.into_iter().map(EmployeeResponse::from).collect()))
}

#[debug_handler(state = AppState)]
async fn suppliers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    if state.mode.is_raw() {
        let rows = state
            .raw_query(
                SQL_SUPPLIERS,
                |s| &s.suppliers,
                &[&(limit as i64), &(offset as i64)],
            )
            .await?;
        return Ok(Json(rows.iter().map(supplier_from_row).collect()));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    let rows: Vec<SelectSupplier> = match state.mode {
        PgMode::DrizzleQuery => db
            .db
            .query(schema.supplier)
            .order_by(asc(schema.supplier.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .await
            .map_err(db_err)?,
        _ => db
            .db
            .select(())
            .from(schema.supplier)
            .order_by([asc(schema.supplier.id)])
            .limit(limit)
            .offset(offset)
            .all()
            .await
            .map_err(db_err)?,
    };
    Ok(Json(rows.into_iter().map(SupplierResponse::from).collect()))
}

#[debug_handler(state = AppState)]
async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_SUPPLIERS as i32);
    if state.mode.is_raw() {
        let rows = state
            .raw_query(SQL_SUPPLIER_BY_ID, |s| &s.supplier_by_id, &[&id])
            .await?;
        return Ok(Json(rows.iter().map(supplier_from_row).collect()));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    let rows: Vec<SelectSupplier> = match state.mode {
        PgMode::DrizzleQuery => db
            .db
            .query(schema.supplier)
            .r#where(eq(schema.supplier.id, id))
            .find_many()
            .await
            .map_err(db_err)?,
        _ => db
            .db
            .select(())
            .from(schema.supplier)
            .r#where(eq(schema.supplier.id, id))
            .all()
            .await
            .map_err(db_err)?,
    };
    Ok(Json(rows.into_iter().map(SupplierResponse::from).collect()))
}

#[debug_handler(state = AppState)]
async fn products_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    if state.mode.is_raw() {
        let rows = state
            .raw_query(
                SQL_PRODUCTS,
                |s| &s.products,
                &[&(limit as i64), &(offset as i64)],
            )
            .await?;
        return Ok(Json(rows.iter().map(product_from_row).collect()));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    let rows: Vec<SelectProduct> = match state.mode {
        PgMode::DrizzleQuery => db
            .db
            .query(schema.product)
            .order_by(asc(schema.product.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .await
            .map_err(db_err)?,
        _ => db
            .db
            .select(())
            .from(schema.product)
            .order_by([asc(schema.product.id)])
            .limit(limit)
            .offset(offset)
            .all()
            .await
            .map_err(db_err)?,
    };
    Ok(Json(rows.into_iter().map(ProductResponse::from).collect()))
}

#[debug_handler(state = AppState)]
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeWithRecipientResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_EMPLOYEES as i32);
    if state.mode.is_raw() {
        let rows = state
            .raw_query(
                SQL_EMPLOYEE_WITH_RECIPIENT,
                |s| &s.employee_with_recipient,
                &[&id],
            )
            .await?;
        return Ok(Json(
            rows.iter()
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
                .collect(),
        ));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    if state.mode == PgMode::DrizzleQuery {
        let rows = db
            .db
            .query(schema.employee)
            .with(schema.employee.recipient())
            .r#where(eq(schema.employee.id, id))
            .find_many()
            .await
            .map_err(db_err)?;
        return Ok(Json(
            rows.into_iter()
                .map(|row| {
                    let recipient = row.recipient;
                    let employee = row.inner;
                    EmployeeWithRecipientResponse {
                        recipient_last_name: recipient.as_ref().map(|r| r.last_name.clone()),
                        recipient_first_name: recipient.and_then(|r| r.first_name),
                        id: employee.id,
                        last_name: employee.last_name,
                        first_name: employee.first_name,
                        title: employee.title,
                        title_of_courtesy: employee.title_of_courtesy,
                        birth_date: employee.birth_date,
                        hire_date: employee.hire_date,
                        address: employee.address,
                        city: employee.city,
                        postal_code: employee.postal_code,
                        country: employee.country,
                        home_phone: employee.home_phone,
                        extension: employee.extension,
                        notes: employee.notes,
                        recipient_id: employee.recipient_id,
                    }
                })
                .collect(),
        ));
    }

    let recipient = Employee::alias::<super::RecipientAlias>();
    let rows: Vec<EmployeeWithRecipientRow> = db
        .db
        .select((
            schema.employee.id,
            schema.employee.last_name,
            schema.employee.first_name,
            schema.employee.title,
            schema.employee.title_of_courtesy,
            schema.employee.birth_date,
            schema.employee.hire_date,
            schema.employee.address,
            schema.employee.city,
            schema.employee.postal_code,
            schema.employee.country,
            schema.employee.home_phone,
            schema.employee.extension,
            schema.employee.notes,
            schema.employee.recipient_id,
            super::nullable(recipient.last_name),
            super::nullable(recipient.first_name),
        ))
        .from(schema.employee)
        .left_join((recipient, eq(schema.employee.recipient_id, recipient.id)))
        .r#where(eq(schema.employee.id, id))
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    id,
                    last_name,
                    first_name,
                    title,
                    title_of_courtesy,
                    birth_date,
                    hire_date,
                    address,
                    city,
                    postal_code,
                    country,
                    home_phone,
                    extension,
                    notes,
                    recipient_id,
                    recipient_last_name,
                    recipient_first_name,
                )| EmployeeWithRecipientResponse {
                    id,
                    last_name,
                    first_name,
                    title,
                    title_of_courtesy,
                    birth_date,
                    hire_date,
                    address,
                    city,
                    postal_code,
                    country,
                    home_phone,
                    extension,
                    notes,
                    recipient_id,
                    recipient_last_name,
                    recipient_first_name,
                },
            )
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductWithSupplierResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_PRODUCTS as i32);
    if state.mode.is_raw() {
        let rows = state
            .raw_query(
                SQL_PRODUCT_WITH_SUPPLIER,
                |s| &s.product_with_supplier,
                &[&id],
            )
            .await?;
        return Ok(Json(
            rows.iter()
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
                .collect(),
        ));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    if state.mode == PgMode::DrizzleQuery {
        let rows = db
            .db
            .query(schema.product)
            .with(schema.product.supplier())
            .r#where(eq(schema.product.id, id))
            .find_many()
            .await
            .map_err(db_err)?;
        return Ok(Json(
            rows.into_iter()
                .map(|row| {
                    let supplier = SupplierResponse::from(row.supplier);
                    let product = row.inner;
                    ProductWithSupplierResponse {
                        id: product.id,
                        name: product.name,
                        qt_per_unit: product.qt_per_unit,
                        unit_price: product.unit_price,
                        units_in_stock: product.units_in_stock,
                        units_on_order: product.units_on_order,
                        reorder_level: product.reorder_level,
                        discontinued: product.discontinued,
                        supplier_id: product.supplier_id,
                        supplier,
                    }
                })
                .collect(),
        ));
    }

    let rows: Vec<ProductWithSupplierRow> = db
        .db
        .select((
            schema.product.id,
            schema.product.name,
            schema.product.qt_per_unit,
            schema.product.unit_price,
            schema.product.units_in_stock,
            schema.product.units_on_order,
            schema.product.reorder_level,
            schema.product.discontinued,
            schema.product.supplier_id,
            schema.supplier.id,
            schema.supplier.company_name,
            schema.supplier.contact_name,
            schema.supplier.contact_title,
            schema.supplier.address,
            schema.supplier.city,
            schema.supplier.region,
            schema.supplier.postal_code,
            schema.supplier.country,
            schema.supplier.phone,
        ))
        .from(schema.product)
        .inner_join((
            schema.supplier,
            eq(schema.product.supplier_id, schema.supplier.id),
        ))
        .r#where(eq(schema.product.id, id))
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    id,
                    name,
                    qt_per_unit,
                    unit_price,
                    units_in_stock,
                    units_on_order,
                    reorder_level,
                    discontinued,
                    supplier_id,
                    s_id,
                    s_company_name,
                    s_contact_name,
                    s_contact_title,
                    s_address,
                    s_city,
                    s_region,
                    s_postal_code,
                    s_country,
                    s_phone,
                )| ProductWithSupplierResponse {
                    id,
                    name,
                    qt_per_unit,
                    unit_price,
                    units_in_stock,
                    units_on_order,
                    reorder_level,
                    discontinued,
                    supplier_id,
                    supplier: SupplierResponse {
                        id: s_id,
                        company_name: s_company_name,
                        contact_name: s_contact_name,
                        contact_title: s_contact_title,
                        address: s_address,
                        city: s_city,
                        region: s_region,
                        postal_code: s_postal_code,
                        country: s_country,
                        phone: s_phone,
                    },
                },
            )
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<OrderWithDetailsResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    if state.mode.is_raw() {
        let rows = state
            .raw_query(
                SQL_ORDERS_WITH_DETAILS,
                |s| &s.orders_with_details,
                &[&(limit as i64), &(offset as i64)],
            )
            .await?;
        return Ok(Json(
            rows.iter()
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
                .collect(),
        ));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    if state.mode == PgMode::DrizzleQuery {
        let rows = db
            .db
            .query(schema.order)
            .with(schema.order.details())
            .order_by(asc(schema.order.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .await
            .map_err(db_err)?;
        return Ok(Json(
            rows.into_iter()
                .map(|row| {
                    let details = row.details;
                    let order = row.inner;
                    OrderWithDetailsResponse {
                        id: order.id,
                        shipped_date: order.shipped_date,
                        ship_name: order.ship_name,
                        ship_city: order.ship_city,
                        ship_country: order.ship_country,
                        products_count: details.len() as i64,
                        quantity_sum: details.iter().map(|d| f64::from(d.quantity)).sum(),
                        total_price: details
                            .iter()
                            .map(|d| f64::from(d.quantity) * d.unit_price)
                            .sum(),
                    }
                })
                .collect(),
        ));
    }

    let rows: Vec<OrderAggregateRow> = db
        .db
        .select((
            schema.order.id,
            schema.order.shipped_date,
            schema.order.ship_name,
            schema.order.ship_city,
            schema.order.ship_country,
            alias(count(schema.detail.product_id), "products_count"),
            alias(coalesce(sum(schema.detail.quantity), 0), "quantity_sum"),
            alias(
                coalesce(sum(schema.detail.quantity * schema.detail.unit_price), 0.0),
                "total_price",
            ),
        ))
        .from(schema.order)
        .left_join((schema.detail, eq(schema.order.id, schema.detail.order_id)))
        .group_by((
            schema.order.id,
            schema.order.shipped_date,
            schema.order.ship_name,
            schema.order.ship_city,
            schema.order.ship_country,
        ))
        .order_by([asc(schema.order.id)])
        .limit(limit)
        .offset(offset)
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(
        rows.into_iter()
            .map(
                |(
                    id,
                    shipped_date,
                    ship_name,
                    ship_city,
                    ship_country,
                    products_count,
                    quantity_sum,
                    total_price,
                )| OrderWithDetailsResponse {
                    id,
                    shipped_date,
                    ship_name,
                    ship_city,
                    ship_country,
                    products_count,
                    quantity_sum: quantity_sum as f64,
                    total_price,
                },
            )
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_ORDERS as i32);
    if state.mode.is_raw() {
        let order_rows = state
            .raw_query(SQL_ORDER_BY_ID, |s| &s.order_by_id, &[&id])
            .await?;
        let detail_rows = state
            .raw_query(
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
        return Ok(Json(
            order_rows
                .iter()
                .map(|r| SingleOrderWithDetailsResponse::new(order_from_row(r), details.clone()))
                .collect(),
        ));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    if state.mode == PgMode::DrizzleQuery {
        let rows = db
            .db
            .query(schema.order)
            .with(schema.order.details())
            .r#where(eq(schema.order.id, id))
            .find_many()
            .await
            .map_err(db_err)?;
        return Ok(Json(
            rows.into_iter()
                .map(|row| {
                    let details = row.details.into_iter().map(OrderDetailResponse::from);
                    SingleOrderWithDetailsResponse::new(row.inner, details.collect())
                })
                .collect(),
        ));
    }

    let orders: Vec<SelectOrder> = db
        .db
        .select(())
        .from(schema.order)
        .r#where(eq(schema.order.id, id))
        .all()
        .await
        .map_err(db_err)?;
    let details: Vec<OrderDetailResponse> = db
        .db
        .select(())
        .from(schema.detail)
        .r#where(eq(schema.detail.order_id, id))
        .all()
        .await
        .map_err(db_err)?
        .into_iter()
        .map(|row: SelectDetail| OrderDetailResponse::from(row))
        .collect();
    Ok(Json(
        orders
            .into_iter()
            .map(|order| SingleOrderWithDetailsResponse::new(order, details.clone()))
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsAndProductsResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_ORDERS as i32);
    if state.mode.is_raw() {
        let order_rows = state
            .raw_query(SQL_ORDER_BY_ID, |s| &s.order_by_id, &[&id])
            .await?;
        let detail_rows = state
            .raw_query(
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
        return Ok(Json(
            order_rows
                .iter()
                .map(|r| {
                    SingleOrderWithDetailsAndProductsResponse::new(
                        order_from_row(r),
                        details.clone(),
                    )
                })
                .collect(),
        ));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    if state.mode == PgMode::DrizzleQuery {
        let rows = db
            .db
            .query(schema.order)
            .with(schema.order.details().with(schema.detail.product()))
            .r#where(eq(schema.order.id, id))
            .find_many()
            .await
            .map_err(db_err)?;
        return Ok(Json(
            rows.into_iter()
                .map(|row| {
                    let details = row
                        .details
                        .into_iter()
                        .map(|detail| {
                            let product_name = detail.product.name;
                            let detail = detail.inner;
                            OrderDetailProductResponse {
                                unit_price: detail.unit_price,
                                quantity: detail.quantity,
                                discount: detail.discount,
                                order_id: detail.order_id,
                                product_id: detail.product_id,
                                product_name,
                            }
                        })
                        .collect();
                    SingleOrderWithDetailsAndProductsResponse::new(row.inner, details)
                })
                .collect(),
        ));
    }

    let orders: Vec<SelectOrder> = db
        .db
        .select(())
        .from(schema.order)
        .r#where(eq(schema.order.id, id))
        .all()
        .await
        .map_err(db_err)?;
    let detail_rows: Vec<DetailWithProductRow> = db
        .db
        .select((
            schema.detail.unit_price,
            schema.detail.quantity,
            schema.detail.discount,
            schema.detail.order_id,
            schema.detail.product_id,
            super::nullable(schema.product.name),
        ))
        .from(schema.detail)
        .left_join((
            schema.product,
            eq(schema.detail.product_id, schema.product.id),
        ))
        .r#where(eq(schema.detail.order_id, id))
        .all()
        .await
        .map_err(db_err)?;
    let details: Vec<OrderDetailProductResponse> = detail_rows
        .into_iter()
        .map(
            |(unit_price, quantity, discount, order_id, product_id, product_name)| {
                OrderDetailProductResponse {
                    unit_price,
                    quantity,
                    discount,
                    order_id,
                    product_id,
                    product_name: product_name.unwrap_or_default(),
                }
            },
        )
        .collect();
    Ok(Json(
        orders
            .into_iter()
            .map(|order| SingleOrderWithDetailsAndProductsResponse::new(order, details.clone()))
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or(""));
    if state.mode.is_raw() {
        let rows = state
            .raw_query(SQL_SEARCH_CUSTOMER, |s| &s.search_customer, &[&pattern])
            .await?;
        return Ok(Json(rows.iter().map(customer_from_row).collect()));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    let rows: Vec<SelectCustomer> = match state.mode {
        PgMode::DrizzleQuery => db
            .db
            .query(schema.customer)
            .r#where(super::ilike_expr(
                schema.customer.company_name,
                pattern.as_str(),
            ))
            .find_many()
            .await
            .map_err(db_err)?,
        _ => db
            .db
            .select(())
            .from(schema.customer)
            .r#where(super::ilike_expr(
                schema.customer.company_name,
                pattern.as_str(),
            ))
            .all()
            .await
            .map_err(db_err)?,
    };
    Ok(Json(rows.into_iter().map(CustomerResponse::from).collect()))
}

#[debug_handler(state = AppState)]
async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or(""));
    if state.mode.is_raw() {
        let rows = state
            .raw_query(SQL_SEARCH_PRODUCT, |s| &s.search_product, &[&pattern])
            .await?;
        return Ok(Json(rows.iter().map(product_from_row).collect()));
    }

    let schema = state.schema;
    let db = state.acquire().await?;
    let rows: Vec<SelectProduct> = match state.mode {
        PgMode::DrizzleQuery => db
            .db
            .query(schema.product)
            .r#where(super::ilike_expr(schema.product.name, pattern.as_str()))
            .find_many()
            .await
            .map_err(db_err)?,
        _ => db
            .db
            .select(())
            .from(schema.product)
            .r#where(super::ilike_expr(schema.product.name, pattern.as_str()))
            .all()
            .await
            .map_err(db_err)?,
    };
    Ok(Json(rows.into_iter().map(ProductResponse::from).collect()))
}

// ---------------------------------------------------------------------------
// Raw-row decoding — shared by every raw baseline route
// ---------------------------------------------------------------------------

fn customer_from_row(r: &Row) -> CustomerResponse {
    CustomerResponse {
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
    }
}

fn employee_from_row(r: &Row) -> EmployeeResponse {
    EmployeeResponse {
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
    }
}

fn supplier_from_row(r: &Row) -> SupplierResponse {
    SupplierResponse {
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
    }
}

fn product_from_row(r: &Row) -> ProductResponse {
    ProductResponse {
        id: r.get(0),
        name: r.get(1),
        qt_per_unit: r.get(2),
        unit_price: r.get(3),
        units_in_stock: r.get(4),
        units_on_order: r.get(5),
        reorder_level: r.get(6),
        discontinued: r.get(7),
        supplier_id: r.get(8),
    }
}

fn order_from_row(r: &Row) -> SelectOrder {
    SelectOrder {
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
    }
}
