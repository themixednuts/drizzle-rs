/// Cross-dialect prepared-statement behavior suite.
///
/// Each dialect supplies its table and schema derives, the integer marker used
/// by an explicitly typed placeholder, and a fresh transaction configuration
/// expression for the transaction and savepoint cases.
macro_rules! shared_prepared_statement_suite {
    ($dialect:ident, $table:ident, $schema:ident, $integer:path, $transaction_config:expr) => {
        mod shared_prepared_statement {
            use super::*;
            #[allow(unused_imports)]
            use crate::common::helpers::AffectedRows;
            use drizzle::core::expr::eq;
            use drizzle::error::DrizzleError;

            #[$table(NAME = "shared_prepared_users")]
            struct SharedPreparedUser {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
                nickname: Option<String>,
            }

            #[derive($schema)]
            struct SharedPreparedSchema {
                users: SharedPreparedUser,
            }

            #[drizzle::test($dialect)]
            fn prepared_select_reuses_typed_parameters(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;
                db.insert(users)
                    .values([
                        InsertSharedPreparedUser::new("Alice").with_id(1),
                        InsertSharedPreparedUser::new("Bob").with_id(2),
                        InsertSharedPreparedUser::new("Charlie").with_id(3),
                    ])
                    .execute();

                let name = users.name.placeholder("shared_prepared_name");
                let prepared = db
                    .select(())
                    .from(users)
                    .r#where(eq(users.name, name))
                    .prepare()
                    .into_owned();

                let alice: Vec<SelectSharedPreparedUser> =
                    prepared.all(drizzle_client!(), [name.bind("Alice")]);
                let bob: SelectSharedPreparedUser =
                    prepared.get(drizzle_client!(), [name.bind("Bob")]);
                let nobody: Vec<SelectSharedPreparedUser> =
                    prepared.all(drizzle_client!(), [name.bind("Nobody")]);

                assert_eq!(alice.len(), 1);
                assert_eq!(alice[0].name, "Alice");
                assert_eq!(bob.name, "Bob");
                assert!(nobody.is_empty());
            }

            #[drizzle::test($dialect)]
            fn prepared_update_executes_bound_parameters(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;
                db.insert(users)
                    .value(InsertSharedPreparedUser::new("Alice").with_id(1))
                    .execute();

                let name = users.name.placeholder("shared_prepared_new_name");
                let user_id =
                    drizzle::core::Placeholder::typed::<$integer>("shared_prepared_user_id");
                let prepared = db
                    .update(users)
                    .set(UpdateSharedPreparedUser::default().with_name(name))
                    .r#where(eq(users.id, user_id))
                    .prepare()
                    .into_owned();

                prepared.execute(drizzle_client!(), [name.bind("Alicia"), user_id.bind(1)]);

                let renamed: SelectSharedPreparedUser =
                    db.select(()).from(users).r#where(eq(users.id, 1)).get();
                assert_eq!(renamed.name, "Alicia");
            }

            #[drizzle::test($dialect)]
            fn prepared_parameter_count_mismatch_fails(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;
                db.insert(users)
                    .value(InsertSharedPreparedUser::new("Alice").with_id(1))
                    .execute();

                let name = users.name.placeholder("shared_prepared_name");
                let extra = users.name.placeholder("shared_prepared_extra");
                let prepared = db
                    .select(())
                    .from(users)
                    .r#where(eq(users.name, name))
                    .prepare()
                    .into_owned();

                // Binding the wrong number of parameters is a debug_assert
                // panic in debug builds and a `ParameterError` in release
                // builds; both are acceptable, silently running is not.
                // Zero-length repeat arrays keep the element type known without
                // naming the driver-specific generic arguments of `all`.
                let missing: Result<drizzle::Result<Vec<SelectSharedPreparedUser>>, _> =
                    catch!(prepared.all(drizzle_client!(), [name.bind("Alice"); 0]));
                match missing {
                    Err(_) | Ok(Err(DrizzleError::ParameterError(_))) => {}
                    Ok(Err(error)) => panic!("expected a parameter mismatch, got {error}"),
                    Ok(Ok(rows)) => {
                        panic!("expected a parameter mismatch, got {} rows", rows.len())
                    }
                }

                let surplus: Result<drizzle::Result<Vec<SelectSharedPreparedUser>>, _> =
                    catch!(prepared.all(
                        drizzle_client!(),
                        [name.bind("Alice"), extra.bind("ignored")],
                    ));
                match surplus {
                    Err(_) | Ok(Err(DrizzleError::ParameterError(_))) => {}
                    Ok(Err(error)) => panic!("expected a parameter mismatch, got {error}"),
                    Ok(Ok(rows)) => {
                        panic!("expected a parameter mismatch, got {} rows", rows.len())
                    }
                }
            }

            #[drizzle::test($dialect)]
            fn prepared_statements_run_without_parameters(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;

                for (id, name) in [(1, "Alice"), (2, "Bob"), (3, "Charlie")] {
                    let insert = db
                        .insert(users)
                        .value(InsertSharedPreparedUser::new(name).with_id(id))
                        .prepare()
                        .into_owned();
                    insert.execute(drizzle_client!(), []);
                }

                let select_all = db.select(()).from(users).prepare().into_owned();
                let rows: Vec<SelectSharedPreparedUser> = select_all.all(drizzle_client!(), []);
                let mut names = rows.iter().map(|row| row.name.as_str()).collect::<Vec<_>>();
                names.sort_unstable();
                assert_eq!(names, ["Alice", "Bob", "Charlie"]);
            }

            #[drizzle::test($dialect)]
            fn prepared_statement_runs_inside_transaction(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;
                db.insert(users)
                    .values([
                        InsertSharedPreparedUser::new("Alice").with_id(1),
                        InsertSharedPreparedUser::new("Bob").with_id(2),
                    ])
                    .execute();

                // Prepared outside the transaction, executed through it.
                let find_alice = db
                    .select(())
                    .from(users)
                    .r#where(eq(users.name, "Alice"))
                    .prepare()
                    .into_owned();
                let find_bob = db
                    .select(())
                    .from(users)
                    .r#where(eq(users.name, "Bob"))
                    .prepare()
                    .into_owned();

                db.transaction($transaction_config, |tx| {
                    let alice: Vec<SelectSharedPreparedUser> = result!(tx.all(&find_alice))?;
                    assert_eq!(alice.len(), 1);
                    assert_eq!(alice[0].name, "Alice");

                    let bob: SelectSharedPreparedUser = result!(tx.get(&find_bob))?;
                    assert_eq!(bob.name, "Bob");
                    Ok(())
                });
            }

            #[drizzle::test($dialect)]
            fn prepared_statement_runs_inside_savepoint(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;
                db.insert(users)
                    .value(InsertSharedPreparedUser::new("Alice").with_id(1))
                    .execute();

                let select_all = db.select(()).from(users).prepare().into_owned();

                db.transaction($transaction_config, |tx| {
                    result!(
                        tx.insert(users)
                            .value(InsertSharedPreparedUser::new("Bob").with_id(2))
                            .execute()
                    )?;

                    result!(tx.savepoint(|sp| {
                        let rows: Vec<SelectSharedPreparedUser> = result!(sp.all(&select_all))?;
                        assert_eq!(rows.len(), 2);
                        Ok(())
                    }))?;
                    Ok(())
                });
            }

            #[drizzle::test($dialect)]
            fn prepared_statement_survives_savepoint_rollback(
                db: &mut TestDb<SharedPreparedSchema>,
            ) {
                let SharedPreparedSchema { users } = schema;
                db.insert(users)
                    .value(InsertSharedPreparedUser::new("Alice").with_id(1))
                    .execute();

                let select_all = db.select(()).from(users).prepare().into_owned();

                db.transaction($transaction_config, |tx| {
                    let rolled_back: drizzle::Result<()> = result!(tx.savepoint(|sp| {
                        result!(
                            sp.insert(users)
                                .value(InsertSharedPreparedUser::new("Ghost").with_id(2))
                                .execute()
                        )?;
                        let rows: Vec<SelectSharedPreparedUser> = result!(sp.all(&select_all))?;
                        assert_eq!(rows.len(), 2);
                        Err(DrizzleError::Other("rollback".into()))
                    }));
                    assert!(rolled_back.is_err());

                    let rows: Vec<SelectSharedPreparedUser> = result!(tx.all(&select_all))?;
                    assert_eq!(rows.len(), 1);
                    assert_eq!(rows[0].name, "Alice");
                    Ok(())
                });
            }

            #[drizzle::test($dialect)]
            fn prepared_write_inside_transaction_rolls_back(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;

                let ghost = db
                    .insert(users)
                    .value(InsertSharedPreparedUser::new("Ghost").with_id(1))
                    .prepare()
                    .into_owned();

                let rolled_back: drizzle::Result<()> =
                    result!(db.transaction($transaction_config, |tx| {
                        result!(tx.execute(&ghost)).expect("prepared insert executes");
                        Err(DrizzleError::Other("rollback".into()))
                    }));
                assert!(rolled_back.is_err());

                let rows: Vec<SelectSharedPreparedUser> = db.select(()).from(users).all();
                assert!(
                    rows.is_empty(),
                    "expected rollback, found {} rows",
                    rows.len()
                );
            }

            #[drizzle::test($dialect)]
            fn prepared_insert_binds_placeholder_values(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;

                let user_id =
                    drizzle::core::Placeholder::typed::<$integer>("shared_prepared_insert_id");
                let name = users.name.placeholder("shared_prepared_insert_name");
                let insert = db
                    .insert(users)
                    .value(InsertSharedPreparedUser::new(name).with_id(user_id));
                assert_eq!(
                    insert.to_sql().params().count(),
                    0,
                    "placeholders must not be bound eagerly"
                );

                let prepared = insert.prepare().into_owned();
                let first =
                    prepared.execute(drizzle_client!(), [user_id.bind(1), name.bind("Alice")]);
                let second =
                    prepared.execute(drizzle_client!(), [user_id.bind(2), name.bind("Bob")]);
                assert_eq!((first.affected_rows(), second.affected_rows()), (1, 1));

                let rows: Vec<SelectSharedPreparedUser> =
                    db.select(()).from(users).order_by(asc(users.id)).all();
                let rows = rows
                    .iter()
                    .map(|row| (row.id, row.name.as_str()))
                    .collect::<Vec<_>>();
                assert_eq!(rows, [(1, "Alice"), (2, "Bob")]);
            }

            #[drizzle::test($dialect)]
            fn update_mixes_concrete_values_and_placeholders(
                db: &mut TestDb<SharedPreparedSchema>,
            ) {
                let SharedPreparedSchema { users } = schema;
                db.insert(users)
                    .values([
                        InsertSharedPreparedUser::new("Alice").with_id(1),
                        InsertSharedPreparedUser::new("Bob").with_id(2),
                    ])
                    .execute();

                let nickname = users.nickname.placeholder("shared_prepared_nickname");
                let update = db
                    .update(users)
                    .set(
                        UpdateSharedPreparedUser::default()
                            .with_name("Alicia")
                            .with_nickname(nickname),
                    )
                    .r#where(eq(users.id, 1));
                assert_eq!(
                    update.to_sql().params().count(),
                    2,
                    "concrete SET value and WHERE id bind; the placeholder does not"
                );

                let prepared = update.prepare().into_owned();
                let updated = prepared.execute(drizzle_client!(), [nickname.bind("Al")]);
                assert_eq!(updated.affected_rows(), 1);

                let rows: Vec<SelectSharedPreparedUser> =
                    db.select(()).from(users).order_by(asc(users.id)).all();
                assert_eq!(rows[0].name, "Alicia");
                assert_eq!(rows[0].nickname.as_deref(), Some("Al"));
                assert_eq!(rows[1].name, "Bob");
                assert_eq!(rows[1].nickname, None);
            }

            #[drizzle::test($dialect)]
            fn update_model_skips_unset_fields(db: &mut TestDb<SharedPreparedSchema>) {
                let SharedPreparedSchema { users } = schema;
                db.insert(users)
                    .value(InsertSharedPreparedUser::new("Alice").with_id(1))
                    .execute();

                let stmt = db
                    .update(users)
                    .set(UpdateSharedPreparedUser::default().with_nickname("Al"))
                    .r#where(eq(users.id, 1));
                let shape = crate::common::helpers::sql_shape(&stmt.to_sql().sql());
                assert!(
                    shape.contains("SETnickname=?WHERE"),
                    "only the assigned column belongs in SET: {shape}"
                );

                let updated = stmt.execute();
                assert_eq!(updated.affected_rows(), 1);
                let row: SelectSharedPreparedUser =
                    db.select(()).from(users).r#where(eq(users.id, 1)).get();
                assert_eq!(
                    (row.name.as_str(), row.nickname.as_deref()),
                    ("Alice", Some("Al"))
                );
            }
        }
    };
}

pub(crate) use shared_prepared_statement_suite;
