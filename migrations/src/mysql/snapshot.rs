//! MySQL v6 entity-array snapshot.

use super::{MySQLDDL, ddl::MySQLEntity};
use crate::snapshot::{Snapshot, SnapshotEntity};
use crate::version::MYSQL_SNAPSHOT_VERSION;
use serde_json::{Map, Value};
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

    #[test]
    fn new_snapshot_uses_mysql_v6_contract() {
        let snapshot = MySQLSnapshot::new();
        assert_eq!(snapshot.version, "6");
        assert_eq!(snapshot.dialect, "mysql");
        assert_eq!(snapshot.prev_ids, [crate::ORIGIN_UUID]);
        assert!(snapshot.ddl.is_empty());
    }
}
