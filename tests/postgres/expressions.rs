//! PostgreSQL-only expression functions, executed against the server.
//!
//! Portable functions run in `crate::common::expressions`; this file covers
//! the `PostgresStringSupport`, `PostgresAggregateSupport`,
//! `PostgresDateTimeSupport`, `SequenceSupport` and operator surface that
//! only PostgreSQL provides.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use drizzle::core::expr::{
    age, and, array_agg, bool_and, bool_or, clock_timestamp, count, currval, date_bin, date_trunc,
    eq, every, extract, greatest, gt, initcap, is_not_null, least, left, localtime, localtimestamp,
    lpad, make_date, make_timestamp, nextval, now, pi, regexp_match, regexp_match_flags,
    regexp_replace, regexp_replace_flags, repeat, reverse, right, rpad, setval, split_part,
    starts_with, stddev_pop, stddev_samp, string_agg, to_char, to_date, to_number, to_timestamp,
    translate, var_pop, var_samp, variance,
};
use drizzle::postgres::expr::{
    ilike, not_ilike, regex_match, regex_match_ci, regex_not_match, regex_not_match_ci,
};
use drizzle::postgres::prelude::*;

#[PostgresTable(NAME = "pg_expression_rows")]
struct ExpressionRow {
    #[column(serial, primary)]
    id: i32,
    name: String,
    quantity: Option<i32>,
    active: bool,
}

/// `VAR_POP`, `VAR_SAMP`, `VARIANCE`, `STDDEV_POP`, `STDDEV_SAMP`.
type Statistics = (
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
    Option<f64>,
);

#[derive(PostgresSchema)]
struct ExpressionSchema {
    rows: ExpressionRow,
}

#[drizzle::test]
fn string_slicing_and_padding(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", true),
            InsertExpressionRow::new("Bob", false),
            InsertExpressionRow::new("carol", true),
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
        .r#where(eq(rows.name, "alice"))
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

    let reshaped: (String, String, String) = db
        .select((
            initcap(rows.name),
            split_part(rows.name, "i", 2),
            translate(rows.name, "ace", "ACE"),
        ))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(
        reshaped,
        ("Alice".to_string(), "ce".to_string(), "AliCE".to_string())
    );

    let with_prefix: Vec<String> = db
        .select(rows.name)
        .from(rows)
        .r#where(starts_with(rows.name, "ca"))
        .all();
    assert_eq!(with_prefix, ["carol"]);
}

#[drizzle::test]
fn pattern_matching_operators(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", true),
            InsertExpressionRow::new("Bob", false),
            InsertExpressionRow::new("carol", true),
        ])
        .execute();

    let case_insensitive: Vec<String> = db
        .select(rows.name)
        .from(rows)
        .r#where(ilike(rows.name, "B%"))
        .all();
    assert_eq!(case_insensitive, ["Bob"]);

    let mut others: Vec<String> = db
        .select(rows.name)
        .from(rows)
        .r#where(not_ilike(rows.name, "b%"))
        .all();
    others.sort_unstable();
    assert_eq!(others, ["alice", "carol"]);

    let sensitive: Vec<String> = db
        .select(rows.name)
        .from(rows)
        .r#where(regex_match(rows.name, "^b"))
        .all();
    assert!(sensitive.is_empty());

    let insensitive: Vec<String> = db
        .select(rows.name)
        .from(rows)
        .r#where(regex_match_ci(rows.name, "^b"))
        .all();
    assert_eq!(insensitive, ["Bob"]);

    let mut not_a: Vec<String> = db
        .select(rows.name)
        .from(rows)
        .r#where(regex_not_match(rows.name, "^a"))
        .all();
    not_a.sort_unstable();
    assert_eq!(not_a, ["Bob", "carol"]);

    let mut not_b: Vec<String> = db
        .select(rows.name)
        .from(rows)
        .r#where(regex_not_match_ci(rows.name, "^b"))
        .all();
    not_b.sort_unstable();
    assert_eq!(not_b, ["alice", "carol"]);
}

#[drizzle::test]
fn regular_expression_functions(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", true),
            InsertExpressionRow::new("Bob", false),
            InsertExpressionRow::new("carol", true),
        ])
        .execute();

    let replaced: (String, String) = db
        .select((
            regexp_replace(rows.name, "l+", "L"),
            regexp_replace_flags(rows.name, "[aeiou]", "_", "g"),
        ))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(replaced, ("aLice".to_string(), "_l_c_".to_string()));

    let captured: (Option<Vec<String>>, Option<Vec<String>>) = db
        .select((
            regexp_match(rows.name, "(a)(l)"),
            regexp_match_flags(rows.name, "(B)", "i"),
        ))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(
        captured,
        (Some(vec!["a".to_string(), "l".to_string()]), None)
    );
}

