//! SpacetimeDB module defining Northwind benchmark tables and reducers.
//!
//! Build: `spacetime build bench/targets/spacetime-module`
//! Publish: `spacetime publish bench-module bench/targets/spacetime-module`
//!
//! Tables mirror the Northwind "micro" schema used by all benchmark targets
//! (customers, employees, suppliers, products, orders, order_details).

use spacetimedb::{reducer, table, ReducerContext, Table};

#[table(accessor = customers, public)]
pub struct Customer {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub company_name: String,
    pub contact_name: String,
    pub contact_title: String,
    pub address: String,
    pub city: String,
    pub postal_code: String,
    pub region: String,
    pub country: String,
    pub phone: String,
    pub fax: String,
}

#[table(accessor = employees, public)]
pub struct Employee {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub last_name: String,
    pub first_name: String,
    pub title: String,
    pub title_of_courtesy: String,
    pub birth_date: i64,
    pub hire_date: i64,
    pub address: String,
    pub city: String,
    pub postal_code: String,
    pub country: String,
    pub home_phone: String,
    pub extension: i32,
    pub notes: String,
    pub recipient_id: i32, // 0 = no recipient (SpacetimeDB doesn't support Option<T> well in PGWire)
}

#[table(accessor = suppliers, public)]
pub struct Supplier {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub company_name: String,
    pub contact_name: String,
    pub contact_title: String,
    pub address: String,
    pub city: String,
    pub region: String,
    pub postal_code: String,
    pub country: String,
    pub phone: String,
}

#[table(accessor = products, public)]
pub struct Product {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub name: String,
    pub qt_per_unit: String,
    pub unit_price: f64,
    pub units_in_stock: i32,
    pub units_on_order: i32,
    pub reorder_level: i32,
    pub discontinued: i32,
    pub supplier_id: u32,
}

#[table(accessor = orders, public)]
pub struct Order {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub order_date: i64,
    pub required_date: i64,
    pub shipped_date: i64, // 0 = not shipped
    pub ship_via: i32,
    pub freight: f64,
    pub ship_name: String,
    pub ship_city: String,
    pub ship_region: String,
    pub ship_postal_code: String,
    pub ship_country: String,
    pub customer_id: u32,
    pub employee_id: u32,
}

#[table(accessor = order_details, public)]
pub struct OrderDetail {
    pub unit_price: f64,
    pub quantity: i32,
    pub discount: f64,
    pub order_id: u32,
    pub product_id: u32,
}

