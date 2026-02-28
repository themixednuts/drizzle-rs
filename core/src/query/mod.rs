//! Relational Query API.
//!
//! Provides type-safe relational queries with nested relation loading.
//!
//! The pipeline: [`QueryBuilder`] configures filtering/pagination and collects
//! [`RelationHandle`]s via `.with()`. At execution time, handles are rendered
//! into SQL via [`RenderRelations`], the query is built by [`build_query_sql`],
//! and results are deserialized via [`DeserializeStore`] + [`FromJsonValue`]
//! into [`QueryRow<Base, Store>`](QueryRow) values.

mod builder;
mod deser;
mod find;
#[doc(hidden)]
pub mod handle;
mod row;
mod sql;
mod store;

pub use builder::{
    AllColumns, BuildStore, Clauses, HasLimit, HasOffset, HasOrderBy, HasWhere,
    IntoColumnSelection, NoLimit, NoOrderBy, NoWhere, PartialColumns, QueryBuilder, QueryTable,
    ResolveSelect,
};
pub use deser::{DeserializeStore, FromJsonColumn, FromJsonValue, deserialize_field};
pub use find::{FindRel, Here, There};
pub use handle::RelationHandle;
pub use row::QueryRow;
pub use sql::{RelCardinality, RenderRelations, RenderedRelation, build_query_sql};
pub use store::RelEntry;
