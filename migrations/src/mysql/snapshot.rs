//! MySQL v6 entity-array snapshot.

use super::{MySQLDDL, ValidationError, ddl::MySQLEntity};
use crate::snapshot::{Snapshot, SnapshotEntity};
use crate::version::MYSQL_SNAPSHOT_VERSION;
use serde_json::{Map, Value};
use std::collections::BTreeSet;
use std::io::{self, ErrorKind};
use std::path::Path;

impl SnapshotEntity for MySQLEntity {
    const DIALECT: &'static str = "mysql";
    const SNAPSHOT_VERSION: &'static str = MYSQL_SNAPSHOT_VERSION;
}

/// Current MySQL schema snapshot.
///
/// It uses the drizzle-kit v6 entity-array envelope plus typed drizzle-rs
/// metadata needed to preserve MySQL table, column, index, and view options.
pub type MySQLSnapshot = Snapshot<MySQLEntity>;

/// A live MySQL snapshot cannot be prepared safely for `push`.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum PushError {
    #[error(transparent)]
    InvalidSnapshot(#[from] ValidationError),
    #[error("MySQL schema targets database `{schema}`, but the connection selected `{selected}`")]
    Database { selected: String, schema: String },
    #[error(
        "MySQL push cannot manage temporary table `{table}` because temporary tables are connection-local"
    )]
    TemporaryTable { table: String },
}

impl Snapshot<MySQLEntity> {
    /// Scopes an introspected snapshot to the objects managed by `desired`.
    ///
    /// MySQL introspection covers the complete selected database. Runtime
    /// `push` must leave unrelated objects alone, align an explicitly
    /// qualified schema with that selected database, and reject temporary
    /// tables whose lifetime cannot survive pooled connection checkout.
    ///
    /// # Errors
    ///
    /// Returns [`PushError`] when either snapshot is invalid, the desired
    /// schema targets another database, or it contains a temporary table.
    pub fn prepare_for_push(
        &self,
        desired: &Self,
        selected_database: &str,
    ) -> Result<Self, PushError> {
        let desired = MySQLDDL::try_from_entities(desired.ddl.clone())?;
        if let Some(table) = desired.tables.list().iter().find(|table| table.temporary) {
            return Err(PushError::TemporaryTable {
                table: table.name.to_string(),
            });
        }

        let database = desired.database_scope()?;
        if let Some(schema) = database.as_deref()
            && schema != selected_database
        {
            return Err(PushError::Database {
                selected: selected_database.to_string(),
                schema: schema.to_string(),
            });
        }

        let tables = desired
            .tables
            .list()
            .iter()
            .map(|table| table.name.to_string())
            .collect::<BTreeSet<_>>();
        let views = desired
            .views
            .list()
            .iter()
            .map(|view| view.name.to_string())
            .collect::<BTreeSet<_>>();

        let mut live = MySQLDDL::try_from_entities(self.ddl.clone())?;
        live.tables
            .list_mut()
            .retain(|table| tables.contains(table.name.as_ref()));
        live.columns
            .list_mut()
            .retain(|column| tables.contains(column.table.as_ref()));
        live.indexes
            .list_mut()
            .retain(|index| tables.contains(index.table.as_ref()));
        live.pks
            .list_mut()
            .retain(|primary_key| tables.contains(primary_key.table.as_ref()));
        live.uniques
            .list_mut()
            .retain(|unique| tables.contains(unique.table.as_ref()));
        live.fks
            .list_mut()
            .retain(|foreign_key| tables.contains(foreign_key.table.as_ref()));
        live.checks
            .list_mut()
            .retain(|check| tables.contains(check.table.as_ref()));
        live.views
            .list_mut()
            .retain(|view| views.contains(view.name.as_ref()));
        live.set_database(database.map(Into::into));

        let mut snapshot = self.clone();
        snapshot.ddl = live.to_entities();
        Ok(snapshot)
    }
}

