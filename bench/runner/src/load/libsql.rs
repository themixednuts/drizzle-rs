//! libsql benchmark targets.
//!
//! Structurally this is the turso module — an async embedded SQLite driver
//! behind a connection pool — but the SQL shapes are the rusqlite module's,
//! because libsql *is* SQLite: it plans `GROUP BY` aggregates and joins, so
//! `/orders-with-details` is one aggregate query rather than turso's order page
//! plus range fetch folded in Rust.
//!
//! The whole module is behind the `libsql` cargo feature. libsql has a history
//! of crashing the benchmark process on Windows and macOS, so the family is
//! Linux-only in CI and the feature is off by default; a default build of
//! `bench-runner` never links libsql at all.

use super::pool::{AsyncResourcePool, PooledResource};
use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use drizzle::core::expr::{alias, coalesce, count, eq, like, sum};
use drizzle::sqlite::prelude::*;
use drizzle_seed::SeedConfig;
use std::collections::BTreeMap;

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

type LibsqlDb = drizzle::sqlite::libsql::Drizzle<Schema>;

// ---------------------------------------------------------------------------
// Response types (camelCase JSON) — identical to the rusqlite module's
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

// ---------------------------------------------------------------------------
// Row tuples decoded by the select builder
// ---------------------------------------------------------------------------

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

// SQLite's `sum()` over an integer column yields an integer; the JSON contract
// reports it as a number either way.
type OrderAggregateRow = (i32, Option<i64>, String, String, String, i32, i64, f64);

type DetailRow = (f64, i32, f64, i32, i32);

type DetailWithProductRow = (f64, i32, f64, i32, i32, Option<String>);

// ---------------------------------------------------------------------------
// Raw SQL, shared by both raw baselines
// ---------------------------------------------------------------------------

const SQL_CUSTOMERS: &str = "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers ORDER BY id LIMIT ?1 OFFSET ?2";
const SQL_CUSTOMER_BY_ID: &str = "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE id = ?1";
const SQL_EMPLOYEES: &str = "SELECT id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id FROM employees ORDER BY id LIMIT ?1 OFFSET ?2";
const SQL_SUPPLIERS: &str = "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers ORDER BY id LIMIT ?1 OFFSET ?2";
const SQL_SUPPLIER_BY_ID: &str = "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers WHERE id = ?1";
const SQL_PRODUCTS: &str = "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products ORDER BY id LIMIT ?1 OFFSET ?2";
const SQL_EMPLOYEE_WITH_RECIPIENT: &str = "SELECT e.id, e.last_name, e.first_name, e.title, e.title_of_courtesy, e.birth_date, e.hire_date, e.address, e.city, e.postal_code, e.country, e.home_phone, e.extension, e.notes, e.recipient_id, r.last_name, r.first_name FROM employees e LEFT JOIN employees r ON e.recipient_id = r.id WHERE e.id = ?1";
const SQL_PRODUCT_WITH_SUPPLIER: &str = "SELECT p.id, p.name, p.qt_per_unit, p.unit_price, p.units_in_stock, p.units_on_order, p.reorder_level, p.discontinued, p.supplier_id, s.id, s.company_name, s.contact_name, s.contact_title, s.address, s.city, s.region, s.postal_code, s.country, s.phone FROM products p INNER JOIN suppliers s ON p.supplier_id = s.id WHERE p.id = ?1";
// `0.0` rather than `0`: an empty group would otherwise make COALESCE return an
// INTEGER, and libsql's `FromValue for f64` does not accept `Value::Integer`.
const SQL_ORDERS_WITH_DETAILS: &str = "SELECT o.id, o.shipped_date, o.ship_name, o.ship_city, o.ship_country, count(d.product_id) AS products_count, COALESCE(sum(d.quantity), 0) AS quantity_sum, COALESCE(sum(d.quantity * d.unit_price), 0.0) AS total_price FROM orders o LEFT JOIN order_details d ON o.id = d.order_id GROUP BY o.id ORDER BY o.id ASC LIMIT ?1 OFFSET ?2";
const SQL_ORDER_BY_ID: &str = "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = ?1";
const SQL_ORDER_DETAILS: &str = "SELECT unit_price, quantity, discount, order_id, product_id FROM order_details WHERE order_id = ?1";
const SQL_ORDER_DETAIL_PRODUCTS: &str = "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name FROM order_details d LEFT JOIN products p ON d.product_id = p.id WHERE d.order_id = ?1";
const SQL_SEARCH_CUSTOMER: &str = "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE company_name LIKE ?1";
const SQL_SEARCH_PRODUCT: &str = "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products WHERE name LIKE ?1";

