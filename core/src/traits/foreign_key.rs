/// Typed (non-dyn) foreign key metadata.
///
/// Compile-time foreign key information via associated types for source/target
/// table and column sets.
pub trait SQLForeignKey {
    type SourceTable;
    type TargetTable;
    type SourceColumns;
    type TargetColumns;
}

/// Marker used for columns that do not define a foreign key.
pub struct NoForeignKey;

impl SQLForeignKey for NoForeignKey {
    type SourceTable = ();
    type TargetTable = ();
    type SourceColumns = ();
    type TargetColumns = ();
}
