use super::pool::{AsyncResourcePool, PooledResource};
use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use drizzle::core::expr::{alias, coalesce, count, eq, like, sum};
use drizzle::sqlite::prelude::*;
use drizzle_seed::SeedConfig;
use std::path::Path;

// ---------------------------------------------------------------------------
// Northwind schema tables
// ---------------------------------------------------------------------------

#[SQLiteTable(name = "customers")]
struct Customer {
    #[column(primary)]
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

#[SQLiteTable(name = "employees")]
struct Employee {
    #[column(primary)]
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
    #[column(references = Employee::id)]
    recipient_id: Option<i32>,
}

#[SQLiteTable(name = "orders")]
struct Order {
    #[column(primary)]
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
    #[column(references = Customer::id)]
    customer_id: i32,
    #[column(references = Employee::id)]
    employee_id: i32,
}

#[SQLiteTable(name = "suppliers")]
struct Supplier {
    #[column(primary)]
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

#[SQLiteTable(name = "products")]
struct Product {
    #[column(primary)]
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

#[SQLiteTable(name = "order_details")]
struct Detail {
    unit_price: f64,
    quantity: i32,
    discount: f64,
    #[column(references = Order::id)]
    order_id: i32,
    #[column(references = Product::id)]
    product_id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    customer: Customer,
    employee: Employee,
    order: Order,
    supplier: Supplier,
    product: Product,
    detail: Detail,
}

type SqliteDb = drizzle::sqlite::rusqlite::Drizzle<Schema>;

// ---------------------------------------------------------------------------
// Response types (camelCase JSON)
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

/// Shared shape of both `/order-with-details*` responses: the order columns are
/// identical, only the detail element type differs.
fn order_envelope<D>(order: SelectOrder, details: Vec<D>) -> SingleOrderResponse<D> {
    SingleOrderResponse {
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

/// Generic envelope used by `order_envelope`; the two concrete response types
/// stay separate so their JSON field order matches the raw-driver baselines.
struct SingleOrderResponse<D> {
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
    details: Vec<D>,
}

impl From<SingleOrderResponse<OrderDetailResponse>> for SingleOrderWithDetailsResponse {
    fn from(row: SingleOrderResponse<OrderDetailResponse>) -> Self {
        Self {
            id: row.id,
            order_date: row.order_date,
            required_date: row.required_date,
            shipped_date: row.shipped_date,
            ship_via: row.ship_via,
            freight: row.freight,
            ship_name: row.ship_name,
            ship_city: row.ship_city,
            ship_region: row.ship_region,
            ship_postal_code: row.ship_postal_code,
            ship_country: row.ship_country,
            customer_id: row.customer_id,
            employee_id: row.employee_id,
            details: row.details,
        }
    }
}

impl From<SingleOrderResponse<OrderDetailProductResponse>>
    for SingleOrderWithDetailsAndProductsResponse
{
    fn from(row: SingleOrderResponse<OrderDetailProductResponse>) -> Self {
        Self {
            id: row.id,
            order_date: row.order_date,
            required_date: row.required_date,
            shipped_date: row.shipped_date,
            ship_via: row.ship_via,
            freight: row.freight,
            ship_name: row.ship_name,
            ship_city: row.ship_city,
            ship_region: row.ship_region,
            ship_postal_code: row.ship_postal_code,
            ship_country: row.ship_country,
            customer_id: row.customer_id,
            employee_id: row.employee_id,
            details: row.details,
        }
    }
}

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

type EmployeeWithRecipientRow = (
    i32,
    String,
    Option<String>,
    String,
    String,
    i64,
    i64,
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

// SQLite's `sum()` over an integer column yields an integer; the JSON contract
// reports it as a number either way.
type OrderAggregateRow = (i32, Option<i64>, String, String, String, i32, i64, f64);

type DetailWithProductRow = (f64, i32, f64, i32, i32, Option<String>);

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    db: AsyncResourcePool<SqliteDb>,
    mode: SqliteMode,
    // Table ZSTs are `Copy`, so the schema handle is hoisted out of the request
    // path instead of being rebuilt per request.
    schema: Schema,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum SqliteMode {
    /// drizzle-rs typed select builder.
    DrizzleSelect,
    /// drizzle-rs relational query API (`db.query(..).with(..)`).
    DrizzleQuery,
    RusqlitePrepared,
    RusqliteUnprepared,
}

// ---------------------------------------------------------------------------
// Server entry point
// ---------------------------------------------------------------------------

pub async fn serve_select(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, SqliteMode::DrizzleSelect).await
}

pub async fn serve_query(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, SqliteMode::DrizzleQuery).await
}

pub async fn serve_raw_rusqlite_prepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, SqliteMode::RusqlitePrepared).await
}

pub async fn serve_raw_rusqlite_unprepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, SqliteMode::RusqliteUnprepared).await
}

