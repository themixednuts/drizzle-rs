/// Expression functions that every dialect supports, executed against the
/// database rather than only rendered.
///
/// The point of running these is to catch SQL that renders but the server
/// rejects (a function the dialect lacks, an argument shape it will not
/// accept) — the type system cannot see that. Functions whose result type
/// differs per dialect (`length`, `sum`, ...) are asserted through a WHERE
/// filter and a portable `count`, so no dialect-specific decode is needed.
/// Dialect-only functions live in each dialect's `expressions.rs`.
macro_rules! shared_expression_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_expressions {
            use super::*;
            use drizzle::core::expr::{
                FrameBound, abs, and, between, char_length, coalesce, coalesce_many, concat,
                concat_ws, count, count_distinct, cume_dist, current_date, current_time,
                current_timestamp, dense_rank, eq, exists, first_value, gt, gte, in_subquery,
                is_distinct_from, is_false, is_not_distinct_from, is_not_null, is_null, lag,
                lag_with_default, last_value, lead, lead_with_default, length, like, lower, ltrim,
                max, min, mod_, ne, not_between, not_exists, not_in_array, not_in_subquery,
                not_like, nth_value, ntile, nullif, octet_length, percent_rank, random, rank,
                replace, round, round_to, row_number, rtrim, sign, string_concat, substr,
                sum_distinct, trim, upper, window,
            };

            #[$table(NAME = "shared_expression_rows")]
            struct SharedExpressionRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                score: f64,
                quantity: Option<i32>,
                active: bool,
            }

            #[$table(NAME = "shared_expression_tags")]
            struct SharedExpressionTag {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                row_id: i32,
                label: String,
            }

            #[derive($schema)]
            struct SharedExpressionSchema {
                rows: SharedExpressionRow,
                tags: SharedExpressionTag,
            }

            /// The standard fixture: `id`, `name`, `score`, `quantity`, `active`.
            ///
            /// Row 2 (`bob`) leaves `quantity` NULL and is inserted separately
            /// because an unset optional column is a different insert typestate.
            type FixtureRow = InsertSharedExpressionRow<
                'static,
                (
                    SharedExpressionRowIdSet,
                    SharedExpressionRowNameSet,
                    SharedExpressionRowScoreSet,
                    SharedExpressionRowQuantitySet,
                    SharedExpressionRowActiveSet,
                ),
            >;

            fn fixture() -> [FixtureRow; 3] {
                [
                    (1, "  Alice  ", 2.5, 4, true),
                    (3, "Carol", 9.0, 4, true),
                    (4, "dave", 0.5, 10, false),
                ]
                .map(|(id, name, score, quantity, active)| {
                    InsertSharedExpressionRow::new(name, score, active)
                        .with_id(id)
                        .with_quantity(quantity)
                })
            }

            fn bob() -> InsertSharedExpressionRow<
                'static,
                (
                    SharedExpressionRowIdSet,
                    SharedExpressionRowNameSet,
                    SharedExpressionRowScoreSet,
                    SharedExpressionRowQuantityNotSet,
                    SharedExpressionRowActiveSet,
                ),
            > {
                InsertSharedExpressionRow::new("bob", -3.75, false).with_id(2)
            }

            #[drizzle::test($dialect)]
            fn rounding_functions(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                let absolute: Vec<f64> = db
                    .select(abs(rows.score))
                    .from(rows)
                    .r#where(eq(rows.id, 2))
                    .all();
                assert_eq!(absolute, [3.75]);

                let rounded: Vec<f64> = db
                    .select(round(rows.score))
                    .from(rows)
                    .r#where(eq(rows.id, 2))
                    .all();
                assert_eq!(rounded, [-4.0]);

                // ROUND(x, n) is numeric on PostgreSQL, so compare in SQL.
                let rounded_to: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(between(round_to(rows.score, 1), -3.85, -3.75))
                    .all();
                assert_eq!(rounded_to, [2]);

                // SIGN is an integer on SQLite and MySQL and a double on
                // PostgreSQL, so compare it in SQL.
                let negatives: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(sign(rows.score), -1))
                    .all();
                assert_eq!(negatives, [2]);

                let remainders: Vec<i32> = db
                    .select(mod_(rows.id, 3))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(remainders, [1, 2, 0, 1]);
            }

            #[drizzle::test($dialect)]
            fn random_is_accepted(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                // `random()` differs per dialect in range and type; it only needs
                // to be accepted and non-null.
                let randoms: i64 = db
                    .select(count(rows.id))
                    .from(rows)
                    .r#where(is_not_null(random()))
                    .get();
                assert_eq!(randoms, 4);
            }

            #[drizzle::test($dialect)]
            fn case_and_trim_functions(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                let shouted: Vec<String> = db
                    .select(upper(rows.name))
                    .from(rows)
                    .r#where(eq(rows.id, 2))
                    .all();
                assert_eq!(shouted, ["BOB"]);

                let hushed: Vec<String> = db
                    .select(lower(rows.name))
                    .from(rows)
                    .r#where(eq(rows.id, 3))
                    .all();
                assert_eq!(hushed, ["carol"]);

                let trimmed: Vec<(String, String, String)> = db
                    .select((trim(rows.name), ltrim(rows.name), rtrim(rows.name)))
                    .from(rows)
                    .r#where(eq(rows.id, 1))
                    .all();
                assert_eq!(
                    trimmed,
                    [(
                        "Alice".to_string(),
                        "Alice  ".to_string(),
                        "  Alice".to_string()
                    )]
                );
            }

            #[drizzle::test($dialect)]
            fn length_functions(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                // The integer width of a length differs per dialect, so match
                // through a filter instead of decoding it.
                let nine_wide: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(length(rows.name), 9))
                    .all();
                assert_eq!(nine_wide, [1]);

                let three_chars: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(char_length(rows.name), 3))
                    .all();
                assert_eq!(three_chars, [2]);

                let four_bytes: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(octet_length(rows.name), 4))
                    .all();
                assert_eq!(four_bytes, [4]);
            }

            #[drizzle::test($dialect)]
            fn substring_and_concatenation_functions(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                let prefixes: Vec<String> = db
                    .select(substr(rows.name, 1, 3))
                    .from(rows)
                    .r#where(eq(rows.id, 3))
                    .all();
                assert_eq!(prefixes, ["Car"]);

                let replaced: Vec<String> = db
                    .select(replace(rows.name, "o", "0"))
                    .from(rows)
                    .r#where(eq(rows.id, 2))
                    .all();
                assert_eq!(replaced, ["b0b"]);

                let joined: Vec<String> = db
                    .select(concat(rows.name, "!"))
                    .from(rows)
                    .r#where(eq(rows.id, 2))
                    .all();
                assert_eq!(joined, ["bob!"]);

                let operator_joined: Vec<String> = db
                    .select(string_concat(rows.name, "?"))
                    .from(rows)
                    .r#where(eq(rows.id, 4))
                    .all();
                assert_eq!(operator_joined, ["dave?"]);

                let separated: Vec<String> = db
                    .select(concat_ws("-", [rows.name, rows.name]))
                    .from(rows)
                    .r#where(eq(rows.id, 2))
                    .all();
                assert_eq!(separated, ["bob-bob"]);
            }

            #[drizzle::test($dialect)]
            fn null_handling_functions(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                let defaulted: Vec<i32> = db
                    .select(coalesce(rows.quantity, 0))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(defaulted, [4, 0, 4, 10]);

                let chained: Vec<i32> = db
                    .select(coalesce_many(rows.quantity, [rows.id]))
                    .from(rows)
                    .r#where(eq(rows.id, 2))
                    .all();
                assert_eq!(chained, [2]);

                let nulled: Vec<Option<i32>> = db
                    .select(nullif(rows.quantity, 4))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(nulled, [None, None, None, Some(10)]);

                let distinct_from_four: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(is_distinct_from(rows.quantity, 4))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(distinct_from_four, [2, 4]);

                let not_distinct_from_four: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(is_not_distinct_from(rows.quantity, 4))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(not_distinct_from_four, [1, 3]);

                let nulls: i64 = db
                    .select(count(rows.id))
                    .from(rows)
                    .r#where(and(is_null(rows.quantity), is_false(rows.active)))
                    .get();
                assert_eq!(nulls, 1);
            }

            #[drizzle::test($dialect)]
            fn comparison_variants(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                let not_bob: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(ne(rows.name, "bob"))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(not_bob, [1, 3, 4]);

                let no_lowercase_b: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(not_like(rows.name, "b%"))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(no_lowercase_b, [1, 3, 4]);

                let outside: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(not_between(rows.score, 0.0, 5.0))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(outside, [2, 3]);

                let inside: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    // LIKE is case-insensitive on SQLite and MySQL but not on
                    // PostgreSQL, so the pattern avoids mixed-case names.
                    .r#where(and(between(rows.score, 0.0, 5.0), like(rows.name, "d%")))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(inside, [4]);

                let excluded_ids: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(not_in_array(rows.id, [1, 4]))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(excluded_ids, [2, 3]);

                let bounded: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(and(gte(rows.id, 2), gt(rows.score, 0.0)))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(bounded, [3, 4]);
            }

            #[drizzle::test($dialect)]
            fn subquery_predicates(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, tags } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();
                db.insert(tags)
                    .values([
                        InsertSharedExpressionTag::new(1, "vip").with_id(1),
                        InsertSharedExpressionTag::new(3, "vip").with_id(2),
                        InsertSharedExpressionTag::new(3, "staff").with_id(3),
                    ])
                    .execute();

                let qb = drizzle::$dialect::builder::QueryBuilder::new::<SharedExpressionSchema>();

                let tagged: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(in_subquery(
                        rows.id,
                        qb.select(tags.row_id)
                            .from(tags)
                            .r#where(eq(tags.label, "vip")),
                    ))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(tagged, [1, 3]);

                let untagged: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(not_in_subquery(rows.id, qb.select(tags.row_id).from(tags)))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(untagged, [2, 4]);

                let with_any_tag: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(exists(
                        qb.select(tags.id)
                            .from(tags)
                            .r#where(eq(tags.row_id, rows.id)),
                    ))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(with_any_tag, [1, 3]);

                let without_tags: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(not_exists(
                        qb.select(tags.id)
                            .from(tags)
                            .r#where(eq(tags.row_id, rows.id)),
                    ))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(without_tags, [2, 4]);
            }

            #[drizzle::test($dialect)]
            fn distinct_aggregates(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                let distinct_quantities: i64 =
                    db.select(count_distinct(rows.quantity)).from(rows).get();
                assert_eq!(distinct_quantities, 2);

                let extremes: (Option<i32>, Option<i32>) = db
                    .select((min(rows.quantity), max(rows.quantity)))
                    .from(rows)
                    .get();
                assert_eq!(extremes, (Some(4), Some(10)));

                // Active rows hold quantities 4 and 4: SUM(DISTINCT) counts the
                // duplicate once. Its numeric type differs per dialect, so
                // compare in SQL.
                let sums_to_four: Vec<bool> = db
                    .select(rows.active)
                    .from(rows)
                    .group_by(rows.active)
                    .having(eq(sum_distinct(rows.quantity), 4))
                    .all();
                assert_eq!(sums_to_four, [true]);
            }

            #[drizzle::test($dialect)]
            fn ranking_window_functions(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                // quantity ordering: NULL (bob) sorts first on SQLite/MySQL and
                // last on PostgreSQL, so rank among the non-null rows only.
                let ranked: Vec<(i32, i64, i64, i64)> = db
                    .select((
                        rows.id,
                        rank().over(window().order_by(asc(rows.quantity))),
                        dense_rank().over(window().order_by(asc(rows.quantity))),
                        row_number().over(window().order_by((asc(rows.quantity), asc(rows.id)))),
                    ))
                    .from(rows)
                    .r#where(is_not_null(rows.quantity))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(ranked, [(1, 1, 1, 1), (3, 1, 1, 2), (4, 3, 2, 3)]);

                let buckets: Vec<(i32, i32)> = db
                    .select((rows.id, ntile(2).over(window().order_by(asc(rows.id)))))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(buckets, [(1, 1), (2, 1), (3, 2), (4, 2)]);

                let fractions: Vec<(i32, f64, f64)> = db
                    .select((
                        rows.id,
                        percent_rank().over(window().order_by(asc(rows.id))),
                        cume_dist().over(window().order_by(asc(rows.id))),
                    ))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(
                    fractions,
                    [
                        (1, 0.0, 0.25),
                        (2, 1.0 / 3.0, 0.5),
                        (3, 2.0 / 3.0, 0.75),
                        (4, 1.0, 1.0)
                    ]
                );

                let partitioned: Vec<(i32, i64)> = db
                    .select((
                        rows.id,
                        row_number()
                            .over(window().partition_by([rows.active]).order_by(asc(rows.id))),
                    ))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(partitioned, [(1, 1), (2, 1), (3, 2), (4, 2)]);
            }

            #[drizzle::test($dialect)]
            fn value_window_functions(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                let neighbours: Vec<(i32, Option<i32>, Option<i32>)> = db
                    .select((
                        rows.id,
                        lag(rows.id).over(window().order_by(asc(rows.id))),
                        lead(rows.id).over(window().order_by(asc(rows.id))),
                    ))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(
                    neighbours,
                    [
                        (1, None, Some(2)),
                        (2, Some(1), Some(3)),
                        (3, Some(2), Some(4)),
                        (4, Some(3), None)
                    ]
                );

                let defaulted: Vec<(i32, i32, i32)> = db
                    .select((
                        rows.id,
                        lag_with_default(rows.id, 2, 0).over(window().order_by(asc(rows.id))),
                        lead_with_default(rows.id, 2, 0).over(window().order_by(asc(rows.id))),
                    ))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(defaulted, [(1, 0, 3), (2, 0, 4), (3, 1, 0), (4, 2, 0)]);

                let frame = || {
                    window().order_by(asc(rows.id)).rows_between(
                        FrameBound::UnboundedPreceding,
                        FrameBound::UnboundedFollowing,
                    )
                };
                let extremes: Vec<(i32, Option<i32>, Option<i32>, Option<i32>)> = db
                    .select((
                        rows.id,
                        first_value(rows.id).over(frame()),
                        last_value(rows.id).over(frame()),
                        nth_value(rows.id, 2).over(frame()),
                    ))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(
                    extremes,
                    [
                        (1, Some(1), Some(4), Some(2)),
                        (2, Some(1), Some(4), Some(2)),
                        (3, Some(1), Some(4), Some(2)),
                        (4, Some(1), Some(4), Some(2))
                    ]
                );

                let trailing: Vec<(i32, Option<i32>)> = db
                    .select((
                        rows.id,
                        first_value(rows.id).over(
                            window()
                                .order_by(asc(rows.id))
                                .range_between(FrameBound::Preceding(1), FrameBound::CurrentRow),
                        ),
                    ))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(
                    trailing,
                    [(1, Some(1)), (2, Some(1)), (3, Some(2)), (4, Some(3))]
                );
            }

            #[drizzle::test($dialect)]
            fn current_time_functions(db: &mut TestDb<SharedExpressionSchema>) {
                let SharedExpressionSchema { rows, .. } = schema;
                db.insert(rows).values(fixture()).execute();
                db.insert(rows).value(bob()).execute();

                // Each dialect returns its own temporal type; the contract here is
                // that the expression is accepted and never NULL.
                let present: i64 = db
                    .select(count(rows.id))
                    .from(rows)
                    .r#where(and(
                        and(is_not_null(current_date()), is_not_null(current_time())),
                        is_not_null(current_timestamp()),
                    ))
                    .get();
                assert_eq!(present, 4);
            }
        }
    };
}

