/// Portable condition and grouping behavior.
macro_rules! shared_condition_suite {
    ($dialect:ident, $table:ident, $schema:ident, $from_row:ident) => {
        mod shared_conditions {
            use super::*;
            use drizzle::core::{
                asc,
                expr::{
                    alias, and, between, count, eq, gt, in_array, is_not_null, is_null, like, not,
                    not_between, not_in_array, or,
                },
            };

            #[$table(NAME = "shared_condition_rows")]
            struct SharedConditionRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                active: bool,
                age: Option<i32>,
            }

            #[derive($schema)]
            struct SharedConditionSchema {
                rows: SharedConditionRow,
            }

            #[derive(Debug, $from_row)]
            struct SharedConditionCount {
                active: bool,
                total: i64,
            }

            #[drizzle::test($dialect)]
            fn comparisons_and_logical_conditions(db: &mut TestDb<SharedConditionSchema>) {
                let SharedConditionSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionRow::new("Alice", true)
                            .with_id(1)
                            .with_age(20),
                        InsertSharedConditionRow::new("Bob", false)
                            .with_id(2)
                            .with_age(30),
                        InsertSharedConditionRow::new("Charlie", true)
                            .with_id(3)
                            .with_age(40),
                    ])
                    .execute();

                let selected: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(and(gt(rows.id, 1), eq(rows.active, true)))
                    .all();
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].name, "Charlie");

                let selected: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(or(eq(rows.name, "Alice"), eq(rows.name, "Bob")))
                    .order_by(asc(rows.id))
                    .all();
                assert_eq!(
                    selected
                        .iter()
                        .map(|row| row.name.as_str())
                        .collect::<Vec<_>>(),
                    ["Alice", "Bob"]
                );

                let selected: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(not(eq(rows.active, true)))
                    .all();
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].name, "Bob");
            }

            #[drizzle::test($dialect)]
            fn set_range_null_and_pattern_conditions(db: &mut TestDb<SharedConditionSchema>) {
                let SharedConditionSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionRow::new("Alpha", true)
                            .with_id(1)
                            .with_age(20),
                        InsertSharedConditionRow::new("Alpine", true)
                            .with_id(3)
                            .with_age(40),
                    ])
                    .execute();
                db.insert(rows)
                    .value(InsertSharedConditionRow::new("Beta", false).with_id(2))
                    .execute();

                let selected: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(in_array(rows.id, [1, 3]))
                    .all();
                assert_eq!(selected.len(), 2);

                let selected: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(not_in_array(rows.id, [1, 3]))
                    .all();
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].name, "Beta");

                let empty_in = db
                    .select(())
                    .from(rows)
                    .r#where(in_array(rows.id, Vec::<i32>::new()));
                assert!(empty_in.to_sql().sql().ends_with(" WHERE FALSE"));
                let selected: Vec<SelectSharedConditionRow> = empty_in.all();
                assert!(selected.is_empty());

                let empty_not_in = db
                    .select(())
                    .from(rows)
                    .r#where(not_in_array(rows.id, Vec::<i32>::new()));
                assert!(empty_not_in.to_sql().sql().ends_with(" WHERE TRUE"));
                let selected: Vec<SelectSharedConditionRow> = empty_not_in.all();
                assert_eq!(selected.len(), 3);

                let selected: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(between(rows.age, 20, 30))
                    .all();
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].name, "Alpha");

                let selected: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(not_between(rows.age, 20, 30))
                    .all();
                assert_eq!(selected.len(), 1);
                assert_eq!(selected[0].name, "Alpine");

                let null_rows: Vec<SelectSharedConditionRow> =
                    db.select(()).from(rows).r#where(is_null(rows.age)).all();
                assert_eq!(null_rows.len(), 1);
                assert_eq!(null_rows[0].name, "Beta");

                let non_null_rows: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(is_not_null(rows.age))
                    .all();
                assert_eq!(non_null_rows.len(), 2);

                let selected: Vec<SelectSharedConditionRow> = db
                    .select(())
                    .from(rows)
                    .r#where(like(rows.name, "Al%"))
                    .all();
                assert_eq!(selected.len(), 2);
            }

            #[drizzle::test($dialect)]
            fn grouping_and_having(db: &mut TestDb<SharedConditionSchema>) {
                let SharedConditionSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedConditionRow::new("Alice", true).with_id(1),
                        InsertSharedConditionRow::new("Bob", true).with_id(2),
                        InsertSharedConditionRow::new("Charlie", false).with_id(3),
                    ])
                    .execute();

                let grouped: Vec<SharedConditionCount> = db
                    .select((rows.active, alias(count(rows.id), "total")))
                    .from(rows)
                    .group_by(rows.active)
                    .having(gt(count(rows.id), 1_i64))
                    .all();

                assert_eq!(grouped.len(), 1);
                assert!(grouped[0].active);
                assert_eq!(grouped[0].total, 2);
            }
        }
    };
}

pub(crate) use shared_condition_suite;
