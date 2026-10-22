//! Enums that derive more than one dialect's enum derive.
//!
//! `SQLiteEnum`, `PostgresEnum` and `MySQLEnum` each generate the same
//! dialect-neutral impls (`Display`, `FromStr`, ...). The highest-priority
//! derive's helper attribute (`#[sqlite_enum]`, then `#[postgres_enum]`)
//! marks which one generates them, so the enum compiles with all of them.
//! Each dialect's tables sit in a module of their own, with that dialect's
//! prelude.

#[cfg(all(feature = "rusqlite", feature = "postgres"))]
mod sqlite_and_postgres {
    use drizzle::postgres::prelude::PostgresEnum;
    use drizzle::sqlite::prelude::SQLiteEnum;

    #[derive(SQLiteEnum, PostgresEnum, Clone, Copy, Debug, Default, PartialEq)]
    #[sqlite_enum]
    enum DualRole {
        #[default]
        Member,
        Admin,
    }

    #[test]
    fn shared_conversions_come_from_one_derive() {
        assert_eq!(DualRole::Admin.to_string(), "Admin");
        assert_eq!("Member".parse::<DualRole>().unwrap(), DualRole::Member);
        assert_eq!(i64::from(DualRole::Admin), 1);
        assert_eq!(DualRole::try_from(1_i64).unwrap(), DualRole::Admin);
    }

    mod sqlite_side {
        use super::DualRole;
        use drizzle::core::expr::eq;
        use drizzle::sqlite::prelude::*;

        #[SQLiteTable(NAME = "dual_enum_sqlite_accounts")]
        struct SqliteAccount {
            #[column(primary)]
            id: i64,
            #[column(enum)]
            role: DualRole,
        }

        #[derive(SQLiteSchema)]
        struct SqliteAccountSchema {
            account: SqliteAccount,
        }

        #[test]
        fn dual_enum_is_a_text_column_on_sqlite() {
            assert!(SqliteAccount::ddl_sql().contains("`role` TEXT NOT NULL"));
        }

        #[drizzle::test(sqlite)]
        fn dual_enum_round_trips_on_sqlite(db: &mut TestDb<SqliteAccountSchema>) {
            let SqliteAccountSchema { account } = schema;

            db.insert(account)
                .values([InsertSqliteAccount::new(DualRole::Admin).with_id(1)])
                .execute();
            let rows: Vec<SelectSqliteAccount> = db
                .select(())
                .from(account)
                .r#where(eq(account.role, DualRole::Admin))
                .all();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].role, DualRole::Admin);
        }
    }

    mod postgres_side {
        use super::DualRole;
        use drizzle::core::expr::eq;
        use drizzle::postgres::prelude::*;

        #[PostgresTable(name = "dual_enum_pg_accounts")]
        struct PgAccount {
            #[column(primary)]
            id: i32,
            #[column(enum)]
            role: DualRole,
        }

        #[derive(PostgresSchema)]
        struct PgAccountSchema {
            dual_role: DualRole,
            account: PgAccount,
        }

        #[test]
        fn dual_enum_is_a_native_enum_on_postgres() {
            assert!(PgAccount::ddl_sql().contains(r#""role" "DualRole" NOT NULL"#));
        }

        #[drizzle::test(postgres)]
        fn dual_enum_round_trips_on_postgres(db: &mut TestDb<PgAccountSchema>) {
            let PgAccountSchema { account, .. } = schema;

            db.insert(account)
                .values([InsertPgAccount::new(1, DualRole::Admin)])
                .execute();
            let rows: Vec<SelectPgAccount> = db
                .select(())
                .from(account)
                .r#where(eq(account.role, DualRole::Admin))
                .all();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0].role, DualRole::Admin);
        }
    }
}

#[cfg(all(feature = "postgres", feature = "mysql"))]
mod postgres_and_mysql {
    use drizzle::mysql::prelude::MySQLEnum;
    use drizzle::postgres::prelude::PostgresEnum;

    // Without `SQLiteEnum`, `#[postgres_enum]` marks the owner; any option
    // (here `text`) counts.
    #[derive(PostgresEnum, MySQLEnum, Clone, Copy, Debug, PartialEq)]
    #[postgres_enum(text)]
    enum DualStatus {
        Draft,
        Published,
    }

    #[test]
    fn shared_conversions_come_from_one_derive() {
        assert_eq!(DualStatus::Published.to_string(), "Published");
        assert_eq!("Draft".parse::<DualStatus>().unwrap(), DualStatus::Draft);
    }

    mod postgres_side {
        use super::DualStatus;
        use drizzle::postgres::prelude::*;

        #[PostgresTable(name = "dual_enum_pg_posts")]
        struct PgPost {
            #[column(primary)]
            id: i32,
            #[column(enum)]
            status: DualStatus,
        }

        #[test]
        fn text_enum_is_a_text_column() {
            assert!(PgPost::ddl_sql().contains(r#""status" text NOT NULL"#));
        }
    }

    mod mysql_side {
        use super::DualStatus;
        use drizzle::mysql::prelude::*;

        #[MySQLTable(NAME = "dual_enum_mysql_posts")]
        struct MySqlPost {
            #[column(PRIMARY)]
            id: i32,
            #[column(ENUM)]
            status: DualStatus,
        }

        #[test]
        fn enum_is_an_inline_mysql_enum() {
            assert!(
                MySqlPost::create_table_sql()
                    .contains("`status` ENUM('Draft', 'Published') NOT NULL")
            );
        }
    }
}

#[cfg(all(feature = "rusqlite", feature = "postgres", feature = "mysql"))]
mod all_three {
    use drizzle::mysql::prelude::MySQLEnum;
    use drizzle::postgres::prelude::PostgresEnum;
    use drizzle::sqlite::prelude::SQLiteEnum;

    #[derive(SQLiteEnum, PostgresEnum, MySQLEnum, Clone, Copy, Debug, PartialEq)]
    #[sqlite_enum]
    #[postgres_enum(text)]
    enum TriKind {
        Small,
        Large,
    }

    #[test]
    fn one_enum_serves_all_three_dialects() {
        assert_eq!(TriKind::Large.to_string(), "Large");
        assert_eq!("Small".parse::<TriKind>().unwrap(), TriKind::Small);
        assert_eq!(
            <TriKind as drizzle::mysql::traits::MySQLEnum>::VARIANTS,
            ["Small", "Large"]
        );
    }
}