async fn serve_with_mode(seed: u64, mode: SqliteMode) -> Result<ServerHandle, Fail> {
    let temp_dir = tempfile::Builder::new()
        .prefix("drizzle-bench-sqlite-")
        .tempdir()
        .map_err(|err| Fail::new(Code::RunFail, format!("sqlite tempdir failed: {err}")))?;
    let db_path = temp_dir.path().join("bench.sqlite3");
    let pool_size = configured_pool_size(SQLITE_POOL_SIZE);

    let resources = tokio::task::spawn_blocking({
        let db_path = db_path.clone();
        move || -> Result<Vec<SqliteDb>, Fail> {
            let seed_conn = ::rusqlite::Connection::open(&db_path)
                .map_err(|err| Fail::new(Code::RunFail, format!("sqlite open failed: {err}")))?;
            seed_conn
                .execute_batch(
                    "PRAGMA journal_mode = WAL;
                     PRAGMA synchronous = NORMAL;
                     PRAGMA temp_store = MEMORY;",
                )
                .map_err(|err| Fail::new(Code::RunFail, format!("sqlite pragmas failed: {err}")))?;
            let (db, schema) = drizzle::sqlite::rusqlite::Drizzle::new(seed_conn, Schema::new());

            // Create tables via drizzle
            db.create()
                .map_err(|err| Fail::new(Code::RunFail, format!("sqlite create failed: {err}")))?;

            // Create indexes via raw SQL
            db.conn()
                .execute_batch(
                    "CREATE INDEX IF NOT EXISTS recepient_idx ON employees(recipient_id);
                     CREATE INDEX IF NOT EXISTS supplier_idx ON products(supplier_id);
                     CREATE INDEX IF NOT EXISTS order_id_idx ON order_details(order_id);
                     CREATE INDEX IF NOT EXISTS product_id_idx ON order_details(product_id);",
                )
                .map_err(|err| {
                    Fail::new(
                        Code::RunFail,
                        format!("sqlite create indexes failed: {err}"),
                    )
                })?;

            // Seed via drizzle-seed (deterministic from seed value)
            let stmts = SeedConfig::sqlite(&schema)
                .seed(seed)
                .count(&schema.customer, super::SEED_CUSTOMERS)
                .count(&schema.employee, super::SEED_EMPLOYEES)
                .count(&schema.supplier, super::SEED_SUPPLIERS)
                .count(&schema.product, super::SEED_PRODUCTS)
                .count(&schema.order, super::SEED_ORDERS)
                .relation(&schema.order, &schema.detail, 6)
                .generate();
            for stmt in stmts {
                db.execute(stmt).map_err(|err| {
                    Fail::new(Code::RunFail, format!("sqlite seed failed: {err}"))
                })?;
            }

            // Without statistics every target plans the same queries from
            // rowid guesses; the ORM comparison would then be measuring plan
            // luck rather than driver overhead.
            db.conn()
                .execute_batch("ANALYZE;")
                .map_err(|err| Fail::new(Code::RunFail, format!("sqlite analyze failed: {err}")))?;

            drop(db);

            let mut resources = Vec::with_capacity(pool_size);
            for _ in 0..pool_size {
                resources.push(open_sqlite_db(&db_path, mode)?);
            }
            Ok(resources)
        }
    })
    .await
    .map_err(|err| Fail::new(Code::RunFail, format!("sqlite setup panicked: {err}")))??;

    let router = Router::new()
        .route("/stats", get(super::stats))
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
            db: AsyncResourcePool::new(resources),
            mode,
            schema: Schema::new(),
        });
    let mut handle = spawn_server(router).await?;
    handle.temp_dirs.push(temp_dir);
    Ok(handle)
}