#[drizzle::test]
fn extrema_and_constants(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", true),
            InsertExpressionRow::new("Bob", false),
            InsertExpressionRow::new("carol", true),
        ])
        .execute();
    db.update(rows)
        .set(UpdateExpressionRow::default().with_quantity(7))
        .r#where(eq(rows.name, "alice"))
        .execute();

    // PostgreSQL ignores NULL arguments in GREATEST / LEAST.
    let bounds: Vec<(i32, i32)> = db
        .select((greatest(rows.quantity, 5), least(rows.quantity, 5)))
        .from(rows)
        .order_by(asc(rows.id))
        .all();
    assert_eq!(bounds, [(7, 5), (5, 5), (5, 5)]);

    let approximately_pi: i64 = db
        .select(count(rows.id))
        .from(rows)
        .r#where(gt(pi(), 3.1))
        .get();
    assert_eq!(approximately_pi, 3);
}

#[drizzle::test]
fn statistical_boolean_and_collecting_aggregates(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", true),
            InsertExpressionRow::new("Bob", false),
            InsertExpressionRow::new("carol", true),
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

    let booleans: (Option<bool>, Option<bool>, Option<bool>) = db
        .select((
            bool_and(rows.active),
            bool_or(rows.active),
            every(rows.active),
        ))
        .from(rows)
        .get();
    assert_eq!(booleans, (Some(false), Some(true), Some(false)));

    let collected: (Option<String>, Option<Vec<String>>) = db
        .select((string_agg(rows.name, ","), array_agg(rows.name)))
        .from(rows)
        .r#where(eq(rows.active, true))
        .get();
    let (joined, gathered) = collected;
    let mut joined: Vec<&str> = joined.as_deref().unwrap().split(',').collect();
    joined.sort_unstable();
    assert_eq!(joined, ["alice", "carol"]);
    let mut gathered = gathered.unwrap();
    gathered.sort_unstable();
    assert_eq!(gathered, ["alice", "carol"]);

    let filtered: (i64, i64) = db
        .select((
            count(rows.id).filter(eq(rows.active, true)),
            count(rows.id).filter(eq(rows.active, false)),
        ))
        .from(rows)
        .get();
    assert_eq!(filtered, (2, 1));
}

#[cfg(feature = "serde")]
#[drizzle::test]
fn json_aggregates(db: &mut TestDb<ExpressionSchema>) {
    use drizzle::core::expr::{json_agg, json_object_agg, jsonb_agg, jsonb_object_agg};

    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", true),
            InsertExpressionRow::new("Bob", false),
            InsertExpressionRow::new("carol", true),
        ])
        .execute();

    let arrays: (Option<serde_json::Value>, Option<serde_json::Value>) = db
        .select((json_agg(rows.name), jsonb_agg(rows.name)))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(
        arrays,
        (
            Some(serde_json::json!(["alice"])),
            Some(serde_json::json!(["alice"]))
        )
    );

    let objects: (Option<serde_json::Value>, Option<serde_json::Value>) = db
        .select((
            json_object_agg(rows.name, rows.active),
            jsonb_object_agg(rows.name, rows.active),
        ))
        .from(rows)
        .r#where(eq(rows.name, "Bob"))
        .get();
    assert_eq!(
        objects,
        (
            Some(serde_json::json!({"Bob": false})),
            Some(serde_json::json!({"Bob": false}))
        )
    );
}

#[cfg(feature = "serde")]
mod json_operators {
    use super::*;
    use drizzle::postgres::expr::{
        json_get, json_get_idx, json_get_path, json_get_path_text, json_get_text,
        json_get_text_idx, jsonb_contained, jsonb_contains, jsonb_exists_all, jsonb_exists_any,
        jsonb_exists_key,
    };

    #[PostgresTable(NAME = "pg_expression_documents")]
    struct Document {
        #[column(serial, primary)]
        id: i32,
        #[column(jsonb)]
        payload: serde_json::Value,
    }

    #[derive(PostgresSchema)]
    struct DocumentSchema {
        documents: Document,
    }

