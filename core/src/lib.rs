pub mod conversions;
pub mod error;
pub mod expressions;
pub mod helpers;
pub mod params;
pub mod prepared;
pub mod schema;
pub mod sql;
pub mod traits;

// Re-export key types and traits
pub use conversions::ToSQL;
pub use params::{OwnedParam, Param, ParamBind, Placeholder, PlaceholderStyle, placeholders};
pub use schema::{OrderBy, SQLSchemaType};
pub use sql::{OwnedSQL, OwnedSQLChunk, SQL, SQLChunk};
pub use traits::*;

/// Creates an aliased table that can be used in joins and queries
/// Usage: alias!(User, "u") creates an alias of the User table with alias "u"
#[macro_export]
macro_rules! alias {
    ($table:ty, $alias_name:literal) => {
        $crate::Alias::<$table>::new($alias_name)
    };
}
