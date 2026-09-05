//! SQLite-only expression functions, executed against the database.
//!
//! Portable functions run in `crate::common::expressions`; this file covers
//! the `SQLiteDateTimeSupport`, `GroupConcatSupport`, `TypeofSupport` and
//! `IFNULL` surface that only SQLite (and, for some, MySQL) provides.

#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]

use drizzle::core::expr::{
    date, datetime, eq, group_concat, ifnull, is_not_null, julianday, time, typeof_, unixepoch,
};
use drizzle::sqlite::prelude::*;

#[SQLiteTable(NAME = "expression_rows")]
struct ExpressionRow {
    #[column(PRIMARY)]
    id: i32,
    name: String,
    quantity: Option<i32>,
    /// ISO-8601 text; SQLite's date functions accept TEXT.
    occurred_at: String,
}

#[derive(SQLiteSchema)]
struct ExpressionSchema {
    rows: ExpressionRow,
}

#[drizzle::test]
fn ifnull_substitutes_null(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", "2024-02-29 13:45:30").with_id(1),
            InsertExpressionRow::new("bob", "1970-01-02 00:00:00").with_id(2),
            InsertExpressionRow::new("carol", "2000-01-01 00:00:00").with_id(3),
        ])
        .execute();
    db.update(rows)
        .set(UpdateExpressionRow::default().with_quantity(7))
        .r#where(eq(rows.id, 1))
        .execute();

    let quantities: Vec<i32> = db
        .select(ifnull(rows.quantity, 0))
        .from(rows)
        .order_by(asc(rows.id))
        .all();
    assert_eq!(quantities, [7, 0, 0]);
}

#[drizzle::test]
fn group_concat_joins_group_members(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", "2024-02-29 13:45:30").with_id(1),
            InsertExpressionRow::new("bob", "1970-01-02 00:00:00").with_id(2),
            InsertExpressionRow::new("carol", "2000-01-01 00:00:00").with_id(3),
        ])
        .execute();

    let joined: Option<String> = db
        .select(group_concat(rows.name))
        .from(rows)
        .r#where(eq(rows.id, 1))
        .get();
    assert_eq!(joined.as_deref(), Some("alice"));

    let all: Option<String> = db.select(group_concat(rows.name)).from(rows).get();
    let mut names: Vec<&str> = all.as_deref().unwrap().split(',').collect();
    names.sort_unstable();
    assert_eq!(names, ["alice", "bob", "carol"]);
}

#[drizzle::test]
fn typeof_reports_storage_class(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", "2024-02-29 13:45:30").with_id(1),
            InsertExpressionRow::new("bob", "1970-01-02 00:00:00").with_id(2),
            InsertExpressionRow::new("carol", "2000-01-01 00:00:00").with_id(3),
        ])
        .execute();

    let classes: (String, String, String) = db
        .select((typeof_(rows.id), typeof_(rows.name), typeof_(rows.quantity)))
        .from(rows)
        .r#where(eq(rows.id, 2))
        .get();
    assert_eq!(
        classes,
        (
            "integer".to_string(),
            "text".to_string(),
            "null".to_string()
        )
    );
}

#[drizzle::test]
fn date_functions_split_and_convert_timestamps(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", "2024-02-29 13:45:30").with_id(1),
            InsertExpressionRow::new("bob", "1970-01-02 00:00:00").with_id(2),
            InsertExpressionRow::new("carol", "2000-01-01 00:00:00").with_id(3),
        ])
        .execute();

    let parts: (String, String, String) = db
        .select((
            date(rows.occurred_at),
            time(rows.occurred_at),
            datetime(rows.occurred_at),
        ))
        .from(rows)
        .r#where(eq(rows.id, 1))
        .get();
    assert_eq!(
        parts,
        (
            "2024-02-29".to_string(),
            "13:45:30".to_string(),
            "2024-02-29 13:45:30".to_string()
        )
    );

    let julian: f64 = db
        .select(julianday(rows.occurred_at))
        .from(rows)
        .r#where(eq(rows.id, 3))
        .get();
    assert_eq!(julian, 2_451_544.5);

    let epoch: i64 = db
        .select(unixepoch(rows.occurred_at))
        .from(rows)
        .r#where(eq(rows.id, 2))
        .get();
    assert_eq!(epoch, 86_400);

    // Literal arguments are accepted wherever a temporal expression is.
    let today: String = db
        .select(date("now"))
        .from(rows)
        .r#where(is_not_null(rows.id))
        .get();
    assert_eq!(today.len(), 10);
}

/// `LOG2` and `PI` are also part of SQLite's optional math functions.
#[cfg(all(
    feature = "math",
    feature = "rusqlite",
    not(any(feature = "libsql", feature = "turso"))
))]
#[drizzle::test]
fn optional_math_functions(db: &mut TestDb<ExpressionSchema>) {
    use drizzle::core::expr::{count, gt, log2, pi};

    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", "2024-02-29 13:45:30").with_id(1),
            InsertExpressionRow::new("bob", "1970-01-02 00:00:00").with_id(2),
            InsertExpressionRow::new("carol", "2000-01-01 00:00:00").with_id(3),
        ])
        .execute();

    let logs: Vec<Option<f64>> = db
        .select(log2(rows.id * 4))
        .from(rows)
        .order_by(asc(rows.id))
        .all();
    assert_eq!(logs, [Some(2.0), Some(3.0), Some(f64::log2(12.0))]);

    let approximately_pi: i64 = db
        .select(count(rows.id))
        .from(rows)
        .r#where(gt(pi(), 3.1))
        .get();
    assert_eq!(approximately_pi, 3);
}
