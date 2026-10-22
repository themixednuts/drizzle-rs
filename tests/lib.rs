#![recursion_limit = "256"]

pub mod common;

#[cfg(any(
    all(feature = "rusqlite", feature = "postgres"),
    all(feature = "postgres", feature = "mysql")
))]
mod cross_dialect;

#[cfg(feature = "mysql")]
pub mod mysql;

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;
