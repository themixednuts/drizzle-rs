//! SpacetimeDB module defining Northwind benchmark tables and reducers.
//!
//! Build: `spacetime build bench/targets/spacetime-module`
//! Publish: `spacetime publish bench-module bench/targets/spacetime-module`
//!
//! Tables mirror the Northwind "micro" schema used by all benchmark targets
//! (customers, employees, suppliers, products, orders, order_details).

use rand::rngs::StdRng;
use rand::{Rng, SeedableRng};
use spacetimedb::{ReducerContext, Table, reducer, table};

const SEED_CUSTOMERS: u32 = 10_000;
const SEED_EMPLOYEES: u32 = 200;
const SEED_SUPPLIERS: u32 = 1_000;
const SEED_PRODUCTS: u32 = 5_000;
const SEED_ORDERS: u32 = 50_000;
const DETAILS_PER_ORDER: u32 = 6;

const CUSTOMER_SEARCH_TERMS: &[&str] = &[
    "ve", "ey", "or", "bb", "te", "ab", "ca", "ki", "ap", "be", "ct", "hi", "er", "pr", "pi", "en",
    "au", "ra", "ti", "ke", "ou", "ur", "me", "ea", "op", "at", "ne", "na", "os", "ri", "on", "ha",
    "il", "to", "as", "io", "di", "zy", "az", "la", "ko", "st", "gh", "ug", "ac", "cc", "ch", "hu",
    "re", "an",
];

const PRODUCT_SEARCH_TERMS: &[&str] = &[
    "ha", "ey", "or", "po", "te", "ab", "er", "ke", "ap", "be", "en", "au", "ra", "ti", "su", "sa",
    "hi", "nu", "ge", "pi", "ou", "ur", "me", "ea", "tu", "at", "ne", "na", "os", "ri", "on", "ka",
    "il", "to", "as", "io", "di", "za", "fa", "la", "ko", "st", "gh", "ug", "ac", "cc", "ch", "pa",
    "re", "an",
];

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
    #[index(btree)]
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
    #[index(btree)]
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
    #[index(btree)]
    pub customer_id: u32,
    #[index(btree)]
    pub employee_id: u32,
}

#[table(accessor = order_details, public)]
pub struct OrderDetail {
    #[primary_key]
    #[auto_inc]
    pub id: u32,
    pub unit_price: f64,
    pub quantity: i32,
    pub discount: f64,
    #[index(btree)]
    pub order_id: u32,
    #[index(btree)]
    pub product_id: u32,
}

