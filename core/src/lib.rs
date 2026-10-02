//! SQL building blocks shared by every Drizzle dialect.
//!
//! Most users depend on the `drizzle` crate, which re-exports this one as
//! `drizzle::core`. This crate holds the parts that do not depend on a
//! database:
//!
//! - [`SQL`] and [`ToSQL`]: SQL fragments with bound parameters.
//! - [`expr`]: typed expressions and functions (`eq`, `count`, `coalesce`, ...).
//! - [`row`] and [`scope`]: compile-time row-type inference and query checks.
//! - [`traits`]: the table, column, and schema traits the macros implement.
//! - [`dialect`]: dialect markers and dialect-only feature gates.
//! - [`error::DrizzleError`]: the error type every driver returns.
//!
//! # Examples
//!
//! Building a fragment by hand. `Value` stands in for a driver's value type
//! (`SQLiteValue`, `PostgresValue`, ...):
//!
//! ```
//! use drizzle_core::{SQL, Token};
//! # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
//! # use std::borrow::Cow;
//! # #[derive(Debug, Clone)]
//! # struct Value(i64);
//! # impl SQLParam for Value {
//! #     const DIALECT: Dialect = Dialect::SQLite;
//! #     type DialectMarker = SQLiteDialect;
//! # }
//! # impl From<Value> for Cow<'_, Value> {
//! #     fn from(value: Value) -> Self { Cow::Owned(value) }
//! # }
//!
//! let query: SQL<'_, Value> = SQL::raw("SELECT * FROM")
//!     .append(SQL::ident("users"))
//!     .push(Token::WHERE)
//!     .append(SQL::ident("id"))
//!     .push(Token::EQ)
//!     .append(SQL::param(Value(42)));
//!
//! assert_eq!(query.sql(), r#"SELECT * FROM "users" WHERE "id" = ?"#);
//! assert_eq!(query.params().count(), 1);
//! ```
//!
//! # Compile-time checks
//!
//! Queries are checked when they are built and run, not when they reach the
//! database. The compiler rejects a query that:
//!
//! - reads a table it never added with `.from(...)` or a join
//!   ([`scope`], [`MarkerScopeValidFor`]);
//! - decodes a column from the nullable side of an outer join as `T`
//!   instead of `Option<T>` ([`MarkerColumnCountValid`]);
//! - selects a non-aggregate column that is not in GROUP BY
//!   ([`MarkerAggValidFor`]);
//! - calls a clause out of order, such as `.r#where(...)` after
//!   `.limit(...)` ([`ClauseAllowed`]);
//! - uses a function the dialect lacks ([`DialectSupports`]).
//!
//! # `no_std` Support
//!
//! This crate supports `no_std` environments with an allocator:
//!
//! ```toml
//! # With std (default)
//! drizzle-core = "0.2"
//!
//! # no_std with allocator
//! drizzle-core = { version = "0.2", default-features = false, features = ["alloc"] }
//! ```

#![cfg_attr(not(feature = "std"), no_std)]
#![recursion_limit = "512"]

#[cfg(not(feature = "std"))]
extern crate alloc;

// Prelude for std/alloc compatibility
pub(crate) mod prelude {
    // Re-export alloc types for std builds too (they're the same underlying types)
    #[cfg(feature = "std")]
    pub use std::{
        borrow::Cow,
        boxed::Box,
        collections::{HashMap, HashSet},
        format,
        rc::Rc,
        string::{String, ToString},
        sync::Arc,
        vec,
        vec::Vec,
    };

    #[cfg(not(feature = "std"))]
    pub use alloc::{
        borrow::Cow,
        boxed::Box,
        format,
        string::{String, ToString},
        vec,
        vec::Vec,
    };

    #[cfg(all(not(feature = "std"), feature = "alloc"))]
    pub use alloc::{rc::Rc, sync::Arc};

    // For no_std, use hashbrown instead of std::collections::{HashMap, HashSet}
    #[cfg(not(feature = "std"))]
    pub use hashbrown::{HashMap, HashSet};
}