/// Seed benchmark data. Clears existing rows first.
///
/// Called by the HTTP wrapper targets on startup before serving requests.
#[reducer]
pub fn seed(ctx: &ReducerContext, seed_val: u64, _trial: u32) {
    // Clear existing data (order matters for referential integrity)
    for row in ctx.db.order_details().iter() {
        ctx.db.order_details().order_id().delete(&row.order_id);
    }
    for row in ctx.db.orders().iter() {
        ctx.db.orders().id().delete(&row.id);
    }
    for row in ctx.db.products().iter() {
        ctx.db.products().id().delete(&row.id);
    }
    for row in ctx.db.suppliers().iter() {
        ctx.db.suppliers().id().delete(&row.id);
    }
    for row in ctx.db.employees().iter() {
        ctx.db.employees().id().delete(&row.id);
    }
    for row in ctx.db.customers().iter() {
        ctx.db.customers().id().delete(&row.id);
    }

    // Simple deterministic seed based on seed_val
    let n_customers = 200_u32;
    let n_employees = 50_u32;
    let n_suppliers = 30_u32;
    let n_products = 100_u32;
    let n_orders = 500_u32;

    for i in 0..n_customers {
        let s = seed_val.wrapping_add(i as u64);
        ctx.db.customers().insert(Customer {
            id: 0,
            company_name: format!("Company-{s}"),
            contact_name: format!("Contact-{s}"),
            contact_title: "Sales Rep".to_string(),
            address: format!("{i} Main St"),
            city: format!("City{}", i % 20),
            postal_code: format!("{:05}", i % 100000),
            region: format!("Region{}", i % 5),
            country: format!("Country{}", i % 10),
            phone: format!("+1-555-{i:04}"),
            fax: if i % 3 == 0 { format!("+1-555-{i:04}-fax") } else { String::new() },
        });
    }

    for i in 0..n_employees {
        let s = seed_val.wrapping_add(1000 + i as u64);
        ctx.db.employees().insert(Employee {
            id: 0,
            last_name: format!("Last-{s}"),
            first_name: format!("First-{s}"),
            title: format!("Title-{}", i % 5),
            title_of_courtesy: ["Mr.", "Ms.", "Mrs.", "Dr."][(i % 4) as usize].to_string(),
            birth_date: -(315_360_000 + i as i64 * 86400 * 365),
            hire_date: 946_684_800 + i as i64 * 86400 * 30,
            address: format!("{i} Work Ave"),
            city: format!("City{}", i % 10),
            postal_code: format!("{:05}", i % 100000),
            country: format!("Country{}", i % 5),
            home_phone: format!("+1-555-{i:04}"),
            extension: 100 + i as i32,
            notes: format!("Notes for employee {i}"),
            recipient_id: if i > 0 { ((i - 1) % n_employees + 1) as i32 } else { 0 },
        });
    }

    for i in 0..n_suppliers {
        let s = seed_val.wrapping_add(2000 + i as u64);
        ctx.db.suppliers().insert(Supplier {
            id: 0,
            company_name: format!("Supplier-{s}"),
            contact_name: format!("Contact-{s}"),
            contact_title: format!("Owner{}", i % 3),
            address: format!("{i} Supply Rd"),
            city: format!("City{}", i % 15),
            region: format!("Region{}", i % 4),
            postal_code: format!("{:05}", i % 100000),
            country: format!("Country{}", i % 8),
            phone: format!("+1-555-{i:04}"),
        });
    }

    for i in 0..n_products {
        ctx.db.products().insert(Product {
            id: 0,
            name: format!("Product-{i}"),
            qt_per_unit: format!("{} boxes x {} pcs", (i % 10) + 1, (i % 20) + 1),
            unit_price: 10.0 + (i as f64 * 1.5),
            units_in_stock: (50 + i % 100) as i32,
            units_on_order: (i % 30) as i32,
            reorder_level: (5 + i % 20) as i32,
            discontinued: if i % 10 == 0 { 1 } else { 0 },
            supplier_id: (i % n_suppliers) + 1,
        });
    }

    for i in 0..n_orders {
        let base = 1609459200_i64;
        ctx.db.orders().insert(Order {
            id: 0,
            order_date: base + i as i64 * 3600,
            required_date: base + i as i64 * 3600 + 86400 * 14,
            shipped_date: if i % 3 != 0 { base + i as i64 * 3600 + 86400 * 7 } else { 0 },
            ship_via: (1 + i % 3) as i32,
            freight: 10.0 + i as f64 * 0.25,
            ship_name: format!("Ship-{i}"),
            ship_city: format!("City{}", i % 20),
            ship_region: if i % 4 == 0 { format!("Region{}", i % 5) } else { String::new() },
            ship_postal_code: if i % 4 == 0 { format!("{:05}", i % 100000) } else { String::new() },
            ship_country: format!("Country{}", i % 10),
            customer_id: (i % n_customers) + 1,
            employee_id: (i % n_employees) + 1,
        });
    }

    // ~3 details per order
    for i in 0..(n_orders * 3) {
        ctx.db.order_details().insert(OrderDetail {
            unit_price: 10.0 + (i as f64 * 0.5),
            quantity: ((i % 50) + 1) as i32,
            discount: if i % 5 == 0 { 0.05 } else { 0.0 },
            order_id: (i % n_orders) + 1,
            product_id: (i % n_products) + 1,
        });
    }
}
