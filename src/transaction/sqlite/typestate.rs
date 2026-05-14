//! Shared typestate-builder macros for SQLite transaction wrappers.
//!
//! Every SQLite driver (`rusqlite`, `libsql`, `turso`, `durable`) wraps its
//! `Transaction` in a per-driver `TransactionBuilder<'tx, [..]>` typestate
//! struct, then mirrors the same set of typestate-advancing builder methods
//! that `drizzle_sqlite::builder` exposes on the outer `QueryBuilder`. The
//! method bodies are all `let builder = self.builder.X(args); Self { ... }`
//! — pure typestate plumbing that doesn't touch anything driver-specific.
//!
//! Hand-written per driver, that's ~250 lines × 4 drivers = ~1,000 lines of
//! near-clones, with the only variation being whether the
//! `TransactionBuilder` struct has a `'conn` lifetime (sync drivers do,
//! async drivers don't).
//!
//! This module supplies one [`impl_tx_delete_where`] macro that emits the
//! `DELETE ... WHERE` typestate impl. Each driver spells out the input and
//! output `TransactionBuilder` types fully (so Rust can resolve the
//! different lifetime shapes) plus the bare path for struct construction.
//! Future commits will add matching macros for `insert.rs` /
//! `update.rs` / `select.rs`.

/// Emit the `WHERE` impl for a `TransactionBuilder` wrapping a
/// `DeleteBuilder<DeleteInitial>`.
///
/// Arguments:
///
/// 1. The bare path to the driver's `TransactionBuilder` struct (used for
///    struct construction in the body — Rust infers the generic args from
///    the return-type context).
/// 2. The complete impl-generics list inside `impl[...]` brackets
///    (square brackets avoid `tt` ambiguity around the closing `>`).
/// 3. The fully-spelled-out *input* `TransactionBuilder` type.
/// 4. The fully-spelled-out *output* `TransactionBuilder` type.
#[macro_export]
macro_rules! impl_tx_delete_where {
    (
        $tx_ctor:path;
        impl[$($impl_gens:tt)*];
        $tx_initial:ty => $tx_where:ty
    ) => {
        impl<$($impl_gens)*> $tx_initial
        where
            T: ::drizzle_sqlite::traits::SQLiteTable<'a>,
        {
            pub fn r#where<E>(self, condition: E) -> $tx_where
            where
                E: ::drizzle_core::expr::Expr<'a, ::drizzle_sqlite::values::SQLiteValue<'a>>,
                E::SQLType: ::drizzle_core::types::BooleanLike,
            {
                let builder = self.builder.r#where(condition);
                $tx_ctor {
                    transaction: self.transaction,
                    builder,
                    _phantom: ::std::marker::PhantomData,
                }
            }
        }
    };
}
