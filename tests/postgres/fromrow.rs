//! Compile-time checks for Postgres FromRow implementations.

#[cfg(feature = "tokio-postgres")]
mod tokio_fromrow_checks {
    use drizzle::postgres::prelude::*;
    use drizzle_macros::{PostgresFromRow, PostgresTable};

    #[PostgresTable(name = "users")]
    struct Users {
        #[column(serial, primary)]
        id: i32,
        name: String,
    }

    #[derive(PostgresFromRow, Debug)]
    struct UserRow {
        id: i32,
        name: String,
    }

    #[allow(dead_code)]
    fn assert_tokio_fromrow<T>()
    where
        for<'a> T: TryFrom<&'a tokio_postgres::Row>,
    {
    }

    #[test]
    fn tokio_postgres_fromrow_compiles() {
        let _ = Users::default();
        assert_tokio_fromrow::<UserRow>();
    }
}
