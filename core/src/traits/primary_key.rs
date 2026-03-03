/// Typed (non-dyn) primary key metadata.
pub trait SQLPrimaryKey {
    type Table;
    type Columns;
}

/// Marker used for tables that do not define a primary key.
pub struct NoPrimaryKey;

impl SQLPrimaryKey for NoPrimaryKey {
    type Table = ();
    type Columns = ();
}
