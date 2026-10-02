//! HTTP contract routes implemented as SpacetimeDB procedures.
//!
//! This is the `spacetime-module-rs` benchmark target's application logic:
//! each contract route is exactly one procedure that reads the tables inside a
//! database transaction and returns the route's finished JSON response body.
//! The HTTP process in front of it (`spacetime-native-rs --module-procedures`)
//! only maps query parameters to procedure arguments and copies the returned
//! body through, so every lookup, join, aggregate, filter and serialization
//! step runs inside the database.
//!
//! Semantics match the other SpacetimeDB targets exactly (see
//! `bench/targets/spacetime-native-rs/src/main.rs`): pages are id ranges
//! `offset+1 ..= offset+limit`, single-row routes wrap the requested id into
//! the seeded id space, and empty strings / zeroes stand in for NULL.
//!
//! Procedures return `String` rather than typed SATS rows so the response is
//! the contract's camelCase JSON with `null` for absent values; a typed return
//! would arrive as SATS-JSON (`{"some": ..}` options), which the forwarder
//! would then have to rewrite.

use crate::{
    Customer, Employee, OrderDetail, Product, SEED_CUSTOMERS, SEED_EMPLOYEES, SEED_ORDERS,
    SEED_PRODUCTS, SEED_SUPPLIERS, Supplier, customers, employees, order_details, orders, products,
    suppliers,
};
use serde::Serialize;
use spacetimedb::{ProcedureContext, Table, TxContext, procedure};

#[derive(Serialize)]
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

#[derive(Serialize)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct EmployeeWithRecipientResponse {
    #[serde(flatten)]
    employee: EmployeeResponse,
    recipient_last_name: Option<String>,
    recipient_first_name: Option<String>,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductWithSupplierResponse {
    #[serde(flatten)]
    product: ProductResponse,
    supplier: SupplierResponse,
}

#[derive(Serialize)]
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct OrderDetailResponse {
    unit_price: f64,
    quantity: i32,
    discount: f64,
    order_id: i32,
    product_id: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    product_name: Option<String>,
}

#[derive(Serialize)]
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

fn as_i32(value: u32) -> i32 {
    value.try_into().unwrap_or(i32::MAX)
}

fn opt_string(value: &str) -> Option<String> {
    (!value.is_empty()).then(|| value.to_string())
}

fn opt_i32(value: i32) -> Option<i32> {
    (value != 0).then_some(value)
}

fn opt_i64(value: i64) -> Option<i64> {
    (value != 0).then_some(value)
}

/// Ids of the page `offset+1 ..= offset+limit`, matching the id-range
/// pagination every SpacetimeDB target uses (seeded ids are dense from 1).
fn page_ids(offset: u32, limit: u32) -> core::range::RangeInclusive<u32> {
    core::range::RangeInclusive {
        start: offset.saturating_add(1),
        last: offset.saturating_add(limit),
    }
}

/// Wrap a requested id into the seeded id space `1..=n`.
fn wrap_id(id: i32, n: u32) -> u32 {
    (id.wrapping_sub(1).rem_euclid(n as i32) + 1) as u32
}

fn json<T: Serialize>(value: &T) -> String {
    serde_json::to_string(value).expect("response serialization cannot fail")
}

fn customer_response(row: Customer) -> CustomerResponse {
    CustomerResponse {
        id: as_i32(row.id),
        postal_code: opt_string(&row.postal_code),
        region: opt_string(&row.region),
        fax: opt_string(&row.fax),
        company_name: row.company_name,
        contact_name: row.contact_name,
        contact_title: row.contact_title,
        address: row.address,
        city: row.city,
        country: row.country,
        phone: row.phone,
    }
}

fn employee_response(row: Employee) -> EmployeeResponse {
    EmployeeResponse {
        id: as_i32(row.id),
        first_name: opt_string(&row.first_name),
        recipient_id: opt_i32(row.recipient_id),
        last_name: row.last_name,
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
    }
}

fn supplier_response(row: Supplier) -> SupplierResponse {
    SupplierResponse {
        id: as_i32(row.id),
        region: opt_string(&row.region),
        company_name: row.company_name,
        contact_name: row.contact_name,
        contact_title: row.contact_title,
        address: row.address,
        city: row.city,
        postal_code: row.postal_code,
        country: row.country,
        phone: row.phone,
    }
}

fn product_response(row: Product) -> ProductResponse {
    ProductResponse {
        id: as_i32(row.id),
        name: row.name,
        qt_per_unit: row.qt_per_unit,
        unit_price: row.unit_price,
        units_in_stock: row.units_in_stock,
        units_on_order: row.units_on_order,
        reorder_level: row.reorder_level,
        discontinued: row.discontinued,
        supplier_id: as_i32(row.supplier_id),
    }
}

