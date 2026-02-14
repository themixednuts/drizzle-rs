use crate::traits::table::SQLTableInfo;

/// Runtime primary key metadata.
pub trait SQLPrimaryKeyInfo: Send + Sync + 'static {
    fn table(&self) -> &'static dyn SQLTableInfo;
    fn columns(&self) -> &'static [&'static str];
}

/// Typed (non-dyn) primary key metadata.
pub trait SQLPrimaryKey: SQLPrimaryKeyInfo {
    type Table;
    type Columns;
}

/// Marker used for tables that do not define a primary key.
pub struct NoPrimaryKey;

impl SQLPrimaryKeyInfo for NoPrimaryKey {
    fn table(&self) -> &'static dyn SQLTableInfo {
        panic!("NoPrimaryKey has no table")
    }

    fn columns(&self) -> &'static [&'static str] {
        &[]
    }
}

impl SQLPrimaryKey for NoPrimaryKey {
    type Table = ();
    type Columns = ();
}
