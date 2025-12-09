//! PostgreSQL-specific tests
//!
//! End-to-end tests that verify database operations work correctly.

#[cfg(feature = "arrayvec")]
pub mod arrayvec;
pub mod conditions;
pub mod delete;
pub mod r#enum;
pub mod index;
pub mod insert;
pub mod joins;
pub mod schema;
pub mod select;
pub mod type_inference;
pub mod update;
