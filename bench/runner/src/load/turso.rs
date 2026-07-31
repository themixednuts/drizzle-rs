use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use drizzle::core::expr::{and, eq, gte, like, lte};
use drizzle::sqlite::prelude::*;
use drizzle_seed::SeedConfig;
use std::collections::BTreeMap;

const TURSO_POOL_SIZE: usize = 4;

/// Journal mode every turso target runs under.
///
/// turso's MVCC engine is what the project is actually shipping, so measuring
/// the legacy journal would benchmark a mode nobody runs. The mode is a
/// persistent property of the file, so setting it once on the setup connection
/// covers the pooled connections opened afterwards.
const TURSO_JOURNAL_MODE: &str = "mvcc";

async fn enable_mvcc(conn: &::turso::Connection) -> Result<(), Fail> {
    conn.pragma_update("journal_mode", format!("'{TURSO_JOURNAL_MODE}'"))
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("turso journal_mode failed: {err}")))?;

    // A pragma that does not apply is reported as success with the old value
    // still in effect, so the read-back is the only real confirmation.
    let mut observed = None;
    conn.pragma_query("journal_mode", |row| {
        if observed.is_none() {
            observed = row.get::<String>(0).ok();
        }
        Ok(())
    })
    .await
    .map_err(|err| {
        Fail::new(
            Code::RunFail,
            format!("turso journal_mode read-back failed: {err}"),
        )
    })?;

    match observed.as_deref() {
        Some(mode) if mode.eq_ignore_ascii_case(TURSO_JOURNAL_MODE) => Ok(()),
        other => Err(Fail::new(
            Code::RunFail,
            format!(
                "turso journal_mode is {other:?}, expected {TURSO_JOURNAL_MODE:?}; \
                 the target would benchmark a different storage engine than declared"
            ),
        )),
    }
}

/// Collect all rows from a turso `Rows` into a `Vec<turso::Row>`.
async fn collect_rows(mut rows: ::turso::Rows) -> Result<Vec<::turso::Row>, StatusCode> {
    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        out.push(row);
    }
    Ok(out)
}

async fn query_cached_rows(
    conn: &::turso::Connection,
    sql: &str,
    params: impl ::turso::IntoParams,
) -> Result<Vec<::turso::Row>, StatusCode> {
    let mut stmt = conn
        .prepare_cached(sql)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = stmt
        .query(params)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    collect_rows(rows).await
}

async fn query_rows(
    conn: &::turso::Connection,
    sql: &str,
    params: impl ::turso::IntoParams,
    mode: TursoMode,
) -> Result<Vec<::turso::Row>, StatusCode> {
    if mode.uses_prepared_statements() {
        query_cached_rows(conn, sql, params).await
    } else {
        let rows = conn
            .query(sql, params)
            .await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        collect_rows(rows).await
    }
}

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

// Response types (same as sqlite module)
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

type CustomerRow = (
    i32,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    String,
    Option<String>,
);

type EmployeeRow = (
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
);

type SupplierRow = (
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

type ProductRow = (i32, String, String, f64, i32, i32, i32, i32, i32);

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

type OrderRow = (
    i32,
    i64,
    i64,
    Option<i64>,
    i32,
    f64,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    i32,
    i32,
);

type OrderHeaderRow = (i32, Option<i64>, String, String, String);

type DetailAggregateRow = (i32, i32, i32, f64);

type DetailRow = (f64, i32, f64, i32, i32);

type DetailWithProductRow = (f64, i32, f64, i32, i32, Option<String>);

type TursoDb = drizzle::sqlite::turso::Drizzle<Schema>;

type TursoPool = super::pool::AsyncResourcePool<TursoDb>;

#[derive(Clone)]
struct AppState {
    db: TursoPool,
    mode: TursoMode,
    // Table ZSTs are `Copy`; the schema handle is built once at startup.
    schema: Schema,
}

async fn acquire_db(state: &AppState) -> Result<super::pool::PooledResource<TursoDb>, StatusCode> {
    state
        .db
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

pub async fn serve(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, TursoMode::Drizzle).await
}

pub async fn serve_raw_prepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, TursoMode::RawPrepared).await
}

pub async fn serve_raw_unprepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, TursoMode::RawUnprepared).await
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum TursoMode {
    Drizzle,
    RawPrepared,
    RawUnprepared,
}

impl TursoMode {
    fn uses_prepared_statements(self) -> bool {
        matches!(self, Self::Drizzle | Self::RawPrepared)
    }

    fn is_raw(self) -> bool {
        matches!(self, Self::RawPrepared | Self::RawUnprepared)
    }
}

