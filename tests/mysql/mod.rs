//! MySQL macro and metadata tests that do not require a concrete driver.

pub mod builder;
pub mod macros;

#[cfg(feature = "mysql-sync")]
pub mod crud;
#[cfg(feature = "mysql-sync")]
pub mod joins;
#[cfg(feature = "mysql-sync")]
pub mod mysql_sync;
#[cfg(feature = "mysql-sync")]
pub mod prepare;
#[cfg(feature = "mysql-sync")]
pub mod transaction;
