//! Schema upgrade functions
//!
//! These functions transform snapshot schemas from older versions to newer versions.
//! The transformations match what drizzle-kit does to maintain compatibility.

use serde_json::{Map, Value};

use crate::config::Dialect;
use crate::version::{POSTGRES_SNAPSHOT_VERSION, SQLITE_SNAPSHOT_VERSION};

/// Upgrade a SQLite snapshot from v5 to v6
///
/// Changes:
/// - JSON object/array defaults are converted to escaped strings
/// - Adds `views: {}` field
pub fn upgrade_sqlite_v5_to_v6(mut json: Value) -> Value {
    let obj = match json.as_object_mut() {
        Some(o) => o,
        None => return json,
    };

    // Transform table column defaults
    if let Some(tables) = obj.get_mut("tables").and_then(|t| t.as_object_mut()) {
        for (_table_name, table) in tables.iter_mut() {
            if let Some(columns) = table.get_mut("columns").and_then(|c| c.as_object_mut()) {
                for (_col_name, column) in columns.iter_mut() {
                    if let Some(default) = column.get_mut("default") {
                        // If default is an object or array, stringify it
                        if default.is_object() || default.is_array() {
                            let stringified =
                                format!("'{}'", serde_json::to_string(default).unwrap_or_default());
                            *default = Value::String(stringified);
                        }
                    }
                }
            }
        }
    }

    // Ensure views field exists
    if !obj.contains_key("views") {
        obj.insert("views".to_string(), Value::Object(Map::new()));
    }

    // Update version
    obj.insert(
        "version".to_string(),
        Value::String(SQLITE_SNAPSHOT_VERSION.to_string()),
    );

    json
}

/// Upgrade a PostgreSQL snapshot from v5 to v6
///
/// Changes:
/// - Table keys become `schema.tablename` format
/// - Enum format changes to include schema and use array values
pub fn upgrade_postgres_v5_to_v6(mut json: Value) -> Value {
    let obj = match json.as_object_mut() {
        Some(o) => o,
        None => return json,
    };

    // Transform tables: key becomes "schema.name"
    if let Some(tables) = obj.remove("tables")
        && let Some(tables_obj) = tables.as_object()
    {
        let mut new_tables = Map::new();
        for (_key, table) in tables_obj {
            if let Some(table_obj) = table.as_object() {
                let schema = table_obj
                    .get("schema")
                    .and_then(|s| s.as_str())
                    .unwrap_or("public");
                let name = table_obj
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown");
                let new_key = format!("{}.{}", schema, name);
                new_tables.insert(new_key, table.clone());
            }
        }
        obj.insert("tables".to_string(), Value::Object(new_tables));
    }

    // Transform enums: add schema, convert values to array
    if let Some(enums) = obj.remove("enums")
        && let Some(enums_obj) = enums.as_object()
    {
        let mut new_enums = Map::new();
        for (_key, enum_val) in enums_obj {
            if let Some(enum_obj) = enum_val.as_object() {
                let name = enum_obj
                    .get("name")
                    .and_then(|n| n.as_str())
                    .unwrap_or("unknown");
                let new_key = format!("public.{}", name);

                // Convert values from object to array
                let values =
                    if let Some(values_obj) = enum_obj.get("values").and_then(|v| v.as_object()) {
                        Value::Array(values_obj.values().cloned().collect())
                    } else {
                        Value::Array(vec![])
                    };

                let mut new_enum = Map::new();
                new_enum.insert("name".to_string(), Value::String(name.to_string()));
                new_enum.insert("schema".to_string(), Value::String("public".to_string()));
                new_enum.insert("values".to_string(), values);

                new_enums.insert(new_key, Value::Object(new_enum));
            }
        }
        obj.insert("enums".to_string(), Value::Object(new_enums));
    }

    // Update dialect and version
    obj.insert(
        "dialect".to_string(),
        Value::String("postgresql".to_string()),
    );
    obj.insert("version".to_string(), Value::String("6".to_string()));

    json
}

/// Upgrade a PostgreSQL snapshot from v6 to v7
///
/// Changes:
/// - Index format changes (columns become objects with expression, isExpression, asc, nulls, opClass)
/// - Adds policies, sequences, roles, views fields to tables and schema
pub fn upgrade_postgres_v6_to_v7(mut json: Value) -> Value {
    let obj = match json.as_object_mut() {
        Some(o) => o,
        None => return json,
    };

    // Transform tables
    if let Some(tables) = obj.get_mut("tables").and_then(|t| t.as_object_mut()) {
        for (_table_key, table) in tables.iter_mut() {
            if let Some(table_obj) = table.as_object_mut() {
                // Transform indexes
                if let Some(indexes) = table_obj.get_mut("indexes").and_then(|i| i.as_object_mut())
                {
                    for (_idx_key, index) in indexes.iter_mut() {
                        if let Some(index_obj) = index.as_object_mut() {
                            // Transform columns from string array to object array
                            if let Some(columns) = index_obj.remove("columns")
                                && let Some(cols_arr) = columns.as_array()
                            {
                                let new_columns: Vec<Value> = cols_arr
                                    .iter()
                                    .map(|col| {
                                        let col_str = col.as_str().unwrap_or("");
                                        let mut col_obj = Map::new();
                                        col_obj.insert(
                                            "expression".to_string(),
                                            Value::String(col_str.to_string()),
                                        );
                                        col_obj
                                            .insert("isExpression".to_string(), Value::Bool(false));
                                        col_obj.insert("asc".to_string(), Value::Bool(true));
                                        col_obj.insert(
                                            "nulls".to_string(),
                                            Value::String("last".to_string()),
                                        );
                                        col_obj.insert("opClass".to_string(), Value::Null);
                                        Value::Object(col_obj)
                                    })
                                    .collect();
                                index_obj.insert("columns".to_string(), Value::Array(new_columns));
                            }
                            // Add `with` field if missing
                            if !index_obj.contains_key("with") {
                                index_obj.insert("with".to_string(), Value::Object(Map::new()));
                            }
                        }
                    }
                }

                // Add missing fields to tables
                if !table_obj.contains_key("policies") {
                    table_obj.insert("policies".to_string(), Value::Object(Map::new()));
                }
                if !table_obj.contains_key("isRLSEnabled") {
                    table_obj.insert("isRLSEnabled".to_string(), Value::Bool(false));
                }
                if !table_obj.contains_key("checkConstraints") {
                    table_obj.insert("checkConstraints".to_string(), Value::Object(Map::new()));
                }
            }
        }
    }

    // Add top-level fields
    if !obj.contains_key("sequences") {
        obj.insert("sequences".to_string(), Value::Object(Map::new()));
    }
    if !obj.contains_key("policies") {
        obj.insert("policies".to_string(), Value::Object(Map::new()));
    }
    if !obj.contains_key("views") {
        obj.insert("views".to_string(), Value::Object(Map::new()));
    }
    if !obj.contains_key("roles") {
        obj.insert("roles".to_string(), Value::Object(Map::new()));
    }

    // Update version
    obj.insert(
        "version".to_string(),
        Value::String(POSTGRES_SNAPSHOT_VERSION.to_string()),
    );

    json
}

