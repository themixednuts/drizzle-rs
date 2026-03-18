use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
use drizzle_seed::SeedConfig;
use std::sync::Arc;

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

#[derive(Clone)]
struct AppState {
    db: Arc<tokio::sync::Mutex<drizzle::sqlite::turso::Drizzle<Schema>>>,
}

pub async fn serve(seed: u64) -> Result<ServerHandle, Fail> {
    let builder = ::turso::Builder::new_local(":memory:")
        .build()
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("turso build failed: {err}")))?;
    let conn = builder
        .connect()
        .map_err(|err| Fail::new(Code::RunFail, format!("turso connect failed: {err}")))?;
    let (db, schema) = drizzle::sqlite::turso::Drizzle::new(conn, Schema::new());
    db.create()
        .await
        .map_err(|err| Fail::new(Code::RunFail, format!("turso create failed: {err}")))?;

    // Create indexes
    db.conn()
        .execute(
            "CREATE INDEX IF NOT EXISTS idx_emp_recipient ON employees(recipient_id);
             CREATE INDEX IF NOT EXISTS idx_prod_supplier ON products(supplier_id);
             CREATE INDEX IF NOT EXISTS idx_det_order ON order_details(order_id);
             CREATE INDEX IF NOT EXISTS idx_det_product ON order_details(product_id);",
            (),
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
            db: Arc::new(tokio::sync::Mutex::new(db)),
        });
    spawn_server(router).await
}

#[debug_handler(state = AppState)]
async fn stats(_: State<AppState>) -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    Json(cpu_usage(&sys))
}

// For turso, we use drizzle ORM for simple queries and raw SQL for complex ones.
// Turso doesn't support table-qualified refs in some contexts, so raw SQL is simpler.

