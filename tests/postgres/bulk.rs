//! `bulk_insert`: rows inserted as multi-row `VALUES` statements, split to
//! stay within PostgreSQL's 65535 bind parameters.
//!
//! A row leaves a cell to its default with `with_*(None)`: rows of one call
//! share a model type, so they call the same setters.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use drizzle::core::expr::count;
use drizzle::postgres::BulkInsertOptions;
use drizzle::postgres::prelude::*;

fn generated_code() -> String {
    "generated".to_string()
}

#[PostgresTable(NAME = "pg_bulk_items")]
struct BulkItem {
    #[column(serial, primary)]
    id: i32,
    #[column(unique)]
    name: String,
    note: Option<String>,
    #[column(default = "untagged")]
    tag: String,
    #[column(default_fn = generated_code)]
    code: String,
}

#[derive(PostgresSchema)]
struct BulkSchema {
    items: BulkItem,
}

/// More rows than one statement takes: each sets only `name` and `code`,
/// two parameters a row.
const CHUNKED_ROWS: usize = 40_000;

/// The rows, with `$last` as the last one's name.
macro_rules! chunked_rows {
    ($last:expr) => {
        (0..CHUNKED_ROWS).map(|i| {
            let name = if i + 1 == CHUNKED_ROWS {
                $last.to_string()
            } else {
                format!("row {i}")
            };
            InsertBulkItem::new(name)
        })
    };
}

#[drizzle::test]
fn bulk_insert_leaves_unset_cells_to_their_default(db: &mut TestDb<BulkSchema>) {
    let BulkSchema { items } = schema;

    let inserted = result!(
        db.bulk_insert(
            items,
            [
                InsertBulkItem::new("a")
                    .with_note(Some("first".to_string()))
                    .with_tag(None::<String>),
                InsertBulkItem::new("b")
                    .with_note(None::<String>)
                    .with_tag(Some("tagged".to_string())),
                InsertBulkItem::new("c")
                    .with_note(None::<String>)
                    .with_tag(None::<String>),
            ],
            BulkInsertOptions::default(),
        )
    )?;
    assert_eq!(inserted, 3);

    let rows: Vec<(String, Option<String>, String, String)> = db
        .select((items.name, items.note, items.tag, items.code))
        .from(items)
        .order_by(asc(items.name))
        .all();
    assert_eq!(
        rows,
        [
            (
                "a".to_string(),
                Some("first".to_string()),
                "untagged".to_string(),
                "generated".to_string(),
            ),
            (
                "b".to_string(),
                None,
                "tagged".to_string(),
                "generated".to_string(),
            ),
            (
                "c".to_string(),
                None,
                "untagged".to_string(),
                "generated".to_string(),
            ),
        ]
    );
}

#[drizzle::test]
fn bulk_insert_splits_rows_past_the_parameter_limit(db: &mut TestDb<BulkSchema>) {
    let BulkSchema { items } = schema;

    let inserted =
        result!(db.bulk_insert(items, chunked_rows!("last"), BulkInsertOptions::default()))?;
    assert_eq!(inserted, CHUNKED_ROWS as u64);

    let total: i64 = db.select(count(items.id)).from(items).get();
    assert_eq!(total, CHUNKED_ROWS as i64);
}

#[drizzle::test]
fn bulk_insert_rolls_back_every_chunk_when_one_fails(db: &mut TestDb<BulkSchema>) {
    let BulkSchema { items } = schema;

    // The last row repeats the first name, so the last statement fails.
    let outcome =
        result!(db.bulk_insert(items, chunked_rows!("row 0"), BulkInsertOptions::default()));
    assert!(outcome.is_err(), "{outcome:?}");

    let total: i64 = db.select(count(items.id)).from(items).get();
    assert_eq!(total, 0);
}

#[drizzle::test]
fn bulk_insert_without_atomic_keeps_the_chunks_before_a_failure(db: &mut TestDb<BulkSchema>) {
    let BulkSchema { items } = schema;

    let outcome = result!(db.bulk_insert(
        items,
        chunked_rows!("row 0"),
        BulkInsertOptions::default().atomic(false)
    ));
    assert!(outcome.is_err(), "{outcome:?}");

    // The first statement takes as many rows as fit in 65535 parameters.
    let total: i64 = db.select(count(items.id)).from(items).get();
    assert_eq!(total, 65535 / 2);
}
