//! Deterministic SQLite/Turso seeding for the toasty benchmark target.
//!
//! Toasty owns the query path, but the *data* must be byte-identical to the
//! built-in `drizzle-rs-turso` / `turso-sqlite-*` targets, otherwise the
//! comparison is meaningless. So the schema and the rows come from the same
//! place they do: the `drizzle-rs` SQLite table macros plus `drizzle-seed`,
//! driven by the same seed value, the same row counts and the same
//! `relation(order, detail, 6)` fan-out that `bench/runner/src/load/turso.rs`
//! uses. Same generator + same seed => same rows.
//!
//! Seeding runs once at startup against a file-backed temp database, before the
//! server prints `LISTENING`; toasty then attaches to that same file.

use crate::common::{DynError, fail};
use drizzle::sqlite::prelude::*;
use drizzle_seed::SeedConfig;
use std::path::Path;

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

/// Row counts, matching `bench/runner/src/load/mod.rs`.
pub const SEED_CUSTOMERS: usize = 10_000;
pub const SEED_EMPLOYEES: usize = 200;
pub const SEED_SUPPLIERS: usize = 1_000;
pub const SEED_PRODUCTS: usize = 5_000;
pub const SEED_ORDERS: usize = 50_000;
/// Order details generated per order by `SeedConfig::relation`.
pub const DETAILS_PER_ORDER: usize = 6;

/// Create the Northwind schema and seed it into `path`, then close the
/// connection so toasty can open the finished file.
pub async fn create_and_seed(path: &Path, seed: u64) -> Result<(), DynError> {
    let builder = ::turso::Builder::new_local(
        path.to_str()
            .ok_or_else(|| fail("temp database path is not valid UTF-8"))?,
    )
    .build()
    .await?;
    let conn = builder.connect()?;
    // The whole turso family runs the MVCC engine, so the mode has to be set
    // before anything is written. It is a persistent property of the file, so
    // toasty inherits it when it attaches to the finished database.
    enable_mvcc(&conn).await?;
    let (db, schema) = drizzle::sqlite::turso::Drizzle::new(conn, Schema::new());
    db.create().await?;

    db.conn()
        .execute_batch(
            "CREATE INDEX IF NOT EXISTS idx_emp_recipient ON employees(recipient_id);
             CREATE INDEX IF NOT EXISTS idx_prod_supplier ON products(supplier_id);
             CREATE INDEX IF NOT EXISTS idx_det_order ON order_details(order_id);
             CREATE INDEX IF NOT EXISTS idx_det_product ON order_details(product_id);",
        )
        .await?;

    // Identical config to bench/runner/src/load/turso.rs, including the 500
    // parameter batch cap (batching changes statement shape, not row values).
    let stmts = SeedConfig::sqlite(&schema)
        .seed(seed)
        .max_params(500)
        .count(&schema.customer, SEED_CUSTOMERS)
        .count(&schema.employee, SEED_EMPLOYEES)
        .count(&schema.supplier, SEED_SUPPLIERS)
        .count(&schema.product, SEED_PRODUCTS)
        .count(&schema.order, SEED_ORDERS)
        .relation(&schema.order, &schema.detail, DETAILS_PER_ORDER)
        .generate();
    for stmt in stmts {
        db.execute(stmt).await?;
    }

    // The built-in turso targets analyze after seeding; without it toasty
    // would plan the same queries from different statistics.
    db.conn().execute_batch("ANALYZE;").await?;

    Ok(())
}

/// Journal mode every turso target runs under.
const TURSO_JOURNAL_MODE: &str = "mvcc";

async fn enable_mvcc(conn: &::turso::Connection) -> Result<(), DynError> {
    conn.pragma_update("journal_mode", format!("'{TURSO_JOURNAL_MODE}'"))
        .await?;

    // A pragma that does not apply is reported as success with the old value
    // still in effect, so the read-back is the only real confirmation.
    let mut observed = None;
    conn.pragma_query("journal_mode", |row| {
        if observed.is_none() {
            observed = row.get::<String>(0).ok();
        }
        Ok(())
    })
    .await?;

    match observed.as_deref() {
        Some(mode) if mode.eq_ignore_ascii_case(TURSO_JOURNAL_MODE) => Ok(()),
        other => Err(fail(format!(
            "turso journal_mode is {other:?}, expected {TURSO_JOURNAL_MODE:?}; \
             the target would benchmark a different storage engine than declared"
        ))),
    }
}
