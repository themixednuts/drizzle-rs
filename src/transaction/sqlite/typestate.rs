//! The transaction query builder names.
//!
//! A transaction's queries use the same builder as the database handle's:
//! `TransactionBuilder` is [`DrizzleBuilder`](crate::builder::sqlite::common::DrizzleBuilder)
//! with the driver's `Transaction` as its runner, so every clause method
//! (`r#where`, `join`, `order_by`, `returning`, ...) is defined once, in
//! `builder/sqlite/common.rs`. SQLite has no `INTERSECT ALL` or `EXCEPT ALL`,
//! so neither builder offers them.

pub use crate::builder::sqlite::common::{
    DrizzleBuilder as TransactionBuilder, DrizzleOnConflictBuilder as TransactionOnConflictBuilder,
};