async fn serve_with_mode(seed: u64, mode: TursoMode) -> Result<ServerHandle, Fail> {
    // File-backed, like the rusqlite family: an in-memory database would let
    // the embedded turso targets skip the storage costs their SQLite
    // counterparts pay, and the two families are ranked against each other.
    let temp_dir = tempfile::Builder::new()
        .prefix("drizzle-bench-turso-")
        .tempdir()
        .map_err(|err| Fail::new(Code::RunFail, format!("turso tempdir failed: {err}")))?;
    let db_path = temp_dir.path().join("bench.sqlite3");
    let builder = ::turso::Builder::new_local(
        db_path
            .to_str()
            .ok_or_else(|| Fail::new(Code::RunFail, "turso temp path is not utf-8"))?,
    )
    .build()
    .await
    .map_err(|err| Fail::new(Code::RunFail, format!("turso build failed: {err}")))?;
    let conn = builder
        .connect()
        .map_err(|err| Fail::new(Code::RunFail, format!("turso connect failed: {err}")))?;
    enable_mvcc(&conn).await?;
    let (db, schema) = drizzle::sqlite::turso::Drizzle::new(conn, Schema::new());
    db.create()
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("turso create failed: {err}")))?;

    // Create indexes
    db.conn()
        .execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_emp_recipient ON employees(recipient_id);
             CREATE INDEX IF NOT EXISTS idx_prod_supplier ON products(supplier_id);
             CREATE INDEX IF NOT EXISTS idx_det_order ON order_details(order_id);
             CREATE INDEX IF NOT EXISTS idx_det_product ON order_details(product_id);",
        )
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("turso create indexes failed: {err}")))?;

    // Seed via drizzle-seed — use smaller batch size for turso's parameter limits
    let stmts = SeedConfig::sqlite(&schema)
        .seed(seed)
        .max_params(500)
        .count(&schema.customer, super::SEED_CUSTOMERS)
        .count(&schema.employee, super::SEED_EMPLOYEES)
        .count(&schema.supplier, super::SEED_SUPPLIERS)
        .count(&schema.product, super::SEED_PRODUCTS)
        .count(&schema.order, super::SEED_ORDERS)
        .relation(&schema.order, &schema.detail, 6)
        .generate();
    for stmt in stmts {
        db.execute(stmt)
            .await
            .map_err(|err| Fail::new(Code::RunFail, format!("turso seed failed: {err}")))?;
    }

    // Statistics, so the planner picks the indexes just created rather than
    // guessing from rowid counts.
    db.conn()
        .execute_batch("ANALYZE;")
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("turso analyze failed: {err}")))?;

    let pool_size = super::configured_pool_size(TURSO_POOL_SIZE);
    let mut connections = Vec::with_capacity(pool_size);
    connections.push(db);
    for _ in 1..pool_size {
        let conn = builder
            .connect()
            .map_err(|err| Fail::new(Code::RunFail, format!("turso connect failed: {err}")))?;
        let (db, _) = drizzle::sqlite::turso::Drizzle::new(conn, Schema::new());
        connections.push(db);
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
            db: super::pool::AsyncResourcePool::new(connections),
            mode,
            schema: Schema::new(),
        });
    let mut handle = spawn_server(router).await?;
    handle.temp_dirs.push(temp_dir);
    Ok(handle)
}

async fn raw_customers(
    state: &AppState,
    params: QueryParams,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let lim = params.limit_or(50) as i64;
    let off = params.offset() as i64;
    let db = acquire_db(state).await?;
    let rows = query_rows(
        db.conn(),
        "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers ORDER BY id LIMIT ?1 OFFSET ?2",
        [::turso::Value::from(lim), ::turso::Value::from(off)],
        state.mode,
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| CustomerResponse {
                id: row.get::<i32>(0).unwrap_or_default(),
                company_name: row.get::<String>(1).unwrap_or_default(),
                contact_name: row.get::<String>(2).unwrap_or_default(),
                contact_title: row.get::<String>(3).unwrap_or_default(),
                address: row.get::<String>(4).unwrap_or_default(),
                city: row.get::<String>(5).unwrap_or_default(),
                postal_code: row.get::<String>(6).ok(),
                region: row.get::<String>(7).ok(),
                country: row.get::<String>(8).unwrap_or_default(),
                phone: row.get::<String>(9).unwrap_or_default(),
                fax: row.get::<String>(10).ok(),
            })
            .collect(),
    ))
}

async fn raw_customer_by_id(
    state: &AppState,
    params: QueryParams,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let target_id = params.user_id(super::SEED_CUSTOMERS as i32);
    let db = acquire_db(state).await?;
    let rows = query_rows(
        db.conn(),
        "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE id = ?1",
        [::turso::Value::from(target_id)],
        state.mode,
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| CustomerResponse {
                id: row.get::<i32>(0).unwrap_or_default(),
                company_name: row.get::<String>(1).unwrap_or_default(),
                contact_name: row.get::<String>(2).unwrap_or_default(),
                contact_title: row.get::<String>(3).unwrap_or_default(),
                address: row.get::<String>(4).unwrap_or_default(),
                city: row.get::<String>(5).unwrap_or_default(),
                postal_code: row.get::<String>(6).ok(),
                region: row.get::<String>(7).ok(),
                country: row.get::<String>(8).unwrap_or_default(),
                phone: row.get::<String>(9).unwrap_or_default(),
                fax: row.get::<String>(10).ok(),
            })
            .collect(),
    ))
}

