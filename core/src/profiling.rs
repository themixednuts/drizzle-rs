//! Profiling hooks for the puffin profiler.
//!
//! The macros below expand to puffin scopes when the *calling* crate has a
//! `profiling` feature enabled, and to nothing otherwise.

/// puffin's own scope macros, re-exported.
#[cfg(feature = "profiling")]
pub use puffin::{profile_function, profile_scope};

/// Opens a puffin scope named `$operation` in `$category` until the end of
/// the enclosing block.
#[macro_export]
macro_rules! drizzle_profile_scope {
    ($category:literal, $operation:literal) => {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!($category, $operation);
    };
}

/// Opens a puffin scope for the enclosing function.
#[macro_export]
macro_rules! drizzle_profile_function {
    () => {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
    };
}

/// Opens a puffin scope for a SQL rendering step (`append`, `join`, ...),
/// in the `sql_render` category.
#[macro_export]
macro_rules! profile_sql {
    ($operation:literal) => {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("sql_render", $operation);
    };
}