fn open_sqlite_db(path: &Path, mode: SqliteMode) -> Result<SqliteDb, Fail> {
    let conn = ::rusqlite::Connection::open(path)
        .map_err(|err| Fail::new(Code::RunFail, format!("sqlite pool open failed: {err}")))?;
    conn.execute_batch(
        "PRAGMA query_only = ON;
         PRAGMA temp_store = MEMORY;",
    )
    .map_err(|err| Fail::new(Code::RunFail, format!("sqlite pool pragmas failed: {err}")))?;
    if mode == SqliteMode::RusqliteUnprepared {
        conn.set_prepared_statement_cache_capacity(0);
    }
    Ok(drizzle::sqlite::rusqlite::Drizzle::new(conn, Schema::new()).0)
}

/// Check a connection out of the pool and run `f` on a blocking thread.
///
/// rusqlite is synchronous: running it inline on a tokio worker stalls the
/// whole reactor, and with `BENCH_WORKERS=1` (the declared fairness setting)
/// that serialises every in-flight request behind one query.
async fn with_db<T, F>(state: &AppState, f: F) -> Result<T, StatusCode>
where
    F: FnOnce(&SqliteDb, &Schema) -> Result<T, StatusCode> + Send + 'static,
    T: Send + 'static,
{
    let db: PooledResource<SqliteDb> = state
        .db
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let schema = state.schema;
    tokio::task::spawn_blocking(move || f(&db, &schema))
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
}

// ---------------------------------------------------------------------------
// Route handlers — one per endpoint, dispatching on the target's mode
// ---------------------------------------------------------------------------

// GET /customers?limit=50&offset=0
#[debug_handler(state = AppState)]
async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let (limit, offset, mode) = (params.limit_or(50), params.offset(), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => Ok(db
            .select(())
            .from(schema.customer)
            .order_by([asc(schema.customer.id)])
            .limit(limit)
            .offset(offset)
            .all::<SelectCustomer, _, _>()
            .map_err(db_err)?
            .into_iter()
            .map(CustomerResponse::from)
            .collect()),
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.customer)
            .order_by(asc(schema.customer.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(CustomerResponse::from)
            .collect()),
        _ => raw_customers(db, limit, offset),
    })
    .await
    .map(Json)
}

// GET /customer-by-id?id=1
#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let (id, mode) = (params.user_id(super::SEED_CUSTOMERS as i32), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => Ok(db
            .select(())
            .from(schema.customer)
            .r#where(eq(schema.customer.id, id))
            .all::<SelectCustomer, _, _>()
            .map_err(db_err)?
            .into_iter()
            .map(CustomerResponse::from)
            .collect()),
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.customer)
            .r#where(eq(schema.customer.id, id))
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(CustomerResponse::from)
            .collect()),
        _ => raw_customer_by_id(db, id),
    })
    .await
    .map(Json)
}

// GET /employees?limit=20&offset=0
#[debug_handler(state = AppState)]
async fn employees(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeResponse>>, StatusCode> {
    let (limit, offset, mode) = (params.limit_or(50), params.offset(), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => Ok(db
            .select(())
            .from(schema.employee)
            .order_by([asc(schema.employee.id)])
            .limit(limit)
            .offset(offset)
            .all::<SelectEmployee, _, _>()
            .map_err(db_err)?
            .into_iter()
            .map(EmployeeResponse::from)
            .collect()),
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.employee)
            .order_by(asc(schema.employee.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(EmployeeResponse::from)
            .collect()),
        _ => raw_employees(db, limit, offset),
    })
    .await
    .map(Json)
}

// GET /suppliers?limit=50&offset=0
#[debug_handler(state = AppState)]
async fn suppliers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let (limit, offset, mode) = (params.limit_or(50), params.offset(), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => Ok(db
            .select(())
            .from(schema.supplier)
            .order_by([asc(schema.supplier.id)])
            .limit(limit)
            .offset(offset)
            .all::<SelectSupplier, _, _>()
            .map_err(db_err)?
            .into_iter()
            .map(SupplierResponse::from)
            .collect()),
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.supplier)
            .order_by(asc(schema.supplier.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(SupplierResponse::from)
            .collect()),
        _ => raw_suppliers(db, limit, offset),
    })
    .await
    .map(Json)
}