/// Load, upgrade, and validate a MySQL snapshot from disk.
///
/// The generic snapshot reader cannot inspect dialect-specific legacy shapes.
/// Keep that compatibility boundary here so callers never deserialize a v5
/// object as though it were a v6 entity document.
pub(crate) fn load(path: &Path) -> io::Result<MySQLSnapshot> {
    let contents = std::fs::read_to_string(path)?;
    let value: Value = serde_json::from_str(&contents).map_err(invalid_data)?;
    let object = value
        .as_object()
        .ok_or_else(|| invalid_snapshot("snapshot root must be a JSON object"))?;
    let dialect = required_string(object, "dialect")?;
    if dialect != MySQLEntity::DIALECT {
        return Err(invalid_snapshot(format!(
            "snapshot dialect `{dialect}` does not match requested dialect `mysql`"
        )));
    }

    let version = required_string(object, "version")?;
    let value = match version {
        MYSQL_SNAPSHOT_VERSION => value,
        "5" => {
            validate_v5_shape(object)?;
            crate::upgrade::upgrade_mysql_v5_to_v6(value)
        }
        other => {
            return Err(invalid_snapshot(format!(
                "unsupported MySQL snapshot version `{other}`; expected `5` or `{MYSQL_SNAPSHOT_VERSION}`"
            )));
        }
    };

    let snapshot: MySQLSnapshot = serde_json::from_value(value).map_err(invalid_data)?;
    if snapshot.dialect != MySQLEntity::DIALECT || snapshot.version != MySQLEntity::SNAPSHOT_VERSION
    {
        return Err(invalid_snapshot(
            "MySQL snapshot upgrade produced invalid dialect/version metadata",
        ));
    }
    if snapshot.id.is_empty()
        || snapshot.prev_ids.is_empty()
        || snapshot.prev_ids.iter().any(String::is_empty)
    {
        return Err(invalid_snapshot(
            "MySQL snapshot id and prevIds must contain non-empty values",
        ));
    }
    let ddl = MySQLDDL::try_from_entities(snapshot.ddl.clone()).map_err(invalid_data)?;
    super::diff::compute_migration(&MySQLDDL::new(), &ddl).map_err(invalid_data)?;
    Ok(snapshot)
}

fn required_string<'a>(object: &'a Map<String, Value>, key: &str) -> io::Result<&'a str> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| invalid_snapshot(format!("snapshot `{key}` must be a non-empty string")))
}

fn validate_v5_shape(object: &Map<String, Value>) -> io::Result<()> {
    required_string(object, "id")?;
    match (object.get("prevIds"), object.get("prevId")) {
        (Some(Value::Array(ids)), _) if !ids.is_empty() => {
            if ids.iter().any(|id| id.as_str().is_none_or(str::is_empty)) {
                return Err(invalid_snapshot(
                    "legacy MySQL snapshot prevIds must contain non-empty strings",
                ));
            }
        }
        (None, Some(Value::String(id))) if !id.is_empty() => {}
        _ => {
            return Err(invalid_snapshot(
                "legacy MySQL snapshot requires a non-empty prevId or prevIds",
            ));
        }
    }

    let tables = required_object(object, "tables", "legacy MySQL snapshot")?;
    let views = required_object(object, "views", "legacy MySQL snapshot")?;
    for (table_key, table) in tables {
        let table = table.as_object().ok_or_else(|| {
            invalid_snapshot(format!(
                "legacy MySQL table `{table_key}` must be an object"
            ))
        })?;
        let columns = required_object(
            table,
            "columns",
            &format!("legacy MySQL table `{table_key}`"),
        )?;
        for (column_key, column) in columns {
            let column = column.as_object().ok_or_else(|| {
                invalid_snapshot(format!(
                    "legacy MySQL column `{table_key}.{column_key}` must be an object"
                ))
            })?;
            required_string(column, "type").map_err(|_| {
                invalid_snapshot(format!(
                    "legacy MySQL column `{table_key}.{column_key}` requires a non-empty type"
                ))
            })?;
            validate_optional_bool(column, "primaryKey", table_key, column_key)?;
            validate_optional_bool(column, "notNull", table_key, column_key)?;
            validate_optional_bool(column, "autoincrement", table_key, column_key)?;
            validate_optional_bool(column, "autoIncrement", table_key, column_key)?;
            validate_optional_bool(column, "unique", table_key, column_key)?;
        }
        for (key, kind) in [
            ("indexes", "index"),
            ("foreignKeys", "foreign key"),
            ("compositePrimaryKeys", "primary key"),
            ("primaryKeys", "primary key"),
            ("uniqueConstraints", "unique constraint"),
            ("checkConstraints", "check constraint"),
            ("checkConstraint", "check constraint"),
        ] {
            let Some(value) = table.get(key) else {
                continue;
            };
            let members = value.as_object().ok_or_else(|| {
                invalid_snapshot(format!(
                    "legacy MySQL table `{table_key}` field `{key}` must be an object"
                ))
            })?;
            for (member_key, member) in members {
                if !member.is_object() {
                    return Err(invalid_snapshot(format!(
                        "legacy MySQL {kind} `{table_key}.{member_key}` must be an object"
                    )));
                }
            }
        }
    }
    for (view_key, view) in views {
        if !view.is_object() {
            return Err(invalid_snapshot(format!(
                "legacy MySQL view `{view_key}` must be an object"
            )));
        }
    }
    Ok(())
}

