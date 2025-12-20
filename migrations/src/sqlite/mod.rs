//! SQLite schema types matching drizzle-kit beta v7 format

// Local DDL module that re-exports from drizzle_types and adds parsing types
pub mod ddl;

pub mod codegen;
pub mod collection;
pub mod diff;
pub mod introspect;
pub mod serializer;
pub mod snapshot;
pub mod statements;

pub use codegen::*;
pub use collection::*;
pub use diff::*;
pub use introspect::*;
pub use serializer::*;
pub use snapshot::*;

// Re-export shared types from core
pub use crate::collection::Collection;
pub use crate::traits::{Entity, EntityKey, EntityKind};

// Re-export commonly used DDL types at the sqlite module level
pub use ddl::{
    CheckConstraint, Column, ForeignKey, Index, IndexColumn, IndexOrigin, PrimaryKey, SqliteEntity,
    Table, UniqueConstraint, View,
};