    #[drizzle::test]
    fn json_path_and_containment_operators(db: &mut TestDb<DocumentSchema>) {
        let DocumentSchema { documents } = schema;
        db.insert(documents)
            .value(InsertDocument::new(serde_json::json!({
                "kind": "note",
                "tags": ["a", "b"],
                "meta": {"level": 3}
            })))
            .execute();

        /// `->>`, `->`, `-> idx`, `->> idx`, `#>`, `#>>` results.
        type Extracted = (
            Option<String>,
            Option<serde_json::Value>,
            Option<serde_json::Value>,
            Option<String>,
            Option<serde_json::Value>,
            Option<String>,
        );
        let extracted: Extracted = db
            .select((
                json_get_text(documents.payload, "kind"),
                json_get(documents.payload, "meta"),
                json_get_idx(json_get(documents.payload, "tags"), 1),
                json_get_text_idx(json_get(documents.payload, "tags"), 0),
                json_get_path(documents.payload, "{meta,level}"),
                json_get_path_text(documents.payload, "{meta,level}"),
            ))
            .from(documents)
            .get();
        assert_eq!(
            extracted,
            (
                Some("note".to_string()),
                Some(serde_json::json!({"level": 3})),
                Some(serde_json::json!("b")),
                Some("a".to_string()),
                Some(serde_json::json!(3)),
                Some("3".to_string()),
            )
        );

        let contains: i64 = db
            .select(count(documents.id))
            .from(documents)
            .r#where(jsonb_contains(
                documents.payload,
                SQL::param(PostgresValue::from(serde_json::json!({"kind": "note"}))),
            ))
            .get();
        assert_eq!(contains, 1);

        let contained: i64 = db
            .select(count(documents.id))
            .from(documents)
            .r#where(jsonb_contained(
                json_get(documents.payload, "meta"),
                SQL::param(PostgresValue::from(
                    serde_json::json!({"level": 3, "extra": true}),
                )),
            ))
            .get();
        assert_eq!(contained, 1);

        let keys: (i64, i64, i64) = db
            .select((
                count(documents.id).filter(jsonb_exists_key(documents.payload, "kind")),
                count(documents.id).filter(jsonb_exists_any(documents.payload, &["nope", "tags"])),
                count(documents.id).filter(jsonb_exists_all(documents.payload, &["kind", "nope"])),
            ))
            .from(documents)
            .get();
        assert_eq!(keys, (1, 1, 0));
    }
}

#[drizzle::test]
fn temporal_functions(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", true),
            InsertExpressionRow::new("Bob", false),
            InsertExpressionRow::new("carol", true),
        ])
        .execute();

    // Clock functions return server types; the contract is that they are
    // accepted and non-NULL.
    let clocks: i64 = db
        .select(count(rows.id))
        .from(rows)
        .r#where(and(is_not_null(now()), is_not_null(localtime())))
        .get();
    assert_eq!(clocks, 3);
    let clocks: i64 = db
        .select(count(rows.id))
        .from(rows)
        .r#where(and(
            is_not_null(localtimestamp()),
            is_not_null(clock_timestamp()),
        ))
        .get();
    assert_eq!(clocks, 3);

    let year: String = db
        .select(to_char(now(), "YYYY"))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(year.len(), 4);

    let constructed: (String, String, String) = db
        .select((
            to_char(make_date(2024, 2, 29), "YYYY-MM-DD"),
            to_char(make_timestamp(2024, 2, 29, 13, 45, 30), "HH24:MI:SS"),
            to_char(to_date("29 Feb 2024", "DD Mon YYYY"), "YYYY-MM-DD"),
        ))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(
        constructed,
        (
            "2024-02-29".to_string(),
            "13:45:30".to_string(),
            "2024-02-29".to_string()
        )
    );

    let truncated: (String, f64) = db
        .select((
            to_char(
                date_trunc("year", make_timestamp(2024, 2, 29, 13, 45, 30)),
                "YYYY-MM-DD",
            ),
            extract("day", make_timestamp(2024, 2, 29, 13, 45, 30)),
        ))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(truncated, ("2024-01-01".to_string(), 29.0));

    let epoch: String = db
        .select(to_char(to_timestamp(86_400), "YYYY-MM-DD"))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(epoch, "1970-01-02");

    let parsed: i64 = db
        .select(count(rows.id))
        .from(rows)
        .r#where(eq(to_number("1,234.50", "9,999.99"), 1234.5))
        .get();
    assert_eq!(parsed, 3);

    let spans: i64 = db
        .select(count(rows.id))
        .from(rows)
        .r#where(and(
            is_not_null(age(
                make_timestamp(2024, 2, 29, 0, 0, 0),
                make_timestamp(2023, 1, 1, 0, 0, 0),
            )),
            is_not_null(date_bin(
                "15 minutes",
                make_timestamp(2024, 2, 29, 13, 45, 30),
                make_timestamp(2024, 1, 1, 0, 0, 0),
            )),
        ))
        .get();
    assert_eq!(spans, 3);
}

#[drizzle::test]
fn sequence_functions(db: &mut TestDb<ExpressionSchema>) {
    let ExpressionSchema { rows } = schema;
    db.insert(rows)
        .values([
            InsertExpressionRow::new("alice", true),
            InsertExpressionRow::new("Bob", false),
            InsertExpressionRow::new("carol", true),
        ])
        .execute();

    let sequence = "pg_expression_rows_id_seq";
    let reset: i64 = db
        .select(setval(sequence, 100))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(reset, 100);

    let advanced: i64 = db
        .select(nextval(sequence))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(advanced, 101);

    let current: i64 = db
        .select(currval(sequence))
        .from(rows)
        .r#where(eq(rows.name, "alice"))
        .get();
    assert_eq!(current, 101);

    let next_id: i32 = db
        .insert(rows)
        .values([InsertExpressionRow::new("dave", true)])
        .returning(rows.id)
        .get();
    assert_eq!(next_id, 102);
}