async fn raw_employees(
    state: &AppState,
    params: QueryParams,
) -> Result<Json<Vec<EmployeeResponse>>, StatusCode> {
    let lim = params.limit_or(50) as i64;
    let off = params.offset() as i64;
    let db = acquire_db(state).await?;
    let rows = query_rows(
        db.conn(),
        "SELECT id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id FROM employees ORDER BY id LIMIT ?1 OFFSET ?2",
        [::turso::Value::from(lim), ::turso::Value::from(off)],
        state.mode,
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| EmployeeResponse {
                id: row.get::<i32>(0).unwrap_or_default(),
                last_name: row.get::<String>(1).unwrap_or_default(),
                first_name: row.get::<String>(2).ok(),
                title: row.get::<String>(3).unwrap_or_default(),
                title_of_courtesy: row.get::<String>(4).unwrap_or_default(),
                birth_date: row.get::<i64>(5).unwrap_or_default(),
                hire_date: row.get::<i64>(6).unwrap_or_default(),
                address: row.get::<String>(7).unwrap_or_default(),
                city: row.get::<String>(8).unwrap_or_default(),
                postal_code: row.get::<String>(9).unwrap_or_default(),
                country: row.get::<String>(10).unwrap_or_default(),
                home_phone: row.get::<String>(11).unwrap_or_default(),
                extension: row.get::<i32>(12).unwrap_or_default(),
                notes: row.get::<String>(13).unwrap_or_default(),
                recipient_id: row.get::<i32>(14).ok(),
            })
            .collect(),
    ))
}

async fn raw_suppliers(
    state: &AppState,
    params: QueryParams,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let lim = params.limit_or(50) as i64;
    let off = params.offset() as i64;
    let db = acquire_db(state).await?;
    let rows = query_rows(
        db.conn(),
        "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers ORDER BY id LIMIT ?1 OFFSET ?2",
        [::turso::Value::from(lim), ::turso::Value::from(off)],
        state.mode,
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| SupplierResponse {
                id: row.get::<i32>(0).unwrap_or_default(),
                company_name: row.get::<String>(1).unwrap_or_default(),
                contact_name: row.get::<String>(2).unwrap_or_default(),
                contact_title: row.get::<String>(3).unwrap_or_default(),
                address: row.get::<String>(4).unwrap_or_default(),
                city: row.get::<String>(5).unwrap_or_default(),
                region: row.get::<String>(6).ok(),
                postal_code: row.get::<String>(7).unwrap_or_default(),
                country: row.get::<String>(8).unwrap_or_default(),
                phone: row.get::<String>(9).unwrap_or_default(),
            })
            .collect(),
    ))
}

async fn raw_supplier_by_id(
    state: &AppState,
    params: QueryParams,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let target_id = params.user_id(super::SEED_SUPPLIERS as i32);
    let db = acquire_db(state).await?;
    let rows = query_rows(
        db.conn(),
        "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers WHERE id = ?1",
        [::turso::Value::from(target_id)],
        state.mode,
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| SupplierResponse {
                id: row.get::<i32>(0).unwrap_or_default(),
                company_name: row.get::<String>(1).unwrap_or_default(),
                contact_name: row.get::<String>(2).unwrap_or_default(),
                contact_title: row.get::<String>(3).unwrap_or_default(),
                address: row.get::<String>(4).unwrap_or_default(),
                city: row.get::<String>(5).unwrap_or_default(),
                region: row.get::<String>(6).ok(),
                postal_code: row.get::<String>(7).unwrap_or_default(),
                country: row.get::<String>(8).unwrap_or_default(),
                phone: row.get::<String>(9).unwrap_or_default(),
            })
            .collect(),
    ))
}

