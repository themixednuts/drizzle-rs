/// Portable DELETE behavior: affected-row counts, predicate shapes, and the
/// no-match / delete-everything edges. `RETURNING` lives in
/// `shared_delete_returning_suite` because MySQL has no such clause.
macro_rules! shared_delete_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_delete {
            use super::*;
            #[allow(unused_imports)]
            use crate::common::helpers::AffectedRows;
            use drizzle::core::expr::{and, between, eq, gt, in_array, is_null, like};

            #[$table(NAME = "shared_delete_rows")]
            struct SharedDeleteRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                active: bool,
                email: Option<String>,
                age: Option<i32>,
            }

            #[derive($schema)]
            struct SharedDeleteSchema {
                rows: SharedDeleteRow,
            }

            #[drizzle::test($dialect)]
            fn delete_reports_affected_rows_and_removes_matches(
                db: &mut TestDb<SharedDeleteSchema>,
            ) {
                let SharedDeleteSchema { rows } = schema;
                let inserted = db
                    .insert(rows)
                    .values([
                        InsertSharedDeleteRow::new("delete_me", true).with_id(1),
                        InsertSharedDeleteRow::new("keep_me", true).with_id(2),
                        InsertSharedDeleteRow::new("delete_me", false).with_id(3),
                    ])
                    .execute();
                assert_eq!(inserted.affected_rows(), 3);

                let deleted = db
                    .delete(rows)
                    .r#where(eq(rows.name, "delete_me"))
                    .execute();
                assert_eq!(deleted.affected_rows(), 2);

                let remaining: Vec<SelectSharedDeleteRow> = db.select(()).from(rows).all();
                assert_eq!(remaining.len(), 1);
                assert_eq!(remaining[0].id, 2);
                assert_eq!(remaining[0].name, "keep_me");

                let gone: Vec<i32> = db
                    .select(rows.id)
                    .from(rows)
                    .r#where(eq(rows.name, "delete_me"))
                    .all();
                assert!(gone.is_empty());
            }

            #[drizzle::test($dialect)]
            fn delete_with_pattern_and_list_predicates(db: &mut TestDb<SharedDeleteSchema>) {
                let SharedDeleteSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedDeleteRow::new("test_one", true).with_id(1),
                        InsertSharedDeleteRow::new("test_two", true).with_id(2),
                        InsertSharedDeleteRow::new("Alice", true).with_id(3),
                        InsertSharedDeleteRow::new("Bob", true).with_id(4),
                        InsertSharedDeleteRow::new("Charlie", true).with_id(5),
                    ])
                    .execute();

                let by_pattern = db.delete(rows).r#where(like(rows.name, "test%")).execute();
                assert_eq!(by_pattern.affected_rows(), 2);

                let by_list = db
                    .delete(rows)
                    .r#where(in_array(rows.name, ["Alice", "Charlie"]))
                    .execute();
                assert_eq!(by_list.affected_rows(), 2);

                let remaining: Vec<String> = db.select(rows.name).from(rows).all();
                assert_eq!(remaining, ["Bob"]);
            }

            #[drizzle::test($dialect)]
            fn delete_with_compound_and_null_predicates(db: &mut TestDb<SharedDeleteSchema>) {
                let SharedDeleteSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedDeleteRow::new("active with email", true)
                            .with_id(1)
                            .with_email("a@example.com"),
                        InsertSharedDeleteRow::new("inactive with email", false)
                            .with_id(2)
                            .with_email("b@example.com"),
                    ])
                    .execute();
                db.insert(rows)
                    .values([
                        InsertSharedDeleteRow::new("active without email", true).with_id(3),
                        InsertSharedDeleteRow::new("inactive without email", false).with_id(4),
                    ])
                    .execute();

                let inactive_without_email = db
                    .delete(rows)
                    .r#where(and(eq(rows.active, false), is_null(rows.email)))
                    .execute();
                assert_eq!(inactive_without_email.affected_rows(), 1);

                let without_email = db.delete(rows).r#where(is_null(rows.email)).execute();
                assert_eq!(without_email.affected_rows(), 1);

                let remaining: Vec<SelectSharedDeleteRow> =
                    db.select(()).from(rows).order_by(asc(rows.id)).all();
                let names = remaining
                    .iter()
                    .map(|row| row.name.as_str())
                    .collect::<Vec<_>>();
                assert_eq!(names, ["active with email", "inactive with email"]);
                assert!(remaining.iter().all(|row| row.email.is_some()));
            }

            #[drizzle::test($dialect)]
            fn delete_with_range_predicates(db: &mut TestDb<SharedDeleteSchema>) {
                let SharedDeleteSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedDeleteRow::new("teen", true)
                            .with_id(1)
                            .with_age(15),
                        InsertSharedDeleteRow::new("young adult", true)
                            .with_id(2)
                            .with_age(25),
                        InsertSharedDeleteRow::new("adult", true)
                            .with_id(3)
                            .with_age(45),
                        InsertSharedDeleteRow::new("senior", true)
                            .with_id(4)
                            .with_age(75),
                    ])
                    .execute();
                db.insert(rows)
                    .value(InsertSharedDeleteRow::new("unknown age", true).with_id(5))
                    .execute();

                let seniors = db.delete(rows).r#where(gt(rows.age, 65)).execute();
                assert_eq!(seniors.affected_rows(), 1);

                let working_age = db.delete(rows).r#where(between(rows.age, 20, 50)).execute();
                assert_eq!(working_age.affected_rows(), 2);

                // NULL ages never satisfy a comparison, so the unknown row survives.
                let remaining: Vec<String> =
                    db.select(rows.name).from(rows).order_by(asc(rows.id)).all();
                assert_eq!(remaining, ["teen", "unknown age"]);
            }

            #[drizzle::test($dialect)]
            fn delete_without_matches_leaves_rows_untouched(db: &mut TestDb<SharedDeleteSchema>) {
                let SharedDeleteSchema { rows } = schema;
                db.insert(rows)
                    .value(InsertSharedDeleteRow::new("Alice", true).with_id(1))
                    .execute();

                let deleted = db.delete(rows).r#where(eq(rows.name, "nobody")).execute();
                assert_eq!(deleted.affected_rows(), 0);

                let remaining: Vec<SelectSharedDeleteRow> = db.select(()).from(rows).all();
                assert_eq!(remaining.len(), 1);
                assert_eq!(remaining[0].name, "Alice");
            }

            #[drizzle::test($dialect)]
            fn delete_without_a_predicate_clears_the_table(db: &mut TestDb<SharedDeleteSchema>) {
                let SharedDeleteSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedDeleteRow::new("Alice", true).with_id(1),
                        InsertSharedDeleteRow::new("Bob", true).with_id(2),
                        InsertSharedDeleteRow::new("Charlie", false).with_id(3),
                    ])
                    .execute();

                let sql = db.delete(rows).to_sql().sql();
                assert!(
                    !sql.to_ascii_uppercase().contains("WHERE"),
                    "unfiltered delete must not render a WHERE clause: {sql}"
                );

                let deleted = db.delete(rows).execute();
                assert_eq!(deleted.affected_rows(), 3);

                let remaining: Vec<SelectSharedDeleteRow> = db.select(()).from(rows).all();
                assert!(remaining.is_empty());
            }
        }
    };
}

