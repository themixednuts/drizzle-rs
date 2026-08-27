use std::collections::HashSet;

use drizzle_migrations::schema::Snapshot;

use super::SnapshotFilters;
use crate::config::{Dialect, Extension};
use crate::error::CliError;

/// Apply table, schema, and extension filters to a snapshot in place.
pub fn apply_snapshot_filters(
    snapshot: &mut Snapshot,
    dialect: Dialect,
    filters: &SnapshotFilters,
) -> Result<(), CliError> {
    if filters.is_empty() {
        return Ok(());
    }

    match (dialect, snapshot) {
        (Dialect::Sqlite | Dialect::Turso, Snapshot::Sqlite(sqlite)) => {
            apply_sqlite_snapshot_filters(sqlite, filters)
        }
        (Dialect::Postgresql, Snapshot::Postgres(postgres)) => {
            apply_postgres_snapshot_filters(postgres, filters)
        }
        (Dialect::Mysql, Snapshot::MySQL(mysql)) => apply_mysql_snapshot_filters(mysql, filters),
        _ => Err(CliError::DialectMismatch),
    }
}

fn apply_mysql_snapshot_filters(
    snapshot: &mut drizzle_migrations::mysql::MySQLSnapshot,
    filters: &SnapshotFilters,
) -> Result<(), CliError> {
    use drizzle_types::mysql::ddl::MySQLEntity;

    let table_patterns = compile_patterns(filters.tables.as_deref())?;
    let Some(table_patterns) = table_patterns.as_deref() else {
        return Ok(());
    };
    let keep_tables = snapshot
        .ddl
        .iter()
        .filter_map(|entity| match entity {
            MySQLEntity::Table(table) if matches_patterns(&table.name, Some(table_patterns)) => {
                Some(table.name.to_string())
            }
            _ => None,
        })
        .collect::<HashSet<_>>();

    snapshot.ddl.retain(|entity| match entity {
        MySQLEntity::Table(table) => keep_tables.contains(table.name.as_ref()),
        MySQLEntity::Column(column) => keep_tables.contains(column.table.as_ref()),
        MySQLEntity::Index(index) => keep_tables.contains(index.table.as_ref()),
        MySQLEntity::PrimaryKey(primary_key) => keep_tables.contains(primary_key.table.as_ref()),
        MySQLEntity::UniqueConstraint(unique) => keep_tables.contains(unique.table.as_ref()),
        MySQLEntity::ForeignKey(foreign_key) => {
            keep_tables.contains(foreign_key.table.as_ref())
                && keep_tables.contains(foreign_key.foreign_table.as_ref())
        }
        MySQLEntity::CheckConstraint(check) => keep_tables.contains(check.table.as_ref()),
        MySQLEntity::View(view) => matches_patterns(view.name.as_ref(), Some(table_patterns)),
    });

    Ok(())
}

fn apply_sqlite_snapshot_filters(
    snapshot: &mut drizzle_migrations::sqlite::SQLiteSnapshot,
    filters: &SnapshotFilters,
) -> Result<(), CliError> {
    use drizzle_types::sqlite::ddl::SqliteEntity;

    let table_patterns = compile_patterns(filters.tables.as_deref())?;
    if table_patterns.is_none() {
        return Ok(());
    }

    let keep_tables = snapshot
        .ddl
        .iter()
        .filter_map(|entity| match entity {
            SqliteEntity::Table(table)
                if matches_patterns(table.name.as_ref(), table_patterns.as_deref()) =>
            {
                Some(table.name.to_string())
            }
            _ => None,
        })
        .collect::<HashSet<_>>();

    snapshot.ddl.retain(|entity| match entity {
        SqliteEntity::Table(table) => keep_tables.contains(table.name.as_ref()),
        SqliteEntity::Column(column) => keep_tables.contains(column.table.as_ref()),
        SqliteEntity::Index(index) => keep_tables.contains(index.table.as_ref()),
        SqliteEntity::ForeignKey(foreign_key) => {
            keep_tables.contains(foreign_key.table.as_ref())
                && keep_tables.contains(foreign_key.table_to.as_ref())
        }
        SqliteEntity::PrimaryKey(primary_key) => keep_tables.contains(primary_key.table.as_ref()),
        SqliteEntity::UniqueConstraint(unique) => keep_tables.contains(unique.table.as_ref()),
        SqliteEntity::CheckConstraint(check) => keep_tables.contains(check.table.as_ref()),
        SqliteEntity::View(view) => matches_patterns(view.name.as_ref(), table_patterns.as_deref()),
    });

    Ok(())
}

