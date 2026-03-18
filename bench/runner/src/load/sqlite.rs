use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
use drizzle_seed::SeedConfig;
use std::sync::{Arc, Mutex};

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

#[derive(Debug, Serialize)]
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

// ---------------------------------------------------------------------------
// App state
// ---------------------------------------------------------------------------

#[derive(Clone)]
struct AppState {
    db: Arc<Mutex<drizzle::sqlite::rusqlite::Drizzle<Schema>>>,
}

// ---------------------------------------------------------------------------
// Server entry point
// ---------------------------------------------------------------------------

pub async fn serve(seed: u64) -> Result<ServerHandle, Fail> {
    let db = tokio::task::spawn_blocking(move || -> Result<_, Fail> {
        let conn = ::rusqlite::Connection::open_in_memory()
            .map_err(|err| Fail::new(Code::RunFail, format!("sqlite open failed: {err}")))?;
        let (db, schema) = drizzle::sqlite::rusqlite::Drizzle::new(conn, Schema::new());

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
            db.execute(stmt)
                .map_err(|err| Fail::new(Code::RunFail, format!("sqlite seed failed: {err}")))?;
        }

        Ok(db)
    })
    .await
    .map_err(|err| Fail::new(Code::RunFail, format!("sqlite setup panicked: {err}")))??;

    let router = Router::new()
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
        .with_state(AppState {
            db: Arc::new(Mutex::new(db)),
        });
    spawn_server(router).await
}

// ---------------------------------------------------------------------------
// Route handlers
// ---------------------------------------------------------------------------

#[debug_handler(state = AppState)]
async fn stats(_: State<AppState>) -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    Json(cpu_usage(&sys))
}

// GET /customers?limit=50&offset=0
#[debug_handler(state = AppState)]
async fn customers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let schema = Schema::new();
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
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
                )| {
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
                },
            )
            .collect(),
    ))
}

// GET /customer-by-id?id=1
#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let schema = Schema::new();
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let target_id = params.user_id(256);
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
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
                )| {
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
                },
            )
            .collect(),
    ))
}

// GET /employees?limit=20&offset=0
#[debug_handler(state = AppState)]
async fn employees(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeResponse>>, StatusCode> {
    let schema = Schema::new();
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
                )| {
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
                },
            )
            .collect(),
    ))
}

// GET /suppliers?limit=50&offset=0
#[debug_handler(state = AppState)]
async fn suppliers(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let schema = Schema::new();
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
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
                )| {
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
                },
            )
            .collect(),
    ))
}

// GET /supplier-by-id?id=1
#[debug_handler(state = AppState)]
async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SupplierResponse>>, StatusCode> {
    let schema = Schema::new();
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let target_id = params.user_id(30);
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(
        rows.into_iter()
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
                )| {
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
                },
            )
            .collect(),
    ))
}

// GET /products?limit=50&offset=0
#[debug_handler(state = AppState)]
async fn products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let schema = Schema::new();
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
                )| {
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
                },
            )
            .collect(),
    ))
}

// GET /employee-with-recipient?id=1
// Left join employees to themselves for the recipient
#[debug_handler(state = AppState)]
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<EmployeeWithRecipientResponse>>, StatusCode> {
    let target_id = params.user_id(50);
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Self-join requires raw SQL since drizzle doesn't support table aliases
    let conn = db.conn();
    let mut stmt = conn
        .prepare_cached(
            "SELECT e.id, e.last_name, e.first_name, e.title, e.title_of_courtesy,
                    e.birth_date, e.hire_date, e.address, e.city, e.postal_code,
                    e.country, e.home_phone, e.extension, e.notes, e.recipient_id,
                    r.last_name, r.first_name
             FROM employees e
             LEFT JOIN employees r ON e.recipient_id = r.id
             WHERE e.id = ?1",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = stmt
        .query_map(::rusqlite::params![target_id], |row| {
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
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}

// GET /product-with-supplier?id=1
// Inner join product with supplier
#[debug_handler(state = AppState)]
async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductWithSupplierResponse>>, StatusCode> {
    let target_id = params.user_id(200);
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let schema = Schema::new();

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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

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
                )| {
                    ProductWithSupplierResponse {
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
                    }
                },
            )
            .collect(),
    ))
}

