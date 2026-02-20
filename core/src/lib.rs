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
#![recursion_limit = "512"]

#[cfg(feature = "alloc")]
extern crate alloc;

// Prelude for std/alloc compatibility
pub(crate) mod prelude {
    // Re-export alloc types for std builds too (they're the same underlying types)
    #[cfg(feature = "std")]
    pub use std::{
        borrow::Cow,
        boxed::Box,
        collections::HashMap,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use alloc::{
        borrow::Cow,
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    // For no_std, use hashbrown instead of std::collections::HashMap
    #[cfg(all(feature = "alloc", not(feature = "std")))]
    pub use hashbrown::HashMap;
}

pub mod bind;
pub mod builder;
pub mod cte;
pub mod dialect;
pub mod error;
#[macro_use]
pub mod traits;
pub mod expr;
pub mod helpers;
pub mod join;
pub mod param;
pub mod placeholder;
pub mod prepared;
#[cfg(feature = "profiling")]
pub mod profiling;
pub mod relation;
pub mod row;
pub mod schema;
pub mod sql;
pub mod tracing;
pub mod types;

// Re-export key types and traits
pub use bind::{BindValue, NullableBindValue, ValueTypeForDialect};
pub use builder::{BuilderInit, ExecutableState};
pub use dialect::{Dialect, DialectExt, DialectTypes, PostgresDialect, SQLiteDialect};
pub use join::{Join, JoinType};
pub use param::{OwnedParam, Param, ParamBind, ParamSet};
pub use placeholder::*;
pub use relation::{Joinable, Relation, SchemaHasTable};
pub use row::{
    AfterFullJoin, AfterJoin, AfterLeftJoin, AfterRightJoin, DecodeSelectedRef, ExprValueType,
    FromDrizzleRow, HasSelectModel, IntoSelectTarget, MarkerColumnCountValid, MarkerScopeValidFor,
    NullProbeRow, ResolveRow, RowColumnList, SQLTypeToRust, ScopePush, Scoped, SelectAs,
    SelectAsFrom, SelectCols, SelectExpr, SelectRequiredTables, SelectStar, WrapNullable,
};
pub use schema::{OrderBy, asc, desc};
pub use sql::{OwnedSQL, OwnedSQLChunk, SQL, SQLChunk, Token};
pub use traits::*;

// =============================================================================
// Helper Macros - Used by proc macros for code generation
// =============================================================================

/// Generates TryFrom implementations for multiple integer types that delegate to i64.
///
/// Used by the SQLiteEnum derive macro to avoid repetitive code.
///
/// # Example
/// ```ignore
/// impl_try_from_int!(MyEnum => isize, usize, i32, u32, i16, u16, i8, u8);
/// ```
#[macro_export]
macro_rules! impl_try_from_int {
    ($name:ty => $($int_type:ty),+ $(,)?) => {
        $(
            impl TryFrom<$int_type> for $name {
                type Error = $crate::error::DrizzleError;

                fn try_from(value: $int_type) -> ::core::result::Result<Self, Self::Error> {
                    Self::try_from(value as i64)
                }
            }
        )+
    };
}