pub mod bind;
pub mod builder;
pub mod conv;
pub mod cte;
pub mod dialect;
pub mod error;
#[macro_use]
pub mod traits;
pub mod derived;
pub mod expr;
pub mod helpers;
pub mod join;
#[cfg(feature = "serde")]
pub mod json;
pub mod pagination;
pub mod param;
pub mod placeholder;
pub mod prepared;
#[cfg(feature = "profiling")]
pub mod profiling;
#[cfg(feature = "query")]
pub mod query;
pub mod relation;
#[cfg(any(feature = "serde", feature = "query"))]
#[doc(hidden)]
pub use serde;
#[cfg(any(feature = "serde", feature = "query"))]
#[doc(hidden)]
pub use serde_json;
pub mod row;
pub mod schema;
pub mod scope;
pub mod sql;
pub mod tracing;
pub mod types;

// Re-export key types and traits
pub use bind::{BindValue, NullableBindValue, ValueTypeForDialect};
pub use builder::{
    BuilderInit, ClauseAllowed, ExecutableState, IncludesRequired, InsertColumn, InsertColumnsSet,
    InsertSelectAllColumns, InsertSelectColumns, InsertSelectCompatible, InsertSelectTable,
    InsertTargetColumnList, InsertTargetColumns, InsertTargetMarker, PartialInsertSelectCompatible,
    clause,
};
pub use derived::{
    Derived, DerivedField, DerivedProjection, DerivedSelection, ProjectionOutput, TableProjection,
};
pub use dialect::{
    Dialect, DialectSupports, DialectTypes, MySQLDialect, PostgresDialect, SQLiteDialect, feature,
};
pub use join::{Join, JoinType, LateralArg, LateralSource};
#[cfg(feature = "serde")]
pub use json::Json;
pub use pagination::PaginationArg;
pub use param::{OwnedParam, Param, ParamBind, ParamSet};
pub use placeholder::*;
#[cfg(feature = "query")]
pub use relation::{AssembleRel, CardWrap, Many, One, OptionalOne, RelationDef};
pub use relation::{Joinable, Relation, SchemaHasTable};
pub use row::{
    DecodeSelectedRef, ExprValueType, FromDrizzleRow, GroupByIdentity, HasSelectModel, IntoGroupBy,
    IntoSelectTarget, JoinedStarRow, LeftLateralSelection, MarkerAggValidFor,
    MarkerColumnCountValid, MarkerScopeValidFor, NullProbeRow, PkGroup, ResolveRow, RowColumnList,
    SQLTypeToRust, SelectAs, SelectAsFrom, SelectCols, SelectExpr, SelectStar, SelectTableFields,
    SelectedExpressionList, TableFields, WrapNullable,
};
#[doc(hidden)]
pub use row::{MaybeNull, ProjectionIn};
pub use schema::{OrderBy, OrderTerm, Ordered, asc, desc};
pub use scope::{
    AliasKey, FromMarker, FullJoin, HasScope, InnerJoin, JoinStep, Lateral, LeftJoin, OuterJoined,
    RightJoin, ScopeContains, ScopeEntry, Scoped, SelectSources, SetOperand, Src,
};
pub use sql::{
    ColumnDialect, ColumnFlags, ColumnRef, ColumnSqlRef, ConstraintRef, ForeignKeyRef, OwnedSQL,
    OwnedSQLChunk, PrimaryKeyRef, SQL, SQLChunk, TableDialect, TableRef, TableSqlRef, Token,
};
pub use traits::*;

// =============================================================================
// Helper Macros - Used by proc macros for code generation
// =============================================================================

/// Implements `TryFrom<int>` for several integer types by converting to
/// `i64` and calling the type's `TryFrom<i64>`.
///
/// The type must already implement `TryFrom<i64, Error = DrizzleError>`.
/// The `SQLiteEnum` derive uses this; you rarely need it directly.
///
/// # Examples
///
/// ```
/// use drizzle_core::error::DrizzleError;
/// use drizzle_core::impl_try_from_int;
///
/// #[derive(Debug, PartialEq)]
/// enum Role {
///     User,
///     Admin,
/// }
///
/// impl TryFrom<i64> for Role {
///     type Error = DrizzleError;
///
///     fn try_from(value: i64) -> Result<Self, Self::Error> {
///         match value {
///             0 => Ok(Role::User),
///             1 => Ok(Role::Admin),
///             _ => Err(DrizzleError::ConversionError("unknown role".into())),
///         }
///     }
/// }
///
/// impl_try_from_int!(Role => i32, u8);
///
/// assert_eq!(Role::try_from(1_i32).unwrap(), Role::Admin);
/// assert!(Role::try_from(7_u8).is_err());
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
