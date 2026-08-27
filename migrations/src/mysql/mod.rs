//! MySQL v6 snapshots, deterministic diffs, and MySQL 8 migration SQL.

pub mod codegen;
pub mod collection;
pub mod ddl;
pub mod diff;
pub mod introspect;
pub mod snapshot;
pub mod statements;

pub use collection::{MySQLDDL, TableEntities, ValidationError};
pub use ddl::*;
pub use diff::*;
pub use snapshot::MySQLSnapshot;
