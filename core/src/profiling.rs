//! Profiling utilities for drizzle SQL rendering and operations
//!
//! This module provides integration with the puffin profiler to track
//! SQL rendering and database operation performance when the "profiling" feature is enabled.

/// Re-export puffin macros for convenience
pub use puffin::{profile_function, profile_scope};

/// Generic profiling scope macro for high-level operation instrumentation.
#[macro_export]
macro_rules! drizzle_profile_scope {
    ($category:literal, $operation:literal) => {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!($category, $operation);
    };
}

/// Generic profiling function marker.
#[macro_export]
macro_rules! drizzle_profile_function {
    () => {
        #[cfg(feature = "profiling")]
        puffin::profile_function!();
    };
}

/// Profile SQL rendering operations (append, append_raw, etc.)
#[macro_export]
macro_rules! profile_sql {
    ($operation:literal) => {
        #[cfg(feature = "profiling")]
        puffin::profile_scope!("sql_render", $operation);
    };
}