fn required_object<'a>(
    object: &'a Map<String, Value>,
    key: &str,
    owner: &str,
) -> io::Result<&'a Map<String, Value>> {
    object
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| invalid_snapshot(format!("{owner} field `{key}` must be an object")))
}

fn validate_optional_bool(
    object: &Map<String, Value>,
    key: &str,
    table: &str,
    column: &str,
) -> io::Result<()> {
    if object.get(key).is_some_and(|value| !value.is_boolean()) {
        return Err(invalid_snapshot(format!(
            "legacy MySQL column `{table}.{column}` field `{key}` must be a boolean"
        )));
    }
    Ok(())
}

fn invalid_data(error: impl std::error::Error + Send + Sync + 'static) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, error)
}

fn invalid_snapshot(message: impl Into<String>) -> io::Error {
    io::Error::new(ErrorKind::InvalidData, message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mysql::ddl::{Column, Table, View};

    fn schema(database: Option<&str>, temporary: bool, unmanaged: bool) -> MySQLSnapshot {
        let mut ddl = MySQLDDL::new();
        let mut table = Table::new("managed");
        table.database = database.map(|database| database.to_string().into());
        table.temporary = temporary;
        ddl.tables.push(table);
        let mut column = Column::new("managed", "id", "int");
        column.database = database.map(|database| database.to_string().into());
        ddl.columns.push(column);
        let mut view = View::new("managed_view", "SELECT id FROM managed");
        view.database = database.map(|database| database.to_string().into());
        ddl.views.push(view);

        if unmanaged {
            ddl.tables.push(Table::new("unmanaged"));
            ddl.columns.push(Column::new("unmanaged", "id", "int"));
            ddl.views
                .push(View::new("unmanaged_view", "SELECT id FROM unmanaged"));
        }

        let mut snapshot = MySQLSnapshot::new();
        snapshot.ddl = ddl.to_entities();
        snapshot
    }

    #[test]
    fn new_snapshot_uses_mysql_v6_contract() {
        let snapshot = MySQLSnapshot::new();
        assert_eq!(snapshot.version, "6");
        assert_eq!(snapshot.dialect, "mysql");
        assert_eq!(snapshot.prev_ids, [crate::ORIGIN_UUID]);
        assert!(snapshot.ddl.is_empty());
    }

    #[test]
    fn prepare_for_push_scopes_live_objects_and_aligns_database() {
        let live = schema(None, false, true);
        let desired = schema(Some("app"), false, false);

        let prepared = live
            .prepare_for_push(&desired, "app")
            .expect("matching selected database");
        let ddl = MySQLDDL::try_from_entities(prepared.ddl).expect("valid prepared snapshot");

        assert!(ddl.tables.one(Some("app"), "managed").is_some());
        assert!(ddl.tables.one(Some("app"), "unmanaged").is_none());
        assert!(ddl.views.one(Some("app"), "managed_view").is_some());
        assert!(ddl.views.one(Some("app"), "unmanaged_view").is_none());
    }

    #[test]
    fn prepare_for_push_rejects_another_database() {
        let error = schema(None, false, false)
            .prepare_for_push(&schema(Some("other"), false, false), "app")
            .expect_err("another database must not be managed through this connection");

        assert_eq!(
            error,
            PushError::Database {
                selected: "app".to_string(),
                schema: "other".to_string(),
            }
        );
    }

    #[test]
    fn prepare_for_push_rejects_temporary_tables() {
        let error = schema(None, false, false)
            .prepare_for_push(&schema(None, true, false), "app")
            .expect_err("temporary tables cannot be managed by push");

        assert_eq!(
            error,
            PushError::TemporaryTable {
                table: "managed".to_string(),
            }
        );
    }
}