// GET /supplier-by-id?id=1
#[debug_handler(state = AppState)]
async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let (id, mode) = (params.user_id(super::SEED_SUPPLIERS as i32), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => Ok(db
            .select(())
            .from(schema.supplier)
            .r#where(eq(schema.supplier.id, id))
            .all::<SelectSupplier, _, _>()
            .map_err(db_err)?
            .into_iter()
            .map(SupplierResponse::from)
            .collect()),
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.supplier)
            .r#where(eq(schema.supplier.id, id))
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(SupplierResponse::from)
            .collect()),
        _ => raw_supplier_by_id(db, id),
    })
    .await
    .map(Json)
}

// GET /products?limit=50&offset=0
#[debug_handler(state = AppState)]
async fn products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let (limit, offset, mode) = (params.limit_or(50), params.offset(), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => Ok(db
            .select(())
            .from(schema.product)
            .order_by([asc(schema.product.id)])
            .limit(limit)
            .offset(offset)
            .all::<SelectProduct, _, _>()
            .map_err(db_err)?
            .into_iter()
            .map(ProductResponse::from)
            .collect()),
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.product)
            .order_by(asc(schema.product.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(ProductResponse::from)
            .collect()),
        _ => raw_products(db, limit, offset),
    })
    .await
    .map(Json)
}

// GET /employee-with-recipient?id=1 — employees LEFT JOIN themselves.
#[debug_handler(state = AppState)]
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeWithRecipientResponse>>, StatusCode> {
    let (id, mode) = (params.user_id(super::SEED_EMPLOYEES as i32), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => {
            let recipient = Employee::alias::<super::RecipientAlias>();
            let rows: Vec<EmployeeWithRecipientRow> = db
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
                .map_err(db_err)?;
            Ok(rows
                .into_iter()
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
                .collect())
        }
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.employee)
            .with(schema.employee.recipient())
            .r#where(eq(schema.employee.id, id))
            .find_many()
            .map_err(db_err)?
            .into_iter()
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
            .collect()),
        _ => raw_employee_with_recipient(db, id),
    })
    .await
    .map(Json)
}

// GET /product-with-supplier?id=1 — products INNER JOIN suppliers.
#[debug_handler(state = AppState)]
async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductWithSupplierResponse>>, StatusCode> {
    let (id, mode) = (params.user_id(super::SEED_PRODUCTS as i32), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => {
            let rows: Vec<ProductWithSupplierRow> = db
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
                .map_err(db_err)?;
            Ok(rows
                .into_iter()
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
                .collect())
        }
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.product)
            .with(schema.product.supplier())
            .r#where(eq(schema.product.id, id))
            .find_many()
            .map_err(db_err)?
            .into_iter()
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
            .collect()),
        _ => raw_product_with_supplier(db, id),
    })
    .await
    .map(Json)
}

// GET /orders-with-details?limit=50&offset=0 — per-order detail aggregates.
#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<OrderWithDetailsResponse>>, StatusCode> {
    let (limit, offset, mode) = (params.limit_or(50), params.offset(), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => {
            let rows: Vec<OrderAggregateRow> = db
                .select((
                    schema.order.id,
                    schema.order.shipped_date,
                    schema.order.ship_name,
                    schema.order.ship_city,
                    schema.order.ship_country,
                    alias(count(schema.detail.product_id), "products_count"),
                    alias(coalesce(sum(schema.detail.quantity), 0), "quantity_sum"),
                    alias(
                        coalesce(sum(schema.detail.quantity * schema.detail.unit_price), 0),
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
                .map_err(db_err)?;
            Ok(rows
                .into_iter()
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
                .collect())
        }
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.order)
            .with(schema.order.details())
            .order_by(asc(schema.order.id))
            .limit(limit)
            .offset(offset)
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(|row| {
                let details = row.details;
                let order = row.inner;
                OrderWithDetailsResponse {
                    id: order.id,
                    shipped_date: order.shipped_date,
                    ship_name: order.ship_name,
                    ship_city: order.ship_city,
                    ship_country: order.ship_country,
                    products_count: details.len() as i32,
                    quantity_sum: details.iter().map(|d| f64::from(d.quantity)).sum(),
                    total_price: details
                        .iter()
                        .map(|d| f64::from(d.quantity) * d.unit_price)
                        .sum(),
                }
            })
            .collect()),
        _ => raw_orders_with_details(db, limit, offset),
    })
    .await
    .map(Json)
}

