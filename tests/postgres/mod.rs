//! PostgreSQL-specific tests
//!
//! End-to-end tests that verify database operations work correctly.

pub mod array_ops;
#[cfg(feature = "arrayvec")]
pub mod arrayvec;
pub mod async_edge_cases;
pub mod conditions;
pub mod delete;
pub mod r#enum;
pub mod for_update;
pub mod foreign_keys;
pub mod fromrow;
pub mod index;
pub mod insert;
pub mod joins;
pub mod migrations;
pub mod prepare;
pub mod schema;
pub mod select;
pub mod transaction;
pub mod type_inference;
pub mod types;
pub mod update;
#[cfg(any(feature = "compact-str", feature = "bytes", feature = "smallvec-types"))]
pub mod wrappers;
