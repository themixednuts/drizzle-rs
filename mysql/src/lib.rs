//! `MySQL` support for drizzle-rs.
//!
//! This crate is the MySQL dialect boundary. It exposes SQL types,
//! client-neutral values, and a typed SQL builder. Feature-gated wire adapters
//! layer execution on those contracts.
//!
//! Wire adapters must set each connection's MySQL session time zone to UTC
//! before executing typed queries. This is the adapter-owned invariant that
//! makes `TIMESTAMP` values round-trip as UTC instants.
//!
//! Adapters must also reject or remove `NO_UNSIGNED_SUBTRACTION` and
//! `REAL_AS_FLOAT` from `sql_mode`. The former changes unsigned subtraction to
//! a signed result, while the latter changes `REAL` from double to float. The
//! static type policy models MySQL's default behavior for both.

#![cfg_attr(not(feature = "std"), no_std)]

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
pub mod common;
pub mod driver;
pub mod helpers;
pub mod index;
pub mod result;
pub mod traits;
pub mod transaction;
pub mod types {
    pub use drizzle_types::mysql::types::*;
}

pub mod values;

pub use common::MySQLViewInfo;
pub use driver::{MySQLRow, MySQLRowAccess};
pub use drizzle_core::{MySQLDialect, ParamBind};
pub use drizzle_types::mysql::ddl::{ViewAlgorithm, ViewCheckOption, ViewSqlSecurity};
pub use index::{
    IndexKeyPart, IndexOrder, MySQLIndexAlgorithm, MySQLIndexLock, MySQLIndexMetadata,
    MySQLIndexMethod,
};
pub use result::MySQLMutationResult;
pub use transaction::{
    AccessMode, IsolationLevel, MySQLAccessMode, MySQLIsolationLevel, MySQLTransactionConfig,
    TransactionConfig,
};
