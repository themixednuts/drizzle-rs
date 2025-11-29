//! SQL generation from schema metadata

pub mod postgres;
pub mod sqlite;

pub use postgres::PostgresGenerator;
pub use sqlite::SqliteGenerator;
