#![cfg(feature = "postgres")]

// Driver modules
#[cfg(feature = "postgres-sync")]
pub(crate) mod postgres_sync;

#[cfg(feature = "tokio-postgres")]
pub(crate) mod tokio_postgres;

// #[cfg(feature = "sqlx-postgres")]
// pub(crate) mod sqlx;

pub use drizzle_postgres::*;
