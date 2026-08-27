#![cfg(any(
    feature = "rusqlite",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "mysql-sync",
    feature = "mysql-async",
))]

mod parity;

#[cfg(feature = "rusqlite")]
mod sqlite {
    use super::parity::Sqlite;

    crate::shared_dialect_contract!(Sqlite);
    crate::shared_live_driver_contract!(Sqlite);
    crate::shared_non_postgres_contract!(Sqlite);
}

#[cfg(feature = "postgres-sync")]
mod postgres_dialect {
    use super::parity::PostgresSync;

    crate::shared_dialect_contract!(PostgresSync);
}

#[cfg(all(not(feature = "postgres-sync"), feature = "tokio-postgres"))]
mod postgres_dialect {
    use super::parity::PostgresAsync;

    crate::shared_dialect_contract!(PostgresAsync);
}

#[cfg(feature = "postgres-sync")]
mod postgres_sync {
    use super::parity::PostgresSync;

    crate::shared_live_driver_contract!(PostgresSync);
}

#[cfg(feature = "tokio-postgres")]
mod postgres_async {
    use super::parity::PostgresAsync;

    crate::shared_live_driver_contract!(PostgresAsync);
}

#[cfg(feature = "mysql-sync")]
mod mysql_dialect {
    use super::parity::MySqlSync;

    crate::shared_dialect_contract!(MySqlSync);
    crate::shared_non_postgres_contract!(MySqlSync);
}

#[cfg(all(not(feature = "mysql-sync"), feature = "mysql-async"))]
mod mysql_dialect {
    use super::parity::MySqlAsync;

    crate::shared_dialect_contract!(MySqlAsync);
    crate::shared_non_postgres_contract!(MySqlAsync);
}

#[cfg(feature = "mysql-sync")]
mod mysql_sync {
    use super::parity::MySqlSync;

    crate::shared_live_driver_contract!(MySqlSync);
}

#[cfg(feature = "mysql-async")]
mod mysql_async {
    use super::parity::MySqlAsync;

    crate::shared_live_driver_contract!(MySqlAsync);
}

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
#[path = "parity/mysql_specific.rs"]
mod mysql_specific;