async fn raw_products(
    state: &AppState,
    params: QueryParams,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let lim = params.limit_or(50) as i64;
    let off = params.offset() as i64;
    let db = acquire_db(state).await?;
    let rows = query_rows(
        db.conn(),
        "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products ORDER BY id LIMIT ?1 OFFSET ?2",
        [::turso::Value::from(lim), ::turso::Value::from(off)],
        state.mode,
    )
    .await?;
    Ok(Json(
        rows.iter()
            .map(|row| ProductResponse {
                id: row.get::<i32>(0).unwrap_or_default(),
                name: row.get::<String>(1).unwrap_or_default(),
                qt_per_unit: row.get::<String>(2).unwrap_or_default(),
                unit_price: row.get::<f64>(3).unwrap_or_default(),
                units_in_stock: row.get::<i32>(4).unwrap_or_default(),
                units_on_order: row.get::<i32>(5).unwrap_or_default(),
                reorder_level: row.get::<i32>(6).unwrap_or_default(),
                discontinued: row.get::<i32>(7).unwrap_or_default(),
                supplier_id: row.get::<i32>(8).unwrap_or_default(),
            })
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn customers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    if state.mode.is_raw() {
        return raw_customers(&state, params).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let rows: Vec<CustomerRow> = db
        .select((
            schema.customer.id,
            schema.customer.company_name,
            schema.customer.contact_name,
            schema.customer.contact_title,
            schema.customer.address,
            schema.customer.city,
            schema.customer.postal_code,
            schema.customer.region,
            schema.customer.country,
            schema.customer.phone,
            schema.customer.fax,
        ))
        .from(schema.customer)
        .order_by([asc(schema.customer.id)])
        .limit(params.limit_or(50))
        .offset(params.offset())
        .all()
        .await
        .map_err(db_err)?;
    let resp: Vec<CustomerResponse> = rows
        .into_iter()
        .map(
            |(
                id,
                company_name,
                contact_name,
                contact_title,
                address,
                city,
                postal_code,
                region,
                country,
                phone,
                fax,
            )| CustomerResponse {
                id,
                company_name,
                contact_name,
                contact_title,
                address,
                city,
                postal_code,
                region,
                country,
                phone,
                fax,
            },
        )
        .collect();
    Ok(Json(resp))
}

#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    if state.mode.is_raw() {
        return raw_customer_by_id(&state, params).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let target_id = params.user_id(super::SEED_CUSTOMERS as i32);
    let rows: Vec<CustomerRow> = db
        .select((
            schema.customer.id,
            schema.customer.company_name,
            schema.customer.contact_name,
            schema.customer.contact_title,
            schema.customer.address,
            schema.customer.city,
            schema.customer.postal_code,
            schema.customer.region,
            schema.customer.country,
            schema.customer.phone,
            schema.customer.fax,
        ))
        .from(schema.customer)
        .r#where(eq(schema.customer.id, target_id))
        .all()
        .await
        .map_err(db_err)?;
    let resp: Vec<CustomerResponse> = rows
        .into_iter()
        .map(
            |(
                id,
                company_name,
                contact_name,
                contact_title,
                address,
                city,
                postal_code,
                region,
                country,
                phone,
                fax,
            )| CustomerResponse {
                id,
                company_name,
                contact_name,
                contact_title,
                address,
                city,
                postal_code,
                region,
                country,
                phone,
                fax,
            },
        )
        .collect();
    Ok(Json(resp))
}

#[debug_handler(state = AppState)]
async fn employees_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeResponse>>, StatusCode> {
    if state.mode.is_raw() {
        return raw_employees(&state, params).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let rows: Vec<EmployeeRow> = db
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
        ))
        .from(schema.employee)
        .order_by([asc(schema.employee.id)])
        .limit(params.limit_or(50))
        .offset(params.offset())
        .all()
        .await
        .map_err(db_err)?;
    let resp: Vec<EmployeeResponse> = rows
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
            )| EmployeeResponse {
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
            },
        )
        .collect();
    Ok(Json(resp))
}

#[debug_handler(state = AppState)]
async fn suppliers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    if state.mode.is_raw() {
        return raw_suppliers(&state, params).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let rows: Vec<SupplierRow> = db
        .select((
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
        .from(schema.supplier)
        .order_by([asc(schema.supplier.id)])
        .limit(params.limit_or(50))
        .offset(params.offset())
        .all()
        .await
        .map_err(db_err)?;
    let resp: Vec<SupplierResponse> = rows
        .into_iter()
        .map(
            |(
                id,
                company_name,
                contact_name,
                contact_title,
                address,
                city,
                region,
                postal_code,
                country,
                phone,
            )| SupplierResponse {
                id,
                company_name,
                contact_name,
                contact_title,
                address,
                city,
                region,
                postal_code,
                country,
                phone,
            },
        )
        .collect();
    Ok(Json(resp))
}

#[debug_handler(state = AppState)]
async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    if state.mode.is_raw() {
        return raw_supplier_by_id(&state, params).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let target_id = params.user_id(super::SEED_SUPPLIERS as i32);
    let rows: Vec<SupplierRow> = db
        .select((
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
        .from(schema.supplier)
        .r#where(eq(schema.supplier.id, target_id))
        .all()
        .await
        .map_err(db_err)?;
    let resp: Vec<SupplierResponse> = rows
        .into_iter()
        .map(
            |(
                id,
                company_name,
                contact_name,
                contact_title,
                address,
                city,
                region,
                postal_code,
                country,
                phone,
            )| SupplierResponse {
                id,
                company_name,
                contact_name,
                contact_title,
                address,
                city,
                region,
                postal_code,
                country,
                phone,
            },
        )
        .collect();
    Ok(Json(resp))
}

#[debug_handler(state = AppState)]
async fn products_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    if state.mode.is_raw() {
        return raw_products(&state, params).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let rows: Vec<ProductRow> = db
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
        ))
        .from(schema.product)
        .order_by([asc(schema.product.id)])
        .limit(params.limit_or(50))
        .offset(params.offset())
        .all()
        .await
        .map_err(db_err)?;
    let resp: Vec<ProductResponse> = rows
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
            )| ProductResponse {
                id,
                name,
                qt_per_unit,
                unit_price,
                units_in_stock,
                units_on_order,
                reorder_level,
                discontinued,
                supplier_id,
            },
        )
        .collect();
    Ok(Json(resp))
}

// ---------------------------------------------------------------------------
// Relation / aggregate routes
//
// turso has no server-side GROUP BY plan the raw baselines can share, so every
// target uses the same request shape here (see the target spec's
// `sql_variant`); what differs is who builds the SQL — the drizzle select
// builder or a hand-written string.
// ---------------------------------------------------------------------------