pub(crate) use shared_expression_suite;

/// Math functions behind `MathExt`.
///
/// `CEIL`, `FLOOR`, `TRUNC`, `SQRT`, `POWER`, `EXP`, `LN` and `LOG` need a
/// SQLite compiled with `SQLITE_ENABLE_MATH_FUNCTIONS`. PostgreSQL and MySQL
/// always run this suite; SQLite runs it under the `math` feature, on
/// rusqlite only, because `.cargo/config.toml` can pass the flag to
/// libsqlite3-sys but not to libsql-ffi or turso.
#[cfg(any(feature = "postgres", feature = "mysql", feature = "math"))]
macro_rules! shared_math_extension_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_math_extensions {
            use super::*;
            use drizzle::core::expr::{
                ceil, count, eq, exp, floor, gt, gte, ln, log, log10, power, sqrt, trunc,
            };

            #[$table(NAME = "shared_math_rows")]
            struct SharedMathRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                score: f64,
                quantity: Option<i32>,
            }

            #[derive($schema)]
            struct SharedMathSchema {
                rows: SharedMathRow,
            }

            #[drizzle::test($dialect)]
            fn rounding_to_integers(db: &mut TestDb<SharedMathSchema>) {
                let SharedMathSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedMathRow::new(2.5).with_id(1).with_quantity(4),
                        InsertSharedMathRow::new(3.75).with_id(2).with_quantity(1),
                    ])
                    .execute();

                let ceilings: Vec<f64> = db
                    .select(ceil(rows.score))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(ceilings, [3.0, 4.0]);

                let floors: Vec<f64> = db
                    .select(floor(rows.score))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(floors, [2.0, 3.0]);

                let truncated: Vec<f64> = db
                    .select(trunc(rows.score))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(truncated, [2.0, 3.0]);
            }

            #[drizzle::test($dialect)]
            fn exponential_functions(db: &mut TestDb<SharedMathSchema>) {
                let SharedMathSchema { rows } = schema;
                // Scores stay positive: PostgreSQL evaluates SQRT / LN for every
                // row and errors on negative input rather than returning NULL.
                db.insert(rows)
                    .values([
                        InsertSharedMathRow::new(2.5).with_id(1).with_quantity(4),
                        InsertSharedMathRow::new(3.75).with_id(2).with_quantity(1),
                        InsertSharedMathRow::new(9.0).with_id(3).with_quantity(4),
                        InsertSharedMathRow::new(0.5).with_id(4).with_quantity(10),
                    ])
                    .execute();

                // SQRT and LN are NULL-able on MySQL (negative input) and not on
                // PostgreSQL (it errors instead), so compare them in SQL.
                let perfect_square: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(sqrt(rows.score), 3.0))
                    .all();
                assert_eq!(perfect_square, [3]);

                let powers: Vec<f64> = db
                    .select(power(rows.score, 2))
                    .from(rows)
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(powers, [6.25, 14.0625, 81.0, 0.25]);

                let above_e_squared: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(gt(ln(rows.score), 2.0))
                    .all();
                assert_eq!(above_e_squared, [3]);

                // EXP / LOG on integer arguments are numeric on PostgreSQL, so
                // these are compared in SQL rather than decoded.
                let above_e: i64 = db
                    .select(count(rows.id))
                    .from(rows)
                    .r#where(gt(exp(rows.score), std::f64::consts::E))
                    .get();
                assert_eq!(above_e, 3);

                let ten_or_more: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(gte(log10(rows.quantity), 1.0))
                    .all();
                assert_eq!(ten_or_more, [4]);

                // Integer input: PostgreSQL would answer NUMERIC without the cast
                // the builder adds, and the declared type is the dialect double.
                let common_logs: Vec<Option<f64>> = db
                    .select(log10(rows.quantity))
                    .from(rows)
                    .r#where(eq(rows.id, 4))
                    .all();
                assert_eq!(common_logs, [Some(1.0)]);

                let above_eight: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(gt(log(2, rows.score), 3.0))
                    .all();
                assert_eq!(above_eight, [3]);
            }
        }
    };
}

#[cfg(any(feature = "postgres", feature = "mysql", feature = "math"))]
pub(crate) use shared_math_extension_suite;