/// Every statement the raw baselines issue, in the order they are compiled.
const RAW_SQL: [&str; 14] = [
    SQL_CUSTOMERS,
    SQL_CUSTOMER_BY_ID,
    SQL_EMPLOYEES,
    SQL_SUPPLIERS,
    SQL_SUPPLIER_BY_ID,
    SQL_PRODUCTS,
    SQL_EMPLOYEE_WITH_RECIPIENT,
    SQL_PRODUCT_WITH_SUPPLIER,
    SQL_ORDERS_WITH_DETAILS,
    SQL_ORDER_BY_ID,
    SQL_ORDER_DETAILS,
    SQL_ORDER_DETAIL_PRODUCTS,
    SQL_SEARCH_CUSTOMER,
    SQL_SEARCH_PRODUCT,
];

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

/// One pooled connection: the drizzle handle plus, for the prepared baseline,
/// its precompiled statements.
///
/// libsql exposes no `prepare_cached` (unlike turso), so "prepared" has to mean
/// statements the target compiled once and kept. They are per connection
/// because a `libsql::Statement` is bound to the connection that made it.
struct LibsqlResource {
    db: LibsqlDb,
    statements: Option<BTreeMap<&'static str, ::libsql::Statement>>,
}

#[derive(Clone)]
struct AppState {
    pool: AsyncResourcePool<LibsqlResource>,
    mode: LibsqlMode,
    // Table ZSTs are `Copy`; the schema handle is built once at startup.
    schema: Schema,
}

#[derive(Clone, Copy, Eq, PartialEq)]
enum LibsqlMode {
    /// drizzle-rs typed select builder.
    Drizzle,
    RawPrepared,
    RawUnprepared,
}

impl LibsqlMode {
    fn is_raw(self) -> bool {
        matches!(self, Self::RawPrepared | Self::RawUnprepared)
    }
}

async fn acquire(state: &AppState) -> Result<PooledResource<LibsqlResource>, StatusCode> {
    state
        .pool
        .acquire()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)
}

// ---------------------------------------------------------------------------
// Row helpers
// ---------------------------------------------------------------------------

/// Run one raw query on a pooled connection and decode its rows.
///
/// The prepared baseline reuses the connection's compiled statement; the
/// unprepared baseline goes through `Connection::query`, which compiles the SQL
/// afresh on every call.
///
/// `map` is applied inside the cursor loop, not to a collected `Vec<Row>`:
/// libsql's local `Row` is a view over the statement cursor rather than a
/// materialized row, so a handle retained past the next `next()` decodes every
/// column as an error. turso's `Row` owns its values, which is why the turso
/// module can collect first and decode later.
async fn raw_query<T>(
    res: &LibsqlResource,
    sql: &'static str,
    params: Vec<::libsql::Value>,
    map: impl Fn(&::libsql::Row) -> T,
) -> Result<Vec<T>, StatusCode> {
    let mut rows = match res.statements.as_ref().and_then(|cache| cache.get(sql)) {
        Some(stmt) => {
            // A `Statement` owns one `sqlite3_stmt` that every execution
            // reuses. Without an explicit reset the previous execution's
            // bindings survive and `query` silently replays them — page 2 of a
            // paginated route comes back as page 1. turso's `prepare_cached`
            // resets internally; libsql leaves it to the caller. The pool hands
            // out a connection exclusively, so no other request can be midway
            // through this statement.
            stmt.reset();
            stmt.query(params).await
        }
        None => res.db.conn().query(sql, params).await,
    }
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut out = Vec::new();
    while let Some(row) = rows
        .next()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
    {
        out.push(map(&row));
    }
    Ok(out)
}

