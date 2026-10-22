#[cfg(any(
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "hyperdrive",
    feature = "aws-data-api"
))]
pub mod typestate;

#[cfg(feature = "postgres-sync")]
pub mod postgres_sync;

#[cfg(any(feature = "tokio-postgres", feature = "hyperdrive"))]
pub mod tokio_postgres;

#[cfg(feature = "aws-data-api")]
pub mod aws_data_api;

/// Exports the running transaction's snapshot.
#[cfg(any(
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "hyperdrive",
    feature = "aws-data-api"
))]
pub(crate) const EXPORT_SNAPSHOT_SQL: &str = "SELECT pg_export_snapshot()";

/// Reads the id `pg_export_snapshot()` returned.
#[cfg(any(
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "hyperdrive",
    feature = "aws-data-api"
))]
pub(crate) fn exported_snapshot(
    id: &str,
) -> drizzle_core::error::Result<drizzle_postgres::SnapshotId> {
    drizzle_postgres::SnapshotId::new(id).ok_or_else(|| {
        drizzle_core::error::DrizzleError::ConversionError(
            format!("pg_export_snapshot() returned an unexpected id {id:?}").into(),
        )
    })
}