/// An order's detail rows in id order, through the `order_id` btree index.
fn details_of(tx: &TxContext, order_id: u32) -> Vec<OrderDetail> {
    let mut rows = tx
        .db
        .order_details()
        .order_id()
        .filter(order_id)
        .collect::<Vec<_>>();
    rows.sort_unstable_by_key(|row| row.id);
    rows
}

fn detail_response(row: &OrderDetail, product_name: Option<String>) -> OrderDetailResponse {
    OrderDetailResponse {
        unit_price: row.unit_price,
        quantity: row.quantity,
        discount: row.discount,
        order_id: as_i32(row.order_id),
        product_id: as_i32(row.product_id),
        product_name,
    }
}

fn contains_term(value: &str, term_lower: &str) -> bool {
    term_lower.is_empty() || value.to_ascii_lowercase().contains(term_lower)
}

/// `GET /customers?limit&offset`
#[procedure]
pub fn route_customers(ctx: &mut ProcedureContext, offset: u32, limit: u32) -> String {
    ctx.with_tx(|tx| {
        let rows = page_ids(offset, limit)
            .into_iter()
            .filter_map(|id| tx.db.customers().id().find(id))
            .map(customer_response)
            .collect::<Vec<_>>();
        json(&rows)
    })
}

/// `GET /customer-by-id?id`
#[procedure]
pub fn route_customer_by_id(ctx: &mut ProcedureContext, id: i32) -> String {
    ctx.with_tx(|tx| {
        let id = wrap_id(id, SEED_CUSTOMERS);
        let rows = tx
            .db
            .customers()
            .id()
            .find(id)
            .map(customer_response)
            .into_iter()
            .collect::<Vec<_>>();
        json(&rows)
    })
}

/// `GET /employees?limit&offset`
#[procedure]
pub fn route_employees(ctx: &mut ProcedureContext, offset: u32, limit: u32) -> String {
    ctx.with_tx(|tx| {
        let rows = page_ids(offset, limit)
            .into_iter()
            .filter_map(|id| tx.db.employees().id().find(id))
            .map(employee_response)
            .collect::<Vec<_>>();
        json(&rows)
    })
}

/// `GET /employee-with-recipient?id` — the employees self-join.
#[procedure]
pub fn route_employee_with_recipient(ctx: &mut ProcedureContext, id: i32) -> String {
    ctx.with_tx(|tx| {
        let id = wrap_id(id, SEED_EMPLOYEES);
        let rows = tx
            .db
            .employees()
            .id()
            .find(id)
            .map(|employee| {
                let recipient = opt_i32(employee.recipient_id)
                    .and_then(|rid| tx.db.employees().id().find(rid as u32));
                EmployeeWithRecipientResponse {
                    recipient_last_name: recipient.as_ref().map(|row| row.last_name.clone()),
                    recipient_first_name: recipient
                        .as_ref()
                        .and_then(|row| opt_string(&row.first_name)),
                    employee: employee_response(employee),
                }
            })
            .into_iter()
            .collect::<Vec<_>>();
        json(&rows)
    })
}

/// `GET /suppliers?limit&offset`
#[procedure]
pub fn route_suppliers(ctx: &mut ProcedureContext, offset: u32, limit: u32) -> String {
    ctx.with_tx(|tx| {
        let rows = page_ids(offset, limit)
            .into_iter()
            .filter_map(|id| tx.db.suppliers().id().find(id))
            .map(supplier_response)
            .collect::<Vec<_>>();
        json(&rows)
    })
}

/// `GET /supplier-by-id?id`
#[procedure]
pub fn route_supplier_by_id(ctx: &mut ProcedureContext, id: i32) -> String {
    ctx.with_tx(|tx| {
        let id = wrap_id(id, SEED_SUPPLIERS);
        let rows = tx
            .db
            .suppliers()
            .id()
            .find(id)
            .map(supplier_response)
            .into_iter()
            .collect::<Vec<_>>();
        json(&rows)
    })
}

/// `GET /products?limit&offset`
#[procedure]
pub fn route_products(ctx: &mut ProcedureContext, offset: u32, limit: u32) -> String {
    ctx.with_tx(|tx| {
        let rows = page_ids(offset, limit)
            .into_iter()
            .filter_map(|id| tx.db.products().id().find(id))
            .map(product_response)
            .collect::<Vec<_>>();
        json(&rows)
    })
}

