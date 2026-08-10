use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Router, debug_handler};
use chrono::NaiveDate;
use drizzle::core::expr::{alias, coalesce, count, eq, sum};
use drizzle::postgres::prelude::*;
use std::sync::mpsc;

const SEED_CACHE_VERSION: &str = "postgres-v3";
const SEED_CACHE_LOCK_KEY: i64 = 0x6472_7a6c_5f62_6e63;

const DROP_PUBLIC_TABLES_SQL: &str = "DROP TABLE IF EXISTS public.order_details;
     DROP TABLE IF EXISTS public.orders;
     DROP TABLE IF EXISTS public.products;
     DROP TABLE IF EXISTS public.suppliers;
     DROP TABLE IF EXISTS public.employees;
     DROP TABLE IF EXISTS public.customers;";

const CREATE_INDEXES_SQL: &str =
    "CREATE INDEX IF NOT EXISTS idx_employees_recipient ON employees(recipient_id);
     CREATE INDEX IF NOT EXISTS idx_products_supplier ON products(supplier_id);
     CREATE INDEX IF NOT EXISTS idx_details_order ON order_details(order_id);
     CREATE INDEX IF NOT EXISTS idx_details_product ON order_details(product_id);";

const RESET_SEQUENCES_SQL: &str =
    "SELECT setval(pg_get_serial_sequence('customers', 'id'), COALESCE((SELECT max(id) FROM customers), 1), true);
     SELECT setval(pg_get_serial_sequence('employees', 'id'), COALESCE((SELECT max(id) FROM employees), 1), true);
     SELECT setval(pg_get_serial_sequence('suppliers', 'id'), COALESCE((SELECT max(id) FROM suppliers), 1), true);
     SELECT setval(pg_get_serial_sequence('products', 'id'), COALESCE((SELECT max(id) FROM products), 1), true);
     SELECT setval(pg_get_serial_sequence('orders', 'id'), COALESCE((SELECT max(id) FROM orders), 1), true);";

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

// ---------------------------------------------------------------------------
// Response types (camelCase JSON) — same as sqlite module
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

// ---------------------------------------------------------------------------
// Select-model conversions and joined-row shapes
// ---------------------------------------------------------------------------

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

// ---------------------------------------------------------------------------
// Commands dispatched to sync worker thread
// ---------------------------------------------------------------------------

