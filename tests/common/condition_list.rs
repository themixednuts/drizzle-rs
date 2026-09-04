/// Tuple conjunctions, the `all` / `any` combinators, and `Option` filters.
///
/// Dialects supply their table and schema macros, the value type their
/// expressions carry, and the SQL type marker a condition renders as (SQLite
/// conditions are `Integer`; PostgreSQL and MySQL use `Boolean`). SQL-shape
/// assertions go through `sql_shape` so quoting and placeholder styles do not
/// leak into the suite.
macro_rules! shared_condition_list_suite {
    ($dialect:ident, $table:ident, $schema:ident, $value:ident, $condition:path) => {
        mod shared_condition_list {
            use super::*;
            use crate::common::helpers::sql_shape;
            use drizzle::core::expr::{
                NonNull, SQLExpr, Scalar, all, any, count, eq, gt, gte, lt, neq, not, or,
            };

            #[$table(NAME = "shared_condition_list_rows")]
            struct SharedConditionListRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[$table(NAME = "shared_condition_list_posts")]
            struct SharedConditionListPost {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                title: String,
            }

            #[$table(NAME = "shared_condition_list_categories")]
            struct SharedConditionListCategory {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[derive($schema)]
            struct SharedConditionListSchema {
                rows: SharedConditionListRow,
                posts: SharedConditionListPost,
                categories: SharedConditionListCategory,
            }

            /// A condition expression with no value behind it, for `Option` slots.
            type Condition<'a> = SQLExpr<'a, $value<'a>, $condition, NonNull, Scalar>;

            const ROWS: &str = "shared_condition_list_rows";

            #[drizzle::test($dialect)]
            fn tuple_renders_flat_conjunction(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;

                let sql = db
                    .select(())
                    .from(rows)
                    .r#where((gt(rows.id, 1), lt(rows.id, 3), neq(rows.name, "zed")))
                    .to_sql();

                assert!(
                    sql_shape(&sql.sql())
                        .contains(&format!("WHERE({ROWS}.id>?AND{ROWS}.id<?AND{ROWS}.name<>?)")),
                    "unexpected SQL: {}",
                    sql.sql()
                );
            }

            #[drizzle::test($dialect)]
            fn tuple_where_matches_chained_and(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionListRow::new("alice").with_id(1),
                        InsertSharedConditionListRow::new("bob").with_id(2),
                        InsertSharedConditionListRow::new("carol").with_id(3),
                    ])
                    .execute();

                let selected: Vec<SelectSharedConditionListRow> = db
                    .select(())
                    .from(rows)
                    .r#where((gt(rows.id, 1), lt(rows.id, 3)))
                    .all();

                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].name, "bob");
            }

            #[drizzle::test($dialect)]
            fn all_and_any_render_flat(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;

                let conjunction = db
                    .select(())
                    .from(rows)
                    .r#where(all((gt(rows.id, 1), lt(rows.id, 3))))
                    .to_sql();
                assert!(
                    sql_shape(&conjunction.sql())
                        .contains(&format!("WHERE({ROWS}.id>?AND{ROWS}.id<?)")),
                    "unexpected SQL: {}",
                    conjunction.sql()
                );

                let disjunction = db
                    .select(())
                    .from(rows)
                    .r#where(any((
                        eq(rows.name, "alice"),
                        eq(rows.name, "bob"),
                        eq(rows.name, "carol"),
                    )))
                    .to_sql();
                assert!(
                    sql_shape(&disjunction.sql())
                        .contains(&format!("WHERE({ROWS}.name=?OR{ROWS}.name=?OR{ROWS}.name=?)")),
                    "unexpected SQL: {}",
                    disjunction.sql()
                );
            }

            #[drizzle::test($dialect)]
            fn any_matches_each_alternative(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionListRow::new("alice").with_id(1),
                        InsertSharedConditionListRow::new("bob").with_id(2),
                        InsertSharedConditionListRow::new("carol").with_id(3),
                    ])
                    .execute();

                let selected: Vec<SelectSharedConditionListRow> = db
                    .select(())
                    .from(rows)
                    .r#where(any((eq(rows.name, "alice"), eq(rows.name, "carol"))))
                    .all();

                assert_eq!(selected.len(), 2);
            }

            #[drizzle::test($dialect)]
            fn tuples_nest_inside_or(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;

                let sql = db
                    .select(())
                    .from(rows)
                    .r#where(or(
                        (gt(rows.id, 1), lt(rows.id, 3)),
                        (eq(rows.name, "alice"), gte(rows.id, 1)),
                    ))
                    .to_sql();

                assert!(
                    sql_shape(&sql.sql()).contains(&format!(
                        "WHERE(({ROWS}.id>?AND{ROWS}.id<?)OR({ROWS}.name=?AND{ROWS}.id>=?))"
                    )),
                    "unexpected SQL: {}",
                    sql.sql()
                );
            }

            #[drizzle::test($dialect)]
            fn nested_tuples_group(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;

                let sql = db
                    .select(())
                    .from(rows)
                    .r#where(((gt(rows.id, 1), lt(rows.id, 3)), eq(rows.name, "bob")))
                    .to_sql();

                assert!(
                    sql_shape(&sql.sql())
                        .contains(&format!("WHERE(({ROWS}.id>?AND{ROWS}.id<?)AND{ROWS}.name=?)")),
                    "unexpected SQL: {}",
                    sql.sql()
                );
            }

            #[drizzle::test($dialect)]
            fn option_elements_are_skipped(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;

                let present = db
                    .select(())
                    .from(rows)
                    .r#where((gt(rows.id, 1), Some(eq(rows.name, "bob"))))
                    .to_sql();
                assert!(
                    sql_shape(&present.sql())
                        .contains(&format!("WHERE({ROWS}.id>?AND{ROWS}.name=?)")),
                    "unexpected SQL: {}",
                    present.sql()
                );

                let absent = db
                    .select(())
                    .from(rows)
                    .r#where((gt(rows.id, 1), None::<Condition<'_>>))
                    .to_sql();
                assert!(
                    sql_shape(&absent.sql()).contains(&format!("WHERE({ROWS}.id>?)")),
                    "unexpected SQL: {}",
                    absent.sql()
                );
            }

            #[drizzle::test($dialect)]
            fn optional_filter_round_trip(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionListRow::new("alice").with_id(1),
                        InsertSharedConditionListRow::new("bob").with_id(2),
                        InsertSharedConditionListRow::new("carol").with_id(3),
                    ])
                    .execute();

                let unfiltered: Vec<SelectSharedConditionListRow> = db
                    .select(())
                    .from(rows)
                    .r#where((gte(rows.id, 1), None::<_>.map(|n: &str| eq(rows.name, n))))
                    .all();
                assert_eq!(unfiltered.len(), 3);

                let filtered: Vec<SelectSharedConditionListRow> = db
                    .select(())
                    .from(rows)
                    .r#where((gte(rows.id, 1), Some("bob").map(|n| eq(rows.name, n))))
                    .all();
                assert_eq!(filtered.len(), 1);
                assert_eq!(filtered[0].name, "bob");
            }

            #[drizzle::test($dialect)]
            fn all_none_conjunction_is_true(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionListRow::new("alice").with_id(1),
                        InsertSharedConditionListRow::new("bob").with_id(2),
                    ])
                    .execute();

                let sql = db
                    .select(())
                    .from(rows)
                    .r#where((None::<Condition<'_>>, None::<Condition<'_>>))
                    .to_sql();
                assert!(
                    sql_shape(&sql.sql()).ends_with("WHERETRUE"),
                    "unexpected SQL: {}",
                    sql.sql()
                );

                let selected: Vec<SelectSharedConditionListRow> = db
                    .select(())
                    .from(rows)
                    .r#where((None::<Condition<'_>>, None::<Condition<'_>>))
                    .all();
                assert_eq!(selected.len(), 2);
            }

            #[drizzle::test($dialect)]
            fn all_none_disjunction_is_false(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;
                db.insert(rows)
                    .value(InsertSharedConditionListRow::new("alice").with_id(1))
                    .execute();

                let sql = db
                    .select(())
                    .from(rows)
                    .r#where(any((None::<Condition<'_>>, None::<Condition<'_>>)))
                    .to_sql();
                assert!(
                    sql_shape(&sql.sql()).ends_with("WHEREFALSE"),
                    "unexpected SQL: {}",
                    sql.sql()
                );

                let selected: Vec<SelectSharedConditionListRow> = db
                    .select(())
                    .from(rows)
                    .r#where(any((None::<Condition<'_>>, None::<Condition<'_>>)))
                    .all();
                assert!(selected.is_empty());
            }

            #[drizzle::test($dialect)]
            fn tuple_in_having(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionListRow::new("alice").with_id(1),
                        InsertSharedConditionListRow::new("alice").with_id(2),
                        InsertSharedConditionListRow::new("bob").with_id(3),
                    ])
                    .execute();

                let sql = db
                    .select((rows.name, count(rows.id)))
                    .from(rows)
                    .group_by(rows.name)
                    .having((gt(count(rows.id), 1_i64), lt(count(rows.id), 10_i64)))
                    .to_sql();
                assert!(
                    sql_shape(&sql.sql())
                        .contains(&format!("HAVING(COUNT({ROWS}.id)>?ANDCOUNT({ROWS}.id)<?)")),
                    "unexpected SQL: {}",
                    sql.sql()
                );

                let grouped: Vec<(String, i64)> = db
                    .select((rows.name, count(rows.id)))
                    .from(rows)
                    .group_by(rows.name)
                    .having((gt(count(rows.id), 1_i64), lt(count(rows.id), 10_i64)))
                    .all();
                assert_eq!(grouped, [("alice".to_string(), 2)]);
            }

            // A join's ON condition is bounded on `ToSQL`, not `Expr`, so a bare
            // tuple there would render as a column list. `all(...)` is the
            // spelling that works.
            #[drizzle::test($dialect)]
            fn all_in_join_on(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema {
                    posts, categories, ..
                } = schema;
                db.insert(categories)
                    .value(InsertSharedConditionListCategory::new("rust").with_id(1))
                    .execute();
                db.insert(posts)
                    .value(InsertSharedConditionListPost::new("hello").with_id(1))
                    .execute();

                let sql = db
                    .select(posts.title)
                    .from(posts)
                    .inner_join((
                        categories,
                        all((eq(categories.name, "rust"), neq(posts.title, "zed"))),
                    ))
                    .to_sql();
                assert!(
                    sql_shape(&sql.sql()).contains(
                        "ON(shared_condition_list_categories.name=?ANDshared_condition_list_posts.title<>?)"
                    ),
                    "unexpected SQL: {}",
                    sql.sql()
                );

                let titles: Vec<String> = db
                    .select(posts.title)
                    .from(posts)
                    .inner_join((
                        categories,
                        all((eq(categories.name, "rust"), neq(posts.title, "zed"))),
                    ))
                    .all();
                assert_eq!(titles, ["hello"]);
            }

            #[drizzle::test($dialect)]
            fn tuple_under_not(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionListRow::new("alice").with_id(1),
                        InsertSharedConditionListRow::new("bob").with_id(2),
                        InsertSharedConditionListRow::new("carol").with_id(3),
                    ])
                    .execute();

                let selected: Vec<SelectSharedConditionListRow> = db
                    .select(())
                    .from(rows)
                    .r#where(not((gt(rows.id, 1), lt(rows.id, 3))))
                    .all();

                assert_eq!(selected.len(), 2);
                assert!(selected.iter().all(|row| row.name != "bob"));
            }

            #[drizzle::test($dialect)]
            fn eight_element_tuple(db: &mut TestDb<SharedConditionListSchema>) {
                let SharedConditionListSchema { rows, .. } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionListRow::new("alice").with_id(1),
                        InsertSharedConditionListRow::new("bob").with_id(2),
                        InsertSharedConditionListRow::new("carol").with_id(3),
                    ])
                    .execute();

                let selected: Vec<SelectSharedConditionListRow> = db
                    .select(())
                    .from(rows)
                    .r#where((
                        gte(rows.id, 1),
                        gte(rows.id, 1),
                        gte(rows.id, 1),
                        gte(rows.id, 1),
                        gte(rows.id, 1),
                        gte(rows.id, 1),
                        gte(rows.id, 1),
                        eq(rows.name, "bob"),
                    ))
                    .all();

                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].name, "bob");
            }
        }
    };
}

pub(crate) use shared_condition_list_suite;