// GET /order-with-details?id=1
#[debug_handler(state = AppState)]
async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsResponse>>, StatusCode> {
    let (id, mode) = (params.user_id(super::SEED_ORDERS as i32), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => {
            let orders: Vec<SelectOrder> = db
                .select(())
                .from(schema.order)
                .r#where(eq(schema.order.id, id))
                .all()
                .map_err(db_err)?;
            let details: Vec<OrderDetailResponse> = db
                .select(())
                .from(schema.detail)
                .r#where(eq(schema.detail.order_id, id))
                .all::<SelectDetail, _, _>()
                .map_err(db_err)?
                .into_iter()
                .map(OrderDetailResponse::from)
                .collect();
            Ok(orders
                .into_iter()
                .map(|order| order_envelope(order, details.clone()).into())
                .collect())
        }
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.order)
            .with(schema.order.details())
            .r#where(eq(schema.order.id, id))
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(|row| {
                let details = row.details.into_iter().map(OrderDetailResponse::from);
                order_envelope(row.inner, details.collect()).into()
            })
            .collect()),
        _ => raw_order_with_details(db, id),
    })
    .await
    .map(Json)
}

// GET /order-with-details-and-products?id=1
#[debug_handler(state = AppState)]
async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsAndProductsResponse>>, StatusCode> {
    let (id, mode) = (params.user_id(super::SEED_ORDERS as i32), state.mode);
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => {
            let orders: Vec<SelectOrder> = db
                .select(())
                .from(schema.order)
                .r#where(eq(schema.order.id, id))
                .all()
                .map_err(db_err)?;
            let rows: Vec<DetailWithProductRow> = db
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
                .map_err(db_err)?;
            let details: Vec<OrderDetailProductResponse> = rows
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
            Ok(orders
                .into_iter()
                .map(|order| order_envelope(order, details.clone()).into())
                .collect())
        }
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.order)
            .with(schema.order.details().with(schema.detail.product()))
            .r#where(eq(schema.order.id, id))
            .find_many()
            .map_err(db_err)?
            .into_iter()
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
                order_envelope(row.inner, details).into()
            })
            .collect()),
        _ => raw_order_with_details_and_products(db, id),
    })
    .await
    .map(Json)
}

// GET /search-customer?term=er
#[debug_handler(state = AppState)]
async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or(""));
    let mode = state.mode;
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => Ok(db
            .select(())
            .from(schema.customer)
            .r#where(like(schema.customer.company_name, pattern.as_str()))
            .all::<SelectCustomer, _, _>()
            .map_err(db_err)?
            .into_iter()
            .map(CustomerResponse::from)
            .collect()),
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.customer)
            .r#where(like(schema.customer.company_name, pattern.as_str()))
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(CustomerResponse::from)
            .collect()),
        _ => raw_search_customer(db, &pattern),
    })
    .await
    .map(Json)
}

// GET /search-product?term=er
#[debug_handler(state = AppState)]
async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or(""));
    let mode = state.mode;
    with_db(&state, move |db, schema| match mode {
        SqliteMode::DrizzleSelect => Ok(db
            .select(())
            .from(schema.product)
            .r#where(like(schema.product.name, pattern.as_str()))
            .all::<SelectProduct, _, _>()
            .map_err(db_err)?
            .into_iter()
            .map(ProductResponse::from)
            .collect()),
        SqliteMode::DrizzleQuery => Ok(db
            .query(schema.product)
            .r#where(like(schema.product.name, pattern.as_str()))
            .find_many()
            .map_err(db_err)?
            .into_iter()
            .map(ProductResponse::from)
            .collect()),
        _ => raw_search_product(db, &pattern),
    })
    .await
    .map(Json)
}

