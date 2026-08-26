//! MySQL transaction adapters.

#[cfg(feature = "mysql-async")]
pub mod mysql_async;

#[cfg(feature = "mysql-sync")]
pub mod mysql_sync;
