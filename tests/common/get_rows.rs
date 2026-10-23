/// `get` returns the row itself for a query that always returns exactly one
/// row (an aggregate without GROUP BY, ORDER BY, LIMIT or a set operation),
/// and `Option` otherwise.
macro_rules! shared_get_rows_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_get_rows {
            use super::*;
            use drizzle::core::expr::{count, eq, max};

            #[$table(NAME = "shared_get_rows")]
            struct GetRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[derive($schema)]
            struct GetRowsSchema {
                rows: GetRow,
            }

            #[drizzle::test($dialect)]
            fn get_returns_an_aggregate_row_directly(db: &mut TestDb<GetRowsSchema>) {
                let GetRowsSchema { rows } = schema;
                db.insert(rows)
                    .values([
                        InsertGetRow::new("a").with_id(1),
                        InsertGetRow::new("b").with_id(2),
                    ])
                    .execute();

                let total: i64 = db.select(count(rows.id)).from(rows).get();
                assert_eq!(total, 2);

                // Still one row when WHERE matches nothing.
                let (none, largest): (i64, Option<String>) = db
                    .select((count(rows.id), max(rows.name)))
                    .from(rows)
                    .r#where(eq(rows.id, 999))
                    .get();
                assert_eq!((none, largest), (0, None));

                let limited: Option<i64> = db.select(count(rows.id)).from(rows).limit(1).get();
                assert_eq!(limited, Some(2));

                let grouped: Option<i64> = db
                    .select(count(rows.id))
                    .from(rows)
                    .r#where(eq(rows.id, 1))
                    .group_by(rows.name)
                    .get();
                assert_eq!(grouped, Some(1));

                // A set operation can remove the row.
                let qb = || drizzle::$dialect::builder::QueryBuilder::new::<GetRowsSchema>();
                let removed: Option<i64> = db
                    .select(count(rows.id))
                    .from(rows)
                    .except(qb().select(count(rows.id)).from(rows))
                    .get();
                assert_eq!(removed, None);
            }

            #[drizzle::test($dialect)]
            fn get_without_a_matching_row_is_none_everywhere(db: &mut TestDb<GetRowsSchema>) {
                let GetRowsSchema { rows } = schema;
                db.insert(rows)
                    .value(InsertGetRow::new("a").with_id(1))
                    .execute();

                let found: Option<String> = db
                    .select(rows.name)
                    .from(rows)
                    .r#where(eq(rows.id, 1))
                    .get();
                assert_eq!(found.as_deref(), Some("a"));

                let missing: Option<SelectGetRow> =
                    db.select(()).from(rows).r#where(eq(rows.id, 999)).get();
                assert!(missing.is_none());

                let raw: Option<SelectGetRow> = db.get(SQL::raw(
                    "SELECT id, name FROM shared_get_rows WHERE id = 999",
                ));
                assert!(raw.is_none());
            }
        }
    };
}

pub(crate) use shared_get_rows_suite;
