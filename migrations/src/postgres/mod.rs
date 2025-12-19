//! PostgreSQL schema types matching drizzle-kit format

pub mod codegen;
pub mod collection;
pub mod diff;
pub mod grammar;
pub mod introspect;
pub mod serializer;
mod snapshot;
pub mod statements;

pub use codegen::*;
pub use collection::*;
pub use diff::{diff_full_snapshots, diff_snapshots};

pub use drizzle_types::postgres::ddl;
pub use grammar::*;
pub use introspect::*;
pub use serializer::*;
pub use snapshot::*;

// Re-export shared types from core
pub use crate::collection::Collection;
pub use crate::traits::{Entity, EntityKey, EntityKind};

// Re-export commonly used DDL types at the postgres module level
pub use ddl::{
    CheckConstraint, Column, Enum, ForeignKey, Identity, Index, IndexColumn, Policy,
    PostgresEntity, PrimaryKey, Role, Schema, Sequence, Table, UniqueConstraint, View,
};