/// `DELETE ... RETURNING` for the dialects that support it (SQLite, PostgreSQL).
#[cfg(any(
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres"
))]
macro_rules! shared_delete_returning_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_delete_returning {
            use super::*;
            use drizzle::core::expr::{eq, gt};

            #[$table(NAME = "shared_delete_returning_rows")]
            struct SharedDeleteReturningRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                age: i32,
            }

            #[derive($schema)]
            struct SharedDeleteReturningSchema {
                rows: SharedDeleteReturningRow,
            }

            #[drizzle::test($dialect)]
            fn returning_star_yields_the_deleted_rows(
                db: &mut TestDb<SharedDeleteReturningSchema>,
            ) {
                let SharedDeleteReturningSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedDeleteReturningRow::new("Alice", 30).with_id(1),
                        InsertSharedDeleteReturningRow::new("Bob", 40).with_id(2),
                    ])
                    .execute();

                let stmt = db.delete(rows).r#where(eq(rows.id, 2)).returning(());
                let sql = stmt.to_sql().sql();
                assert!(
                    sql.ends_with("RETURNING *"),
                    "expected a trailing RETURNING *: {sql}"
                );

                let deleted: Vec<SelectSharedDeleteReturningRow> = stmt.all();
                assert_eq!(deleted.len(), 1);
                assert_eq!(deleted[0].id, 2);
                assert_eq!(deleted[0].name, "Bob");
                assert_eq!(deleted[0].age, 40);

                let remaining: Vec<i32> = db.select(rows.id).from(rows).all();
                assert_eq!(remaining, [1]);
            }

            #[drizzle::test($dialect)]
            fn returning_columns_yields_only_the_requested_values(
                db: &mut TestDb<SharedDeleteReturningSchema>,
            ) {
                let SharedDeleteReturningSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertSharedDeleteReturningRow::new("Alice", 30).with_id(1),
                        InsertSharedDeleteReturningRow::new("Bob", 40).with_id(2),
                        InsertSharedDeleteReturningRow::new("Cleo", 50).with_id(3),
                    ])
                    .execute();

                let mut names: Vec<String> = db
                    .delete(rows)
                    .r#where(gt(rows.age, 35))
                    .returning(rows.name)
                    .all();
                names.sort();
                assert_eq!(names, ["Bob", "Cleo"]);

                let remaining: Vec<i32> = db.select(rows.id).from(rows).all();
                assert_eq!(remaining, [1]);
            }

            #[drizzle::test($dialect)]
            fn returning_with_no_matches_yields_nothing(
                db: &mut TestDb<SharedDeleteReturningSchema>,
            ) {
                let SharedDeleteReturningSchema { rows } = schema;
                db.insert(rows)
                    .value(InsertSharedDeleteReturningRow::new("Alice", 30).with_id(1))
                    .execute();

                let deleted: Vec<SelectSharedDeleteReturningRow> =
                    db.delete(rows).r#where(eq(rows.id, 99)).returning(()).all();
                assert!(deleted.is_empty());
            }
        }
    };
}

#[cfg(any(
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres"
))]
pub(crate) use shared_delete_returning_suite;
pub(crate) use shared_delete_suite;
