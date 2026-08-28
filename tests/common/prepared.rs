/// Cross-dialect prepared-statement behavior suite.
///
/// Each dialect supplies its table and schema derives plus the integer marker
/// used by an explicitly typed placeholder.
macro_rules! shared_prepared_statement_suite {
    ($dialect:ident, $table:ident, $schema:ident, $integer:path) => {
        mod shared_prepared_statement {
            use super::*;

            #[$table(NAME = "shared_prepared_users")]
            struct SharedPreparedUser {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                name: String,
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
                    .r#where(drizzle::core::expr::eq(users.name, name))
                    .prepare()
                    .into_owned();

                let alice: Vec<SelectSharedPreparedUser> =
                    prepared.all(drizzle_client!(), [name.bind("Alice")]);
                let bob: SelectSharedPreparedUser =
                    prepared.get(drizzle_client!(), [name.bind("Bob")]);

                assert_eq!(alice.len(), 1);
                assert_eq!(alice[0].name, "Alice");
                assert_eq!(bob.name, "Bob");
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
                    .r#where(drizzle::core::expr::eq(users.id, user_id))
                    .prepare()
                    .into_owned();

                prepared.execute(drizzle_client!(), [name.bind("Alicia"), user_id.bind(1)]);

                let renamed: SelectSharedPreparedUser = db
                    .select(())
                    .from(users)
                    .r#where(drizzle::core::expr::eq(users.id, 1))
                    .get();
                assert_eq!(renamed.name, "Alicia");
            }
        }
    };
}

pub(crate) use shared_prepared_statement_suite;
