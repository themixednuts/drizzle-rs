/// Portable subquery behavior: scalar subqueries as comparison operands,
/// nested subqueries, `IN (SELECT ...)` with single columns and row values,
/// correlated `EXISTS`, and the parenthesization every dialect must render.
///
/// Dialect-specific quirks (SQLite reading the first row of a multi-row scalar
/// subquery, CTE-backed subqueries on PostgreSQL) stay in the dialect modules.
macro_rules! shared_subquery_suite {
    ($dialect:ident, $table:ident, $schema:ident, $from_row:ident) => {
        mod shared_subquery {
            use super::*;
            use crate::common::helpers::sql_shape;
            use drizzle::core::expr::{
                eq, exists, gt, in_subquery, lt, max, min, not_exists, not_in_subquery,
            };

            #[$table(NAME = "shared_subquery_rows")]
            struct SharedSubqueryRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                score: i32,
            }

            #[$table(NAME = "shared_subquery_tags")]
            struct SharedSubqueryTag {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(references = SharedSubqueryRow::id)]
                row_id: i32,
                label: String,
            }

            #[derive($schema)]
            struct SharedSubquerySchema {
                rows: SharedSubqueryRow,
                tags: SharedSubqueryTag,
            }

            #[derive(Debug, $from_row)]
            struct RowSummary {
                id: i32,
                name: String,
            }

            type RowSeed = InsertSharedSubqueryRow<
                'static,
                (
                    SharedSubqueryRowIdSet,
                    SharedSubqueryRowNameSet,
                    SharedSubqueryRowScoreSet,
                ),
            >;

            fn row_seed() -> [RowSeed; 4] {
                [
                    InsertSharedSubqueryRow::new("alice", 10).with_id(1),
                    InsertSharedSubqueryRow::new("bob", 20).with_id(2),
                    InsertSharedSubqueryRow::new("charlie", 30).with_id(3),
                    InsertSharedSubqueryRow::new("dana", 40).with_id(4),
                ]
            }

            /// Names in id order, so unordered result sets compare stably.
            fn names(mut rows: Vec<RowSummary>) -> Vec<String> {
                rows.sort_by_key(|row| row.id);
                rows.into_iter().map(|row| row.name).collect()
            }

            #[drizzle::test($dialect)]
            fn scalar_subqueries_compare_against_aggregates(
                db: &mut TestDb<SharedSubquerySchema>,
            ) {
                let SharedSubquerySchema { rows, .. } = schema;
                db.insert(rows).values(row_seed()).execute();
                let qb = drizzle::$dialect::builder::QueryBuilder::new::<SharedSubquerySchema>();

                let min_id = qb.select(min(rows.id)).from(rows);
                let above_min: Vec<RowSummary> = db
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(gt(rows.id, min_id))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(names(above_min), ["bob", "charlie", "dana"]);

                let max_score = qb.select(max(rows.score)).from(rows);
                let below_max: Vec<RowSummary> = db
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(lt(rows.score, max_score))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(names(below_max), ["alice", "bob", "charlie"]);
            }

            #[drizzle::test($dialect)]
            fn nested_subqueries_compose(db: &mut TestDb<SharedSubquerySchema>) {
                let SharedSubquerySchema { rows, .. } = schema;
                db.insert(rows).values(row_seed()).execute();
                let qb = drizzle::$dialect::builder::QueryBuilder::new::<SharedSubquerySchema>();

                // ids whose score beats the lowest score → everyone but alice
                let min_score = qb.select(min(rows.score)).from(rows);
                let above_min = qb
                    .select(rows.id)
                    .from(rows)
                    .r#where(gt(rows.score, min_score));
                // ...and of those, the lowest id → bob
                let lowest_above_min = qb.select(min(rows.id)).from(rows).r#where(in_subquery(
                    rows.id,
                    above_min,
                ));

                let found: Vec<RowSummary> = db
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(eq(rows.id, lowest_above_min))
                    .all();
                assert_eq!(names(found), ["bob"]);
            }

            #[drizzle::test($dialect)]
            fn in_subqueries_match_single_columns_and_row_values(
                db: &mut TestDb<SharedSubquerySchema>,
            ) {
                let SharedSubquerySchema { rows, .. } = schema;
                db.insert(rows).values(row_seed()).execute();
                let qb = drizzle::$dialect::builder::QueryBuilder::new::<SharedSubquerySchema>();

                let bob_id = qb
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(rows.name, "bob"));
                let single: Vec<RowSummary> = db
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(in_subquery(rows.id, bob_id))
                    .all();
                assert_eq!(names(single), ["bob"]);

                let bob_row = qb
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(eq(rows.name, "bob"));
                let row_value: Vec<RowSummary> = db
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(in_subquery((rows.id, rows.name), bob_row))
                    .all();
                assert_eq!(names(row_value), ["bob"]);

                let high_scores = qb
                    .select(rows.id)
                    .from(rows)
                    .r#where(gt(rows.score, 20));
                let excluded: Vec<RowSummary> = db
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(not_in_subquery(rows.id, high_scores))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(names(excluded), ["alice", "bob"]);
            }

            #[drizzle::test($dialect)]
            fn exists_subqueries_correlate_with_the_outer_row(
                db: &mut TestDb<SharedSubquerySchema>,
            ) {
                let SharedSubquerySchema { rows, tags } = schema;
                db.insert(rows).values(row_seed()).execute();
                db.insert(tags)
                    .values([
                        InsertSharedSubqueryTag::new(1, "vip").with_id(1),
                        InsertSharedSubqueryTag::new(1, "early").with_id(2),
                        InsertSharedSubqueryTag::new(3, "vip").with_id(3),
                    ])
                    .execute();
                let qb = drizzle::$dialect::builder::QueryBuilder::new::<SharedSubquerySchema>();

                let has_tag = qb
                    .select(tags.id)
                    .from(tags)
                    .r#where(eq(tags.row_id, rows.id));
                let tagged: Vec<RowSummary> = db
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(exists(has_tag))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(names(tagged), ["alice", "charlie"]);

                let has_tag = qb
                    .select(tags.id)
                    .from(tags)
                    .r#where(eq(tags.row_id, rows.id));
                let untagged: Vec<RowSummary> = db
                    .select((rows.id, rows.name))
                    .from(rows)
                    .r#where(not_exists(has_tag))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(names(untagged), ["bob", "dana"]);
            }

            #[drizzle::test($dialect)]
            fn subqueries_render_parenthesized(db: &mut TestDb<SharedSubquerySchema>) {
                let SharedSubquerySchema { rows, tags } = schema;
                let qb = drizzle::$dialect::builder::QueryBuilder::new::<SharedSubquerySchema>();

                let min_id = qb.select(min(rows.id)).from(rows);
                let comparison = sql_shape(
                    &db.select(rows.id)
                        .from(rows)
                        .r#where(gt(rows.id, min_id))
                        .to_sql()
                        .sql(),
                );
                assert!(
                    comparison
                        .contains("WHEREshared_subquery_rows.id>(SELECTMIN(shared_subquery_rows.id)FROMshared_subquery_rows)"),
                    "{comparison}"
                );

                let ids = qb.select(rows.id).from(rows);
                let membership = sql_shape(
                    &db.select(rows.id)
                        .from(rows)
                        .r#where(in_subquery(rows.id, ids))
                        .to_sql()
                        .sql(),
                );
                assert!(
                    membership.contains("WHEREshared_subquery_rows.idIN(SELECTshared_subquery_rows.idFROMshared_subquery_rows)"),
                    "{membership}"
                );

                let has_tag = qb
                    .select(tags.id)
                    .from(tags)
                    .r#where(eq(tags.row_id, rows.id));
                let existence = sql_shape(
                    &db.select(rows.id)
                        .from(rows)
                        .r#where(exists(has_tag))
                        .to_sql()
                        .sql(),
                );
                assert!(
                    existence.contains("WHEREEXISTS(SELECTshared_subquery_tags.idFROMshared_subquery_tagsWHEREshared_subquery_tags.row_id=shared_subquery_rows.id)"),
                    "{existence}"
                );
            }
        }
    };
}

pub(crate) use shared_subquery_suite;