// GET /orders-with-details?limit=50&offset=0
// Aggregate: orders LEFT JOIN order_details, GROUP BY o.id
#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<OrderWithDetailsResponse>>, StatusCode> {
    let lim = params.limit_or(50) as i64;
    let off = params.offset() as i64;
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let conn = db.conn();
    let mut stmt = conn
        .prepare_cached(
            "SELECT o.id, o.shipped_date, o.ship_name, o.ship_city, o.ship_country,
                    count(d.product_id) as products_count,
                    COALESCE(sum(d.quantity), 0) as quantity_sum,
                    COALESCE(sum(d.quantity * d.unit_price), 0) as total_price
             FROM orders o
             LEFT JOIN order_details d ON o.id = d.order_id
             GROUP BY o.id
             ORDER BY o.id ASC
             LIMIT ?1 OFFSET ?2",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let rows = stmt
        .query_map(::rusqlite::params![lim, off], |row| {
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
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(rows))
}

// GET /order-with-details?id=1
// Single order with aggregated details
#[debug_handler(state = AppState)]
async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsResponse>>, StatusCode> {
    let target_id = params.user_id(500);
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let conn = db.conn();

    // Fetch the order
    let mut order_stmt = conn
        .prepare_cached(
            "SELECT id, order_date, required_date, shipped_date, ship_via, freight,
                    ship_name, ship_city, ship_region, ship_postal_code, ship_country,
                    customer_id, employee_id
             FROM orders WHERE id = ?1",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let orders: Vec<OrderRow> = order_stmt
        .query_map(::rusqlite::params![target_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
                row.get(12)?,
            ))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fetch details for this order
    let mut detail_stmt = conn
        .prepare_cached(
            "SELECT unit_price, quantity, discount, order_id, product_id
             FROM order_details WHERE order_id = ?1",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut results = Vec::new();
    for (
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
    ) in orders
    {
        let details = detail_stmt
            .query_map(::rusqlite::params![id], |row| {
                Ok(OrderDetailResponse {
                    unit_price: row.get(0)?,
                    quantity: row.get(1)?,
                    discount: row.get(2)?,
                    order_id: row.get(3)?,
                    product_id: row.get(4)?,
                })
            })
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        results.push(SingleOrderWithDetailsResponse {
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
        });
    }

    Ok(Json(results))
}

// GET /order-with-details-and-products?id=1
// Single order with details joined to products
#[debug_handler(state = AppState)]
async fn order_with_details_and_products(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<SingleOrderWithDetailsAndProductsResponse>>, StatusCode> {
    let target_id = params.user_id(500);
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let conn = db.conn();

    // Fetch the order
    let mut order_stmt = conn
        .prepare_cached(
            "SELECT id, order_date, required_date, shipped_date, ship_via, freight,
                    ship_name, ship_city, ship_region, ship_postal_code, ship_country,
                    customer_id, employee_id
             FROM orders WHERE id = ?1",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let orders: Vec<OrderRow> = order_stmt
        .query_map(::rusqlite::params![target_id], |row| {
            Ok((
                row.get(0)?,
                row.get(1)?,
                row.get(2)?,
                row.get(3)?,
                row.get(4)?,
                row.get(5)?,
                row.get(6)?,
                row.get(7)?,
                row.get(8)?,
                row.get(9)?,
                row.get(10)?,
                row.get(11)?,
                row.get(12)?,
            ))
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Fetch details with product names
    let mut detail_stmt = conn
        .prepare_cached(
            "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name
             FROM order_details d
             LEFT JOIN products p ON d.product_id = p.id
             WHERE d.order_id = ?1",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let mut results = Vec::new();
    for (
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
    ) in orders
    {
        let details = detail_stmt
            .query_map(::rusqlite::params![id], |row| {
                Ok(OrderDetailProductResponse {
                    unit_price: row.get(0)?,
                    quantity: row.get(1)?,
                    discount: row.get(2)?,
                    order_id: row.get(3)?,
                    product_id: row.get(4)?,
                    product_name: row.get::<_, Option<String>>(5)?.unwrap_or_default(),
                })
            })
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

        results.push(SingleOrderWithDetailsAndProductsResponse {
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
        });
    }

    Ok(Json(results))
}

// GET /search-customer?term=er
#[debug_handler(state = AppState)]
async fn search_customer(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<CustomerResponse>>, StatusCode> {
    let term = params.term.as_deref().unwrap_or("");
    let pattern = format!("%{term}%");
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let conn = db.conn();
    let mut stmt = conn
        .prepare_cached(
            "SELECT id, company_name, contact_name, contact_title, address, city,
                    postal_code, region, country, phone, fax
             FROM customers WHERE company_name LIKE ?1",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = stmt
        .query_map(::rusqlite::params![pattern], |row| {
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
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}

// GET /search-product?term=er
#[debug_handler(state = AppState)]
async fn search_product(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<Vec<ProductResponse>>, StatusCode> {
    let term = params.term.as_deref().unwrap_or("");
    let pattern = format!("%{term}%");
    let db = state
        .db
        .lock()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let conn = db.conn();
    let mut stmt = conn
        .prepare_cached(
            "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order,
                    reorder_level, discontinued, supplier_id
             FROM products WHERE name LIKE ?1",
        )
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let rows = stmt
        .query_map(::rusqlite::params![pattern], |row| {
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
        })
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    Ok(Json(rows))
}