enum DbCmd {
    Customers {
        offset: usize,
        limit: usize,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    CustomerById {
        id: i32,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    Employees {
        offset: usize,
        limit: usize,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    Suppliers {
        offset: usize,
        limit: usize,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    SupplierById {
        id: i32,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    Products {
        offset: usize,
        limit: usize,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    EmployeeWithRecipient {
        id: i32,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    ProductWithSupplier {
        id: i32,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    OrdersWithDetails {
        offset: usize,
        limit: usize,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    OrderWithDetails {
        id: i32,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    OrderWithDetailsAndProducts {
        id: i32,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    SearchCustomer {
        term: String,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
    SearchProduct {
        term: String,
        reply: oneshot::Sender<Result<String, StatusCode>>,
    },
}

/// Queue depth per worker before the HTTP side blocks.
///
/// A bounded queue is the backpressure: without it a slow database just grows
/// an unbounded backlog and the measured latency becomes queueing delay in the
/// runner's own process.
const QUEUE_DEPTH_PER_WORKER: usize = 4;

#[derive(Clone)]
struct AppState {
    /// One shared queue for every worker.
    ///
    /// Per-worker channels with round-robin dispatch head-of-line block: a
    /// request assigned to a busy worker waits even when another is idle.
    tx: mpsc::SyncSender<DbCmd>,
}

impl AppState {
    fn tx(&self) -> &mpsc::SyncSender<DbCmd> {
        &self.tx
    }
}

pub async fn serve(seed: u64) -> Result<ServerHandle, Fail> {
    let database_url = pg_url();
    tokio::task::spawn_blocking(move || seed_database_url(&database_url, seed))
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("pg_sync seed panicked: {err}")))?
        .map_err(|msg| Fail::new(Code::RunFail, msg))?;

    let pool_size = super::configured_pool_size(super::POSTGRES_POOL_SIZE);
    let (cmd_tx, cmd_rx) = mpsc::sync_channel::<DbCmd>(pool_size * QUEUE_DEPTH_PER_WORKER);
    let cmd_rx = Arc::new(Mutex::new(cmd_rx));
    let mut workers = Vec::with_capacity(pool_size);
    let mut ready = Vec::with_capacity(pool_size);

    for worker_id in 0..pool_size {
        let (ready_tx, ready_rx) = oneshot::channel::<Result<(), String>>();
        ready.push(ready_rx);
        let cmd_rx = Arc::clone(&cmd_rx);

        workers.push(std::thread::spawn(move || {
            let mut db = match connect_db(&pg_url()) {
                Ok(db) => {
                    let _ = ready_tx.send(Ok(()));
                    db
                }
                Err(msg) => {
                    let _ = ready_tx.send(Err(msg.clone()));
                    return Err(msg);
                }
            };

            loop {
                // The lock is only held for the dequeue, never across a query.
                let next = cmd_rx.lock().unwrap_or_else(|err| err.into_inner()).recv();
                let Ok(cmd) = next else { break };
                if let Err(msg) = handle_cmd(&mut db, cmd) {
                    eprintln!("pg_sync worker {worker_id} error: {msg}");
                }
            }
            Ok(())
        }));
    }

    for ready_rx in ready {
        ready_rx
            .await
            .map_err(|_| Fail::new(Code::RunFail, "pg_sync worker dropped before ready"))?
            .map_err(|msg| Fail::new(Code::RunFail, msg))?;
    }

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
        .with_state(AppState { tx: cmd_tx });
    let mut handle = spawn_server(router).await?;
    handle.workers.extend(workers);
    Ok(handle)
}

/// Server-side tuning every PostgreSQL target runs against.
///
/// Two of these correct assumptions the stock image makes about hardware it is
/// not running on. `random_page_cost` defaults to 4.0, a spinning-disk figure
/// that biases the planner away from index scans; this dataset is ~50k orders on
/// an SSD and, after warmup, in RAM. `effective_cache_size` tells the planner how
/// much of it the machine is actually caching. `work_mem`'s 4 MB default is what
/// the `/orders-with-details` aggregate sorts inside.
///
/// `shared_buffers` is deliberately absent: it needs a restart, and a GitHub
/// Actions `services:` block cannot pass a `command`, so it cannot be set there
/// at all. Setting it locally and not in CI would mean the local and published
/// numbers came from different servers — worse than leaving it at a default that
/// already holds a dataset this size.
///
/// Applied with `ALTER DATABASE` rather than `SET`, so it reaches every client of
/// this database — including the external ORM crates, which open their own
/// connections and never run this code.
const PG_TUNING: &[(&str, &str)] = &[
    ("random_page_cost", "1.1"),
    ("effective_cache_size", "2GB"),
    ("work_mem", "16MB"),
];

pub(crate) fn seed_database_url(database_url: &str, seed: u64) -> Result<(), String> {
    apply_server_tuning(database_url)?;
    seed_database_url_from_schema_cache(database_url, seed)
}

/// Apply `PG_TUNING`, then confirm on a fresh session that it took.
///
/// `ALTER DATABASE` only affects sessions opened after it, so the readback needs
/// its own connection — and it is a readback rather than an assumption because
/// every PostgreSQL target spec *declares* this tuning to the reader. A setting
/// the server rejected or clamped would make that declaration false.
fn apply_server_tuning(database_url: &str) -> Result<(), String> {
    let mut conn = ::postgres::Client::connect(database_url, ::postgres::NoTls)
        .map_err(|err| format!("postgres tuning connect failed: {err}"))?;

    // `ALTER DATABASE` needs the name as an identifier, which no placeholder can
    // carry, so resolve it and quote it here. Setting names and values are
    // compile-time constants.
    let dbname: String = conn
        .query_one("SELECT current_database()", &[])
        .map_err(|err| format!("postgres current_database failed: {err}"))?
        .get(0);
    let quoted = format!("\"{}\"", dbname.replace('"', "\"\""));

    for (name, value) in PG_TUNING {
        conn.batch_execute(&format!("ALTER DATABASE {quoted} SET {name} = '{value}'"))
            .map_err(|err| format!("postgres ALTER DATABASE {name} failed: {err}"))?;
    }
    drop(conn);

    let mut fresh = ::postgres::Client::connect(database_url, ::postgres::NoTls)
        .map_err(|err| format!("postgres tuning readback connect failed: {err}"))?;
    for (name, expected) in PG_TUNING {
        let row = fresh
            .query_one(&format!("SHOW {name}"), &[])
            .map_err(|err| format!("postgres SHOW {name} failed: {err}"))?;
        let actual: String = row.get(0);
        if !same_pg_setting(&actual, expected) {
            return Err(format!(
                "postgres {name} is {actual}, expected {expected}; the declared server tuning \
                 would be false for this run"
            ));
        }
    }
    Ok(())
}

/// PostgreSQL echoes a setting in its own canonical spelling — `2GB` may come
/// back as `2GB`, and a float as `1.1`. Compare case-insensitively with
/// whitespace stripped rather than demanding an exact byte match.
fn same_pg_setting(actual: &str, expected: &str) -> bool {
    let norm = |s: &str| s.trim().replace(' ', "").to_ascii_lowercase();
    norm(actual) == norm(expected)
}

fn seed_database_url_from_schema_cache(database_url: &str, seed: u64) -> Result<(), String> {
    let conn = ::postgres::Client::connect(database_url, ::postgres::NoTls)
        .map_err(|err| format!("postgres connect failed: {err}"))?;
    let (mut db, schema) = drizzle::postgres::sync::Drizzle::new(conn, Schema::new());

    db.conn_mut()
        .execute("SELECT pg_advisory_lock($1)", &[&SEED_CACHE_LOCK_KEY])
        .map_err(|err| format!("postgres seed cache lock failed: {err}"))?;

    let result = (|| {
        ensure_seed_cache(&mut db, &schema, seed)?;
        reset_public_from_cache(&mut db, seed)
    })();

    let unlock = db
        .conn_mut()
        .execute("SELECT pg_advisory_unlock($1)", &[&SEED_CACHE_LOCK_KEY])
        .map_err(|err| format!("postgres seed cache unlock failed: {err}"));

    match (result, unlock) {
        (Ok(()), Ok(_)) => Ok(()),
        (Err(err), Ok(_)) => Err(err),
        (Ok(()), Err(err)) => Err(err),
        (Err(seed_err), Err(unlock_err)) => Err(format!("{seed_err}; {unlock_err}")),
    }
}

fn seed_cache_schema(seed: u64) -> String {
    format!("bench_seed_{SEED_CACHE_VERSION}_{seed}").replace('-', "_")
}

fn postgres_seed_statements(
    schema: &Schema,
    seed: u64,
) -> Vec<drizzle_seed::PostgresSeedStatement> {
    drizzle_seed::SeedConfig::postgres(schema)
        .seed(seed)
        .count(&schema.customer, super::SEED_CUSTOMERS)
        .count(&schema.employee, super::SEED_EMPLOYEES)
        .count(&schema.supplier, super::SEED_SUPPLIERS)
        .count(&schema.product, super::SEED_PRODUCTS)
        .count(&schema.order, super::SEED_ORDERS)
        .relation(&schema.order, &schema.detail, 6)
        .generate()
}

fn ensure_seed_cache(
    db: &mut drizzle::postgres::sync::Drizzle<Schema>,
    schema: &Schema,
    seed: u64,
) -> Result<(), String> {
    let cache_schema = seed_cache_schema(seed);
    if seed_cache_ready(db, &cache_schema, seed)? {
        return Ok(());
    }

    let cache_ident = quote_ident(&cache_schema);
    db.conn_mut()
        .batch_execute(&format!(
            "BEGIN;
             DROP SCHEMA IF EXISTS {cache_ident} CASCADE;
             CREATE SCHEMA {cache_ident};
             SET LOCAL search_path TO {cache_ident};"
        ))
        .map_err(|err| format!("postgres seed cache init failed: {err}"))?;

    let result = (|| {
        db.create()
            .map_err(|err| format!("postgres seed cache create failed: {err}"))?;

        for stmt in postgres_seed_statements(schema, seed) {
            let preview = stmt.sql();
            db.execute(stmt).map_err(|err| {
                format!("postgres seed cache insert failed in `{preview}`: {err:?}")
            })?;
        }

        db.conn_mut()
            .batch_execute(CREATE_INDEXES_SQL)
            .map_err(|err| format!("postgres seed cache indexes failed: {err}"))?;
        write_seed_cache_meta(db, &cache_ident, seed)
    })();

    finish_transaction(db, result, "postgres seed cache")
}

fn seed_cache_ready(
    db: &mut drizzle::postgres::sync::Drizzle<Schema>,
    cache_schema: &str,
    seed: u64,
) -> Result<bool, String> {
    let row = db
        .conn_mut()
        .query_one(
            "SELECT EXISTS (
               SELECT 1
               FROM pg_class c
               JOIN pg_namespace n ON n.oid = c.relnamespace
               WHERE n.nspname = $1 AND c.relname = '__bench_seed_meta'
             )",
            &[&cache_schema],
        )
        .map_err(|err| format!("postgres seed cache lookup failed: {err}"))?;
    if !row.get::<_, bool>(0) {
        return Ok(false);
    }

    let cache_ident = quote_ident(cache_schema);
    let Ok(meta) = db.conn_mut().query_one(
        &format!(
            "SELECT version, seed, customers, employees, suppliers, products, orders
             FROM {cache_ident}.__bench_seed_meta
             LIMIT 1"
        ),
        &[],
    ) else {
        return Ok(false);
    };

    Ok(meta.get::<_, String>(0) == SEED_CACHE_VERSION
        && meta.get::<_, i64>(1) == seed as i64
        && meta.get::<_, i64>(2) == super::SEED_CUSTOMERS as i64
        && meta.get::<_, i64>(3) == super::SEED_EMPLOYEES as i64
        && meta.get::<_, i64>(4) == super::SEED_SUPPLIERS as i64
        && meta.get::<_, i64>(5) == super::SEED_PRODUCTS as i64
        && meta.get::<_, i64>(6) == super::SEED_ORDERS as i64)
}

fn write_seed_cache_meta(
    db: &mut drizzle::postgres::sync::Drizzle<Schema>,
    cache_ident: &str,
    seed: u64,
) -> Result<(), String> {
    db.conn_mut()
        .batch_execute(&format!(
            "CREATE TABLE {cache_ident}.__bench_seed_meta (
               version text PRIMARY KEY,
               seed bigint NOT NULL,
               customers bigint NOT NULL,
               employees bigint NOT NULL,
               suppliers bigint NOT NULL,
               products bigint NOT NULL,
               orders bigint NOT NULL
             );"
        ))
        .map_err(|err| format!("postgres seed cache metadata create failed: {err}"))?;

    db.conn_mut()
        .execute(
            &format!(
                "INSERT INTO {cache_ident}.__bench_seed_meta
                 (version, seed, customers, employees, suppliers, products, orders)
                 VALUES ($1, $2, $3, $4, $5, $6, $7)"
            ),
            &[
                &SEED_CACHE_VERSION,
                &(seed as i64),
                &(super::SEED_CUSTOMERS as i64),
                &(super::SEED_EMPLOYEES as i64),
                &(super::SEED_SUPPLIERS as i64),
                &(super::SEED_PRODUCTS as i64),
                &(super::SEED_ORDERS as i64),
            ],
        )
        .map(|_| ())
        .map_err(|err| format!("postgres seed cache metadata insert failed: {err}"))
}

fn reset_public_from_cache(
    db: &mut drizzle::postgres::sync::Drizzle<Schema>,
    seed: u64,
) -> Result<(), String> {
    let cache_ident = quote_ident(&seed_cache_schema(seed));
    let replica_role = set_replication_role_replica(db);
    db.conn_mut()
        .batch_execute(
            "BEGIN;
             SET LOCAL search_path TO public;
             SET LOCAL synchronous_commit TO off;",
        )
        .map_err(|err| {
            reset_replication_role(db, replica_role);
            format!("postgres reset begin failed: {err}")
        })?;

    let result = (|| {
        db.conn_mut()
            .batch_execute(DROP_PUBLIC_TABLES_SQL)
            .map_err(|err| format!("postgres drop failed: {err}"))?;
        db.create()
            .map_err(|err| format!("postgres create failed: {err}"))?;

        for table in [
            "customers",
            "employees",
            "suppliers",
            "products",
            "orders",
            "order_details",
        ] {
            db.conn_mut()
                .batch_execute(&format!(
                    "INSERT INTO public.{table} SELECT * FROM {cache_ident}.{table};"
                ))
                .map_err(|err| format!("postgres copy {table} from seed cache failed: {err}"))?;
        }

        db.conn_mut()
            .batch_execute(CREATE_INDEXES_SQL)
            .map_err(|err| format!("postgres create indexes failed: {err}"))?;
        db.conn_mut()
            .batch_execute(RESET_SEQUENCES_SQL)
            .map_err(|err| format!("postgres reset sequences failed: {err}"))?;
        // Every postgres target (including the external TS/ORM ones, which call
        // `seed-postgres`) plans against these statistics; without ANALYZE the
        // first target to warm the table would get a different plan than the
        // rest.
        db.conn_mut()
            .batch_execute("ANALYZE;")
            .map_err(|err| format!("postgres analyze failed: {err}"))?;

        Ok(())
    })();

    let result = finish_transaction(db, result, "postgres public reset");
    reset_replication_role(db, replica_role);
    result
}

fn finish_transaction(
    db: &mut drizzle::postgres::sync::Drizzle<Schema>,
    result: Result<(), String>,
    context: &str,
) -> Result<(), String> {
    let end = if result.is_ok() { "COMMIT" } else { "ROLLBACK" };
    let tx = db
        .conn_mut()
        .batch_execute(end)
        .map_err(|err| format!("{context} {end} failed: {err}"));
    match (result, tx) {
        (Ok(()), Ok(())) => Ok(()),
        (Err(err), Ok(())) => Err(err),
        (Ok(()), Err(err)) => Err(err),
        (Err(err), Err(tx_err)) => Err(format!("{err}; {tx_err}")),
    }
}

fn quote_ident(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

fn set_replication_role_replica(db: &mut drizzle::postgres::sync::Drizzle<Schema>) -> bool {
    db.conn_mut()
        .batch_execute("SET session_replication_role = replica;")
        .is_ok()
}

fn reset_replication_role(db: &mut drizzle::postgres::sync::Drizzle<Schema>, enabled: bool) {
    if enabled {
        let _ = db
            .conn_mut()
            .batch_execute("SET session_replication_role = DEFAULT;");
    }
}

fn connect_db(database_url: &str) -> Result<drizzle::postgres::sync::Drizzle<Schema>, String> {
    let conn = ::postgres::Client::connect(database_url, ::postgres::NoTls)
        .map_err(|err| format!("postgres connect failed: {err}"))?;
    Ok(drizzle::postgres::sync::Drizzle::new(conn, Schema::new()).0)
}

/// Serialize a response on the worker thread and hand the bytes back.
///
/// Serializing here keeps the JSON work on the pool thread that already owns
/// the rows, so the async side only moves a `String`.
fn reply_json<T: Serialize>(
    reply: oneshot::Sender<Result<String, StatusCode>>,
    value: &T,
) -> Result<(), String> {
    let _ = reply.send(serde_json::to_string(value).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
    Ok(())
}

fn handle_cmd(db: &mut drizzle::postgres::sync::Drizzle<Schema>, cmd: DbCmd) -> Result<(), String> {
    let schema = Schema::new();
    match cmd {
        DbCmd::Customers {
            offset,
            limit,
            reply,
        } => {
            let rows: Vec<SelectCustomer> = db
                .select(())
                .from(schema.customer)
                .order_by([asc(schema.customer.id)])
                .limit(limit)
                .offset(offset)
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<CustomerResponse> =
                rows.into_iter().map(CustomerResponse::from).collect();
            reply_json(reply, &resp)
        }
        DbCmd::CustomerById { id, reply } => {
            let rows: Vec<SelectCustomer> = db
                .select(())
                .from(schema.customer)
                .r#where(eq(schema.customer.id, id))
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<CustomerResponse> =
                rows.into_iter().map(CustomerResponse::from).collect();
            reply_json(reply, &resp)
        }
        DbCmd::Employees {
            offset,
            limit,
            reply,
        } => {
            let rows: Vec<SelectEmployee> = db
                .select(())
                .from(schema.employee)
                .order_by([asc(schema.employee.id)])
                .limit(limit)
                .offset(offset)
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<EmployeeResponse> =
                rows.into_iter().map(EmployeeResponse::from).collect();
            reply_json(reply, &resp)
        }
        DbCmd::Suppliers {
            offset,
            limit,
            reply,
        } => {
            let rows: Vec<SelectSupplier> = db
                .select(())
                .from(schema.supplier)
                .order_by([asc(schema.supplier.id)])
                .limit(limit)
                .offset(offset)
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<SupplierResponse> =
                rows.into_iter().map(SupplierResponse::from).collect();
            reply_json(reply, &resp)
        }
        DbCmd::SupplierById { id, reply } => {
            let rows: Vec<SelectSupplier> = db
                .select(())
                .from(schema.supplier)
                .r#where(eq(schema.supplier.id, id))
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<SupplierResponse> =
                rows.into_iter().map(SupplierResponse::from).collect();
            reply_json(reply, &resp)
        }
        DbCmd::Products {
            offset,
            limit,
            reply,
        } => {
            let rows: Vec<SelectProduct> = db
                .select(())
                .from(schema.product)
                .order_by([asc(schema.product.id)])
                .limit(limit)
                .offset(offset)
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<ProductResponse> = rows.into_iter().map(ProductResponse::from).collect();
            reply_json(reply, &resp)
        }
        DbCmd::EmployeeWithRecipient { id, reply } => {
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
                .map_err(|e| e.to_string())?;
            let resp: Vec<EmployeeWithRecipientResponse> = rows
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
                .collect();
            reply_json(reply, &resp)
        }
        DbCmd::ProductWithSupplier { id, reply } => {
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
                .map_err(|e| e.to_string())?;
            let resp: Vec<ProductWithSupplierResponse> = rows
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
                .collect();
            reply_json(reply, &resp)
        }
        DbCmd::OrdersWithDetails {
            offset,
            limit,
            reply,
        } => {
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
                        coalesce(sum(schema.detail.quantity * schema.detail.unit_price), 0.0),
                        "total_price",
                    ),
                ))
                .from(schema.order)
                .left_join((schema.detail, eq(schema.order.id, schema.detail.order_id)))
                .group_by(schema.order.id)
                .order_by([asc(schema.order.id)])
                .limit(limit)
                .offset(offset)
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<OrderWithDetailsResponse> = rows
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
                .collect();
            reply_json(reply, &resp)
        }
        DbCmd::OrderWithDetails { id, reply } => {
            let orders: Vec<SelectOrder> = db
                .select(())
                .from(schema.order)
                .r#where(eq(schema.order.id, id))
                .all()
                .map_err(|e| e.to_string())?;
            let details: Vec<OrderDetailResponse> = db
                .select(())
                .from(schema.detail)
                .r#where(eq(schema.detail.order_id, id))
                .all()
                .map_err(|e| e.to_string())?
                .into_iter()
                .map(|row: SelectDetail| OrderDetailResponse::from(row))
                .collect();
            let resp: Vec<SingleOrderWithDetailsResponse> = orders
                .into_iter()
                .map(|order| SingleOrderWithDetailsResponse::new(order, details.clone()))
                .collect();
            reply_json(reply, &resp)
        }
        DbCmd::OrderWithDetailsAndProducts { id, reply } => {
            let orders: Vec<SelectOrder> = db
                .select(())
                .from(schema.order)
                .r#where(eq(schema.order.id, id))
                .all()
                .map_err(|e| e.to_string())?;
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
                .map_err(|e| e.to_string())?;
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
            let resp: Vec<SingleOrderWithDetailsAndProductsResponse> = orders
                .into_iter()
                .map(|order| SingleOrderWithDetailsAndProductsResponse::new(order, details.clone()))
                .collect();
            reply_json(reply, &resp)
        }
        DbCmd::SearchCustomer { term, reply } => {
            let pattern = format!("%{term}%");
            let rows: Vec<SelectCustomer> = db
                .select(())
                .from(schema.customer)
                .r#where(super::ilike_expr(
                    schema.customer.company_name,
                    pattern.as_str(),
                ))
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<CustomerResponse> =
                rows.into_iter().map(CustomerResponse::from).collect();
            reply_json(reply, &resp)
        }
        DbCmd::SearchProduct { term, reply } => {
            let pattern = format!("%{term}%");
            let rows: Vec<SelectProduct> = db
                .select(())
                .from(schema.product)
                .r#where(super::ilike_expr(schema.product.name, pattern.as_str()))
                .all()
                .map_err(|e| e.to_string())?;
            let resp: Vec<ProductResponse> = rows.into_iter().map(ProductResponse::from).collect();
            reply_json(reply, &resp)
        }
    }
}

// ---------------------------------------------------------------------------
// Route handlers — dispatch to worker thread, return pre-serialized JSON
// ---------------------------------------------------------------------------

macro_rules! dispatch {
    ($state:expr, $variant:ident { $($field:ident: $val:expr),* $(,)? }) => {{
        let (tx, rx) = oneshot::channel();
        $state.tx().send(DbCmd::$variant { $($field: $val,)* reply: tx })
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let json_str = rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;
        let body = axum::body::Body::from(json_str);
        Ok(axum::response::Response::builder()
            .header("content-type", "application/json")
            .body(body)
            .unwrap())
    }};
}

#[debug_handler(state = AppState)]
async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        Customers {
            offset: params.offset(),
            limit: params.limit_or(50)
        }
    )
}

#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        CustomerById {
            id: params.user_id(super::SEED_CUSTOMERS as i32)
        }
    )
}

