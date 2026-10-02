//! The transaction query builder names.
//!
//! A transaction's queries use the same builder as the database handle's:
//! `TransactionBuilder` is
//! [`DrizzleBuilder`](crate::builder::postgres::common::DrizzleBuilder) with
//! the driver's `Transaction` as its runner, so every clause method is defined
//! once, in `builder/postgres/common.rs`.

pub use crate::builder::postgres::common::{
    DrizzleBuilder as TransactionBuilder, DrizzleOnConflictBuilder as TransactionOnConflictBuilder,
};