fn apply_postgres_snapshot_filters(
    snapshot: &mut drizzle_migrations::postgres::PostgresSnapshot,
    filters: &SnapshotFilters,
) -> Result<(), CliError> {
    use drizzle_types::postgres::ddl::PostgresEntity;

    let schema_patterns = compile_patterns(filters.schemas.as_deref())?;
    let table_patterns = compile_patterns(filters.tables.as_deref())?;
    let exclude_postgis = filters
        .extensions
        .as_ref()
        .is_some_and(|extensions| extensions.contains(&Extension::Postgis));

    let is_schema_allowed = |schema: &str| {
        !(exclude_postgis && matches!(schema, "topology" | "tiger" | "tiger_data"))
            && matches_patterns(schema, schema_patterns.as_deref())
    };

    let keep_tables = snapshot
        .ddl
        .iter()
        .filter_map(|entity| match entity {
            PostgresEntity::Table(table) => {
                let schema = table.schema.as_ref();
                let name = table.name.as_ref();
                let is_postgis_table = exclude_postgis
                    && matches!(
                        name,
                        "spatial_ref_sys"
                            | "geometry_columns"
                            | "geography_columns"
                            | "raster_columns"
                            | "raster_overviews"
                    );
                (is_schema_allowed(schema)
                    && !is_postgis_table
                    && matches_patterns(name, table_patterns.as_deref()))
                .then(|| (schema.to_string(), name.to_string()))
            }
            _ => None,
        })
        .collect::<HashSet<_>>();

    let mut keep_schemas = keep_tables
        .iter()
        .map(|(schema, _)| schema.clone())
        .collect::<HashSet<_>>();
    if schema_patterns.is_some() || table_patterns.is_none() {
        keep_schemas.extend(snapshot.ddl.iter().filter_map(|entity| match entity {
            PostgresEntity::Schema(schema) if is_schema_allowed(schema.name.as_ref()) => {
                Some(schema.name.to_string())
            }
            _ => None,
        }));
    }

    snapshot.ddl.retain(|entity| match entity {
        PostgresEntity::Schema(schema) => keep_schemas.contains(schema.name.as_ref()),
        PostgresEntity::Enum(value) => keep_schemas.contains(value.schema.as_ref()),
        PostgresEntity::Sequence(sequence) => keep_schemas.contains(sequence.schema.as_ref()),
        PostgresEntity::Role(_) | PostgresEntity::Privilege(_) => true,
        PostgresEntity::Policy(policy) => {
            keep_tables.contains(&(policy.schema.to_string(), policy.table.to_string()))
        }
        PostgresEntity::Table(table) => {
            keep_tables.contains(&(table.schema.to_string(), table.name.to_string()))
        }
        PostgresEntity::Column(column) => {
            keep_tables.contains(&(column.schema.to_string(), column.table.to_string()))
        }
        PostgresEntity::Index(index) => {
            keep_tables.contains(&(index.schema.to_string(), index.table.to_string()))
        }
        PostgresEntity::ForeignKey(foreign_key) => {
            keep_tables.contains(&(
                foreign_key.schema.to_string(),
                foreign_key.table.to_string(),
            )) && keep_tables.contains(&(
                foreign_key.schema_to.to_string(),
                foreign_key.table_to.to_string(),
            ))
        }
        PostgresEntity::PrimaryKey(primary_key) => keep_tables.contains(&(
            primary_key.schema.to_string(),
            primary_key.table.to_string(),
        )),
        PostgresEntity::UniqueConstraint(unique) => {
            keep_tables.contains(&(unique.schema.to_string(), unique.table.to_string()))
        }
        PostgresEntity::CheckConstraint(check) => {
            keep_tables.contains(&(check.schema.to_string(), check.table.to_string()))
        }
        PostgresEntity::View(view) => {
            keep_schemas.contains(view.schema.as_ref())
                && matches_patterns(view.name.as_ref(), table_patterns.as_deref())
        }
    });

    Ok(())
}

#[derive(Debug, Clone)]
pub(super) struct FilterPattern {
    pattern: glob::Pattern,
    negated: bool,
}

