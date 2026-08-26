//! MySQL macro and metadata tests that do not require a concrete driver.

pub mod builder;
pub mod macros;

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod crud;
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod joins;
#[cfg(feature = "mysql-async")]
pub mod mysql_async;
#[cfg(feature = "mysql-sync")]
pub mod mysql_sync;
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod prepare;
#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub mod transaction;
