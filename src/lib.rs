//! # Drizzle for Rust
//!
//! A type-safe SQL query builder for Rust, supporting SQLite and PostgreSQL.
//!
//! ## Quick Start
//!
//! ```rust
//! # #[cfg(feature = "rusqlite")]
//! # fn main() -> drizzle::Result<()> {
//! use drizzle::sqlite::prelude::*;
//! use drizzle::sqlite::rusqlite::Drizzle;
//!
//! #[SQLiteTable(name = "Users")]
//! struct User {
//!     #[column(primary)]
//!     id: i32,
//!     name: String,
//!     email: Option<String>,
//! }
//!
//! #[derive(SQLiteSchema)]
//! struct Schema {
//!     user: User,
//! }
//!
//! let conn = rusqlite::Connection::open_in_memory()?;
//! let (db, Schema { user, .. }) = Drizzle::new(conn, Schema::new());
//!
//! db.create()?;
//! db.insert(user)
//!     .values([InsertUser::new("John Doe").with_email("john@example.com")])
//!     .execute()?;
//!
//! let users: Vec<SelectUser> = db.select(()).from(user).all()?;
//! # Ok(())
//! # }
//! # #[cfg(not(feature = "rusqlite"))]
//! # fn main() {}
//! ```
//!
//! ## Database Support
//!
//! | Database   | Driver         | Feature Flag     | Status |
//! |------------|----------------|------------------|--------|
//! | SQLite     | rusqlite       | `rusqlite`       | ✅     |
//! | SQLite     | libsql         | `libsql`         | ✅     |
//! | SQLite     | turso          | `turso`          | ✅     |
//! | PostgreSQL | postgres       | `postgres-sync`  | ✅     |
//! | PostgreSQL | tokio-postgres | `tokio-postgres` | ✅     |
//!
//! For schema declarations, import the database prelude:
//! - `drizzle::sqlite::prelude::*`
//! - `drizzle::postgres::prelude::*`
//!
//! For expressions and conditions, import from `drizzle::core::expr`.

#![cfg_attr(not(feature = "std"), no_std)]
#![cfg_attr(docsrs, feature(doc_cfg, rustdoc_internals))]
#![allow(
    unexpected_cfgs,
    unused_imports,
    unused_macros,
    unused_mut,
    dead_code,
    clippy::redundant_closure,
    clippy::needless_question_mark,
    clippy::await_holding_refcell_ref,
    clippy::duplicated_attributes,
    clippy::single_component_path_imports
)]

#[cfg(all(not(feature = "std"), feature = "alloc"))]
extern crate alloc;

#[macro_use]
mod builder;

#[macro_use]
mod macros;

#[macro_use]
mod transaction;

#[doc(hidden)]
pub(crate) use drizzle_builder_join_impl;
#[doc(hidden)]
pub(crate) use drizzle_pg_builder_join_impl;
#[doc(hidden)]
pub(crate) use drizzle_pg_builder_join_using_impl;
#[doc(hidden)]
pub(crate) use transaction_builder_join_impl;

/// Result type for drizzle operations.
#[doc(inline)]
pub use drizzle_core::error::Result;

#[cfg(feature = "std")]
#[doc(inline)]
pub use drizzle_macros::include_migrations;
/// SQL template macro.
#[doc(inline)]
pub use drizzle_macros::sql;

/// Attribute macro for integration tests with DI-style `db` / `schema`
/// parameters. See [`drizzle_macros::test`] for full documentation.
#[doc(inline)]
pub use drizzle_macros::test;

/// Database dialect enum.
#[doc(inline)]
pub use drizzle_types::Dialect;

/// Re-export const_format for proc macro generated compile-time SQL.
#[doc(hidden)]
pub use const_format;

/// Error types.
pub mod error {
    #[doc(inline)]
    pub use drizzle_core::error::DrizzleError;
}

/// DDL types and schema definitions.
pub mod ddl {
    #[doc(inline)]
    pub use drizzle_types::postgres;
    #[doc(inline)]
    pub use drizzle_types::sqlite;
}

/// Migration helpers and schema snapshots.
#[cfg(feature = "std")]
pub mod migrations {
    #[doc(inline)]
    pub use drizzle_migrations::*;
}

