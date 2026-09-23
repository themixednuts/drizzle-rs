/// JSON column contracts shared by every dialect.
///
/// Table macros convert JSON payloads through `drizzle::core::Json<T>` instead
/// of implementing traits on the payload type, so one payload type can back
/// columns in several tables, and foreign, path-qualified, and generic payloads
/// all work. JSON columns compare against `Json(..)` values.
///
/// `$json` spells the column type of plain JSON columns and `$comparable_json`
/// the type of the column compared with `eq` (PostgreSQL defines `=` only for
/// `jsonb`).
macro_rules! shared_json_suite {
    (
        $dialect:ident,
        $table:ident,
        $schema:ident,
        $update_value:ident,
        $json:ident,
        $comparable_json:ident
    ) => {
        mod shared_json {
            #[allow(unused_imports)]
            use super::*;
            #[allow(unused_imports)]
            use drizzle::core::asc;
            use drizzle::core::Json;
            use drizzle::core::expr::eq;
            use serde::{Deserialize, Serialize};
            // Imported unqualified on purpose: the macro must not need to
            // recognise the `serde_json::Value` spelling.
            use serde_json::Value;
            use std::collections::BTreeMap;

            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
            pub struct Address {
                pub street: String,
                pub city: String,
            }

            pub mod models {
                #[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq, Default)]
                pub struct Meta {
                    pub level: i64,
                    pub labels: Vec<String>,
                }
            }

            #[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
            pub struct Envelope<T> {
                pub kind: String,
                pub value: T,
            }

            // `Address` backs JSON columns in both tables.
            #[$table(NAME = "shared_json_customers")]
            struct JsonCustomer {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                #[column($json)]
                address: Address,
                #[column($json)]
                previous_address: Option<Address>,
            }

            #[$table(NAME = "shared_json_orders")]
            struct JsonOrder {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(REFERENCES = JsonCustomer::id)]
                customer_id: i32,
                #[column($comparable_json)]
                shipping: Address,
                #[column($json)]
                tags: Vec<String>,
                #[column($json)]
                counts: BTreeMap<String, i64>,
                #[column($json)]
                meta: self::models::Meta,
                #[column($json)]
                envelope: Envelope<i64>,
                #[column($json)]
                extra: Value,
            }

            #[derive($schema)]
            struct JsonSchema {
                customers: JsonCustomer,
                orders: JsonOrder,
            }

            fn address(street: &str, city: &str) -> Address {
                Address {
                    street: street.to_owned(),
                    city: city.to_owned(),
                }
            }

            fn counts(pairs: &[(&str, i64)]) -> BTreeMap<String, i64> {
                pairs
                    .iter()
                    .map(|(key, value)| ((*key).to_owned(), *value))
                    .collect()
            }

            fn meta(level: i64, labels: &[&str]) -> self::models::Meta {
                self::models::Meta {
                    level,
                    labels: labels.iter().map(|label| (*label).to_owned()).collect(),
                }
            }

            fn envelope(kind: &str, value: i64) -> Envelope<i64> {
                Envelope {
                    kind: kind.to_owned(),
                    value,
                }
            }

            #[drizzle::test($dialect)]
            fn one_payload_round_trips_through_two_tables(db: &mut TestDb<JsonSchema>) {
                let JsonSchema { customers, orders } = schema;
                let home = address("1 Main St", "Springfield");
                let previous = address("9 Elm St", "Shelbyville");
                let extra = serde_json::json!({ "gift": true, "note": "leave at door" });

                db.insert(customers)
                    .value(
                        InsertJsonCustomer::new("Ada", home.clone())
                            .with_id(1)
                            .with_previous_address(previous.clone()),
                    )
                    .execute();
                db.insert(orders)
                    .value(
                        InsertJsonOrder::new(
                            1,
                            home.clone(),
                            vec!["fragile".to_owned(), "gift".to_owned()],
                            counts(&[("apples", 3), ("pears", 2)]),
                            meta(2, &["priority"]),
                            envelope("amount", 1_250),
                            extra.clone(),
                        )
                        .with_id(10),
                    )
                    .execute();

                let customer: SelectJsonCustomer = db.select(()).from(customers).get();
                assert_eq!(customer.address, home);
                assert_eq!(customer.previous_address, Some(previous));

                let order: SelectJsonOrder = db.select(()).from(orders).get();
                assert_eq!(order.shipping, home);
                assert_eq!(order.tags, ["fragile", "gift"]);
                assert_eq!(order.counts, counts(&[("apples", 3), ("pears", 2)]));
                assert_eq!(order.meta, meta(2, &["priority"]));
                assert_eq!(order.envelope, envelope("amount", 1_250));
                assert_eq!(order.extra, extra);

                let moved = address("5 Oak Ave", "Capital City");
                db.update(customers)
                    .set(
                        UpdateJsonCustomer::default()
                            .with_address(moved.clone())
                            .with_previous_address(home.clone()),
                    )
                    .r#where(eq(customers.id, 1))
                    .execute();
                db.update(orders)
                    .set(
                        UpdateJsonOrder::default()
                            .with_shipping(moved.clone())
                            .with_tags(vec!["express".to_owned()])
                            .with_counts(counts(&[("plums", 7)]))
                            .with_meta(meta(3, &["late", "priority"]))
                            .with_envelope(envelope("refund", -40))
                            .with_extra(Json(serde_json::json!(["wrapped", 1]))),
                    )
                    .r#where(eq(orders.id, 10))
                    .execute();

                let customer: SelectJsonCustomer = db.select(()).from(customers).get();
                assert_eq!(customer.address, moved);
                assert_eq!(customer.previous_address, Some(home));

                let order: SelectJsonOrder = db.select(()).from(orders).get();
                assert_eq!(order.shipping, moved);
                assert_eq!(order.tags, ["express"]);
                assert_eq!(order.counts, counts(&[("plums", 7)]));
                assert_eq!(order.meta, meta(3, &["late", "priority"]));
                assert_eq!(order.envelope, envelope("refund", -40));
                assert_eq!(order.extra, serde_json::json!(["wrapped", 1]));
            }

            #[drizzle::test($dialect)]
            fn json_columns_compare_against_json_values(db: &mut TestDb<JsonSchema>) {
                let JsonSchema { customers, orders } = schema;
                let home = address("1 Main St", "Springfield");
                let office = address("2 Work Rd", "Springfield");

                db.insert(customers)
                    .value(InsertJsonCustomer::new("Ada", home.clone()).with_id(1))
                    .execute();
                db.insert(orders)
                    .values([
                        InsertJsonOrder::new(
                            1,
                            home.clone(),
                            Vec::new(),
                            BTreeMap::new(),
                            meta(1, &[]),
                            envelope("none", 0),
                            Value::Null,
                        )
                        .with_id(10),
                        InsertJsonOrder::new(
                            1,
                            office.clone(),
                            Vec::new(),
                            BTreeMap::new(),
                            meta(1, &[]),
                            envelope("none", 0),
                            Value::Null,
                        )
                        .with_id(11),
                    ])
                    .execute();

                let to_office: Vec<i32> = db
                    .select(orders.id)
                    .from(orders)
                    .r#where(eq(orders.shipping, Json(office.clone())))
                    .all();
                assert_eq!(to_office, [11]);

                let wrapped_home = Json(home);
                let to_home: Vec<i32> = db
                    .select(orders.id)
                    .from(orders)
                    .r#where(eq(orders.shipping, &wrapped_home))
                    .all();
                assert_eq!(to_home, [10]);
            }

            #[drizzle::test($dialect)]
            fn json_update_setters_accept_sql_null(db: &mut TestDb<JsonSchema>) {
                let JsonSchema { customers, .. } = schema;
                let home = address("1 Main St", "Springfield");

                db.insert(customers)
                    .value(
                        InsertJsonCustomer::new("Ada", home.clone())
                            .with_id(1)
                            .with_previous_address(home.clone()),
                    )
                    .execute();
                db.update(customers)
                    .set(UpdateJsonCustomer::default().with_previous_address($update_value::Null))
                    .r#where(eq(customers.id, 1))
                    .execute();

                let customer: SelectJsonCustomer = db.select(()).from(customers).get();
                assert_eq!(customer.address, home);
                assert_eq!(customer.previous_address, None);
            }

            #[cfg(feature = "query")]
            #[drizzle::test($dialect)]
            fn relational_queries_decode_json_fields(db: &mut TestDb<JsonSchema>) {
                let JsonSchema { customers, orders } = schema;
                let home = address("1 Main St", "Springfield");
                let previous = address("9 Elm St", "Shelbyville");
                let extra = serde_json::json!({ "gift": true });

                db.insert(customers)
                    .value(
                        InsertJsonCustomer::new("Ada", home.clone())
                            .with_id(1)
                            .with_previous_address(previous.clone()),
                    )
                    .execute();
                db.insert(orders)
                    .value(
                        InsertJsonOrder::new(
                            1,
                            home.clone(),
                            vec!["fragile".to_owned()],
                            counts(&[("apples", 3)]),
                            meta(2, &["priority"]),
                            envelope("amount", 99),
                            extra.clone(),
                        )
                        .with_id(10),
                    )
                    .execute();

                let rows = db.query(customers).find_many();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].address, home);
                assert_eq!(rows[0].previous_address, Some(previous.clone()));

                // JSON columns of a loaded relation arrive inside the
                // relation's JSON projection rather than as row columns.
                let rows = db
                    .query(customers)
                    .with(customers.json_orders())
                    .find_many();
                assert_eq!(rows.len(), 1);
                let loaded = &rows[0].json_orders;
                assert_eq!(loaded.len(), 1);
                assert_eq!(loaded[0].shipping, home);
                assert_eq!(loaded[0].tags, ["fragile"]);
                assert_eq!(loaded[0].counts, counts(&[("apples", 3)]));
                assert_eq!(loaded[0].meta, meta(2, &["priority"]));
                assert_eq!(loaded[0].envelope, envelope("amount", 99));
                assert_eq!(loaded[0].extra, extra);

                let rows = db.query(orders).with(orders.customer()).find_many();
                assert_eq!(rows.len(), 1);
                assert_eq!(rows[0].shipping, home);
                assert_eq!(rows[0].customer.address, home);
                assert_eq!(rows[0].customer.previous_address, Some(previous));
            }
        }
    };
}

pub(crate) use shared_json_suite;
