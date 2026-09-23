//! MySQL-only expression functions, executed against the server.
//!
//! Portable functions run in `crate::common::expressions`; this file covers
//! the string, math and statistical functions MySQL shares only with
//! PostgreSQL or SQLite.

use drizzle::core::expr::{
    count, eq, greatest, group_concat, gt, ifnull, instr, least, left, log2, lpad, pi, repeat,
    reverse, right, rpad, stddev_pop, stddev_samp, var_pop, var_samp, variance,
};
use drizzle::mysql::prelude::*;

#[MySQLTable(NAME = "mysql_expression_rows")]
struct ExpressionRow {
    #[column(PRIMARY)]
    id: i32,
    #[column(VARCHAR(32))]
    name: String,
    quantity: Option<i32>,
}

/// `VAR_POP`, `VAR_SAMP`, `VARIANCE`, `STDDEV_POP`, `STDDEV_SAMP`.
type Statistics = (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

#[derive(MySQLSchema)]
struct ExpressionSchema {
    rows: ExpressionRow,
}

#[drizzle::test]
fn string_slicing_and_padding(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new(1, "alice"),
            InsertExpressionRow::new(2, "bob"),
            InsertExpressionRow::new(3, "carol"),
        ])
        .execute();

    let sliced: (String, String, String, String, String, String) = db
        .select((
            left(rows.name, 2),
            right(rows.name, 2),
            lpad(rows.name, 7, "*"),
            rpad(rows.name, 7, "*"),
            repeat(rows.name, 2),
            reverse(rows.name),
        ))
        .from(rows)
        .r#where(eq(rows.id, 1))
        .get();
    assert_eq!(
        sliced,
        (
            "al".to_string(),
            "ce".to_string(),
            "**alice".to_string(),
            "alice**".to_string(),
            "alicealice".to_string(),
            "ecila".to_string()
        )
    );

    // INSTR is 1-based and 0 when absent; its width differs, so compare in SQL.
    let with_ar: Vec<i32> = db
        .select(rows.id)
        .from(rows)
        .r#where(gt(instr(rows.name, "ar"), 0))
        .all();
    assert_eq!(with_ar, [3]);
}

#[drizzle::test]
fn extrema_and_null_substitution(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new(1, "alice"),
            InsertExpressionRow::new(2, "bob"),
            InsertExpressionRow::new(3, "carol"),
        ])
        .execute();
    db.update(rows)
        .set(UpdateExpressionRow::default().with_quantity(7))
        .r#where(eq(rows.id, 1))
        .execute();

    let substituted: Vec<i32> = db
        .select(ifnull(rows.quantity, 0))
        .from(rows)
        .order_by(asc(rows.id))
        .all();
    assert_eq!(substituted, [7, 0, 0]);

    // MySQL returns NULL from GREATEST / LEAST when any argument is NULL.
    let bounds: Vec<(Option<i32>, Option<i32>)> = db
        .select((greatest(rows.quantity, 5), least(rows.quantity, 5)))
        .from(rows)
        .order_by(asc(rows.id))
        .all();
    assert_eq!(bounds, [(Some(7), Some(5)), (None, None), (None, None)]);
}

#[drizzle::test]
fn math_constants_and_logarithms(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new(1, "alice"),
            InsertExpressionRow::new(2, "bob"),
            InsertExpressionRow::new(3, "carol"),
        ])
        .execute();

    let approximately_pi: i64 = db
        .select(count(rows.id))
        .from(rows)
        .r#where(gt(pi(), 3.1))
        .get();
    assert_eq!(approximately_pi, 3);

    let logs: Vec<Option<f64>> = db
        .select(log2(rows.id * 4))
        .from(rows)
        .order_by(asc(rows.id))
        .all();
    assert_eq!(logs, [Some(2.0), Some(3.0), Some(f64::log2(12.0))]);
}

#[drizzle::test]
fn statistical_aggregates(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new(1, "alice"),
            InsertExpressionRow::new(2, "bob"),
            InsertExpressionRow::new(3, "carol"),
        ])
        .execute();

    // ids 1, 2, 3: population variance 2/3, sample variance 1.
    let stats: Statistics = db
        .select((
            var_pop(rows.id),
            var_samp(rows.id),
            variance(rows.id),
            stddev_pop(rows.id),
            stddev_samp(rows.id),
        ))
        .from(rows)
        .get();
    let (var_pop, var_samp, variance, stddev_pop, stddev_samp) = stats;
    assert!((var_pop.unwrap() - 2.0 / 3.0).abs() < 1e-9);
    assert_eq!(var_samp, Some(1.0));
    assert_eq!(variance, Some(1.0));
    assert!((stddev_pop.unwrap() - (2.0_f64 / 3.0).sqrt()).abs() < 1e-9);
    assert_eq!(stddev_samp, Some(1.0));

    let joined: Option<String> = db
        .select(group_concat(rows.name))
        .from(rows)
        .r#where(eq(rows.id, 2))
        .get();
    assert_eq!(joined.as_deref(), Some("bob"));
}