/// Seed benchmark data. Clears existing rows first.
///
/// This mirrors the PGWire runner's deterministic Northwind seed so the SDK
/// target does not skip data work or benchmark a smaller/different dataset.
#[reducer]
pub fn seed(ctx: &ReducerContext, seed_val: u64, _trial: u32) {
    for row in ctx.db.order_details().iter() {
        ctx.db.order_details().id().delete(&row.id);
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

    let mut rng = StdRng::seed_from_u64(seed_val);

    for i in 0..SEED_CUSTOMERS {
        let postal_code = if rng.random_bool(0.8) {
            format!("{:05}", rng.random_range(10000..99999))
        } else {
            String::new()
        };
        let region = if rng.random_bool(0.5) {
            format!("Region-{}", rng.random_range(1..50))
        } else {
            String::new()
        };
        let fax = if rng.random_bool(0.3) {
            format!("555-{:04}", rng.random_range(1000..9999))
        } else {
            String::new()
        };
        ctx.db.customers().insert(Customer {
            id: i + 1,
            company_name: format!(
                "C-{}-{i}",
                CUSTOMER_SEARCH_TERMS[i as usize % CUSTOMER_SEARCH_TERMS.len()]
            ),
            contact_name: format!("Contact-{i}"),
            contact_title: format!("Title-{i}"),
            address: format!("{i} Main St"),
            city: format!("City-{i}"),
            postal_code,
            region,
            country: format!("Country-{}", i % 50),
            phone: format!("555-{i:04}"),
            fax,
        });
    }

    for i in 0..SEED_EMPLOYEES {
        let first_name = if rng.random_bool(0.9) {
            format!("First-{i}")
        } else {
            String::new()
        };
        ctx.db.employees().insert(Employee {
            id: i + 1,
            last_name: format!("Last-{i}"),
            first_name,
            title: format!("Title-{i}"),
            title_of_courtesy: ["Mr.", "Ms.", "Mrs.", "Dr."][(i % 4) as usize].to_string(),
            birth_date: rng.random_range(315_360_000..946_080_000_i64),
            hire_date: rng.random_range(946_684_800..1_672_531_200_i64),
            address: format!("{i} Elm St"),
            city: format!("City-{i}"),
            postal_code: format!("{:05}", rng.random_range(10000..99999)),
            country: format!("Country-{}", i % 20),
            home_phone: format!("555-{i:04}"),
            extension: rng.random_range(100..9999_i32),
            notes: format!("Notes for employee {i}"),
            recipient_id: if i > 0 {
                rng.random_range(1..=i as i32)
            } else {
                0
            },
        });
    }

    for i in 0..SEED_SUPPLIERS {
        let region = if rng.random_bool(0.5) {
            format!("Region-{}", rng.random_range(1..50))
        } else {
            String::new()
        };
        ctx.db.suppliers().insert(Supplier {
            id: i + 1,
            company_name: format!("Supplier-{i}"),
            contact_name: format!("Contact-{i}"),
            contact_title: format!("Title-{i}"),
            address: format!("{i} Oak Ave"),
            city: format!("City-{i}"),
            region,
            postal_code: format!("{:05}", rng.random_range(10000..99999)),
            country: format!("Country-{}", i % 20),
            phone: format!("555-{i:04}"),
        });
    }

    for i in 0..SEED_PRODUCTS {
        ctx.db.products().insert(Product {
            id: i + 1,
            name: format!(
                "P-{}-{i}",
                PRODUCT_SEARCH_TERMS[i as usize % PRODUCT_SEARCH_TERMS.len()]
            ),
            qt_per_unit: format!(
                "{} boxes x {} bags",
                rng.random_range(1..20),
                rng.random_range(1..50)
            ),
            unit_price: (rng.random_range(1.0..500.0_f64) * 100.0).round() / 100.0,
            units_in_stock: rng.random_range(0..200_i32),
            units_on_order: rng.random_range(0..100_i32),
            reorder_level: rng.random_range(0..50_i32),
            discontinued: if rng.random_bool(0.1) { 1 } else { 0 },
            supplier_id: rng.random_range(1..=SEED_SUPPLIERS),
        });
    }

    for i in 0..SEED_ORDERS {
        let order_date = rng.random_range(946_684_800..1_672_531_200_i64);
        let required_date = order_date + rng.random_range(604_800..2_592_000);
        let shipped_date = if rng.random_bool(0.85) {
            order_date + rng.random_range(86_400..1_209_600)
        } else {
            0
        };
        let ship_region = if rng.random_bool(0.5) {
            format!("Region-{}", rng.random_range(1..50))
        } else {
            String::new()
        };
        let ship_postal_code = if rng.random_bool(0.8) {
            format!("{:05}", rng.random_range(10000..99999))
        } else {
            String::new()
        };
        ctx.db.orders().insert(Order {
            id: i + 1,
            order_date,
            required_date,
            shipped_date,
            ship_via: rng.random_range(1..=3_i32),
            freight: (rng.random_range(0.5..500.0_f64) * 100.0).round() / 100.0,
            ship_name: format!("Ship-{i}"),
            ship_city: format!("City-{i}"),
            ship_region,
            ship_postal_code,
            ship_country: format!("Country-{}", i % 50),
            customer_id: rng.random_range(1..=SEED_CUSTOMERS),
            employee_id: rng.random_range(1..=SEED_EMPLOYEES),
        });
    }

    for order_i in 0..SEED_ORDERS {
        let order_id = order_i + 1;
        for detail_i in 0..DETAILS_PER_ORDER {
            let id = order_i * DETAILS_PER_ORDER + detail_i + 1;
            ctx.db.order_details().insert(OrderDetail {
                id,
                unit_price: (rng.random_range(1.0..200.0_f64) * 100.0).round() / 100.0,
                quantity: rng.random_range(1..=100_i32),
                discount: if rng.random_bool(0.3) {
                    (rng.random_range(0.05..0.25_f64) * 100.0).round() / 100.0
                } else {
                    0.0
                },
                order_id,
                product_id: rng.random_range(1..=SEED_PRODUCTS),
            });
        }
    }
}