/// Upgrade a snapshot to the latest version for the given dialect
pub fn upgrade_to_latest(json: Value, dialect: Dialect) -> Value {
    // Get version before consuming json
    let version = json
        .get("version")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown")
        .to_string();

    match dialect {
        Dialect::Sqlite => match version.as_str() {
            "5" => upgrade_sqlite_v5_to_v6(json),
            _ => json, // Already latest or unknown
        },
        Dialect::Postgresql => {
            let mut current = json;
            let mut current_version = version;

            // Chain upgrades: v5 → v6 → v7
            if current_version == "5" {
                current = upgrade_postgres_v5_to_v6(current);
                current_version = "6".to_string();
            }
            if current_version == "6" {
                current = upgrade_postgres_v6_to_v7(current);
            }

            current
        }
        Dialect::Mysql => json, // MySQL v5 is current, no upgrades needed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sqlite_v5_to_v6_json_defaults() {
        let v5 = json!({
            "version": "5",
            "dialect": "sqlite",
            "tables": {
                "users": {
                    "name": "users",
                    "columns": {
                        "metadata": {
                            "name": "metadata",
                            "type": "text",
                            "default": {"key": "value"}
                        }
                    }
                }
            }
        });

        let v6 = upgrade_sqlite_v5_to_v6(v5);

        assert_eq!(v6["version"], "6");
        assert!(v6["views"].is_object());

        let default = v6["tables"]["users"]["columns"]["metadata"]["default"]
            .as_str()
            .unwrap();
        assert!(default.starts_with('\''));
        assert!(default.contains("key"));
    }

    #[test]
    fn test_postgres_v5_to_v6_table_keys() {
        let v5 = json!({
            "version": "5",
            "dialect": "pg",
            "tables": {
                "users": {
                    "name": "users",
                    "schema": "public",
                    "columns": {}
                }
            },
            "enums": {
                "status": {
                    "name": "status",
                    "values": {"active": "active", "inactive": "inactive"}
                }
            }
        });

        let v6 = upgrade_postgres_v5_to_v6(v5);

        assert_eq!(v6["version"], "6");
        assert_eq!(v6["dialect"], "postgresql");
        assert!(v6["tables"]["public.users"].is_object());
        assert!(v6["enums"]["public.status"].is_object());
        assert!(v6["enums"]["public.status"]["values"].is_array());
    }

    #[test]
    fn test_postgres_v6_to_v7_index_format() {
        let v6 = json!({
            "version": "6",
            "dialect": "postgresql",
            "tables": {
                "public.users": {
                    "name": "users",
                    "schema": "public",
                    "columns": {},
                    "indexes": {
                        "idx_name": {
                            "name": "idx_name",
                            "columns": ["name", "email"]
                        }
                    }
                }
            },
            "enums": {}
        });

        let v7 = upgrade_postgres_v6_to_v7(v6);

        assert_eq!(v7["version"], "7");

        let columns = &v7["tables"]["public.users"]["indexes"]["idx_name"]["columns"];
        assert!(columns.is_array());
        assert_eq!(columns[0]["expression"], "name");
        assert_eq!(columns[0]["isExpression"], false);
        assert_eq!(columns[0]["asc"], true);
        assert_eq!(columns[0]["nulls"], "last");

        // Check new fields
        assert!(v7["tables"]["public.users"]["policies"].is_object());
        assert!(v7["sequences"].is_object());
        assert!(v7["roles"].is_object());
    }

    #[test]
    fn test_upgrade_to_latest_chains_correctly() {
        let v5 = json!({
            "version": "5",
            "dialect": "pg",
            "tables": {
                "users": {
                    "name": "users",
                    "schema": "public",
                    "columns": {},
                    "indexes": {
                        "idx": {
                            "name": "idx",
                            "columns": ["id"]
                        }
                    }
                }
            },
            "enums": {}
        });

        let latest = upgrade_to_latest(v5, Dialect::Postgresql);

        assert_eq!(latest["version"], "7");
        assert!(
            latest["tables"]["public.users"]["indexes"]["idx"]["columns"][0]["expression"]
                .is_string()
        );
    }
}
