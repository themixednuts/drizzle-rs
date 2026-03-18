use super::*;
use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::routing::get;
use axum::{Json, Router, debug_handler};
use chrono::NaiveDate;
use drizzle::postgres::prelude::*;
use std::sync::mpsc;

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

#[derive(Clone)]
struct AppState {
    tx: mpsc::Sender<DbCmd>,
}

pub async fn serve(seed: u64) -> Result<ServerHandle, Fail> {
    let (cmd_tx, cmd_rx) = mpsc::channel::<DbCmd>();
    let (ready_tx, ready_rx) = oneshot::channel::<Result<(), String>>();

    let worker = std::thread::spawn(move || {
        let mut conn = ::postgres::Client::connect(&pg_url(), ::postgres::NoTls)
            .map_err(|err| format!("postgres connect failed: {err}"))?;
        conn.batch_execute(
            "DROP TABLE IF EXISTS order_details;
             DROP TABLE IF EXISTS orders;
             DROP TABLE IF EXISTS products;
             DROP TABLE IF EXISTS suppliers;
             DROP TABLE IF EXISTS employees;
             DROP TABLE IF EXISTS customers;",
        )
        .map_err(|err| format!("postgres drop failed: {err}"))?;

        let (mut db, schema) = drizzle::postgres::sync::Drizzle::new(conn, Schema::new());
        db.create()
            .map_err(|err| format!("postgres create failed: {err}"))?;

        // Seed via drizzle-seed (deterministic from seed value)
        use drizzle_seed::SeedConfig;

        let stmts = SeedConfig::postgres(&schema)
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
                .map_err(|err| format!("postgres seed failed: {err}"))?;
        }

        // Create indexes
        db.conn_mut()
            .batch_execute(
                "CREATE INDEX IF NOT EXISTS idx_employees_recipient ON employees(recipient_id);
             CREATE INDEX IF NOT EXISTS idx_products_supplier ON products(supplier_id);
             CREATE INDEX IF NOT EXISTS idx_details_order ON order_details(order_id);
             CREATE INDEX IF NOT EXISTS idx_details_product ON order_details(product_id);",
            )
            .map_err(|err| format!("postgres create indexes failed: {err}"))?;

        let _ = ready_tx.send(Ok(()));

        // Handle commands using raw SQL for complex queries
        while let Ok(cmd) = cmd_rx.recv() {
            let result = handle_cmd(&mut db, cmd);
            if let Err(msg) = result {
                eprintln!("pg_sync worker error: {msg}");
            }
        }
        Ok(())
    });

    ready_rx
        .await
        .map_err(|_| Fail::new(Code::RunFail, "pg_sync worker dropped before ready"))?
        .map_err(|msg| Fail::new(Code::RunFail, msg))?;

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
        .with_state(AppState { tx: cmd_tx });
    let mut handle = spawn_server(router).await?;
    handle.workers.push(worker);
    Ok(handle)
}

