use drizzle_core::{SQLIndexInfo, SQLSchemaType, TableRef};

/// The kind of object contributed by a generated MySQL schema item.
#[derive(Debug, Clone)]
pub enum MySQLSchemaType {
    /// A table definition.
    Table(&'static TableRef),
    /// An index definition.
    Index(&'static dyn SQLIndexInfo),
}

impl SQLSchemaType for MySQLSchemaType {}