#[debug_handler(state = AppState)]
async fn customers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn customer_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
    let target_id = params.user_id(10000);
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn employees_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn suppliers_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn supplier_by_id(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
    let target_id = params.user_id(1000);
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn products_handler(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let schema = Schema::new();
    let db = state.db.lock().await;
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
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
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
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

// Complex queries use raw SQL via turso connection
#[debug_handler(state = AppState)]
async fn employee_with_recipient(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target_id = params.user_id(200);
    let db = state.db.lock().await;
    let rows = db.conn().query(
        "SELECT e.id, e.last_name, e.first_name, e.title, e.title_of_courtesy, e.birth_date, e.hire_date, e.address, e.city, e.postal_code, e.country, e.home_phone, e.extension, e.notes, e.recipient_id, r.last_name, r.first_name FROM employees e LEFT JOIN employees r ON e.recipient_id = r.id WHERE e.id = ?1",
        [::turso::Value::from(target_id)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let collected = collect_rows(rows).await?;
    let resp: Vec<EmployeeWithRecipientResponse> = collected
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
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn product_with_supplier(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target_id = params.user_id(5000);
    let db = state.db.lock().await;
    let rows = db.conn().query(
        "SELECT p.id, p.name, p.qt_per_unit, p.unit_price, p.units_in_stock, p.units_on_order, p.reorder_level, p.discontinued, p.supplier_id, s.id, s.company_name, s.contact_name, s.contact_title, s.address, s.city, s.region, s.postal_code, s.country, s.phone FROM products p INNER JOIN suppliers s ON p.supplier_id = s.id WHERE p.id = ?1",
        [::turso::Value::from(target_id)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let collected = collect_rows(rows).await?;
    let resp: Vec<ProductWithSupplierResponse> = collected
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
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn orders_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let lim = params.limit_or(50) as i64;
    let off = params.offset() as i64;
    let db = state.db.lock().await;
    let rows = db.conn().query(
        "SELECT o.id, o.shipped_date, o.ship_name, o.ship_city, o.ship_country, count(d.product_id), COALESCE(sum(d.quantity), 0), COALESCE(sum(d.quantity * d.unit_price), 0) FROM orders o LEFT JOIN order_details d ON o.id = d.order_id GROUP BY o.id ORDER BY o.id LIMIT ?1 OFFSET ?2",
        [::turso::Value::from(lim), ::turso::Value::from(off)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let collected = collect_rows(rows).await?;
    let resp: Vec<OrderWithDetailsResponse> = collected
        .iter()
        .map(|r| OrderWithDetailsResponse {
            id: r.get::<i32>(0).unwrap_or_default(),
            shipped_date: r.get::<i64>(1).ok(),
            ship_name: r.get::<String>(2).unwrap_or_default(),
            ship_city: r.get::<String>(3).unwrap_or_default(),
            ship_country: r.get::<String>(4).unwrap_or_default(),
            products_count: r.get::<i32>(5).unwrap_or_default(),
            quantity_sum: r.get::<f64>(6).unwrap_or_default(),
            total_price: r.get::<f64>(7).unwrap_or_default(),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}

#[debug_handler(state = AppState)]
async fn order_with_details(
    State(state): State<AppState>,
    Query(params): Query<QueryParams>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    let target_id = params.user_id(50000);
    let db = state.db.lock().await;
    let order_rows = db.conn().query(
        "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = ?1",
        [::turso::Value::from(target_id)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let detail_rows = db.conn().query(
        "SELECT unit_price, quantity, discount, order_id, product_id FROM order_details WHERE order_id = ?1",
        [::turso::Value::from(target_id)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let detail_collected = collect_rows(detail_rows).await?;
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
    let order_collected = collect_rows(order_rows).await?;
    let resp: Vec<SingleOrderWithDetailsResponse> = order_collected
        .iter()
        .map(|r| SingleOrderWithDetailsResponse {
            id: r.get::<i32>(0).unwrap_or_default(),
            order_date: r.get::<i64>(1).unwrap_or_default(),
            required_date: r.get::<i64>(2).unwrap_or_default(),
            shipped_date: r.get::<i64>(3).ok(),
            ship_via: r.get::<i32>(4).unwrap_or_default(),
            freight: r.get::<f64>(5).unwrap_or_default(),
            ship_name: r.get::<String>(6).unwrap_or_default(),
            ship_city: r.get::<String>(7).unwrap_or_default(),
            ship_region: r.get::<String>(8).ok(),
            ship_postal_code: r.get::<String>(9).ok(),
            ship_country: r.get::<String>(10).unwrap_or_default(),
            customer_id: r.get::<i32>(11).unwrap_or_default(),
            employee_id: r.get::<i32>(12).unwrap_or_default(),
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
    let target_id = params.user_id(50000);
    let db = state.db.lock().await;
    let order_rows = db.conn().query(
        "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = ?1",
        [::turso::Value::from(target_id)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let detail_rows = db.conn().query(
        "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name FROM order_details d LEFT JOIN products p ON d.product_id = p.id WHERE d.order_id = ?1",
        [::turso::Value::from(target_id)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let detail_collected = collect_rows(detail_rows).await?;
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
    let order_collected = collect_rows(order_rows).await?;
    let resp: Vec<SingleOrderWithDetailsAndProductsResponse> = order_collected
        .iter()
        .map(|r| SingleOrderWithDetailsAndProductsResponse {
            id: r.get::<i32>(0).unwrap_or_default(),
            order_date: r.get::<i64>(1).unwrap_or_default(),
            required_date: r.get::<i64>(2).unwrap_or_default(),
            shipped_date: r.get::<i64>(3).ok(),
            ship_via: r.get::<i32>(4).unwrap_or_default(),
            freight: r.get::<f64>(5).unwrap_or_default(),
            ship_name: r.get::<String>(6).unwrap_or_default(),
            ship_city: r.get::<String>(7).unwrap_or_default(),
            ship_region: r.get::<String>(8).ok(),
            ship_postal_code: r.get::<String>(9).ok(),
            ship_country: r.get::<String>(10).unwrap_or_default(),
            customer_id: r.get::<i32>(11).unwrap_or_default(),
            employee_id: r.get::<i32>(12).unwrap_or_default(),
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
    let db = state.db.lock().await;
    let rows = db.conn().query(
        "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE company_name LIKE ?1",
        [::turso::Value::from(pattern)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let collected = collect_rows(rows).await?;
    let resp: Vec<CustomerResponse> = collected
        .iter()
        .map(|r| CustomerResponse {
            id: r.get::<i32>(0).unwrap_or_default(),
            company_name: r.get::<String>(1).unwrap_or_default(),
            contact_name: r.get::<String>(2).unwrap_or_default(),
            contact_title: r.get::<String>(3).unwrap_or_default(),
            address: r.get::<String>(4).unwrap_or_default(),
            city: r.get::<String>(5).unwrap_or_default(),
            postal_code: r.get::<String>(6).ok(),
            region: r.get::<String>(7).ok(),
            country: r.get::<String>(8).unwrap_or_default(),
            phone: r.get::<String>(9).unwrap_or_default(),
            fax: r.get::<String>(10).ok(),
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
    let db = state.db.lock().await;
    let rows = db.conn().query(
        "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products WHERE name LIKE ?1",
        [::turso::Value::from(pattern)],
    ).await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
    let collected = collect_rows(rows).await?;
    let resp: Vec<ProductResponse> = collected
        .iter()
        .map(|r| ProductResponse {
            id: r.get::<i32>(0).unwrap_or_default(),
            name: r.get::<String>(1).unwrap_or_default(),
            qt_per_unit: r.get::<String>(2).unwrap_or_default(),
            unit_price: r.get::<f64>(3).unwrap_or_default(),
            units_in_stock: r.get::<i32>(4).unwrap_or_default(),
            units_on_order: r.get::<i32>(5).unwrap_or_default(),
            reorder_level: r.get::<i32>(6).unwrap_or_default(),
            discontinued: r.get::<i32>(7).unwrap_or_default(),
            supplier_id: r.get::<i32>(8).unwrap_or_default(),
        })
        .collect();
    Ok(Json(serde_json::to_value(&resp).unwrap()))
}