// ---------------------------------------------------------------------------
// Raw rusqlite baselines
// ---------------------------------------------------------------------------

/// Run one cached-prepared query and collect its rows.
///
/// `prepare_cached` is the whole point of the prepared baseline; the unprepared
/// baseline gets the same call path with the connection's statement cache
/// capacity set to zero.
fn raw_query<T>(
    db: &SqliteDb,
    sql: &str,
    params: &[&dyn ::rusqlite::ToSql],
    map: impl FnMut(&::rusqlite::Row<'_>) -> ::rusqlite::Result<T>,
) -> Result<Vec<T>, StatusCode> {
    let conn = db.conn();
    let mut stmt = conn.prepare_cached(sql).map_err(db_err)?;
    let rows = stmt
        .query_map(params, map)
        .map_err(db_err)?
        .collect::<::rusqlite::Result<Vec<T>>>()
        .map_err(db_err)?;
    Ok(rows)
}

fn raw_customers(
    db: &SqliteDb,
    limit: usize,
    offset: usize,
) -> Result<Vec<CustomerResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT id, company_name, contact_name, contact_title, address, city,
                postal_code, region, country, phone, fax
         FROM customers
         ORDER BY id
         LIMIT ?1 OFFSET ?2",
        &[&(limit as i64), &(offset as i64)],
        customer_response_from_row,
    )
}

fn raw_customer_by_id(db: &SqliteDb, id: i32) -> Result<Vec<CustomerResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT id, company_name, contact_name, contact_title, address, city,
                postal_code, region, country, phone, fax
         FROM customers
         WHERE id = ?1",
        &[&id],
        customer_response_from_row,
    )
}

fn raw_employees(
    db: &SqliteDb,
    limit: usize,
    offset: usize,
) -> Result<Vec<EmployeeResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT id, last_name, first_name, title, title_of_courtesy, birth_date,
                hire_date, address, city, postal_code, country, home_phone,
                extension, notes, recipient_id
         FROM employees
         ORDER BY id
         LIMIT ?1 OFFSET ?2",
        &[&(limit as i64), &(offset as i64)],
        |row| {
            Ok(EmployeeResponse {
                id: row.get(0)?,
                last_name: row.get(1)?,
                first_name: row.get(2)?,
                title: row.get(3)?,
                title_of_courtesy: row.get(4)?,
                birth_date: row.get(5)?,
                hire_date: row.get(6)?,
                address: row.get(7)?,
                city: row.get(8)?,
                postal_code: row.get(9)?,
                country: row.get(10)?,
                home_phone: row.get(11)?,
                extension: row.get(12)?,
                notes: row.get(13)?,
                recipient_id: row.get(14)?,
            })
        },
    )
}

fn raw_suppliers(
    db: &SqliteDb,
    limit: usize,
    offset: usize,
) -> Result<Vec<SupplierResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT id, company_name, contact_name, contact_title, address, city,
                region, postal_code, country, phone
         FROM suppliers
         ORDER BY id
         LIMIT ?1 OFFSET ?2",
        &[&(limit as i64), &(offset as i64)],
        supplier_response_from_row,
    )
}

fn raw_supplier_by_id(db: &SqliteDb, id: i32) -> Result<Vec<SupplierResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT id, company_name, contact_name, contact_title, address, city,
                region, postal_code, country, phone
         FROM suppliers
         WHERE id = ?1",
        &[&id],
        supplier_response_from_row,
    )
}

fn raw_products(
    db: &SqliteDb,
    limit: usize,
    offset: usize,
) -> Result<Vec<ProductResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order,
                reorder_level, discontinued, supplier_id
         FROM products
         ORDER BY id
         LIMIT ?1 OFFSET ?2",
        &[&(limit as i64), &(offset as i64)],
        product_response_from_row,
    )
}