#[debug_handler(state = AppState)]
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeWithRecipientResponse>>, StatusCode> {
    let target_id = params.user_id(super::SEED_EMPLOYEES as i32);
    if state.mode.is_raw() {
        return raw_employee_with_recipient(&state, target_id).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
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
        .r#where(eq(schema.employee.id, target_id))
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
    let target_id = params.user_id(super::SEED_PRODUCTS as i32);
    if state.mode.is_raw() {
        return raw_product_with_supplier(&state, target_id).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
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
        .r#where(eq(schema.product.id, target_id))
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

/// Fold the detail rows of an id range into per-order aggregates.
fn aggregate_details(
    details: impl IntoIterator<Item = (i32, i32, f64)>,
) -> BTreeMap<i32, (i32, f64, f64)> {
    let mut aggregates: BTreeMap<i32, (i32, f64, f64)> = BTreeMap::new();
    for (order_id, quantity, unit_price) in details {
        let entry = aggregates.entry(order_id).or_insert((0, 0.0, 0.0));
        entry.0 += 1;
        entry.1 += f64::from(quantity);
        entry.2 += f64::from(quantity) * unit_price;
    }
    aggregates
}

fn order_with_details_response(
    header: (i32, Option<i64>, String, String, String),
    aggregates: &BTreeMap<i32, (i32, f64, f64)>,
) -> OrderWithDetailsResponse {
    let (id, shipped_date, ship_name, ship_city, ship_country) = header;
    let (products_count, quantity_sum, total_price) =
        aggregates.get(&id).copied().unwrap_or_default();
    OrderWithDetailsResponse {
        id,
        shipped_date,
        ship_name,
        ship_city,
        ship_country,
        products_count,
        quantity_sum,
        total_price,
    }
}

#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<OrderWithDetailsResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    if state.mode.is_raw() {
        return raw_orders_with_details(&state, limit, offset).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let orders: Vec<OrderHeaderRow> = db
        .select((
            schema.order.id,
            schema.order.shipped_date,
            schema.order.ship_name,
            schema.order.ship_city,
            schema.order.ship_country,
        ))
        .from(schema.order)
        .order_by([asc(schema.order.id)])
        .limit(limit)
        .offset(offset)
        .all()
        .await
        .map_err(db_err)?;
    let (Some(first), Some(last)) = (orders.first(), orders.last()) else {
        return Ok(Json(Vec::new()));
    };
    let (min_id, max_id) = (first.0, last.0);

    let details: Vec<DetailAggregateRow> = db
        .select((
            schema.detail.order_id,
            schema.detail.product_id,
            schema.detail.quantity,
            schema.detail.unit_price,
        ))
        .from(schema.detail)
        .r#where(and(
            gte(schema.detail.order_id, min_id),
            lte(schema.detail.order_id, max_id),
        ))
        .all()
        .await
        .map_err(db_err)?;
    let aggregates = aggregate_details(
        details
            .into_iter()
            .map(|(order_id, _product_id, quantity, unit_price)| (order_id, quantity, unit_price)),
    );

    Ok(Json(
        orders
            .into_iter()
            .map(|header| order_with_details_response(header, &aggregates))
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsResponse>>, StatusCode> {
    let target_id = params.user_id(super::SEED_ORDERS as i32);
    if state.mode.is_raw() {
        return raw_order_with_details(&state, target_id).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let orders: Vec<OrderRow> = db
        .select((
            schema.order.id,
            schema.order.order_date,
            schema.order.required_date,
            schema.order.shipped_date,
            schema.order.ship_via,
            schema.order.freight,
            schema.order.ship_name,
            schema.order.ship_city,
            schema.order.ship_region,
            schema.order.ship_postal_code,
            schema.order.ship_country,
            schema.order.customer_id,
            schema.order.employee_id,
        ))
        .from(schema.order)
        .r#where(eq(schema.order.id, target_id))
        .all()
        .await
        .map_err(db_err)?;
    let details: Vec<DetailRow> = db
        .select((
            schema.detail.unit_price,
            schema.detail.quantity,
            schema.detail.discount,
            schema.detail.order_id,
            schema.detail.product_id,
        ))
        .from(schema.detail)
        .r#where(eq(schema.detail.order_id, target_id))
        .all()
        .await
        .map_err(db_err)?;
    let details: Vec<OrderDetailResponse> = details
        .into_iter()
        .map(
            |(unit_price, quantity, discount, order_id, product_id)| OrderDetailResponse {
                unit_price,
                quantity,
                discount,
                order_id,
                product_id,
            },
        )
        .collect();
    Ok(Json(
        orders
            .into_iter()
            .map(|order| single_order_response(order, details.clone()))
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsAndProductsResponse>>, StatusCode> {
    let target_id = params.user_id(super::SEED_ORDERS as i32);
    if state.mode.is_raw() {
        return raw_order_with_details_and_products(&state, target_id).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let orders: Vec<OrderRow> = db
        .select((
            schema.order.id,
            schema.order.order_date,
            schema.order.required_date,
            schema.order.shipped_date,
            schema.order.ship_via,
            schema.order.freight,
            schema.order.ship_name,
            schema.order.ship_city,
            schema.order.ship_region,
            schema.order.ship_postal_code,
            schema.order.ship_country,
            schema.order.customer_id,
            schema.order.employee_id,
        ))
        .from(schema.order)
        .r#where(eq(schema.order.id, target_id))
        .all()
        .await
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
        .r#where(eq(schema.detail.order_id, target_id))
        .all()
        .await
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
    Ok(Json(
        orders
            .into_iter()
            .map(|order| single_order_and_products_response(order, details.clone()))
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
        return raw_search_customer(&state, &pattern).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let rows: Vec<CustomerRow> = db
        .select((
            schema.customer.id,
            schema.customer.company_name,
            schema.customer.contact_name,
            schema.customer.contact_title,
            schema.customer.address,
            schema.customer.city,
            schema.customer.postal_code,
            schema.customer.region,
            schema.customer.country,
            schema.customer.phone,
            schema.customer.fax,
        ))
        .from(schema.customer)
        .r#where(like(schema.customer.company_name, pattern.as_str()))
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(customer_response).collect()))
}

#[debug_handler(state = AppState)]
async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or(""));
    if state.mode.is_raw() {
        return raw_search_product(&state, &pattern).await;
    }

    let schema = state.schema;
    let db = acquire_db(&state).await?;
    let rows: Vec<ProductRow> = db
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
        ))
        .from(schema.product)
        .r#where(like(schema.product.name, pattern.as_str()))
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(product_response).collect()))
}

// ---------------------------------------------------------------------------
// Raw turso baselines
// ---------------------------------------------------------------------------

async fn raw_employee_with_recipient(
    state: &AppState,
    target_id: i32,
) -> Result<Json<Vec<EmployeeWithRecipientResponse>>, StatusCode> {
    let db = acquire_db(state).await?;
    let collected = query_rows(
        db.conn(),
        "SELECT e.id, e.last_name, e.first_name, e.title, e.title_of_courtesy, e.birth_date, e.hire_date, e.address, e.city, e.postal_code, e.country, e.home_phone, e.extension, e.notes, e.recipient_id, r.last_name, r.first_name FROM employees e LEFT JOIN employees r ON e.recipient_id = r.id WHERE e.id = ?1",
        [::turso::Value::from(target_id)],
        state.mode,
    )
    .await?;
    Ok(Json(
        collected
            .iter()
            .map(|r| EmployeeWithRecipientResponse {
                id: r.get::<i32>(0).unwrap_or_default(),
                last_name: r.get::<String>(1).unwrap_or_default(),
                first_name: r.get::<String>(2).ok(),
                title: r.get::<String>(3).unwrap_or_default(),
                title_of_courtesy: r.get::<String>(4).unwrap_or_default(),
                birth_date: r.get::<i64>(5).unwrap_or_default(),
                hire_date: r.get::<i64>(6).unwrap_or_default(),
                address: r.get::<String>(7).unwrap_or_default(),
                city: r.get::<String>(8).unwrap_or_default(),
                postal_code: r.get::<String>(9).unwrap_or_default(),
                country: r.get::<String>(10).unwrap_or_default(),
                home_phone: r.get::<String>(11).unwrap_or_default(),
                extension: r.get::<i32>(12).unwrap_or_default(),
                notes: r.get::<String>(13).unwrap_or_default(),
                recipient_id: r.get::<i32>(14).ok(),
                recipient_last_name: r.get::<String>(15).ok(),
                recipient_first_name: r.get::<String>(16).ok(),
            })
            .collect(),
    ))
}

async fn raw_product_with_supplier(
    state: &AppState,
    target_id: i32,
) -> Result<Json<Vec<ProductWithSupplierResponse>>, StatusCode> {
    let db = acquire_db(state).await?;
    let collected = query_rows(
        db.conn(),
        "SELECT p.id, p.name, p.qt_per_unit, p.unit_price, p.units_in_stock, p.units_on_order, p.reorder_level, p.discontinued, p.supplier_id, s.id, s.company_name, s.contact_name, s.contact_title, s.address, s.city, s.region, s.postal_code, s.country, s.phone FROM products p INNER JOIN suppliers s ON p.supplier_id = s.id WHERE p.id = ?1",
        [::turso::Value::from(target_id)],
        state.mode,
    )
    .await?;
    Ok(Json(
        collected
            .iter()
            .map(|r| ProductWithSupplierResponse {
                id: r.get::<i32>(0).unwrap_or_default(),
                name: r.get::<String>(1).unwrap_or_default(),
                qt_per_unit: r.get::<String>(2).unwrap_or_default(),
                unit_price: r.get::<f64>(3).unwrap_or_default(),
                units_in_stock: r.get::<i32>(4).unwrap_or_default(),
                units_on_order: r.get::<i32>(5).unwrap_or_default(),
                reorder_level: r.get::<i32>(6).unwrap_or_default(),
                discontinued: r.get::<i32>(7).unwrap_or_default(),
                supplier_id: r.get::<i32>(8).unwrap_or_default(),
                supplier: SupplierResponse {
                    id: r.get::<i32>(9).unwrap_or_default(),
                    company_name: r.get::<String>(10).unwrap_or_default(),
                    contact_name: r.get::<String>(11).unwrap_or_default(),
                    contact_title: r.get::<String>(12).unwrap_or_default(),
                    address: r.get::<String>(13).unwrap_or_default(),
                    city: r.get::<String>(14).unwrap_or_default(),
                    region: r.get::<String>(15).ok(),
                    postal_code: r.get::<String>(16).unwrap_or_default(),
                    country: r.get::<String>(17).unwrap_or_default(),
                    phone: r.get::<String>(18).unwrap_or_default(),
                },
            })
            .collect(),
    ))
}

async fn raw_orders_with_details(
    state: &AppState,
    limit: usize,
    offset: usize,
) -> Result<Json<Vec<OrderWithDetailsResponse>>, StatusCode> {
    let db = acquire_db(state).await?;
    let orders = query_rows(
        db.conn(),
        "SELECT id, shipped_date, ship_name, ship_city, ship_country FROM orders ORDER BY id LIMIT ?1 OFFSET ?2",
        [
            ::turso::Value::from(limit as i64),
            ::turso::Value::from(offset as i64),
        ],
        state.mode,
    )
    .await?;
    if orders.is_empty() {
        return Ok(Json(Vec::new()));
    }

    let min_id = orders
        .first()
        .and_then(|row| row.get::<i32>(0).ok())
        .unwrap_or_default();
    let max_id = orders
        .last()
        .and_then(|row| row.get::<i32>(0).ok())
        .unwrap_or(min_id);
    let details = query_rows(
        db.conn(),
        "SELECT order_id, product_id, quantity, unit_price FROM order_details WHERE order_id >= ?1 AND order_id <= ?2",
        [::turso::Value::from(min_id), ::turso::Value::from(max_id)],
        state.mode,
    )
    .await?;
    let aggregates = aggregate_details(details.iter().map(|row| {
        (
            row.get::<i32>(0).unwrap_or_default(),
            row.get::<i32>(2).unwrap_or_default(),
            row.get::<f64>(3).unwrap_or_default(),
        )
    }));

    Ok(Json(
        orders
            .iter()
            .map(|row| {
                order_with_details_response(
                    (
                        row.get::<i32>(0).unwrap_or_default(),
                        row.get::<i64>(1).ok(),
                        row.get::<String>(2).unwrap_or_default(),
                        row.get::<String>(3).unwrap_or_default(),
                        row.get::<String>(4).unwrap_or_default(),
                    ),
                    &aggregates,
                )
            })
            .collect(),
    ))
}

async fn raw_order_with_details(
    state: &AppState,
    target_id: i32,
) -> Result<Json<Vec<SingleOrderWithDetailsResponse>>, StatusCode> {
    let db = acquire_db(state).await?;
    let order_collected = query_rows(
        db.conn(),
        RAW_ORDER_BY_ID_SQL,
        [::turso::Value::from(target_id)],
        state.mode,
    )
    .await?;
    let detail_collected = query_rows(
        db.conn(),
        "SELECT unit_price, quantity, discount, order_id, product_id FROM order_details WHERE order_id = ?1",
        [::turso::Value::from(target_id)],
        state.mode,
    )
    .await?;
    let details: Vec<OrderDetailResponse> = detail_collected
        .iter()
        .map(|r| OrderDetailResponse {
            unit_price: r.get::<f64>(0).unwrap_or_default(),
            quantity: r.get::<i32>(1).unwrap_or_default(),
            discount: r.get::<f64>(2).unwrap_or_default(),
            order_id: r.get::<i32>(3).unwrap_or_default(),
            product_id: r.get::<i32>(4).unwrap_or_default(),
        })
        .collect();
    Ok(Json(
        order_collected
            .iter()
            .map(|r| single_order_response(raw_order_row(r), details.clone()))
            .collect(),
    ))
}

async fn raw_order_with_details_and_products(
    state: &AppState,
    target_id: i32,
) -> Result<Json<Vec<SingleOrderWithDetailsAndProductsResponse>>, StatusCode> {
    let db = acquire_db(state).await?;
    let order_collected = query_rows(
        db.conn(),
        RAW_ORDER_BY_ID_SQL,
        [::turso::Value::from(target_id)],
        state.mode,
    )
    .await?;
    let detail_collected = query_rows(
        db.conn(),
        "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name FROM order_details d LEFT JOIN products p ON d.product_id = p.id WHERE d.order_id = ?1",
        [::turso::Value::from(target_id)],
        state.mode,
    )
    .await?;
    let details: Vec<OrderDetailProductResponse> = detail_collected
        .iter()
        .map(|r| OrderDetailProductResponse {
            unit_price: r.get::<f64>(0).unwrap_or_default(),
            quantity: r.get::<i32>(1).unwrap_or_default(),
            discount: r.get::<f64>(2).unwrap_or_default(),
            order_id: r.get::<i32>(3).unwrap_or_default(),
            product_id: r.get::<i32>(4).unwrap_or_default(),
            product_name: r.get::<String>(5).unwrap_or_default(),
        })
        .collect();
    Ok(Json(
        order_collected
            .iter()
            .map(|r| single_order_and_products_response(raw_order_row(r), details.clone()))
            .collect(),
    ))
}

async fn raw_search_customer(
    state: &AppState,
    pattern: &str,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let db = acquire_db(state).await?;
    let collected = query_rows(
        db.conn(),
        "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE company_name LIKE ?1",
        [::turso::Value::from(pattern.to_string())],
        state.mode,
    )
    .await?;
    Ok(Json(collected.iter().map(raw_customer_response).collect()))
}

async fn raw_search_product(
    state: &AppState,
    pattern: &str,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let db = acquire_db(state).await?;
    let collected = query_rows(
        db.conn(),
        "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products WHERE name LIKE ?1",
        [::turso::Value::from(pattern.to_string())],
        state.mode,
    )
    .await?;
    Ok(Json(collected.iter().map(raw_product_response).collect()))
}

const RAW_ORDER_BY_ID_SQL: &str = "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = ?1";

// ---------------------------------------------------------------------------
// Row → response conversions shared by both paths
// ---------------------------------------------------------------------------

fn single_order_response(
    order: OrderRow,
    details: Vec<OrderDetailResponse>,
) -> SingleOrderWithDetailsResponse {
    let (
        id,
        order_date,
        required_date,
        shipped_date,
        ship_via,
        freight,
        ship_name,
        ship_city,
        ship_region,
        ship_postal_code,
        ship_country,
        customer_id,
        employee_id,
    ) = order;
    SingleOrderWithDetailsResponse {
        id,
        order_date,
        required_date,
        shipped_date,
        ship_via,
        freight,
        ship_name,
        ship_city,
        ship_region,
        ship_postal_code,
        ship_country,
        customer_id,
        employee_id,
        details,
    }
}

fn single_order_and_products_response(
    order: OrderRow,
    details: Vec<OrderDetailProductResponse>,
) -> SingleOrderWithDetailsAndProductsResponse {
    let (
        id,
        order_date,
        required_date,
        shipped_date,
        ship_via,
        freight,
        ship_name,
        ship_city,
        ship_region,
        ship_postal_code,
        ship_country,
        customer_id,
        employee_id,
    ) = order;
    SingleOrderWithDetailsAndProductsResponse {
        id,
        order_date,
        required_date,
        shipped_date,
        ship_via,
        freight,
        ship_name,
        ship_city,
        ship_region,
        ship_postal_code,
        ship_country,
        customer_id,
        employee_id,
        details,
    }
}

fn raw_order_row(r: &::turso::Row) -> OrderRow {
    (
        r.get::<i32>(0).unwrap_or_default(),
        r.get::<i64>(1).unwrap_or_default(),
        r.get::<i64>(2).unwrap_or_default(),
        r.get::<i64>(3).ok(),
        r.get::<i32>(4).unwrap_or_default(),
        r.get::<f64>(5).unwrap_or_default(),
        r.get::<String>(6).unwrap_or_default(),
        r.get::<String>(7).unwrap_or_default(),
        r.get::<String>(8).ok(),
        r.get::<String>(9).ok(),
        r.get::<String>(10).unwrap_or_default(),
        r.get::<i32>(11).unwrap_or_default(),
        r.get::<i32>(12).unwrap_or_default(),
    )
}

fn customer_response(row: CustomerRow) -> CustomerResponse {
    let (
        id,
        company_name,
        contact_name,
        contact_title,
        address,
        city,
        postal_code,
        region,
        country,
        phone,
        fax,
    ) = row;
    CustomerResponse {
        id,
        company_name,
        contact_name,
        contact_title,
        address,
        city,
        postal_code,
        region,
        country,
        phone,
        fax,
    }
}

fn product_response(row: ProductRow) -> ProductResponse {
    let (
        id,
        name,
        qt_per_unit,
        unit_price,
        units_in_stock,
        units_on_order,
        reorder_level,
        discontinued,
        supplier_id,
    ) = row;
    ProductResponse {
        id,
        name,
        qt_per_unit,
        unit_price,
        units_in_stock,
        units_on_order,
        reorder_level,
        discontinued,
        supplier_id,
    }
}

fn raw_customer_response(row: &::turso::Row) -> CustomerResponse {
    CustomerResponse {
        id: row.get::<i32>(0).unwrap_or_default(),
        company_name: row.get::<String>(1).unwrap_or_default(),
        contact_name: row.get::<String>(2).unwrap_or_default(),
        contact_title: row.get::<String>(3).unwrap_or_default(),
        address: row.get::<String>(4).unwrap_or_default(),
        city: row.get::<String>(5).unwrap_or_default(),
        postal_code: row.get::<String>(6).ok(),
        region: row.get::<String>(7).ok(),
        country: row.get::<String>(8).unwrap_or_default(),
        phone: row.get::<String>(9).unwrap_or_default(),
        fax: row.get::<String>(10).ok(),
    }
}

fn raw_product_response(row: &::turso::Row) -> ProductResponse {
    ProductResponse {
        id: row.get::<i32>(0).unwrap_or_default(),
        name: row.get::<String>(1).unwrap_or_default(),
        qt_per_unit: row.get::<String>(2).unwrap_or_default(),
        unit_price: row.get::<f64>(3).unwrap_or_default(),
        units_in_stock: row.get::<i32>(4).unwrap_or_default(),
        units_on_order: row.get::<i32>(5).unwrap_or_default(),
        reorder_level: row.get::<i32>(6).unwrap_or_default(),
        discontinued: row.get::<i32>(7).unwrap_or_default(),
        supplier_id: row.get::<i32>(8).unwrap_or_default(),
    }
}