/// `GET /product-with-supplier?id` — product inner-joined to its supplier.
#[procedure]
pub fn route_product_with_supplier(ctx: &mut ProcedureContext, id: i32) -> String {
    ctx.with_tx(|tx| {
        let id = wrap_id(id, SEED_PRODUCTS);
        let rows = tx
            .db
            .products()
            .id()
            .find(id)
            .and_then(|product| {
                tx.db
                    .suppliers()
                    .id()
                    .find(product.supplier_id)
                    .map(|supplier| ProductWithSupplierResponse {
                        product: product_response(product),
                        supplier: supplier_response(supplier),
                    })
            })
            .into_iter()
            .collect::<Vec<_>>();
        json(&rows)
    })
}

/// `GET /orders-with-details?limit&offset` — per-order detail aggregates.
#[procedure]
pub fn route_orders_with_details(ctx: &mut ProcedureContext, offset: u32, limit: u32) -> String {
    ctx.with_tx(|tx| {
        let rows = page_ids(offset, limit)
            .into_iter()
            .filter_map(|id| tx.db.orders().id().find(id))
            .map(|order| {
                let details = details_of(tx, order.id);
                OrderWithDetailsResponse {
                    id: as_i32(order.id),
                    shipped_date: opt_i64(order.shipped_date),
                    ship_name: order.ship_name,
                    ship_city: order.ship_city,
                    ship_country: order.ship_country,
                    products_count: details.iter().filter(|row| row.product_id != 0).count() as i32,
                    quantity_sum: details.iter().map(|row| f64::from(row.quantity)).sum(),
                    total_price: details
                        .iter()
                        .map(|row| f64::from(row.quantity) * row.unit_price)
                        .sum(),
                }
            })
            .collect::<Vec<_>>();
        json(&rows)
    })
}

fn single_order(tx: &TxContext, id: i32, with_product_names: bool) -> String {
    let id = wrap_id(id, SEED_ORDERS);
    let rows = tx
        .db
        .orders()
        .id()
        .find(id)
        .map(|order| {
            let details = details_of(tx, order.id)
                .iter()
                .map(|detail| {
                    let product_name = with_product_names.then(|| {
                        tx.db
                            .products()
                            .id()
                            .find(detail.product_id)
                            .map(|row| row.name)
                            .unwrap_or_default()
                    });
                    detail_response(detail, product_name)
                })
                .collect();
            SingleOrderWithDetailsResponse {
                id: as_i32(order.id),
                order_date: order.order_date,
                required_date: order.required_date,
                shipped_date: opt_i64(order.shipped_date),
                ship_via: order.ship_via,
                freight: order.freight,
                ship_region: opt_string(&order.ship_region),
                ship_postal_code: opt_string(&order.ship_postal_code),
                ship_name: order.ship_name,
                ship_city: order.ship_city,
                ship_country: order.ship_country,
                customer_id: as_i32(order.customer_id),
                employee_id: as_i32(order.employee_id),
                details,
            }
        })
        .into_iter()
        .collect::<Vec<_>>();
    json(&rows)
}

/// `GET /order-with-details?id`
#[procedure]
pub fn route_order_with_details(ctx: &mut ProcedureContext, id: i32) -> String {
    ctx.with_tx(|tx| single_order(tx, id, false))
}

/// `GET /order-with-details-and-products?id`
#[procedure]
pub fn route_order_with_details_and_products(ctx: &mut ProcedureContext, id: i32) -> String {
    ctx.with_tx(|tx| single_order(tx, id, true))
}

/// `GET /search-customer?term` — case-insensitive substring scan.
#[procedure]
pub fn route_search_customer(ctx: &mut ProcedureContext, term: String) -> String {
    let term = term.to_ascii_lowercase();
    ctx.with_tx(|tx| {
        let rows = tx
            .db
            .customers()
            .iter()
            .filter(|row| contains_term(&row.company_name, &term))
            .map(customer_response)
            .collect::<Vec<_>>();
        json(&sorted_by_id(rows, |row| row.id))
    })
}

/// `GET /search-product?term` — case-insensitive substring scan.
#[procedure]
pub fn route_search_product(ctx: &mut ProcedureContext, term: String) -> String {
    let term = term.to_ascii_lowercase();
    ctx.with_tx(|tx| {
        let rows = tx
            .db
            .products()
            .iter()
            .filter(|row| contains_term(&row.name, &term))
            .map(product_response)
            .collect::<Vec<_>>();
        json(&sorted_by_id(rows, |row| row.id))
    })
}

/// Table iteration order is unspecified; the other SpacetimeDB targets answer
/// searches in id order, so this one does too.
fn sorted_by_id<T>(mut rows: Vec<T>, id: impl Fn(&T) -> i32) -> Vec<T> {
    rows.sort_unstable_by_key(|row| id(row));
    rows
}
