//! MySQL macro and metadata tests that do not require a concrete driver.

pub mod builder;
pub mod macros;
pub mod seed;

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod crud;
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod custom_column;
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod migrations;
#[cfg(feature = "mysql-async")]
pub mod mysql_async;
#[cfg(feature = "mysql-sync")]
pub mod mysql_sync;
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod prepare;
#[cfg(feature = "query")]
pub mod query;
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod shared_contracts;
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod transaction;
