//! Drizzle Core - SQL generation library
//!
//! # no_std Support
//!
//! This crate supports `no_std` environments with an allocator:
//!
//! ```toml
//! # With std (default)
//! drizzle-core = "0.1"
//!
//! # no_std with allocator
//! drizzle-core = { version = "0.1", default-features = false, features = ["alloc"] }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]

#[cfg(feature = "alloc")]
extern crate alloc;

// Prelude for std/alloc compatibility
pub(crate) mod prelude {
    #[cfg(feature = "std")]
    pub use std::{
        borrow::Cow,
        boxed::Box,
        collections::HashMap,
        format,
        string::{String, ToString},
        vec::Vec,
    };

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::{
        borrow::Cow,
        boxed::Box,
        format,
        string::{String, ToString},
        vec::Vec,
    };

    // For no_std, use hashbrown with default hasher
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use hashbrown::HashMap;
}

pub mod conversions;
pub mod error;
pub mod expressions;
pub mod helpers;
pub mod param;
pub mod placeholder;
pub mod prepared;
#[cfg(feature = "profiling")]
pub mod profiling;
pub mod query;
pub mod schema;
pub mod sql;
pub mod traits;

// Re-export key types and traits
pub use conversions::ToSQL;
pub use param::{OwnedParam, Param, ParamBind};
pub use placeholder::*;
pub use query::*;
pub use schema::OrderBy;
pub use sql::{OwnedSQL, OwnedSQLChunk, SQL, SQLChunk, Token};
pub use traits::*;