fn raw_employee_with_recipient(
    db: &SqliteDb,
    id: i32,
) -> Result<Vec<EmployeeWithRecipientResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT e.id, e.last_name, e.first_name, e.title, e.title_of_courtesy,
                e.birth_date, e.hire_date, e.address, e.city, e.postal_code,
                e.country, e.home_phone, e.extension, e.notes, e.recipient_id,
                r.last_name, r.first_name
         FROM employees e
         LEFT JOIN employees r ON e.recipient_id = r.id
         WHERE e.id = ?1",
        &[&id],
        |row| {
            Ok(EmployeeWithRecipientResponse {
                id: row.get(0)?,
                last_name: row.get(1)?,
                first_name: row.get(2)?,
                title: row.get(3)?,
                title_of_courtesy: row.get(4)?,
                birth_date: row.get(5)?,
                hire_date: row.get(6)?,
                address: row.get(7)?,
                city: row.get(8)?,
                postal_code: row.get(9)?,
                country: row.get(10)?,
                home_phone: row.get(11)?,
                extension: row.get(12)?,
                notes: row.get(13)?,
                recipient_id: row.get(14)?,
                recipient_last_name: row.get(15)?,
                recipient_first_name: row.get(16)?,
            })
        },
    )
}

fn raw_product_with_supplier(
    db: &SqliteDb,
    id: i32,
) -> Result<Vec<ProductWithSupplierResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT p.id, p.name, p.qt_per_unit, p.unit_price, p.units_in_stock,
                p.units_on_order, p.reorder_level, p.discontinued, p.supplier_id,
                s.id, s.company_name, s.contact_name, s.contact_title, s.address,
                s.city, s.region, s.postal_code, s.country, s.phone
         FROM products p
         INNER JOIN suppliers s ON p.supplier_id = s.id
         WHERE p.id = ?1",
        &[&id],
        |row| {
            Ok(ProductWithSupplierResponse {
                id: row.get(0)?,
                name: row.get(1)?,
                qt_per_unit: row.get(2)?,
                unit_price: row.get(3)?,
                units_in_stock: row.get(4)?,
                units_on_order: row.get(5)?,
                reorder_level: row.get(6)?,
                discontinued: row.get(7)?,
                supplier_id: row.get(8)?,
                supplier: SupplierResponse {
                    id: row.get(9)?,
                    company_name: row.get(10)?,
                    contact_name: row.get(11)?,
                    contact_title: row.get(12)?,
                    address: row.get(13)?,
                    city: row.get(14)?,
                    region: row.get(15)?,
                    postal_code: row.get(16)?,
                    country: row.get(17)?,
                    phone: row.get(18)?,
                },
            })
        },
    )
}

fn raw_orders_with_details(
    db: &SqliteDb,
    limit: usize,
    offset: usize,
) -> Result<Vec<OrderWithDetailsResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT o.id, o.shipped_date, o.ship_name, o.ship_city, o.ship_country,
                count(d.product_id) as products_count,
                COALESCE(sum(d.quantity), 0) as quantity_sum,
                COALESCE(sum(d.quantity * d.unit_price), 0) as total_price
         FROM orders o
         LEFT JOIN order_details d ON o.id = d.order_id
         GROUP BY o.id
         ORDER BY o.id ASC
         LIMIT ?1 OFFSET ?2",
        &[&(limit as i64), &(offset as i64)],
        |row| {
            Ok(OrderWithDetailsResponse {
                id: row.get(0)?,
                shipped_date: row.get(1)?,
                ship_name: row.get(2)?,
                ship_city: row.get(3)?,
                ship_country: row.get(4)?,
                products_count: row.get(5)?,
                quantity_sum: row.get(6)?,
                total_price: row.get(7)?,
            })
        },
    )
}

fn raw_order_with_details(
    db: &SqliteDb,
    id: i32,
) -> Result<Vec<SingleOrderWithDetailsResponse>, StatusCode> {
    let orders = raw_query(
        db,
        RAW_ORDER_BY_ID_SQL,
        &[&id],
        raw_order_response::<OrderDetailResponse>,
    )?;
    let details = raw_query(
        db,
        "SELECT unit_price, quantity, discount, order_id, product_id
         FROM order_details WHERE order_id = ?1",
        &[&id],
        |row| {
            Ok(OrderDetailResponse {
                unit_price: row.get(0)?,
                quantity: row.get(1)?,
                discount: row.get(2)?,
                order_id: row.get(3)?,
                product_id: row.get(4)?,
            })
        },
    )?;
    Ok(orders
        .into_iter()
        .map(|mut order| {
            order.details = details.clone();
            order.into()
        })
        .collect())
}