/// Core traits, SQL types, and expressions shared across drivers.
pub mod core {
    /// SQL building blocks.
    #[doc(inline)]
    pub use drizzle_core::{
        ColumnDialect, ColumnFlags, ColumnRef, ConstraintRef, ForeignKeyRef, OrderBy, Param,
        ParamBind, ParamSet, PrimaryKeyRef, SQL, SQLChunk, TableDialect, TableRef, Token,
        TypedPlaceholder, asc, desc,
    };

    /// Conversion trait for SQL generation.
    #[doc(inline)]
    pub use drizzle_core::ToSQL;

    /// Core traits (SQLTable, SQLColumn, SQLSchema, SQLModel, etc.).
    #[doc(inline)]
    pub use drizzle_core::traits::*;

    /// Relation metadata types and traits.
    #[doc(inline)]
    pub use drizzle_core::relation::{Joinable, Relation, SchemaHasTable};

    /// Full relation module exports.
    pub mod relation {
        #[doc(inline)]
        pub use drizzle_core::relation::*;
    }

    /// Prepared statement types.
    #[doc(inline)]
    pub use drizzle_core::prepared::{OwnedPreparedStatement, PreparedStatement};

    /// SQL type markers used by expressions.
    #[doc(inline)]
    pub use drizzle_core::types;

    /// Type-safe expressions and helpers.
    #[doc(inline)]
    pub use drizzle_core::expr;

    #[doc(hidden)]
    pub use drizzle_core::impl_try_from_int;

    #[doc(hidden)]
    pub use drizzle_core::schema::SQLEnumInfo;

    /// Bind parameter type mapping trait.
    #[doc(inline)]
    pub use drizzle_core::ValueTypeForDialect;

    /// Dialect markers (SQLiteDialect, PostgresDialect, etc.).
    pub mod dialect {
        #[doc(inline)]
        pub use drizzle_core::dialect::*;
    }

    /// Query API types (relational queries with nested loading).
    #[cfg(feature = "query")]
    pub mod query {
        #[doc(inline)]
        pub use drizzle_core::query::*;
    }

    /// Re-export serde_json for proc macro generated code.
    #[cfg(feature = "query")]
    #[doc(hidden)]
    pub use drizzle_core::serde_json;

    /// Row inference types and traits.
    #[doc(inline)]
    pub use drizzle_core::row::{
        AfterFullJoin, AfterJoin, AfterLeftJoin, AfterRightJoin, DecodeSelectedRef, ExprValueType,
        FromDrizzleRow, GroupByIdentity, HasSelectModel, IntoGroupBy, IntoSelectTarget,
        MarkerColumnCountValid, MarkerScopeValidFor, NullProbeRow, ResolveRow, RowColumnList,
        SQLTypeToRust, ScopePush, Scoped, SelectAs, SelectAsFrom, SelectCols, SelectExpr,
        SelectRequiredTables, SelectStar, WrapNullable,
    };
}

/// SQLite types, macros, and query builder.
#[cfg(feature = "sqlite")]
#[cfg_attr(docsrs, doc(cfg(feature = "sqlite")))]
pub mod sqlite {
    #[doc(inline)]
    pub use drizzle_macros::{
        SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable, SQLiteView,
    };
    #[doc(inline)]
    pub use drizzle_sqlite::{
        attrs, builder, common, connection, expr, helpers, pragma, traits, types, values,
    };

    #[cfg(feature = "rusqlite")]
    #[cfg_attr(docsrs, doc(cfg(feature = "rusqlite")))]
    pub mod rusqlite {
        #[doc(inline)]
        pub use crate::builder::sqlite::rusqlite::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::sqlite::rusqlite::Transaction;
    }

    #[cfg(feature = "libsql")]
    #[cfg_attr(docsrs, doc(cfg(feature = "libsql")))]
    pub mod libsql {
        #[doc(inline)]
        pub use crate::builder::sqlite::libsql::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::sqlite::libsql::Transaction;
    }

    #[cfg(feature = "turso")]
    #[cfg_attr(docsrs, doc(cfg(feature = "turso")))]
    pub mod turso {
        #[doc(inline)]
        pub use crate::builder::sqlite::turso::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::sqlite::turso::Transaction;
    }

    /// Cloudflare D1 driver (async, WASM-only).
    #[cfg(all(feature = "d1", target_arch = "wasm32"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "d1", target_arch = "wasm32"))))]
    pub mod d1 {
        #[doc(inline)]
        pub use crate::builder::sqlite::d1::{Drizzle, DrizzleBuilder};
    }