fn customer_from_row(row: &::libsql::Row) -> CustomerResponse {
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

fn customer_from_tuple(row: CustomerRow) -> CustomerResponse {
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

fn employee_from_row(row: &::libsql::Row) -> EmployeeResponse {
    EmployeeResponse {
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
    }
}

fn employee_from_tuple(row: EmployeeRow) -> EmployeeResponse {
    let (
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
    ) = row;
    EmployeeResponse {
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
    }
}

fn supplier_from_row(row: &::libsql::Row, base: i32) -> SupplierResponse {
    SupplierResponse {
        id: row.get::<i32>(base).unwrap_or_default(),
        company_name: row.get::<String>(base + 1).unwrap_or_default(),
        contact_name: row.get::<String>(base + 2).unwrap_or_default(),
        contact_title: row.get::<String>(base + 3).unwrap_or_default(),
        address: row.get::<String>(base + 4).unwrap_or_default(),
        city: row.get::<String>(base + 5).unwrap_or_default(),
        region: row.get::<String>(base + 6).ok(),
        postal_code: row.get::<String>(base + 7).unwrap_or_default(),
        country: row.get::<String>(base + 8).unwrap_or_default(),
        phone: row.get::<String>(base + 9).unwrap_or_default(),
    }
}

fn supplier_from_tuple(row: SupplierRow) -> SupplierResponse {
    let (
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
    ) = row;
    SupplierResponse {
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
    }
}

fn product_from_row(row: &::libsql::Row) -> ProductResponse {
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

fn product_from_tuple(row: ProductRow) -> ProductResponse {
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

fn order_envelope<D>(row: OrderRow, details: Vec<D>) -> SingleOrder<D> {
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
    ) = row;
    SingleOrder {
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

/// Generic envelope shared by both `/order-with-details*` responses; the two
/// concrete response types stay separate so their JSON field order matches the
/// other SQLite-family targets.
struct SingleOrder<D> {
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

impl From<SingleOrder<OrderDetailResponse>> for SingleOrderWithDetailsResponse {
    fn from(row: SingleOrder<OrderDetailResponse>) -> Self {
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

impl From<SingleOrder<OrderDetailProductResponse>> for SingleOrderWithDetailsAndProductsResponse {
    fn from(row: SingleOrder<OrderDetailProductResponse>) -> Self {
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

fn order_row_from_libsql(row: &::libsql::Row) -> OrderRow {
    (
        row.get::<i32>(0).unwrap_or_default(),
        row.get::<i64>(1).unwrap_or_default(),
        row.get::<i64>(2).unwrap_or_default(),
        row.get::<i64>(3).ok(),
        row.get::<i32>(4).unwrap_or_default(),
        row.get::<f64>(5).unwrap_or_default(),
        row.get::<String>(6).unwrap_or_default(),
        row.get::<String>(7).unwrap_or_default(),
        row.get::<String>(8).ok(),
        row.get::<String>(9).ok(),
        row.get::<String>(10).unwrap_or_default(),
        row.get::<i32>(11).unwrap_or_default(),
        row.get::<i32>(12).unwrap_or_default(),
    )
}

fn value_i64(value: i64) -> ::libsql::Value {
    ::libsql::Value::from(value)
}

// ---------------------------------------------------------------------------
// Server entry points
// ---------------------------------------------------------------------------

pub async fn serve(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, LibsqlMode::Drizzle).await
}

pub async fn serve_raw_prepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, LibsqlMode::RawPrepared).await
}

pub async fn serve_raw_unprepared(seed: u64) -> Result<ServerHandle, Fail> {
    serve_with_mode(seed, LibsqlMode::RawUnprepared).await
}

/// Journal mode every libsql target runs under, matching the rusqlite family.
const LIBSQL_JOURNAL_MODE: &str = "wal";

/// Set WAL and read the mode back.
///
/// libsql has no `pragma_update`/`pragma_query` (turso does), so the pragma is
/// issued as SQL. A pragma that does not apply reports success with the old
/// value still in effect, so the read-back is the only real confirmation.
async fn enable_wal(conn: &::libsql::Connection) -> Result<(), Fail> {
    let mut rows = conn
        .query("PRAGMA journal_mode = WAL", ())
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("libsql journal_mode failed: {err}")))?;
    let observed = rows
        .next()
        .await
        .map_err(|err| {
            Fail::new(
                Code::RunFail,
                format!("libsql journal_mode read-back failed: {err}"),
            )
        })?
        .and_then(|row| row.get::<String>(0).ok());

    match observed.as_deref() {
        Some(mode) if mode.eq_ignore_ascii_case(LIBSQL_JOURNAL_MODE) => Ok(()),
        other => Err(Fail::new(
            Code::RunFail,
            format!(
                "libsql journal_mode is {other:?}, expected {LIBSQL_JOURNAL_MODE:?}; \
                 the target would benchmark a different storage engine than declared"
            ),
        )),
    }
}

async fn open_connection(
    database: &::libsql::Database,
    mode: LibsqlMode,
) -> Result<LibsqlResource, Fail> {
    let conn = database
        .connect()
        .map_err(|err| Fail::new(Code::RunFail, format!("libsql connect failed: {err}")))?;
    conn.execute("PRAGMA temp_store = MEMORY", ())
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("libsql pool pragmas failed: {err}")))?;

    // Compile every statement now, so the first measured request of each route
    // does not pay for its own prepare.
    let statements = if mode == LibsqlMode::RawPrepared {
        let mut map = BTreeMap::new();
        for sql in RAW_SQL {
            let stmt = conn
                .prepare(sql)
                .await
                .map_err(|err| Fail::new(Code::RunFail, format!("libsql prepare failed: {err}")))?;
            map.insert(sql, stmt);
        }
        Some(map)
    } else {
        None
    };

    let (db, _) = drizzle::sqlite::libsql::Drizzle::new(conn, Schema::new());
    Ok(LibsqlResource { db, statements })
}

async fn serve_with_mode(seed: u64, mode: LibsqlMode) -> Result<ServerHandle, Fail> {
    // File-backed, like the rest of the SQLite family: an in-memory database
    // would let libsql skip the storage costs its counterparts pay, and the
    // families are ranked against each other.
    let temp_dir = tempfile::Builder::new()
        .prefix("drizzle-bench-libsql-")
        .tempdir()
        .map_err(|err| Fail::new(Code::RunFail, format!("libsql tempdir failed: {err}")))?;
    let db_path = temp_dir.path().join("bench.sqlite3");

    let database = ::libsql::Builder::new_local(&db_path)
        .build()
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("libsql build failed: {err}")))?;
    let conn = database
        .connect()
        .map_err(|err| Fail::new(Code::RunFail, format!("libsql connect failed: {err}")))?;
    enable_wal(&conn).await?;
    let (db, schema) = drizzle::sqlite::libsql::Drizzle::new(conn, Schema::new());
    db.create()
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("libsql create failed: {err}")))?;

    db.conn()
        .execute_batch(
            "CREATE INDEX IF NOT EXISTS recepient_idx ON employees(recipient_id);
             CREATE INDEX IF NOT EXISTS supplier_idx ON products(supplier_id);
             CREATE INDEX IF NOT EXISTS order_id_idx ON order_details(order_id);
             CREATE INDEX IF NOT EXISTS product_id_idx ON order_details(product_id);",
        )
        .await
        .map_err(|err| {
            Fail::new(
                Code::RunFail,
                format!("libsql create indexes failed: {err}"),
            )
        })?;

    // Same generator, seed and row counts as every other SQLite-family target,
    // so the data is identical.
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
        db.execute(stmt)
            .await
            .map_err(|err| Fail::new(Code::RunFail, format!("libsql seed failed: {err}")))?;
    }

    // Statistics, so the planner picks the indexes just created rather than
    // guessing from rowid counts.
    db.conn()
        .execute_batch("ANALYZE;")
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("libsql analyze failed: {err}")))?;
    drop(db);

    let pool_size = configured_pool_size(SQLITE_POOL_SIZE);
    let mut resources = Vec::with_capacity(pool_size);
    for _ in 0..pool_size {
        resources.push(open_connection(&database, mode).await?);
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
        .with_state(AppState {
            pool: AsyncResourcePool::new(resources),
            mode,
            schema: Schema::new(),
        });
    let mut handle = spawn_server(router).await?;
    handle.temp_dirs.push(temp_dir);
    Ok(handle)
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

#[debug_handler(state = AppState)]
async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_CUSTOMERS,
                vec![value_i64(limit as i64), value_i64(offset as i64)],
                customer_from_row,
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<CustomerRow> = res
        .db
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
        .limit(limit)
        .offset(offset)
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(customer_from_tuple).collect()))
}