fn handle_cmd(db: &mut drizzle::postgres::sync::Drizzle<Schema>, cmd: DbCmd) -> Result<(), String> {
    match cmd {
        DbCmd::Customers {
            offset,
            limit,
            reply,
        } => {
            let sql = format!(
                "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers ORDER BY id LIMIT {limit} OFFSET {offset}"
            );
            let rows = db.conn_mut().query(&sql, &[]).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::CustomerById { id, reply } => {
            let rows = db.conn_mut().query("SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE id = $1", &[&id]).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::Employees {
            offset,
            limit,
            reply,
        } => {
            let sql = format!(
                "SELECT id, last_name, first_name, title, title_of_courtesy, birth_date, hire_date, address, city, postal_code, country, home_phone, extension, notes, recipient_id FROM employees ORDER BY id LIMIT {limit} OFFSET {offset}"
            );
            let rows = db.conn_mut().query(&sql, &[]).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::Suppliers {
            offset,
            limit,
            reply,
        } => {
            let sql = format!(
                "SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers ORDER BY id LIMIT {limit} OFFSET {offset}"
            );
            let rows = db.conn_mut().query(&sql, &[]).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::SupplierById { id, reply } => {
            let rows = db.conn_mut().query("SELECT id, company_name, contact_name, contact_title, address, city, region, postal_code, country, phone FROM suppliers WHERE id = $1", &[&id]).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::Products {
            offset,
            limit,
            reply,
        } => {
            let sql = format!(
                "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products ORDER BY id LIMIT {limit} OFFSET {offset}"
            );
            let rows = db.conn_mut().query(&sql, &[]).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::EmployeeWithRecipient { id, reply } => {
            let rows = db.conn_mut().query(
                "SELECT e.id, e.last_name, e.first_name, e.title, e.title_of_courtesy, e.birth_date, e.hire_date, e.address, e.city, e.postal_code, e.country, e.home_phone, e.extension, e.notes, e.recipient_id, r.last_name, r.first_name FROM employees e LEFT JOIN employees r ON e.recipient_id = r.id WHERE e.id = $1",
                &[&id],
            ).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::ProductWithSupplier { id, reply } => {
            let rows = db.conn_mut().query(
                "SELECT p.id, p.name, p.qt_per_unit, p.unit_price, p.units_in_stock, p.units_on_order, p.reorder_level, p.discontinued, p.supplier_id, s.id, s.company_name, s.contact_name, s.contact_title, s.address, s.city, s.region, s.postal_code, s.country, s.phone FROM products p INNER JOIN suppliers s ON p.supplier_id = s.id WHERE p.id = $1",
                &[&id],
            ).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::OrdersWithDetails {
            offset,
            limit,
            reply,
        } => {
            let sql = format!(
                "SELECT o.id, o.shipped_date, o.ship_name, o.ship_city, o.ship_country, \
                 count(d.product_id), COALESCE(sum(d.quantity), 0), COALESCE(sum(d.quantity::float8 * d.unit_price), 0) \
                 FROM orders o LEFT JOIN order_details d ON o.id = d.order_id \
                 GROUP BY o.id ORDER BY o.id LIMIT {limit} OFFSET {offset}"
            );
            let rows = db.conn_mut().query(&sql, &[]).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::OrderWithDetails { id, reply } => {
            let order_rows = db.conn_mut().query(
                "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = $1",
                &[&id],
            ).map_err(|e| e.to_string())?;
            let detail_rows = db.conn_mut().query(
                "SELECT unit_price, quantity, discount, order_id, product_id FROM order_details WHERE order_id = $1",
                &[&id],
            ).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::OrderWithDetailsAndProducts { id, reply } => {
            let order_rows = db.conn_mut().query(
                "SELECT id, order_date, required_date, shipped_date, ship_via, freight, ship_name, ship_city, ship_region, ship_postal_code, ship_country, customer_id, employee_id FROM orders WHERE id = $1",
                &[&id],
            ).map_err(|e| e.to_string())?;
            let detail_rows = db.conn_mut().query(
                "SELECT d.unit_price, d.quantity, d.discount, d.order_id, d.product_id, p.name FROM order_details d LEFT JOIN products p ON d.product_id = p.id WHERE d.order_id = $1",
                &[&id],
            ).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::SearchCustomer { term, reply } => {
            let pattern = format!("%{term}%");
            let rows = db.conn_mut().query(
                "SELECT id, company_name, contact_name, contact_title, address, city, postal_code, region, country, phone, fax FROM customers WHERE company_name ILIKE $1",
                &[&pattern],
            ).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
        DbCmd::SearchProduct { term, reply } => {
            let pattern = format!("%{term}%");
            let rows = db.conn_mut().query(
                "SELECT id, name, qt_per_unit, unit_price, units_in_stock, units_on_order, reorder_level, discontinued, supplier_id FROM products WHERE name ILIKE $1",
                &[&pattern],
            ).map_err(|e| e.to_string())?;
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
            let _ = reply
                .send(serde_json::to_string(&resp).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR));
            Ok(())
        }
    }
}

// ---------------------------------------------------------------------------
// Route handlers — dispatch to worker thread, return pre-serialized JSON
// ---------------------------------------------------------------------------

#[debug_handler(state = AppState)]
async fn stats(_: State<AppState>) -> Json<Vec<f64>> {
    let mut sys = System::new_all();
    sys.refresh_cpu_usage();
    Json(cpu_usage(&sys))
}

macro_rules! dispatch {
    ($state:expr, $variant:ident { $($field:ident: $val:expr),* $(,)? }) => {{
        let (tx, rx) = oneshot::channel();
        $state.tx.send(DbCmd::$variant { $($field: $val,)* reply: tx })
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        let json_str = rx.await.map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)??;
        let body = axum::body::boxed(axum::body::Full::from(json_str));
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
            id: params.user_id(10000)
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
            id: params.user_id(1000)
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
            id: params.user_id(200)
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
            id: params.user_id(5000)
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
            id: params.user_id(50000)
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
            id: params.user_id(50000)
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
