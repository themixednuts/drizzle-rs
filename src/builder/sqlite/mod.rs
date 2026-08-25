#[cfg(feature = "rusqlite")]
pub mod rusqlite;

#[cfg(feature = "turso")]
pub mod turso;

#[cfg(feature = "libsql")]
pub mod libsql;

#[cfg(all(feature = "d1", target_arch = "wasm32"))]
pub mod d1;

#[cfg(all(feature = "durable", target_arch = "wasm32"))]
pub mod durable;

pub mod common;
pub mod prepared_common;
pub mod rows;

pub(super) fn finish_foreign_key_scope<T>(
    result: drizzle_core::error::Result<T>,
    restore: drizzle_core::error::Result<()>,
) -> drizzle_core::error::Result<T> {
    match (result, restore) {
        (Err(primary), Err(restore)) => Err(drizzle_core::error::DrizzleError::Other(
            format!(
                "{primary}; additionally failed to restore SQLite foreign-key enforcement: {restore}"
            )
            .into(),
        )),
        (Err(primary), Ok(())) => Err(primary),
        (Ok(_), Err(restore)) => Err(restore),
        (Ok(value), Ok(())) => Ok(value),
    }
}

pub(super) fn reject_unsafe_dirty_rebuild_repair(
    set: &drizzle_migrations::Migrations,
    dirty: &[String],
) -> drizzle_core::error::Result<()> {
    let migrations = set
        .resolve_dirty_migrations(dirty)
        .map_err(|error| drizzle_core::error::DrizzleError::Other(error.to_string().into()))?;
    for migration in migrations {
        let execution = migration
            .sqlite_execution()
            .map_err(|error| drizzle_core::error::DrizzleError::Other(error.to_string().into()))?;
        if execution.suspends_foreign_keys() {
            return Err(
                drizzle_core::error::DrizzleError::UnsafeMigrationRepair {
                    tag: migration.tag().into(),
                    reason: "foreign-key-suspending table rebuilds require the original connection-owned transaction scope"
                        .into(),
                },
            );
        }
    }
    Ok(())
}

#[cfg(any(
    test,
    all(feature = "d1", target_arch = "wasm32"),
    all(feature = "durable", target_arch = "wasm32")
))]
pub(super) fn reject_foreign_key_suspending_migrations<'a>(
    migrations: impl IntoIterator<Item = &'a drizzle_migrations::Migration>,
    adapter: &str,
) -> drizzle_core::error::Result<()> {
    let executions = migrations
        .into_iter()
        .map(drizzle_migrations::Migration::sqlite_execution)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| drizzle_core::error::DrizzleError::Other(error.to_string().into()))?;
    if executions
        .iter()
        .any(|execution| execution.suspends_foreign_keys())
    {
        return Err(
            drizzle_core::error::DrizzleError::UnsupportedMigrationExecution {
                adapter: adapter.into(),
                requirement: "SQLite table rebuilds that suspend foreign keys require a connection-owned executor"
                    .into(),
            },
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_executor_rejects_foreign_key_suspension_before_side_effects() {
        let migration = drizzle_migrations::Migration::new(
            "0001_rebuild",
            "PRAGMA foreign_keys=OFF;\n--> statement-breakpoint\nDROP TABLE parent;\n--> statement-breakpoint\nPRAGMA foreign_keys=ON;",
        );

        let migrations = [migration];
        let error = reject_foreign_key_suspending_migrations(migrations.iter(), "Test adapter")
            .expect_err("unsupported adapter must reject rebuild");
        assert!(matches!(
            error,
            drizzle_core::error::DrizzleError::UnsupportedMigrationExecution {
                ref adapter,
                ref requirement,
            } if adapter == "Test adapter" && requirement.contains("connection-owned executor")
        ));
    }

    #[test]
    fn unsupported_executor_ignores_an_applied_historical_rebuild() {
        let rebuild = drizzle_migrations::Migration::new(
            "0001_rebuild",
            "PRAGMA foreign_keys=OFF;\n--> statement-breakpoint\nDROP TABLE parent;\n--> statement-breakpoint\nPRAGMA foreign_keys=ON;",
        );
        let ordinary = drizzle_migrations::Migration::new(
            "0002_index",
            "CREATE INDEX records_name ON records(name);",
        );
        let set = drizzle_migrations::Migrations::new(
            vec![rebuild, ordinary],
            drizzle_types::Dialect::SQLite,
        );
        let applied = ["0001_rebuild".to_string()];

        reject_foreign_key_suspending_migrations(set.pending(&applied), "Test adapter")
            .expect("ordinary pending migration remains supported");
    }

    #[test]
    fn dirty_foreign_key_rebuild_is_not_auto_repairable() {
        let migration = drizzle_migrations::Migration::new(
            "0001_rebuild",
            "PRAGMA foreign_keys=OFF;\n--> statement-breakpoint\nDROP TABLE parent;\n--> statement-breakpoint\nPRAGMA foreign_keys=ON;",
        );
        let set =
            drizzle_migrations::Migrations::new(vec![migration], drizzle_types::Dialect::SQLite);

        let error = reject_unsafe_dirty_rebuild_repair(&set, &["0001_rebuild".to_string()])
            .expect_err("dirty rebuild must require manual recovery");
        assert!(matches!(
            error,
            drizzle_core::error::DrizzleError::UnsafeMigrationRepair { ref tag, .. }
                if tag == "0001_rebuild"
        ));
    }
}
