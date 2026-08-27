/// Cross-dialect transaction and savepoint behavior suite.
///
/// Dialect modules supply their table and schema macros plus a fresh transaction
/// configuration expression. Driver-specific cleanup and configuration behavior
/// stays in the dialect test modules.
#[macro_export]
macro_rules! shared_transaction_suite {
    ($dialect:ident, $table:ident, $schema:ident, $transaction_config:expr) => {
        mod shared_transaction {
            use super::*;

            #[$table(NAME = "shared_transaction_rows")]
            struct SharedTransactionRow {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
            }

            #[derive($schema)]
            struct SharedTransactionSchema {
                rows: SharedTransactionRow,
            }

            #[drizzle::test($dialect)]
            fn callback_commit_and_rollback(db: &mut TestDb<SharedTransactionSchema>) {
                let SharedTransactionSchema { rows } = schema;

                let rolled_back: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .value(
                                    InsertSharedTransactionRow::new("rolled back").with_id(1),
                                )
                                .execute()
                        )?;
                        Err(drizzle::error::DrizzleError::Other("rollback".into()))
                    }));
                assert!(matches!(
                    rolled_back,
                    Err(drizzle::error::DrizzleError::Other(message)) if message == "rollback"
                ));

                let after_rollback: Vec<SelectSharedTransactionRow> =
                    db.select(()).from(rows).all();
                assert!(after_rollback.is_empty());

                let committed: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("committed").with_id(1))
                                .execute()
                        )?;
                        Ok(())
                    }));
                assert!(committed.is_ok());

                let after_commit: Vec<SelectSharedTransactionRow> =
                    db.select(()).from(rows).all();
                assert_eq!(after_commit.len(), 1);
                assert_eq!(after_commit[0].name, "committed");
            }

            #[drizzle::test($dialect)]
            fn nested_savepoint_rollback_preserves_enclosing_transactions(
                db: &mut TestDb<SharedTransactionSchema>,
            ) {
                let SharedTransactionSchema { rows } = schema;

                let committed: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |outer| {
                        result!(
                            outer
                                .insert(rows)
                                .value(InsertSharedTransactionRow::new("outer").with_id(1))
                                .execute()
                        )?;

                        result!(outer.savepoint(|first| {
                            result!(
                                first
                                    .insert(rows)
                                    .value(
                                        InsertSharedTransactionRow::new("first savepoint")
                                            .with_id(2),
                                    )
                                    .execute()
                            )?;

                            let rolled_back: drizzle::Result<()> =
                                result!(first.savepoint(|second| {
                                    result!(
                                        second
                                            .insert(rows)
                                            .value(
                                                InsertSharedTransactionRow::new(
                                                    "rolled-back savepoint",
                                                )
                                                .with_id(3),
                                            )
                                            .execute()
                                    )?;
                                    Err(drizzle::error::DrizzleError::Other(
                                        "rollback inner savepoint".into(),
                                    ))
                                }));
                            assert!(matches!(
                                rolled_back,
                                Err(drizzle::error::DrizzleError::Other(message))
                                    if message == "rollback inner savepoint"
                            ));

                            result!(
                                first
                                    .insert(rows)
                                    .value(
                                        InsertSharedTransactionRow::new("after inner rollback")
                                            .with_id(4),
                                    )
                                    .execute()
                            )?;
                            Ok(())
                        }))?;

                        result!(
                            outer
                                .insert(rows)
                                .value(
                                    InsertSharedTransactionRow::new("after first savepoint")
                                        .with_id(5),
                                )
                                .execute()
                        )?;
                        Ok(())
                    }));
                assert!(committed.is_ok());

                let persisted: Vec<SelectSharedTransactionRow> =
                    db.select(()).from(rows).all();
                let names = persisted
                    .iter()
                    .map(|row| row.name.as_str())
                    .collect::<Vec<_>>();
                assert_eq!(persisted.len(), 4);
                assert!(names.contains(&"outer"));
                assert!(names.contains(&"first savepoint"));
                assert!(names.contains(&"after inner rollback"));
                assert!(names.contains(&"after first savepoint"));
                assert!(!names.contains(&"rolled-back savepoint"));
            }
        }
    };
}
