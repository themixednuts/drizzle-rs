/// Portable decoded-row cursor contracts.
///
/// Dialect modules supply the schema macros and transaction configuration.
/// The adapter test attribute expands each case for both available driver
/// implementations of that dialect.
macro_rules! shared_rows_suite {
    ($dialect:ident, $table:ident, $schema:ident, $transaction_config:expr) => {
        mod shared_rows {
            use super::*;

            #[$table(NAME = "shared_rows")]
            struct SharedRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[derive($schema)]
            struct SharedRowsSchema {
                rows: SharedRow,
            }

            #[drizzle::test($dialect)]
            fn typed_builder_rows_decode_in_query_order(db: &mut TestDb<SharedRowsSchema>) {
                let SharedRowsSchema { rows } = schema;
                result!(
                    db.insert(rows)
                        .values([
                            InsertSharedRow::new("first").with_id(1),
                            InsertSharedRow::new("second").with_id(2),
                            InsertSharedRow::new("third").with_id(3),
                        ])
                        .execute()
                )?;

                let mut decoded = result!(db.select(()).from(rows).order_by(asc(rows.id)).rows())?;
                let first = next_row!(decoded)?.expect("first decoded row");
                assert_eq!((first.id, first.name.as_str()), (1, "first"));

                let remaining = collect_rows!(decoded)?;
                assert_eq!(
                    remaining
                        .into_iter()
                        .map(|row| (row.id, row.name))
                        .collect::<Vec<_>>(),
                    [(2, "second".to_owned()), (3, "third".to_owned())]
                );
            }

            #[drizzle::test($dialect)]
            fn direct_rows_release_after_partial_drop(db: &mut TestDb<SharedRowsSchema>) {
                let SharedRowsSchema { rows } = schema;
                result!(
                    db.insert(rows)
                        .values([
                            InsertSharedRow::new("first").with_id(1),
                            InsertSharedRow::new("second").with_id(2),
                            InsertSharedRow::new("third").with_id(3),
                        ])
                        .execute()
                )?;

                let query = SQL::raw("SELECT id, name FROM shared_rows ORDER BY id");
                let mut decoded = result!(db.rows::<_, SelectSharedRow>(query))?;
                assert_eq!(next_row!(decoded)?.expect("first decoded row").id, 1);
                drop(decoded);

                let after_drop: Vec<SelectSharedRow> =
                    result!(db.select(()).from(rows).order_by(asc(rows.id)).all())?;
                assert_eq!(
                    after_drop.into_iter().map(|row| row.id).collect::<Vec<_>>(),
                    [1, 2, 3]
                );
            }

            #[drizzle::test($dialect)]
            fn transaction_rows_and_typed_builders_decode_output(
                db: &mut TestDb<SharedRowsSchema>,
            ) {
                let SharedRowsSchema { rows } = schema;
                result!(
                    db.insert(rows)
                        .values([
                            InsertSharedRow::new("first").with_id(1),
                            InsertSharedRow::new("second").with_id(2),
                        ])
                        .execute()
                )?;

                result!(db.transaction($transaction_config, |tx| {
                    let query = SQL::raw("SELECT id, name FROM shared_rows ORDER BY id");
                    let mut direct = result!(tx.rows::<_, SelectSharedRow>(query))?;
                    assert_eq!(next_row!(direct)?.expect("first transaction row").id, 1);
                    drop(direct);

                    let decoded = result!(tx.select(()).from(rows).order_by(asc(rows.id)).rows())?;
                    let names = collect_rows!(decoded)?
                        .into_iter()
                        .map(|row| row.name)
                        .collect::<Vec<_>>();
                    assert_eq!(names, ["first", "second"]);
                    Ok(())
                }))?;
            }
        }
    };
}

pub(crate) use shared_rows_suite;
