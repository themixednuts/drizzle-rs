/// Constraint failures report the same [`ErrorKind`] on every driver, and
/// the constraint's name where the database gives one.
///
/// [`ErrorKind`]: drizzle::error::ErrorKind
macro_rules! shared_error_kind_suite {
    ($dialect:ident, $table:ident, $schema:ident) => {
        mod shared_error_kinds {
            use super::*;
            use drizzle::error::ErrorKind;

            #[$table(NAME = "shared_error_teams")]
            struct ErrorTeam {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
            }

            #[$table(
                NAME = "shared_error_users",
                CHECK(name = "shared_error_score_check", expr = "score >= 0")
            )]
            struct ErrorUser {
                #[column(PRIMARY, DEFAULT = 0)]
                id: i32,
                #[column(UNIQUE)]
                code: i32,
                score: i32,
                #[column(REFERENCES = ErrorTeam::id)]
                team_id: Option<i32>,
            }

            #[derive($schema)]
            struct SharedErrorSchema {
                teams: ErrorTeam,
                users: ErrorUser,
            }

            #[drizzle::test($dialect)]
            fn constraint_failures_have_the_same_kind_everywhere(
                db: &mut TestDb<SharedErrorSchema>,
            ) {
                let SharedErrorSchema { teams, users } = schema;
                db.insert(teams)
                    .value(InsertErrorTeam::new().with_id(1))
                    .execute();
                db.insert(users)
                    .value(InsertErrorUser::new(100, 1).with_id(1))
                    .execute();

                let duplicate = result!(
                    db.insert(users)
                        .value(InsertErrorUser::new(100, 2).with_id(2))
                        .execute()
                )
                .expect_err("a second user with the same code");
                assert_eq!(duplicate.kind(), ErrorKind::UniqueViolation, "{duplicate}");
                assert!(duplicate.kind().is_constraint_violation());
                assert!(!duplicate.is_retryable());
                assert!(
                    duplicate
                        .query()
                        .is_some_and(|query| query.sql.contains("shared_error_users")),
                    "{duplicate}"
                );

                let orphan = result!(
                    db.insert(users)
                        .value(InsertErrorUser::new(101, 1).with_id(3).with_team_id(99))
                        .execute()
                )
                .expect_err("a user of a team that does not exist");
                assert_eq!(orphan.kind(), ErrorKind::ForeignKeyViolation, "{orphan}");

                let negative = result!(
                    db.insert(users)
                        .value(InsertErrorUser::new(102, -1).with_id(4))
                        .execute()
                )
                .expect_err("a negative score");
                assert_eq!(negative.kind(), ErrorKind::CheckViolation, "{negative}");
                assert_eq!(
                    negative.constraint(),
                    Some("shared_error_score_check"),
                    "{negative}"
                );

                let missing = result!(db.execute(SQL::raw(
                    "INSERT INTO shared_error_users (id, code, score) VALUES (5, NULL, 1)"
                )))
                .expect_err("a NULL code");
                assert_eq!(missing.kind(), ErrorKind::NotNullViolation, "{missing}");
                // Raw SQL errors carry their SQL too.
                assert!(
                    missing
                        .query()
                        .is_some_and(|query| query.sql.contains("VALUES (5, NULL, 1)")),
                    "{missing}"
                );
            }
        }
    };
}

pub(crate) use shared_error_kind_suite;
