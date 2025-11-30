//! PostgreSQL-specific tests
//!
//! These tests focus on SQL generation, macro validation, and type safety
//! for PostgreSQL-specific features.

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
pub mod sql_generation;
pub mod update;
