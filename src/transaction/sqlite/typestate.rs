//! Re-exports of the shared `DrizzleBuilder` / `DrizzleOnConflictBuilder`
//! under the `TransactionBuilder` / `TransactionOnConflictBuilder` names.
//!
//! The query-builder typestate machinery used to live here in a duplicated
//! `TransactionBuilder<'tx, Tx, Schema, Builder, State>` struct with its own
//! `delete` / `insert` / `update` / `select` impls — structurally identical
//! to the `DrizzleBuilder` in [`crate::builder::sqlite::common`]. Those two
//! collapsed into one generic struct keyed on an opaque `Runner` parameter,
//! so the Transaction layer just aliases the canonical type with
//! `Runner = Transaction<'conn, Schema>` (per driver). All typestate-advancing
//! methods (`.value` / `.values` / `.r#where` / `.set` / `.on_conflict[*]` /
//! `.returning` / `.from` / `.join_*` / `.group_by` / `.having` /
//! `.order_by` / `.limit` / `.offset` / `.union[_all]` / `.intersect[_all]` /
//! `.except[_all]` / `.into_cte`) now live exactly once in
//! `builder/sqlite/common.rs`.
//!
//! Re-exporting under the old names preserves the `TransactionBuilder` /
//! `TransactionOnConflictBuilder` identifiers in user-facing error messages
//! without paying for the duplicate definition.

pub use crate::builder::sqlite::common::{
    DrizzleBuilder as TransactionBuilder, DrizzleOnConflictBuilder as TransactionOnConflictBuilder,
};