#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_CUSTOMERS as i32);
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_CUSTOMER_BY_ID,
                vec![value_i64(i64::from(id))],
                customer_from_row,
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<CustomerRow> = res
        .db
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
        .r#where(eq(schema.customer.id, id))
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(customer_from_tuple).collect()))
}

#[debug_handler(state = AppState)]
async fn employees(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_EMPLOYEES,
                vec![value_i64(limit as i64), value_i64(offset as i64)],
                employee_from_row,
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<EmployeeRow> = res
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
        ))
        .from(schema.employee)
        .order_by([asc(schema.employee.id)])
        .limit(limit)
        .offset(offset)
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(employee_from_tuple).collect()))
}

#[debug_handler(state = AppState)]
async fn suppliers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_SUPPLIERS,
                vec![value_i64(limit as i64), value_i64(offset as i64)],
                |row| supplier_from_row(row, 0),
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<SupplierRow> = res
        .db
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
        .limit(limit)
        .offset(offset)
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(supplier_from_tuple).collect()))
}

#[debug_handler(state = AppState)]
async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_SUPPLIERS as i32);
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_SUPPLIER_BY_ID,
                vec![value_i64(i64::from(id))],
                |row| supplier_from_row(row, 0),
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<SupplierRow> = res
        .db
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
        .r#where(eq(schema.supplier.id, id))
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(supplier_from_tuple).collect()))
}

