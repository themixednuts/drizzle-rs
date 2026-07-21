//! Relational Query API.
//!
//! Provides type-safe relational queries with nested relation loading.
//!
//! [`QueryBuilder`] collects [`RelationHandle`]s via `.with()`. At execution
//! time, handles are rendered into SQL via [`RenderRelations`] and
//! [`build_query_sql`]. Results decode through [`DeserializeStore`] /
//! [`FromJsonObject`] and are assembled into concrete `*With*` row types by
//! [`BuildRow`].

mod builder;
mod deser;
#[doc(hidden)]
pub mod handle;
mod row;
mod sql;
mod store;

pub use builder::{
    AllColumns, BuildRow, BuildStore, Clauses, HasLimit, HasOffset, HasOrderBy, HasWhere,
    IntoColumnSelection, NoLimit, NoOrderBy, NoWhere, PartialColumns, QueryBuilder, QueryTable,
    ResolveSelect,
};
pub use deser::{
    DeserializeStore, FromJsonColumn, FromJsonField, FromJsonObject, JsonBool, JsonObjectDecoder,
    JsonOptionalBool,
};
pub use handle::RelationHandle;
pub use row::QueryRow;
pub use sql::{RelCardinality, RenderRelations, RenderedRelation, build_query_sql};
pub use store::RelEntry;