    /// Cloudflare Durable Objects SQL storage driver (sync, WASM-only).
    #[cfg(all(feature = "durable", target_arch = "wasm32"))]
    #[cfg_attr(docsrs, doc(cfg(all(feature = "durable", target_arch = "wasm32"))))]
    pub mod durable {
        #[doc(inline)]
        pub use crate::builder::sqlite::durable::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::sqlite::durable::Transaction;
    }

    /// SQLite prelude for schema declarations.
    pub mod prelude {
        // Core types and traits
        pub use crate::core::ToSQL;
        pub use crate::core::{Joinable, Relation, SchemaHasTable};
        pub use crate::core::{
            OrderBy, Param, ParamBind, ParamSet, SQL, SQLChunk, Token, TypedPlaceholder, asc, desc,
        };
        pub use crate::core::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::tag;
        pub use drizzle_core::traits::*;
        // SQLite macros
        pub use drizzle_macros::{
            SQLiteEnum, SQLiteFromRow, SQLiteIndex, SQLiteSchema, SQLiteTable, SQLiteView,
        };
        // SQLite types
        pub use drizzle_sqlite::attrs::*;
        pub use drizzle_sqlite::common::SQLiteSchemaType;
        pub use drizzle_sqlite::traits::{SQLiteColumn, SQLiteTable};
        pub use drizzle_sqlite::values::{SQLiteInsertValue, SQLiteUpdateValue, SQLiteValue};
    }
}

/// PostgreSQL types, macros, and query builder.
#[cfg(feature = "postgres")]
#[cfg_attr(docsrs, doc(cfg(feature = "postgres")))]
pub mod postgres {
    #[doc(inline)]
    pub use drizzle_macros::{
        PostgresEnum, PostgresFromRow, PostgresIndex, PostgresSchema, PostgresTable, PostgresView,
    };
    #[doc(inline)]
    pub use drizzle_postgres::{attrs, builder, common, expr, helpers, traits, types, values};

    #[cfg(all(feature = "postgres-sync", not(feature = "tokio-postgres")))]
    #[doc(inline)]
    pub use drizzle_postgres::Row;
    #[cfg(feature = "tokio-postgres")]
    #[doc(inline)]
    pub use drizzle_postgres::Row;

    /// AWS Aurora Data API row + helpers (re-exported from drizzle-postgres).
    ///
    /// Required by the `#[PostgresTable]` macro when the `aws-data-api` feature
    /// is enabled — macro-generated code refers to
    /// `drizzle::postgres::aws_data_api::Row` and
    /// `drizzle::postgres::aws_data_api::is_null_at`.
    #[cfg(feature = "aws-data-api")]
    #[cfg_attr(docsrs, doc(cfg(feature = "aws-data-api")))]
    #[doc(inline)]
    pub use drizzle_postgres::aws_data_api;

    #[cfg(feature = "postgres-sync")]
    #[cfg_attr(docsrs, doc(cfg(feature = "postgres-sync")))]
    pub mod sync {
        #[doc(inline)]
        pub use crate::builder::postgres::postgres_sync::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::postgres::postgres_sync::Transaction;
    }

    #[cfg(feature = "tokio-postgres")]
    #[cfg_attr(docsrs, doc(cfg(feature = "tokio-postgres")))]
    pub mod tokio {
        #[doc(inline)]
        pub use crate::builder::postgres::tokio_postgres::{Drizzle, DrizzleBuilder};
        #[doc(inline)]
        pub use crate::transaction::postgres::tokio_postgres::Transaction;
    }

    /// AWS Aurora Serverless Data API driver (HTTP-based, async).
    #[cfg(feature = "aws-data-api")]
    #[cfg_attr(docsrs, doc(cfg(feature = "aws-data-api")))]
    pub mod aws {
        #[doc(inline)]
        pub use crate::builder::postgres::aws_data_api::{Drizzle, DrizzleBuilder, Rows};
        #[doc(inline)]
        pub use crate::transaction::postgres::aws_data_api::{Transaction, TransactionBuilder};
    }