#[debug_handler(state = AppState)]
async fn products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let (limit, offset) = (params.limit_or(50), params.offset());
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_PRODUCTS,
                vec![value_i64(limit as i64), value_i64(offset as i64)],
                product_from_row,
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<ProductRow> = res
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
        ))
        .from(schema.product)
        .order_by([asc(schema.product.id)])
        .limit(limit)
        .offset(offset)
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(product_from_tuple).collect()))
}

#[debug_handler(state = AppState)]
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeWithRecipientResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_EMPLOYEES as i32);
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_EMPLOYEE_WITH_RECIPIENT,
                vec![value_i64(i64::from(id))],
                |row| {
                    let base = employee_from_row(row);
                    EmployeeWithRecipientResponse {
                        id: base.id,
                        last_name: base.last_name,
                        first_name: base.first_name,
                        title: base.title,
                        title_of_courtesy: base.title_of_courtesy,
                        birth_date: base.birth_date,
                        hire_date: base.hire_date,
                        address: base.address,
                        city: base.city,
                        postal_code: base.postal_code,
                        country: base.country,
                        home_phone: base.home_phone,
                        extension: base.extension,
                        notes: base.notes,
                        recipient_id: base.recipient_id,
                        recipient_last_name: row.get::<String>(15).ok(),
                        recipient_first_name: row.get::<String>(16).ok(),
                    }
                },
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let recipient = Employee::alias::<super::RecipientAlias>();
    let rows: Vec<EmployeeWithRecipientRow> = res
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
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_PRODUCT_WITH_SUPPLIER,
                vec![value_i64(i64::from(id))],
                |row| {
                    let product = product_from_row(row);
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
                        supplier: supplier_from_row(row, 9),
                    }
                },
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<ProductWithSupplierRow> = res
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
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_ORDERS_WITH_DETAILS,
                vec![value_i64(limit as i64), value_i64(offset as i64)],
                |row| OrderWithDetailsResponse {
                    id: row.get::<i32>(0).unwrap_or_default(),
                    shipped_date: row.get::<i64>(1).ok(),
                    ship_name: row.get::<String>(2).unwrap_or_default(),
                    ship_city: row.get::<String>(3).unwrap_or_default(),
                    ship_country: row.get::<String>(4).unwrap_or_default(),
                    products_count: row.get::<i32>(5).unwrap_or_default(),
                    quantity_sum: row.get::<i64>(6).unwrap_or_default() as f64,
                    total_price: row.get::<f64>(7).unwrap_or_default(),
                },
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<OrderAggregateRow> = res
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
                coalesce(sum(schema.detail.quantity * schema.detail.unit_price), 0),
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
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        let orders = raw_query(
            &res,
            SQL_ORDER_BY_ID,
            vec![value_i64(i64::from(id))],
            order_row_from_libsql,
        )
        .await?;
        let details = raw_query(
            &res,
            SQL_ORDER_DETAILS,
            vec![value_i64(i64::from(id))],
            |row| OrderDetailResponse {
                unit_price: row.get::<f64>(0).unwrap_or_default(),
                quantity: row.get::<i32>(1).unwrap_or_default(),
                discount: row.get::<f64>(2).unwrap_or_default(),
                order_id: row.get::<i32>(3).unwrap_or_default(),
                product_id: row.get::<i32>(4).unwrap_or_default(),
            },
        )
        .await?;
        return Ok(Json(
            orders
                .into_iter()
                .map(|row| order_envelope(row, details.clone()).into())
                .collect(),
        ));
    }

    let schema = state.schema;
    let orders: Vec<OrderRow> = res
        .db
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
        .r#where(eq(schema.order.id, id))
        .all()
        .await
        .map_err(db_err)?;
    let details: Vec<DetailRow> = res
        .db
        .select((
            schema.detail.unit_price,
            schema.detail.quantity,
            schema.detail.discount,
            schema.detail.order_id,
            schema.detail.product_id,
        ))
        .from(schema.detail)
        .r#where(eq(schema.detail.order_id, id))
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
            .map(|order| order_envelope(order, details.clone()).into())
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsAndProductsResponse>>, StatusCode> {
    let id = params.user_id(super::SEED_ORDERS as i32);
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        let orders = raw_query(
            &res,
            SQL_ORDER_BY_ID,
            vec![value_i64(i64::from(id))],
            order_row_from_libsql,
        )
        .await?;
        let details = raw_query(
            &res,
            SQL_ORDER_DETAIL_PRODUCTS,
            vec![value_i64(i64::from(id))],
            |row| OrderDetailProductResponse {
                unit_price: row.get::<f64>(0).unwrap_or_default(),
                quantity: row.get::<i32>(1).unwrap_or_default(),
                discount: row.get::<f64>(2).unwrap_or_default(),
                order_id: row.get::<i32>(3).unwrap_or_default(),
                product_id: row.get::<i32>(4).unwrap_or_default(),
                product_name: row.get::<String>(5).unwrap_or_default(),
            },
        )
        .await?;
        return Ok(Json(
            orders
                .into_iter()
                .map(|row| order_envelope(row, details.clone()).into())
                .collect(),
        ));
    }

    let schema = state.schema;
    let orders: Vec<OrderRow> = res
        .db
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
        .r#where(eq(schema.order.id, id))
        .all()
        .await
        .map_err(db_err)?;
    let rows: Vec<DetailWithProductRow> = res
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
            .map(|order| order_envelope(order, details.clone()).into())
            .collect(),
    ))
}

#[debug_handler(state = AppState)]
async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or(""));
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_SEARCH_CUSTOMER,
                vec![::libsql::Value::from(pattern.as_str())],
                customer_from_row,
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<CustomerRow> = res
        .db
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
    Ok(Json(rows.into_iter().map(customer_from_tuple).collect()))
}

#[debug_handler(state = AppState)]
async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let pattern = format!("%{}%", params.term.as_deref().unwrap_or(""));
    let res = acquire(&state).await?;
    if state.mode.is_raw() {
        return Ok(Json(
            raw_query(
                &res,
                SQL_SEARCH_PRODUCT,
                vec![::libsql::Value::from(pattern.as_str())],
                product_from_row,
            )
            .await?,
        ));
    }

    let schema = state.schema;
    let rows: Vec<ProductRow> = res
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
        ))
        .from(schema.product)
        .r#where(like(schema.product.name, pattern.as_str()))
        .all()
        .await
        .map_err(db_err)?;
    Ok(Json(rows.into_iter().map(product_from_tuple).collect()))
}
