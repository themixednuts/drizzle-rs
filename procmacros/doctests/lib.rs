//! The `drizzle` that `drizzle-macros`' doctests compile against.
//!
//! `drizzle-macros` depends on this crate under the name `drizzle`, and this
//! crate re-exports `drizzle`. A proc-macro's dev-dependencies don't share
//! features with the rest of the build, so each `drizzle-macros` feature
//! forwards here, and each feature here forwards to `drizzle`.
//!
//! The indirection is for release-plz. It orders a dev-dependency that a
//! feature names before the crate naming it, and `drizzle` depends on
//! `drizzle-macros`, so forwarding straight to `drizzle` reads as a
//! release-order cycle. This crate is not published, so release-plz leaves it
//! out of the order.

pub use drizzle::*;