    /// PostgreSQL prelude for schema declarations.
    pub mod prelude {
        // Core types and traits
        pub use crate::core::ToSQL;
        pub use crate::core::{Joinable, Relation, SchemaHasTable};
        pub use crate::core::{
            OrderBy, Param, ParamBind, ParamSet, SQL, SQLChunk, Token, TypedPlaceholder, asc, desc,
        };
        pub use crate::core::{OwnedPreparedStatement, PreparedStatement};
        pub use drizzle_core::tag;
        pub use drizzle_core::traits::*;
        // PostgreSQL macros
        pub use drizzle_macros::{
            PostgresEnum, PostgresFromRow, PostgresIndex, PostgresSchema, PostgresTable,
            PostgresView,
        };
        // PostgreSQL types
        pub use drizzle_postgres::attrs::*;
        pub use drizzle_postgres::common::PostgresSchemaType;
        pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
        pub use drizzle_postgres::values::{
            PostgresInsertValue, PostgresUpdateValue, PostgresValue,
        };
    }
}

/// MySQL types and macros (WIP).
#[cfg(feature = "mysql")]
#[cfg_attr(docsrs, doc(cfg(feature = "mysql")))]
pub mod mysql {}

// =============================================================================
// Compile-fail tests (verified during `cargo test --doc`)
// =============================================================================

/// Type safety: abs() rejects non-numeric columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::abs;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = abs(user.name);
/// }
/// ```
///
/// Type safety: avg() rejects non-numeric columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::avg;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = avg(user.name);
/// }
/// ```
///
/// Type safety: sum() rejects non-numeric columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::sum;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = sum(user.name);
/// }
/// ```
///
/// Type safety: Blob is not compatible with Integer.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::eq;
///
/// #[SQLiteTable]
/// struct Config {
///     #[column(primary)]
///     id: i32,
///     data: Vec<u8>,
/// }
///
/// fn main() {
///     let config = Config::default();
///     let _ = eq(config.data, 42);
/// }
/// ```
///
/// Type safety: Int is not compatible with Text.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::eq;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = eq(user.id, "hello");
/// }
/// ```
///
/// Type safety: coalesce() rejects incompatible types.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::coalesce;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = coalesce(user.id, "default");
/// }
/// ```
///
/// Type safety: concat() rejects non-textual columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::concat;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = concat(user.id, user.name);
/// }
/// ```
///
/// Type safety: date() rejects non-temporal columns (Blob).
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::date;
///
/// #[SQLiteTable]
/// struct Data {
///     #[column(primary)]
///     id: i32,
///     content: Vec<u8>,
/// }
///
/// fn main() {
///     let data = Data::default();
///     let _ = date(data.content);
/// }
/// ```
///
/// Type safety: like() rejects non-textual columns.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
/// use drizzle::core::expr::like;
///
/// #[SQLiteTable]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// fn main() {
///     let user = User::default();
///     let _ = like(user.id, "%test%");
/// }
/// ```
///
/// Type safety: FK column type must match referenced column type.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Parent {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[SQLiteTable]
/// struct Child {
///     #[column(primary)]
///     id: i32,
///     #[column(references = Parent::id)]
///     parent_ref: String,
/// }
///
/// fn main() {}
/// ```
///
/// Type safety: FK target table must be in the schema.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Parent {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[SQLiteTable]
/// struct Child {
///     #[column(primary)]
///     id: i32,
///     #[column(references = Parent::id)]
///     parent_id: Option<i32>,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct BadSchema {
///     child: Child,
/// }
///
/// fn main() {}
/// ```
///
/// Type safety: HasConstraint requires actual FK on the table.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Simple {
///     #[column(primary)]
///     id: i32,
///     value: String,
/// }
///
/// fn requires_fk_constraint<T: HasConstraint<ForeignKeyK>>() {}
///
/// fn main() {
///     requires_fk_constraint::<Simple>();
/// }
/// ```
///
/// Type safety: Relation requires a FK between the tables.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Parent {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[SQLiteTable]
/// struct Simple {
///     #[column(primary)]
///     id: i32,
///     value: String,
/// }
///
/// fn requires_relation<T: Relation<Parent>>() {}
///
/// fn main() {
///     requires_relation::<Simple>();
/// }
/// ```
///
/// Type safety: Joinable requires a FK relationship.
/// ```compile_fail,E0277
/// use drizzle::sqlite::prelude::*;
///
/// #[SQLiteTable]
/// struct Parent {
///     #[column(primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[SQLiteTable]
/// struct Unrelated {
///     #[column(primary)]
///     id: i32,
///     value: String,
/// }
///
/// fn requires_joinable<A: Joinable<B>, B>() {}
///
/// fn main() {
///     requires_joinable::<Unrelated, Parent>();
/// }
/// ```
#[cfg(doctest)]
struct _CompileFailTests;
