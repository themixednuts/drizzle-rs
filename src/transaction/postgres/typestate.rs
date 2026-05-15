//! Re-exports of the shared `DrizzleBuilder` / `DrizzleOnConflictBuilder`
//! under the `TransactionBuilder` / `TransactionOnConflictBuilder` names.
//!
//! See `src/transaction/sqlite/typestate.rs` for the parallel SQLite
//! re-export module. The query-builder typestate machinery used to be
//! duplicated here for Postgres transactions; it now lives once on the
//! shared [`crate::builder::postgres::common::DrizzleBuilder`], keyed on
//! an opaque `Runner` parameter that each driver fills in via a type
//! alias.

pub use crate::builder::postgres::common::{
    DrizzleBuilder as TransactionBuilder, DrizzleOnConflictBuilder as TransactionOnConflictBuilder,
};