#[debug_handler(state = AppState)]
async fn employees(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        Employees {
            offset: params.offset(),
            limit: params.limit_or(50)
        }
    )
}

#[debug_handler(state = AppState)]
async fn suppliers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        Suppliers {
            offset: params.offset(),
            limit: params.limit_or(50)
        }
    )
}

#[debug_handler(state = AppState)]
async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        SupplierById {
            id: params.user_id(super::SEED_SUPPLIERS as i32)
        }
    )
}

#[debug_handler(state = AppState)]
async fn products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        Products {
            offset: params.offset(),
            limit: params.limit_or(50)
        }
    )
}

#[debug_handler(state = AppState)]
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        EmployeeWithRecipient {
            id: params.user_id(super::SEED_EMPLOYEES as i32)
        }
    )
}

#[debug_handler(state = AppState)]
async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        ProductWithSupplier {
            id: params.user_id(super::SEED_PRODUCTS as i32)
        }
    )
}

#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        OrdersWithDetails {
            offset: params.offset(),
            limit: params.limit_or(50)
        }
    )
}

#[debug_handler(state = AppState)]
async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        OrderWithDetails {
            id: params.user_id(super::SEED_ORDERS as i32)
        }
    )
}

#[debug_handler(state = AppState)]
async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        OrderWithDetailsAndProducts {
            id: params.user_id(super::SEED_ORDERS as i32)
        }
    )
}

#[debug_handler(state = AppState)]
async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        SearchCustomer {
            term: params.term.unwrap_or_default()
        }
    )
}

#[debug_handler(state = AppState)]
async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<axum::response::Response, StatusCode> {
    dispatch!(
        state,
        SearchProduct {
            term: params.term.unwrap_or_default()
        }
    )
}
