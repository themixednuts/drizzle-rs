#![allow(
    dead_code,
    unused_variables,
    unused_imports,
    unused_mut,
    clippy::clone_on_copy,
    clippy::useless_vec,
    clippy::len_zero,
    clippy::bool_assert_comparison,
    clippy::approx_constant,
    clippy::assertions_on_constants,
    clippy::crate_in_macro_def,
    ambiguous_glob_reexports
)]

pub mod common;

#[cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
pub mod sqlite;

#[cfg(feature = "postgres")]
pub mod postgres;