fn raw_order_with_details_and_products(
    db: &SqliteDb,
    id: i32,
) -> Result<Vec<SingleOrderWithDetailsAndProductsResponse>, StatusCode> {
    let orders = raw_query(
        db,
        RAW_ORDER_BY_ID_SQL,
        &[&id],
        raw_order_response::<OrderDetailProductResponse>,
    )?;
    let details = raw_query(
        db,
        "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name
         FROM order_details d
         LEFT JOIN products p ON d.product_id = p.id
         WHERE d.order_id = ?1",
        &[&id],
        |row| {
            Ok(OrderDetailProductResponse {
                unit_price: row.get(0)?,
                quantity: row.get(1)?,
                discount: row.get(2)?,
                order_id: row.get(3)?,
                product_id: row.get(4)?,
                product_name: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
            })
        },
    )?;
    Ok(orders
        .into_iter()
        .map(|mut order| {
            order.details = details.clone();
            order.into()
        })
        .collect())
}

fn raw_search_customer(db: &SqliteDb, pattern: &str) -> Result<Vec<CustomerResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT id, company_name, contact_name, contact_title, address, city,
                postal_code, region, country, phone, fax
         FROM customers WHERE company_name LIKE ?1",
        &[&pattern],
        customer_response_from_row,
    )
}

fn raw_search_product(db: &SqliteDb, pattern: &str) -> Result<Vec<ProductResponse>, StatusCode> {
    raw_query(
        db,
        "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order,
                reorder_level, discontinued, supplier_id
         FROM products WHERE name LIKE ?1",
        &[&pattern],
        product_response_from_row,
    )
}

const RAW_ORDER_BY_ID_SQL: &str =
    "SELECT id, order_date, required_date, shipped_date, ship_via, freight,
            ship_name, ship_city, ship_region, ship_postal_code, ship_country,
            customer_id, employee_id
     FROM orders WHERE id = ?1";

fn raw_order_response<D>(row: &::rusqlite::Row<'_>) -> ::rusqlite::Result<SingleOrderResponse<D>> {
    Ok(SingleOrderResponse {
        id: row.get(0)?,
        order_date: row.get(1)?,
        required_date: row.get(2)?,
        shipped_date: row.get(3)?,
        ship_via: row.get(4)?,
        freight: row.get(5)?,
        ship_name: row.get(6)?,
        ship_city: row.get(7)?,
        ship_region: row.get(8)?,
        ship_postal_code: row.get(9)?,
        ship_country: row.get(10)?,
        customer_id: row.get(11)?,
        employee_id: row.get(12)?,
        details: Vec::new(),
    })
}

fn customer_response_from_row(row: &::rusqlite::Row<'_>) -> ::rusqlite::Result<CustomerResponse> {
    Ok(CustomerResponse {
        id: row.get(0)?,
        company_name: row.get(1)?,
        contact_name: row.get(2)?,
        contact_title: row.get(3)?,
        address: row.get(4)?,
        city: row.get(5)?,
        postal_code: row.get(6)?,
        region: row.get(7)?,
        country: row.get(8)?,
        phone: row.get(9)?,
        fax: row.get(10)?,
    })
}

fn supplier_response_from_row(row: &::rusqlite::Row<'_>) -> ::rusqlite::Result<SupplierResponse> {
    Ok(SupplierResponse {
        id: row.get(0)?,
        company_name: row.get(1)?,
        contact_name: row.get(2)?,
        contact_title: row.get(3)?,
        address: row.get(4)?,
        city: row.get(5)?,
        region: row.get(6)?,
        postal_code: row.get(7)?,
        country: row.get(8)?,
        phone: row.get(9)?,
    })
}

fn product_response_from_row(row: &::rusqlite::Row<'_>) -> ::rusqlite::Result<ProductResponse> {
    Ok(ProductResponse {
        id: row.get(0)?,
        name: row.get(1)?,
        qt_per_unit: row.get(2)?,
        unit_price: row.get(3)?,
        units_in_stock: row.get(4)?,
        units_on_order: row.get(5)?,
        reorder_level: row.get(6)?,
        discontinued: row.get(7)?,
        supplier_id: row.get(8)?,
    })
}
