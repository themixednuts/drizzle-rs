//! `MySQL` dialect for drizzle-rs.
//!
//! This crate holds the `MySQL` parts of drizzle-rs: the typed query builder
//! ([`builder::QueryBuilder`]), values ([`values::MySQLValue`]), row
//! decoding ([`driver`]), transaction options ([`transaction`]), and the
//! traits that `#[MySQLTable]` and friends implement. It does not connect to
//! a server. Most applications use it through the `drizzle` crate
//! (`drizzle::mysql`), whose `mysql` and `mysql_async` drivers run the
//! queries.
//!
//! A driver must set each connection's session time zone to UTC before
//! running typed queries, so `TIMESTAMP` values round-trip as UTC instants.
//! It must also remove `NO_UNSIGNED_SUBTRACTION` and `REAL_AS_FLOAT` from
//! `sql_mode`: the first makes unsigned subtraction signed, the second makes
//! `REAL` a single-precision float, and the static types assume `MySQL`'s
//! default behavior for both. The `drizzle` drivers do both on connect.

#![cfg_attr(not(feature = "std"), no_std)]
#![warn(missing_docs)]

#[cfg(not(feature = "std"))]
extern crate alloc;

pub(crate) mod prelude {
    #[cfg(feature = "std")]
    pub use std::{borrow::Cow, boxed::Box, rc::Rc, string::String, sync::Arc, vec::Vec};

    #[cfg(not(feature = "std"))]
    pub use alloc::{
        borrow::Cow,
        boxed::Box,
        rc::Rc,
        string::{String, ToString},
        sync::Arc,
        vec::Vec,
    };
}

pub mod attrs;
pub mod builder;
/// The `MySQL` schema marker and `CREATE VIEW` rendering for generated
/// views.
pub mod common;
pub mod driver;
pub mod helpers;
pub mod index;
pub mod result;
/// Traits implemented by generated MySQL schema types and custom columns.
pub mod traits;
pub mod transaction;
/// SQL type markers for `MySQL` columns (`Int`, `Varchar`, `Json`, ...),
/// re-exported from `drizzle-types`.
pub mod types {
    pub use drizzle_types::mysql::types::*;
}

pub mod values;

pub use common::MySQLViewInfo;
pub use driver::{MySQLRow, MySQLRowAccess};
pub use drizzle_core::{MySQLDialect, ParamBind};
pub use drizzle_types::mysql::ddl::{ViewAlgorithm, ViewCheckOption, ViewSqlSecurity};
pub use index::{IndexKeyPart, IndexOrder, MySQLIndexAlgorithm, MySQLIndexLock, MySQLIndexMethod};
pub use result::MySQLMutationResult;
pub use transaction::{AccessMode, IsolationLevel, TransactionConfig};
