use crate::traits::table::SQLTableInfo;

/// Runtime foreign key metadata used by engines that need FK grouping info
/// (e.g. composite key-aware seed generation).
pub trait SQLForeignKeyInfo: Send + Sync + 'static {
    fn source_table(&self) -> &'static dyn SQLTableInfo;
    fn target_table(&self) -> &'static dyn SQLTableInfo;
    fn source_columns(&self) -> &'static [&'static str];
    fn target_columns(&self) -> &'static [&'static str];
}

/// Typed (non-dyn) foreign key metadata.
///
/// Mirrors [`SQLForeignKeyInfo`] in a compile-time friendly form via
/// associated types for source/target table and column sets.
pub trait SQLForeignKey: SQLForeignKeyInfo {
    type SourceTable;
    type TargetTable;
    type SourceColumns;
    type TargetColumns;
}

/// Marker used for columns that do not define a foreign key.
pub struct NoForeignKey;

impl SQLForeignKeyInfo for NoForeignKey {
    fn source_table(&self) -> &'static dyn SQLTableInfo {
        panic!("NoForeignKey has no source table")
    }

    fn target_table(&self) -> &'static dyn SQLTableInfo {
        panic!("NoForeignKey has no target table")
    }

    fn source_columns(&self) -> &'static [&'static str] {
        &[]
    }

    fn target_columns(&self) -> &'static [&'static str] {
        &[]
    }
}

impl SQLForeignKey for NoForeignKey {
    type SourceTable = ();
    type TargetTable = ();
    type SourceColumns = ();
    type TargetColumns = ();
}
