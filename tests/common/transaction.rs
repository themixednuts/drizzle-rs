/// Cross-dialect transaction and savepoint behavior suite.
///
/// Dialect modules supply their table and schema macros plus a fresh transaction
/// configuration expression. Driver-specific cleanup and configuration behavior
/// stays in the dialect test modules.
macro_rules! shared_transaction_suite {
    ($dialect:ident, $table:ident, $schema:ident, $transaction_config:expr) => {
        mod shared_transaction {
            use super::*;
            #[allow(unused_imports)]
            use crate::common::helpers::AffectedRows;
            use drizzle::core::expr::{count, eq};

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

            fn names(db_rows: Vec<SelectSharedTransactionRow>) -> Vec<String> {
                let mut names = db_rows.into_iter().map(|row| row.name).collect::<Vec<_>>();
                names.sort();
                names
            }

            #[drizzle::test($dialect)]
            fn callback_panic_rolls_back(db: &mut TestDb<SharedTransactionSchema>) {
                let SharedTransactionSchema { rows } = schema;
                db.insert(rows)
                    .value(InsertSharedTransactionRow::new("before panic").with_id(1))
                    .execute();

                let panicked: Result<drizzle::Result<()>, _> =
                    catch!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("should roll back").with_id(2))
                                .execute()
                        )?;
                        panic!("simulated panic inside the transaction callback");
                    }));
                assert!(panicked.is_err());

                let persisted: Vec<SelectSharedTransactionRow> = db.select(()).from(rows).all();
                assert_eq!(names(persisted), ["before panic"]);
            }

            #[drizzle::test($dialect)]
            fn database_errors_inside_the_callback_roll_back(
                db: &mut TestDb<SharedTransactionSchema>,
            ) {
                let SharedTransactionSchema { rows } = schema;
                db.insert(rows)
                    .value(InsertSharedTransactionRow::new("initial").with_id(1))
                    .execute();

                let failed: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("valid").with_id(2))
                                .execute()
                        )?;
                        // Duplicate primary key: the driver error propagates
                        // out of the callback and must roll the whole thing back.
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("duplicate").with_id(1))
                                .execute()
                        )?;
                        Ok(())
                    }));
                assert!(failed.is_err());

                let persisted: Vec<SelectSharedTransactionRow> = db.select(()).from(rows).all();
                assert_eq!(names(persisted), ["initial"]);
            }

            #[drizzle::test($dialect)]
            fn mixed_writes_commit_together(db: &mut TestDb<SharedTransactionSchema>) {
                let SharedTransactionSchema { rows } = schema;
                db.insert(rows)
                    .value(InsertSharedTransactionRow::new("alice").with_id(1))
                    .execute();

                let committed: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .values([
                                    InsertSharedTransactionRow::new("bob").with_id(2),
                                    InsertSharedTransactionRow::new("charlie").with_id(3),
                                ])
                                .execute()
                        )?;
                        let updated = result!(
                            tx.update(rows)
                                .set(UpdateSharedTransactionRow::default().with_name("robert"))
                                .r#where(eq(rows.name, "bob"))
                                .execute()
                        )?;
                        assert_eq!(updated.affected_rows(), 1);
                        let deleted = result!(
                            tx.delete(rows)
                                .r#where(eq(rows.name, "charlie"))
                                .execute()
                        )?;
                        assert_eq!(deleted.affected_rows(), 1);

                        // Uncommitted writes are visible to the transaction itself.
                        let inside: Vec<SelectSharedTransactionRow> =
                            result!(tx.select(()).from(rows).all())?;
                        assert_eq!(names(inside), ["alice", "robert"]);
                        Ok(())
                    }));
                assert!(committed.is_ok(), "{committed:?}");

                let persisted: Vec<SelectSharedTransactionRow> = db.select(()).from(rows).all();
                assert_eq!(names(persisted), ["alice", "robert"]);
            }

            #[drizzle::test($dialect)]
            fn many_writes_roll_back_together(db: &mut TestDb<SharedTransactionSchema>) {
                let SharedTransactionSchema { rows } = schema;

                let rolled_back: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        for i in 1..=100 {
                            let name = format!("row {i}");
                            result!(
                                tx.insert(rows)
                                    .value(InsertSharedTransactionRow::new(name.as_str()).with_id(i))
                                    .execute()
                            )?;
                        }
                        let inside: i64 = result!(tx.select(count(rows.id)).from(rows).get())?;
                        assert_eq!(inside, 100);
                        Err(drizzle::error::DrizzleError::Other("rollback".into()))
                    }));
                assert!(rolled_back.is_err());

                let persisted: i64 = db.select(count(rows.id)).from(rows).get();
                assert_eq!(persisted, 0);
            }

            #[drizzle::test($dialect)]
            fn savepoints_commit_with_their_transaction(
                db: &mut TestDb<SharedTransactionSchema>,
            ) {
                let SharedTransactionSchema { rows } = schema;

                let committed: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("outer").with_id(1))
                                .execute()
                        )?;
                        result!(tx.savepoint(|sp| {
                            result!(
                                sp.insert(rows)
                                    .value(InsertSharedTransactionRow::new("inner").with_id(2))
                                    .execute()
                            )?;
                            Ok(())
                        }))?;
                        Ok(())
                    }));
                assert!(committed.is_ok(), "{committed:?}");

                let persisted: Vec<SelectSharedTransactionRow> = db.select(()).from(rows).all();
                assert_eq!(names(persisted), ["inner", "outer"]);
            }

            #[drizzle::test($dialect)]
            fn outer_rollback_discards_released_savepoints(
                db: &mut TestDb<SharedTransactionSchema>,
            ) {
                let SharedTransactionSchema { rows } = schema;

                let rolled_back: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("outer").with_id(1))
                                .execute()
                        )?;
                        result!(tx.savepoint(|sp| {
                            result!(
                                sp.insert(rows)
                                    .value(InsertSharedTransactionRow::new("inner").with_id(2))
                                    .execute()
                            )?;
                            Ok(())
                        }))?;
                        Err(drizzle::error::DrizzleError::Other("rollback outer".into()))
                    }));
                assert!(rolled_back.is_err());

                let persisted: Vec<SelectSharedTransactionRow> = db.select(()).from(rows).all();
                assert!(persisted.is_empty());
            }

            #[drizzle::test($dialect)]
            fn sequential_savepoints_recover_from_failures(
                db: &mut TestDb<SharedTransactionSchema>,
            ) {
                let SharedTransactionSchema { rows } = schema;

                let committed: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("before").with_id(1))
                                .execute()
                        )?;
                        for i in 1..=5u8 {
                            let id = i32::from(i) + 1;
                            let keeps = i % 2 == 1;
                            let name = format!("sp{i} {}", if keeps { "kept" } else { "dropped" });
                            let outcome: drizzle::Result<()> = result!(tx.savepoint(|sp| {
                                result!(
                                    sp.insert(rows)
                                        .value(
                                            InsertSharedTransactionRow::new(name.as_str())
                                                .with_id(id),
                                        )
                                        .execute()
                                )?;
                                if keeps {
                                    Ok(())
                                } else {
                                    Err(drizzle::error::DrizzleError::Other(
                                        "rollback savepoint".into(),
                                    ))
                                }
                            }));
                            assert_eq!(outcome.is_ok(), keeps, "savepoint {i}");
                        }
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("after").with_id(7))
                                .execute()
                        )?;
                        Ok(())
                    }));
                assert!(committed.is_ok(), "{committed:?}");

                let persisted: Vec<SelectSharedTransactionRow> = db.select(()).from(rows).all();
                assert_eq!(
                    names(persisted),
                    ["after", "before", "sp1 kept", "sp3 kept", "sp5 kept"]
                );
            }

            #[drizzle::test($dialect)]
            fn savepoints_see_enclosing_writes_and_hide_rolled_back_ones(
                db: &mut TestDb<SharedTransactionSchema>,
            ) {
                let SharedTransactionSchema { rows } = schema;

                let committed: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .value(InsertSharedTransactionRow::new("outer").with_id(1))
                                .execute()
                        )?;

                        result!(tx.savepoint(|sp| {
                            let before: Vec<SelectSharedTransactionRow> =
                                result!(sp.select(()).from(rows).all())?;
                            assert_eq!(names(before), ["outer"]);
                            result!(
                                sp.insert(rows)
                                    .value(InsertSharedTransactionRow::new("inner").with_id(2))
                                    .execute()
                            )?;
                            let after: Vec<SelectSharedTransactionRow> =
                                result!(sp.select(()).from(rows).all())?;
                            assert_eq!(names(after), ["inner", "outer"]);
                            Ok(())
                        }))?;

                        let ghost: drizzle::Result<()> = result!(tx.savepoint(|sp| {
                            result!(
                                sp.insert(rows)
                                    .value(InsertSharedTransactionRow::new("ghost").with_id(3))
                                    .execute()
                            )?;
                            let visible: i64 =
                                result!(sp.select(count(rows.id)).from(rows).get())?;
                            assert_eq!(visible, 3);
                            Err(drizzle::error::DrizzleError::Other("drop ghost".into()))
                        }));
                        assert!(ghost.is_err());

                        let outer_view: Vec<SelectSharedTransactionRow> =
                            result!(tx.select(()).from(rows).all())?;
                        assert_eq!(names(outer_view), ["inner", "outer"]);
                        Ok(())
                    }));
                assert!(committed.is_ok(), "{committed:?}");

                let persisted: Vec<SelectSharedTransactionRow> = db.select(()).from(rows).all();
                assert_eq!(names(persisted), ["inner", "outer"]);
            }

            #[drizzle::test($dialect)]
            fn savepoint_rollback_restores_updates_and_deletes(
                db: &mut TestDb<SharedTransactionSchema>,
            ) {
                let SharedTransactionSchema { rows } = schema;

                let committed: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(
                            tx.insert(rows)
                                .values([
                                    InsertSharedTransactionRow::new("alice").with_id(1),
                                    InsertSharedTransactionRow::new("bob").with_id(2),
                                    InsertSharedTransactionRow::new("charlie").with_id(3),
                                ])
                                .execute()
                        )?;

                        let undone: drizzle::Result<()> = result!(tx.savepoint(|sp| {
                            result!(
                                sp.update(rows)
                                    .set(UpdateSharedTransactionRow::default().with_name("alicia"))
                                    .r#where(eq(rows.name, "alice"))
                                    .execute()
                            )?;
                            result!(sp.delete(rows).r#where(eq(rows.name, "bob")).execute())?;
                            let inside: Vec<SelectSharedTransactionRow> =
                                result!(sp.select(()).from(rows).all())?;
                            assert_eq!(names(inside), ["alicia", "charlie"]);
                            Err(drizzle::error::DrizzleError::Other("undo".into()))
                        }));
                        assert!(undone.is_err());

                        let restored: Vec<SelectSharedTransactionRow> =
                            result!(tx.select(()).from(rows).all())?;
                        assert_eq!(names(restored), ["alice", "bob", "charlie"]);
                        Ok(())
                    }));
                assert!(committed.is_ok(), "{committed:?}");

                let persisted: Vec<SelectSharedTransactionRow> = db.select(()).from(rows).all();
                assert_eq!(names(persisted), ["alice", "bob", "charlie"]);
            }
        }
    };
}

pub(crate) use shared_transaction_suite;