pub(super) fn compile_patterns(
    patterns: Option<&[String]>,
) -> Result<Option<Vec<FilterPattern>>, CliError> {
    let Some(patterns) = patterns.filter(|patterns| !patterns.is_empty()) else {
        return Ok(None);
    };

    patterns
        .iter()
        .map(|pattern| {
            let raw = pattern.trim();
            let (negated, source) = raw
                .strip_prefix('!')
                .map_or((false, raw), |stripped| (true, stripped));
            if source.is_empty() {
                return Err(CliError::Other(format!(
                    "invalid filter pattern '{pattern}': empty pattern"
                )));
            }
            Ok(FilterPattern {
                pattern: glob::Pattern::new(source).map_err(|error| {
                    CliError::Other(format!("invalid filter pattern '{pattern}': {error}"))
                })?,
                negated,
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

pub(super) fn matches_patterns(value: &str, patterns: Option<&[FilterPattern]>) -> bool {
    let Some(patterns) = patterns else {
        return true;
    };
    if patterns
        .iter()
        .any(|pattern| pattern.negated && pattern.pattern.matches(value))
    {
        return false;
    }
    let has_positive = patterns.iter().any(|pattern| !pattern.negated);
    !has_positive
        || patterns
            .iter()
            .any(|pattern| !pattern.negated && pattern.pattern.matches(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_migrations::mysql::MySQLSnapshot;
    use drizzle_types::mysql::ddl::{Column, ForeignKey, MySQLEntity, Table, View};

    #[test]
    fn mysql_table_filter_projects_complete_entity_graph() {
        let mut snapshot = MySQLSnapshot::new();
        for table in ["users", "posts", "audit_log"] {
            snapshot.add_entity(MySQLEntity::Table(Table::new(table)));
            snapshot.add_entity(MySQLEntity::Column(Column::new(table, "id", "int")));
        }
        snapshot.add_entity(MySQLEntity::ForeignKey(ForeignKey::new(
            "posts",
            "posts_user_fk",
            ["id"],
            "users",
            ["id"],
        )));
        snapshot.add_entity(MySQLEntity::View(View::new(
            "post_view",
            "SELECT id FROM posts",
        )));

        let mut snapshot = Snapshot::MySQL(snapshot);
        apply_snapshot_filters(
            &mut snapshot,
            Dialect::Mysql,
            &SnapshotFilters {
                tables: Some(vec!["posts".to_string(), "post_view".to_string()]),
                schemas: None,
                extensions: None,
            },
        )
        .expect("filter");

        let Snapshot::MySQL(snapshot) = snapshot else {
            panic!("expected MySQL snapshot")
        };
        assert!(
            snapshot
                .ddl
                .iter()
                .any(|entity| matches!(entity, MySQLEntity::Table(table) if table.name == "posts"))
        );
        assert!(
            snapshot.ddl.iter().any(
                |entity| matches!(entity, MySQLEntity::View(view) if view.name == "post_view")
            )
        );
        assert!(
            !snapshot
                .ddl
                .iter()
                .any(|entity| matches!(entity, MySQLEntity::ForeignKey(_)))
        );
        assert!(!snapshot.ddl.iter().any(
            |entity| matches!(entity, MySQLEntity::Table(table) if table.name == "users" || table.name == "audit_log")
        ));
    }

    #[test]
    fn postgres_table_filter_preserves_existing_allowed_schema() {
        use drizzle_migrations::postgres::PostgresSnapshot;
        use drizzle_types::postgres::ddl::{PostgresEntity, Schema};

        let mut postgres = PostgresSnapshot::new();
        postgres.add_entity(PostgresEntity::Schema(Schema::new("public")));
        let mut snapshot = Snapshot::Postgres(postgres);

        apply_snapshot_filters(
            &mut snapshot,
            Dialect::Postgresql,
            &SnapshotFilters {
                tables: Some(vec!["new_table".to_string()]),
                schemas: Some(vec!["public".to_string()]),
                extensions: None,
            },
        )
        .expect("filter");

        let Snapshot::Postgres(postgres) = snapshot else {
            panic!("expected PostgreSQL snapshot")
        };
        assert!(postgres.ddl.iter().any(
            |entity| matches!(entity, PostgresEntity::Schema(schema) if schema.name == "public")
        ));
    }
}
