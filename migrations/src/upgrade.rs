//! Schema upgrade functions
//!
//! These functions transform snapshot schemas from older versions to newer versions.
//! The transformations match what drizzle-kit does to maintain compatibility.
//!
//! Two kinds of upgrades exist:
//!
//! * **In-shape upgrades** (`v5 → v6` for `SQLite`, `v5 → v6 → v7` for
//!   `PostgreSQL`) — tweaks within the legacy object format (tables/enums as
//!   nested dictionaries).
//! * **Structural upgrades** (`v6 → v7` for `SQLite`, `v7 → v8` for
//!   `PostgreSQL`) — rebuild the document in the current entity-array format
//!   (`{version, dialect, id, prevIds, ddl: [...], renames}`) used by
//!   [`crate::snapshot::Snapshot`].

use std::borrow::Cow;

use serde_json::{Map, Value};

use crate::mysql::{MySQLDDL, MySQLSnapshot};
use crate::postgres::PostgresSnapshot;
use crate::sqlite::SQLiteSnapshot;
use crate::version::{
    MYSQL_SNAPSHOT_VERSION, ORIGIN_UUID, POSTGRES_SNAPSHOT_VERSION, SQLITE_SNAPSHOT_VERSION,
};
use drizzle_types::Dialect;
use drizzle_types::mysql::ddl as mysql;
use drizzle_types::postgres::ddl as pg;
use drizzle_types::sqlite::ddl as lite;

/// Upgrade a `SQLite` snapshot from v5 to v6
///
/// Changes:
/// - JSON object/array defaults are converted to escaped strings
/// - Adds `views: {}` field
#[must_use]
pub fn upgrade_sqlite_v5_to_v6(mut json: Value) -> Value {
    let Some(obj) = json.as_object_mut() else {
        return json;
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

    // Update version. The result is still the legacy object shape, so it is
    // stamped "6" — the structural v6 → v7 upgrade below produces the
    // current entity-array format.
    obj.insert("version".to_string(), Value::String("6".to_string()));

    json
}

/// Upgrade a `PostgreSQL` snapshot from v5 to v6
///
/// Changes:
/// - Table keys become `schema.tablename` format
/// - Enum format changes to include schema and use array values
#[must_use]
pub fn upgrade_postgres_v5_to_v6(mut json: Value) -> Value {
    let Some(obj) = json.as_object_mut() else {
        return json;
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
                let new_key = format!("{schema}.{name}");
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
                let new_key = format!("public.{name}");

                // Convert values from object to array
                let values = enum_obj
                    .get("values")
                    .and_then(|v| v.as_object())
                    .map_or_else(
                        || Value::Array(vec![]),
                        |values_obj| Value::Array(values_obj.values().cloned().collect()),
                    );

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

/// Upgrade a `PostgreSQL` snapshot from v6 to v7
///
/// Changes:
/// - Index format changes (columns become objects with expression, isExpression, asc, nulls, opClass)
/// - Adds policies, sequences, roles, views fields to tables and schema
#[must_use]
pub fn upgrade_postgres_v6_to_v7(mut json: Value) -> Value {
    let Some(obj) = json.as_object_mut() else {
        return json;
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

    // Update version. The result is still the legacy object shape, so it is
    // stamped "7" — the structural v7 → v8 upgrade below produces the
    // current entity-array format.
    obj.insert("version".to_string(), Value::String("7".to_string()));

    json
}

// =============================================================================
// Legacy JSON helpers (shared by the structural converters)
// =============================================================================

/// Collect a legacy JSON dictionary's object entries in sorted key order.
///
/// serde_json's map iteration order depends on the `preserve_order` feature,
/// so deterministic converter output requires an explicit sort.
fn sorted_objects(value: Option<&Value>) -> Vec<(&str, &Map<String, Value>)> {
    let mut entries: Vec<(&str, &Map<String, Value>)> = value
        .and_then(Value::as_object)
        .map(|map| {
            map.iter()
                .filter_map(|(key, value)| value.as_object().map(|obj| (key.as_str(), obj)))
                .collect()
        })
        .unwrap_or_default();
    entries.sort_by_key(|(key, _)| *key);
    entries
}

fn str_of<'a>(obj: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    obj.get(key).and_then(Value::as_str)
}

fn bool_of(obj: &Map<String, Value>, key: &str) -> bool {
    obj.get(key).and_then(Value::as_bool).unwrap_or(false)
}

fn string_list(obj: &Map<String, Value>, key: &str) -> Vec<String> {
    obj.get(key)
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

/// Read a scalar that legacy snapshots store either as a string or a bare
/// number (e.g. sequence options) and render it as a string.
fn scalar_string(obj: &Map<String, Value>, key: &str) -> Option<String> {
    match obj.get(key)? {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

/// Read an integer that legacy snapshots store either as a number or a
/// stringified number (e.g. identity `cache`).
fn scalar_i32(obj: &Map<String, Value>, key: &str) -> Option<i32> {
    match obj.get(key)? {
        Value::Number(n) => n.as_i64().and_then(|n| i32::try_from(n).ok()),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Convert a legacy `default` value into the SQL-ready string the entity
/// format stores: strings arrive pre-quoted from drizzle-kit and pass through
/// unchanged; numbers and booleans are rendered as bare SQL literals.
fn default_literal(value: &Value) -> Option<Cow<'static, str>> {
    match value {
        Value::String(s) => Some(Cow::Owned(s.clone())),
        Value::Number(n) => Some(Cow::Owned(n.to_string())),
        Value::Bool(b) => Some(Cow::Owned(b.to_string())),
        // v5 → v6 already stringifies object/array defaults; anything else
        // has no SQL representation.
        _ => None,
    }
}

/// Map a legacy FK action to the entity format: `"no action"` is the SQL
/// default and is omitted (`None`), matching macro-generated snapshots and
/// the differ's normalization; anything else is uppercased.
fn fk_action(obj: &Map<String, Value>, key: &str) -> Option<Cow<'static, str>> {
    str_of(obj, key)
        .filter(|action| !action.eq_ignore_ascii_case("no action"))
        .map(|action| Cow::Owned(action.to_ascii_uppercase()))
}

/// Carry the legacy `id`/`prevId` chain over to the entity format. Both fall
/// back to [`ORIGIN_UUID`] so the converter stays deterministic even for
/// malformed inputs.
fn carry_identity(obj: &Map<String, Value>) -> (String, Vec<String>) {
    let id = str_of(obj, "id")
        .filter(|s| !s.is_empty())
        .unwrap_or(ORIGIN_UUID)
        .to_string();
    let prev_id = str_of(obj, "prevId")
        .filter(|s| !s.is_empty())
        .unwrap_or(ORIGIN_UUID)
        .to_string();
    (id, vec![prev_id])
}

/// Preserve a MySQL v5 `prevIds` chain when that newer legacy spelling is
/// present, while retaining the original `prevId` fallback used by v5 files.
fn carry_mysql_identity(obj: &Map<String, Value>) -> (String, Vec<String>) {
    let id = str_of(obj, "id")
        .filter(|id| !id.is_empty())
        .unwrap_or(ORIGIN_UUID)
        .to_string();
    let prev_ids = obj
        .get("prevIds")
        .and_then(Value::as_array)
        .map(|ids| {
            ids.iter()
                .filter_map(Value::as_str)
                .filter(|id| !id.is_empty())
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_else(|| {
            vec![
                str_of(obj, "prevId")
                    .filter(|id| !id.is_empty())
                    .unwrap_or(ORIGIN_UUID)
                    .to_string(),
            ]
        });
    (id, prev_ids)
}

// =============================================================================
// MySQL v5 -> v6 (structural)
// =============================================================================

/// Upgrade a legacy MySQL v5 object snapshot into the v6 entity-array
/// snapshot format.
///
/// The old document keeps child objects inside each table. The v6 format
/// serializes the same facts as independently keyed entities, so this
/// converter deliberately sorts every legacy dictionary before adding it to
/// [`MySQLDDL`]. That gives repeatable JSON regardless of serde_json map
/// configuration, while `MySQLDDL::to_entities()` supplies the stable entity
/// category order used by new snapshots.
#[must_use]
pub fn upgrade_mysql_v5_to_v6(json: Value) -> Value {
    let Some(obj) = json.as_object() else {
        return json;
    };

    let mut snapshot = MySQLSnapshot::new();
    snapshot.version = MYSQL_SNAPSHOT_VERSION.to_string();
    let (id, prev_ids) = carry_mysql_identity(obj);
    snapshot.id = id;
    snapshot.prev_ids = prev_ids;

    // A legacy config/snapshot may carry the selected database once at the
    // top level. Every table child inherits that scope unless its table
    // explicitly selects another database.
    let selected_database = str_of(obj, "database")
        .or_else(|| str_of(obj, "schema"))
        .filter(|database| !database.is_empty())
        .map(str::to_string);
    let mut ddl = MySQLDDL::new();

    for (table_key, table) in sorted_objects(obj.get("tables")) {
        let (key_database, key_name) = split_mysql_table_key(table_key);
        let table_database = mysql_database(table, selected_database.as_deref().or(key_database));
        let table_name = str_of(table, "name").unwrap_or(key_name).to_string();

        let mut entity = mysql::Table::new(table_name.clone());
        entity.database = table_database.clone();
        entity.temporary = bool_of(table, "temporary");
        entity.engine = mysql_optional_string(table, "engine");
        entity.charset = mysql_optional_string_any(table, &["charset", "characterSet"]);
        entity.collation = mysql_optional_string_any(table, &["collation", "collate"]);
        entity.comment = mysql_optional_string(table, "comment");
        entity.options = mysql_table_options(table);
        ddl.push_entity(mysql::MySQLEntity::Table(entity));

        let mut inline_primary_key_columns = Vec::new();
        for (column_key, column) in sorted_objects(table.get("columns")) {
            let column_name = str_of(column, "name").unwrap_or(column_key).to_string();
            let sql_type = str_of(column, "type").unwrap_or_default().to_string();
            let mut entity = mysql::Column::new(table_name.clone(), column_name, sql_type.clone());
            entity.database = table_database.clone();
            entity.not_null = bool_of(column, "notNull");
            entity.autoincrement =
                bool_of(column, "autoincrement") || bool_of(column, "autoIncrement");
            entity.primary_key = bool_of(column, "primaryKey");
            entity.unique = bool_of(column, "unique");
            entity.default = column.get("default").and_then(default_literal);
            entity.on_update = mysql_optional_string_any(column, &["onUpdate", "on_update"]);
            entity.generated = column
                .get("generated")
                .and_then(Value::as_object)
                .and_then(mysql_generated);
            entity.inline_type = mysql_inline_type(&sql_type);
            entity.charset = mysql_optional_string_any(column, &["charset", "characterSet"]);
            entity.collation = mysql_optional_string_any(column, &["collation", "collate"]);
            entity.comment = mysql_optional_string(column, "comment");
            if entity.primary_key {
                inline_primary_key_columns.push(entity.name.to_string());
            }
            ddl.push_entity(mysql::MySQLEntity::Column(entity));
        }

        let mut primary_keys = sorted_objects(table.get("compositePrimaryKeys"));
        if primary_keys.is_empty() {
            primary_keys = sorted_objects(table.get("primaryKeys"));
        }
        let has_table_primary_key = !primary_keys.is_empty();
        for (primary_key_key, primary_key) in primary_keys {
            let name = str_of(primary_key, "name")
                .filter(|name| !name.is_empty())
                .or_else(|| (!primary_key_key.is_empty()).then_some(primary_key_key))
                .map(|name| Cow::Owned(name.to_string()));
            ddl.push_entity(mysql::MySQLEntity::PrimaryKey(mysql::PrimaryKey {
                database: table_database.clone(),
                table: Cow::Owned(table_name.clone()),
                name,
                columns: to_cow_list(string_list(primary_key, "columns")),
            }));
        }
        if !has_table_primary_key && !inline_primary_key_columns.is_empty() {
            ddl.push_entity(mysql::MySQLEntity::PrimaryKey(mysql::PrimaryKey {
                database: table_database.clone(),
                table: Cow::Owned(table_name.clone()),
                name: None,
                columns: to_cow_list(inline_primary_key_columns),
            }));
        }

        for (unique_key, unique) in sorted_objects(table.get("uniqueConstraints")) {
            ddl.push_entity(mysql::MySQLEntity::UniqueConstraint(
                mysql::UniqueConstraint {
                    database: table_database.clone(),
                    table: Cow::Owned(table_name.clone()),
                    name: Cow::Owned(str_of(unique, "name").unwrap_or(unique_key).to_string()),
                    columns: to_cow_list(string_list(unique, "columns")),
                },
            ));
        }

        for (index_key, index) in sorted_objects(table.get("indexes")) {
            ddl.push_entity(mysql::MySQLEntity::Index(mysql::Index {
                database: table_database.clone(),
                table: Cow::Owned(
                    str_of(index, "table")
                        .or_else(|| str_of(index, "tableFrom"))
                        .unwrap_or(&table_name)
                        .to_string(),
                ),
                name: Cow::Owned(str_of(index, "name").unwrap_or(index_key).to_string()),
                columns: mysql_index_columns(index.get("columns")),
                unique: bool_of(index, "isUnique") || bool_of(index, "unique"),
                using: mysql_index_method(
                    str_of(index, "using").or_else(|| str_of(index, "method")),
                ),
                algorithm: mysql_index_algorithm(str_of(index, "algorithm")),
                lock: mysql_index_lock(str_of(index, "lock")),
                comment: mysql_optional_string(index, "comment"),
                visible: index
                    .get("visible")
                    .or_else(|| index.get("isVisible"))
                    .and_then(Value::as_bool),
            }));
        }

        for (foreign_key_key, foreign_key) in sorted_objects(table.get("foreignKeys")) {
            ddl.push_entity(mysql::MySQLEntity::ForeignKey(mysql::ForeignKey {
                database: table_database.clone(),
                table: Cow::Owned(
                    str_of(foreign_key, "tableFrom")
                        .unwrap_or(&table_name)
                        .to_string(),
                ),
                name: Cow::Owned(
                    str_of(foreign_key, "name")
                        .unwrap_or(foreign_key_key)
                        .to_string(),
                ),
                columns: to_cow_list(string_list(foreign_key, "columnsFrom")),
                foreign_database: mysql_optional_string_any(
                    foreign_key,
                    &["foreignDatabase", "databaseTo", "schemaTo"],
                ),
                foreign_table: Cow::Owned(
                    str_of(foreign_key, "tableTo")
                        .or_else(|| str_of(foreign_key, "foreignTable"))
                        .unwrap_or_default()
                        .to_string(),
                ),
                foreign_columns: to_cow_list(if foreign_key.get("columnsTo").is_some() {
                    string_list(foreign_key, "columnsTo")
                } else {
                    string_list(foreign_key, "foreignColumns")
                }),
                on_delete: mysql_referential_action(str_of(foreign_key, "onDelete")),
                on_update: mysql_referential_action(str_of(foreign_key, "onUpdate")),
            }));
        }

        let checks = table
            .get("checkConstraints")
            .or_else(|| table.get("checkConstraint"));
        for (check_key, check) in sorted_objects(checks) {
            ddl.push_entity(mysql::MySQLEntity::CheckConstraint(
                mysql::CheckConstraint {
                    database: table_database.clone(),
                    table: Cow::Owned(table_name.clone()),
                    name: Cow::Owned(str_of(check, "name").unwrap_or(check_key).to_string()),
                    expression: Cow::Owned(
                        str_of(check, "value")
                            .or_else(|| str_of(check, "expression"))
                            .unwrap_or_default()
                            .to_string(),
                    ),
                    enforced: check.get("enforced").and_then(Value::as_bool),
                },
            ));
        }
    }

    for (view_key, view) in sorted_objects(obj.get("views")) {
        let (key_database, key_name) = split_mysql_table_key(view_key);
        ddl.push_entity(mysql::MySQLEntity::View(mysql::View {
            database: mysql_database(view, selected_database.as_deref().or(key_database)),
            name: Cow::Owned(str_of(view, "name").unwrap_or(key_name).to_string()),
            definition: mysql_optional_string(view, "definition"),
            algorithm: mysql_view_algorithm(str_of(view, "algorithm")),
            definer: mysql_optional_string(view, "definer"),
            sql_security: mysql_view_security(
                str_of(view, "sqlSecurity").or_else(|| str_of(view, "security")),
            ),
            check_option: mysql_view_check_option(str_of(view, "checkOption")),
            charset: mysql_optional_string_any(view, &["charset", "characterSet"]),
            collation: mysql_optional_string_any(view, &["collation", "collate"]),
            is_existing: bool_of(view, "isExisting"),
        }));
    }

    snapshot.ddl = ddl.to_entities();
    serde_json::to_value(snapshot).unwrap_or(Value::Null)
}

fn split_mysql_table_key(key: &str) -> (Option<&str>, &str) {
    match key.rsplit_once('.') {
        Some((database, name)) if !database.is_empty() && !name.is_empty() => {
            (Some(database), name)
        }
        _ => (None, key),
    }
}

fn mysql_database(obj: &Map<String, Value>, fallback: Option<&str>) -> Option<Cow<'static, str>> {
    str_of(obj, "database")
        .or_else(|| str_of(obj, "schema"))
        .filter(|database| !database.is_empty())
        .or(fallback)
        .filter(|database| !database.is_empty())
        .map(|database| Cow::Owned(database.to_string()))
}

fn mysql_optional_string(obj: &Map<String, Value>, key: &str) -> Option<Cow<'static, str>> {
    obj.get(key).and_then(mysql_value_string).map(Cow::Owned)
}

fn mysql_optional_string_any(obj: &Map<String, Value>, keys: &[&str]) -> Option<Cow<'static, str>> {
    keys.iter().find_map(|key| mysql_optional_string(obj, key))
}

fn mysql_value_string(value: &Value) -> Option<String> {
    match value {
        Value::String(value) => Some(value.clone()),
        Value::Number(value) => Some(value.to_string()),
        Value::Bool(value) => Some(value.to_string()),
        _ => None,
    }
}

fn mysql_generated(generated: &Map<String, Value>) -> Option<mysql::Generated> {
    let expression = str_of(generated, "as").or_else(|| str_of(generated, "expression"))?;
    let generation_type = match str_of(generated, "type")
        .or_else(|| str_of(generated, "kind"))
        .unwrap_or("stored")
        .to_ascii_lowercase()
        .as_str()
    {
        "virtual" => mysql::GeneratedType::Virtual,
        _ => mysql::GeneratedType::Stored,
    };
    Some(mysql::Generated {
        expression: Cow::Owned(expression.to_string()),
        generation_type,
    })
}

fn mysql_inline_type(sql_type: &str) -> Option<mysql::InlineType> {
    let sql_type = sql_type.trim();
    let (kind, values) = match (sql_type.get(..4), sql_type.get(4..)) {
        (Some(prefix), Some(values)) if prefix.eq_ignore_ascii_case("enum") => ("enum", values),
        _ => match (sql_type.get(..3), sql_type.get(3..)) {
            (Some(prefix), Some(values)) if prefix.eq_ignore_ascii_case("set") => ("set", values),
            _ => return None,
        },
    };
    let values = mysql_inline_values(values)?;
    let values = mysql::InlineEnum {
        values: to_cow_list(values),
    };
    match kind {
        "enum" => Some(mysql::InlineType::Enum(values)),
        "set" => Some(mysql::InlineType::Set(values)),
        _ => None,
    }
}

fn mysql_inline_values(input: &str) -> Option<Vec<String>> {
    let input = input.trim_start();
    let body = input.strip_prefix('(')?;
    let end = body.rfind(')')?;
    if !body[end + 1..].trim().is_empty() {
        return None;
    }
    let body = &body[..end];
    let mut values = Vec::new();
    let mut start = 0;
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in body.char_indices() {
        if let Some(delimiter) = quote {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == delimiter {
                quote = None;
            }
        } else if matches!(character, '\'' | '"') {
            quote = Some(character);
        } else if character == ',' {
            values.push(mysql_unquote_inline_value(&body[start..index]));
            start = index + character.len_utf8();
        }
    }
    quote.is_none().then(|| {
        values.push(mysql_unquote_inline_value(&body[start..]));
        values
    })
}

fn mysql_unquote_inline_value(value: &str) -> String {
    let value = value.trim();
    let Some(delimiter) = value
        .chars()
        .next()
        .filter(|delimiter| matches!(delimiter, '\'' | '"'))
    else {
        return value.to_string();
    };
    let Some(inner) = value
        .strip_prefix(delimiter)
        .and_then(|value| value.strip_suffix(delimiter))
    else {
        return value.to_string();
    };

    let mut output = String::new();
    let mut characters = inner.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '\\' {
            if let Some(escaped) = characters.next() {
                output.push(escaped);
            }
        } else if character == delimiter && characters.peek() == Some(&delimiter) {
            output.push(delimiter);
            characters.next();
        } else {
            output.push(character);
        }
    }
    output
}

fn mysql_index_columns(value: Option<&Value>) -> Vec<mysql::IndexColumn> {
    let Some(columns) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    columns
        .iter()
        .filter_map(|column| match column {
            Value::String(name) => Some(mysql::IndexColumn {
                expression: Cow::Owned(name.clone()),
                is_expression: false,
                length: None,
                ascending: None,
            }),
            Value::Object(column) => {
                let expression = str_of(column, "expression")
                    .or_else(|| str_of(column, "name"))
                    .or_else(|| str_of(column, "value"))?;
                let ascending = column.get("asc").and_then(Value::as_bool).or_else(|| {
                    str_of(column, "order").and_then(|order| {
                        match order.to_ascii_lowercase().as_str() {
                            "asc" => Some(true),
                            "desc" => Some(false),
                            _ => None,
                        }
                    })
                });
                Some(mysql::IndexColumn {
                    expression: Cow::Owned(expression.to_string()),
                    is_expression: bool_of(column, "isExpression"),
                    length: column.get("length").and_then(|length| match length {
                        Value::Number(length) => length
                            .as_u64()
                            .and_then(|length| u32::try_from(length).ok()),
                        Value::String(length) => length.parse().ok(),
                        _ => None,
                    }),
                    ascending,
                })
            }
            _ => None,
        })
        .collect()
}

fn mysql_index_method(value: Option<&str>) -> Option<mysql::IndexMethod> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "btree" => Some(mysql::IndexMethod::Btree),
        "hash" => Some(mysql::IndexMethod::Hash),
        _ => None,
    }
}

fn mysql_index_algorithm(value: Option<&str>) -> Option<mysql::IndexAlgorithm> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "default" => Some(mysql::IndexAlgorithm::Default),
        "inplace" => Some(mysql::IndexAlgorithm::Inplace),
        "copy" => Some(mysql::IndexAlgorithm::Copy),
        _ => None,
    }
}

fn mysql_index_lock(value: Option<&str>) -> Option<mysql::IndexLock> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "default" => Some(mysql::IndexLock::Default),
        "none" => Some(mysql::IndexLock::None),
        "shared" => Some(mysql::IndexLock::Shared),
        "exclusive" => Some(mysql::IndexLock::Exclusive),
        _ => None,
    }
}

fn mysql_referential_action(value: Option<&str>) -> Option<mysql::ReferentialAction> {
    match value?
        .trim()
        .to_ascii_uppercase()
        .replace('_', " ")
        .as_str()
    {
        "CASCADE" => Some(mysql::ReferentialAction::Cascade),
        "SET NULL" => Some(mysql::ReferentialAction::SetNull),
        "RESTRICT" => Some(mysql::ReferentialAction::Restrict),
        "NO ACTION" => Some(mysql::ReferentialAction::NoAction),
        // MySQL does not support SET DEFAULT. The v6 typed model deliberately
        // has no variant, so an invalid historical value cannot be emitted.
        _ => None,
    }
}

fn mysql_view_algorithm(value: Option<&str>) -> Option<mysql::ViewAlgorithm> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "undefined" => Some(mysql::ViewAlgorithm::Undefined),
        "merge" => Some(mysql::ViewAlgorithm::Merge),
        "temptable" | "temp_table" => Some(mysql::ViewAlgorithm::Temptable),
        _ => None,
    }
}

fn mysql_view_security(value: Option<&str>) -> Option<mysql::ViewSqlSecurity> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "definer" => Some(mysql::ViewSqlSecurity::Definer),
        "invoker" => Some(mysql::ViewSqlSecurity::Invoker),
        _ => None,
    }
}

fn mysql_view_check_option(value: Option<&str>) -> Option<mysql::ViewCheckOption> {
    match value?.trim().to_ascii_lowercase().as_str() {
        "cascaded" => Some(mysql::ViewCheckOption::Cascaded),
        "local" => Some(mysql::ViewCheckOption::Local),
        _ => None,
    }
}

fn mysql_table_options(table: &Map<String, Value>) -> Vec<mysql::TableOption> {
    let Some(options) = table.get("options").and_then(Value::as_object) else {
        return Vec::new();
    };
    let mut options: Vec<_> = options
        .iter()
        .filter_map(|(name, value)| mysql_value_string(value).map(|value| (name, value)))
        .collect();
    options.sort_unstable_by_key(|(name, _)| *name);
    options
        .into_iter()
        .map(|(name, value)| mysql::TableOption {
            name: Cow::Owned(name.to_string()),
            value: Cow::Owned(value),
        })
        .collect()
}

// =============================================================================
// SQLite v6 → v7 (structural)
// =============================================================================

/// Upgrade a `SQLite` snapshot from the v6 object format (TS drizzle-kit's
/// current stable format) to the v7 entity-array format.
///
/// Mapping decisions (chosen to diff as a no-op against a macro-generated
/// snapshot of the same schema wherever the legacy data allows):
///
/// * Tables carry no `strict`/`without_rowid` info in v6 → both `false`.
/// * Column-level `primaryKey` flags are folded into a single `PrimaryKey`
///   entity per table (macro snapshots never set the column flag); a
///   `compositePrimaryKeys` entry wins over the flags. The entity is named
///   [`lite::name_for_pk`] unless the legacy entry provides a name, which is
///   then preserved with `nameExplicit: true`.
/// * FK/unique-constraint names are preserved with `nameExplicit: true` —
///   drizzle-kit's default names differ from ours, and renaming constraints
///   would rewrite history.
/// * FK actions drop `"no action"` (the SQL default) and uppercase the rest.
/// * Indexes get `origin: manual` — everything drizzle-kit records in
///   `indexes` came from `CREATE INDEX`, not from UNIQUE autoindexes.
/// * `_meta` rename bookkeeping has no entity-format counterpart and is
///   dropped (`renames: []`); it only ever informed already-generated SQL.
#[must_use]
pub fn upgrade_sqlite_v6_to_v7(json: Value) -> Value {
    let Some(obj) = json.as_object() else {
        return json;
    };

    let mut snapshot = SQLiteSnapshot::new();
    let (id, prev_ids) = carry_identity(obj);
    snapshot.id = id;
    snapshot.prev_ids = prev_ids;

    for (table_key, table) in sorted_objects(obj.get("tables")) {
        let table_name = str_of(table, "name").unwrap_or(table_key).to_string();

        // v6 carries no STRICT / WITHOUT ROWID info — TS drizzle-kit does not
        // model either, so `false` is exact, not a guess.
        snapshot.add_entity(lite::SqliteEntity::Table(lite::Table::new(
            table_name.clone(),
        )));

        let mut flagged_pk_columns: Vec<String> = Vec::new();
        for (col_key, col) in sorted_objects(table.get("columns")) {
            let col_name = str_of(col, "name").unwrap_or(col_key).to_string();
            let mut column = lite::Column::new(
                table_name.clone(),
                col_name.clone(),
                str_of(col, "type").unwrap_or_default().to_string(),
            );
            column.not_null = bool_of(col, "notNull");
            // `autoincrement: false` maps to the field being absent (None),
            // matching macro output.
            column.autoincrement = bool_of(col, "autoincrement").then_some(true);
            column.default = col.get("default").and_then(default_literal);
            // Legacy `generated: {as, type}` matches the runtime type's serde
            // renames exactly.
            column.generated = col
                .get("generated")
                .and_then(|g| serde_json::from_value(g.clone()).ok());
            if bool_of(col, "primaryKey") {
                flagged_pk_columns.push(col_name);
            }
            snapshot.add_entity(lite::SqliteEntity::Column(column));
        }

        // Composite entry wins; otherwise fold the column flags into one
        // PrimaryKey entity (macro snapshots model even single-column PKs as
        // a table-level entity).
        let composite_pks = sorted_objects(table.get("compositePrimaryKeys"));
        if let Some((cpk_key, cpk)) = composite_pks.first() {
            let legacy_name = str_of(cpk, "name")
                .filter(|s| !s.is_empty())
                .or_else(|| Some(cpk_key).filter(|s| !s.is_empty()).copied());
            let (name, name_explicit) = match legacy_name {
                Some(name) => (name.to_string(), true),
                None => (lite::name_for_pk(&table_name), false),
            };
            let mut pk = lite::PrimaryKey::from_strings(
                table_name.clone(),
                name,
                string_list(cpk, "columns"),
            );
            pk.name_explicit = name_explicit;
            snapshot.add_entity(lite::SqliteEntity::PrimaryKey(pk));
        } else if !flagged_pk_columns.is_empty() {
            snapshot.add_entity(lite::SqliteEntity::PrimaryKey(
                lite::PrimaryKey::from_strings(
                    table_name.clone(),
                    lite::name_for_pk(&table_name),
                    flagged_pk_columns,
                ),
            ));
        }

        for (fk_key, fk) in sorted_objects(table.get("foreignKeys")) {
            let mut foreign_key = lite::ForeignKey::from_strings(
                str_of(fk, "tableFrom").unwrap_or(&table_name).to_string(),
                str_of(fk, "name").unwrap_or(fk_key).to_string(),
                string_list(fk, "columnsFrom"),
                str_of(fk, "tableTo").unwrap_or_default().to_string(),
                string_list(fk, "columnsTo"),
            );
            foreign_key.name_explicit = true;
            foreign_key.on_delete = fk_action(fk, "onDelete");
            foreign_key.on_update = fk_action(fk, "onUpdate");
            snapshot.add_entity(lite::SqliteEntity::ForeignKey(foreign_key));
        }

        for (uq_key, uq) in sorted_objects(table.get("uniqueConstraints")) {
            let mut unique = lite::UniqueConstraint::from_strings(
                table_name.clone(),
                str_of(uq, "name").unwrap_or(uq_key).to_string(),
                string_list(uq, "columns"),
            );
            unique.name_explicit = true;
            snapshot.add_entity(lite::SqliteEntity::UniqueConstraint(unique));
        }

        for (ck_key, ck) in sorted_objects(table.get("checkConstraints")) {
            snapshot.add_entity(lite::SqliteEntity::CheckConstraint(
                lite::CheckConstraint::new(
                    table_name.clone(),
                    str_of(ck, "name").unwrap_or(ck_key).to_string(),
                    str_of(ck, "value").unwrap_or_default().to_string(),
                ),
            ));
        }

        for (idx_key, idx) in sorted_objects(table.get("indexes")) {
            let columns = string_list(idx, "columns")
                .into_iter()
                .map(lite::IndexColumn::new)
                .collect();
            let mut index = lite::Index::new(
                table_name.clone(),
                str_of(idx, "name").unwrap_or(idx_key).to_string(),
                columns,
            );
            index.is_unique = bool_of(idx, "isUnique");
            index.where_clause = str_of(idx, "where").map(|s| Cow::Owned(s.to_string()));
            index.origin = lite::IndexOrigin::Manual;
            snapshot.add_entity(lite::SqliteEntity::Index(index));
        }
    }

    for (view_key, view) in sorted_objects(obj.get("views")) {
        let mut entity = lite::View::new(str_of(view, "name").unwrap_or(view_key).to_string());
        entity.definition = str_of(view, "definition").map(|s| Cow::Owned(s.to_string()));
        entity.is_existing = bool_of(view, "isExisting");
        entity.error = None;
        snapshot.add_entity(lite::SqliteEntity::View(entity));
    }

    serde_json::to_value(&snapshot).unwrap_or(Value::Null)
}

// =============================================================================
// PostgreSQL v7 → v8 (structural)
// =============================================================================

/// Upgrade a `PostgreSQL` snapshot from the v7 object format (TS
/// drizzle-kit's current stable format) to the v8 entity-array format.
///
/// Mapping decisions (chosen to diff as a no-op against a macro-generated
/// snapshot of the same schema wherever the legacy data allows):
///
/// * The `public` schema is implicit and gets no `Schema` entity, matching
///   macro output; every other `schemas` entry becomes one.
/// * v7's top-level `sequences` dictionary only ever lists standalone
///   sequences (serial-owned ones are folded into their columns), so every
///   entry maps to a `Sequence` entity.
/// * PK entities: a `compositePrimaryKeys` entry wins; otherwise the
///   column-level `primaryKey` flags fold into one entity per table, named
///   `{table}_pkey` (`PostgreSQL`'s default) unless the legacy entry names
///   it, which is then preserved with `nameExplicit: true`.
/// * FK/unique/index names are preserved with `nameExplicit: true` (see the
///   `SQLite` converter for rationale). FK `schemaTo` defaults to `public`;
///   v7 tracks no deferrability, and `false` matches both dialect defaults
///   and our runtime types.
/// * Index columns map `nulls: "first"/"last"` to `nullsFirst`, and `opClass`
///   strings become non-default `Opclass` values. A v7 `with` dictionary is
///   rendered as comma-separated `key=value` storage parameters (the form
///   `CREATE INDEX ... WITH (...)` takes); `method` is carried verbatim —
///   the differ already treats `"btree"` and absent as equal.
/// * Policy `as`/`for`/`to`/`using`/`withCheck` are carried verbatim; absent
///   values stay absent (the differ normalizes them to `PERMISSIVE`/`ALL`
///   when comparing).
/// * `isRLSEnabled: false` maps to the field being absent, matching macro
///   output.
/// * Identity columns keep drizzle-kit's materialized sequence options; the
///   differ fills defaults on the macro side, so both compare equal.
/// * `_meta` rename bookkeeping is dropped (`renames: []`), as in `SQLite`.
///
/// The serialized document additionally gets explicit `null`s for a handful
/// of optional fields (policy `as`/`for`/`using`/`withCheck`, view
/// `using`/`tablespace`/`with` options, identity options): their runtime
/// types pair `deserialize_with` with `skip_serializing_if` but no serde
/// `default`, so an *absent* field is a hard error on load while an explicit
/// `null` decodes as `None`.
#[must_use]
pub fn upgrade_postgres_v7_to_v8(json: Value) -> Value {
    let Some(obj) = json.as_object() else {
        return json;
    };

    let mut snapshot = PostgresSnapshot::new();
    let (id, prev_ids) = carry_identity(obj);
    snapshot.id = id;
    snapshot.prev_ids = prev_ids;

    // Schemas: v7 stores `{name: name}` string entries; `public` is implicit.
    if let Some(schemas) = obj.get("schemas").and_then(Value::as_object) {
        let mut names: Vec<&str> = schemas
            .iter()
            .map(|(key, value)| value.as_str().unwrap_or(key.as_str()))
            .filter(|name| !name.is_empty() && *name != "public")
            .collect();
        names.sort_unstable();
        names.dedup();
        for name in names {
            snapshot.add_entity(pg::PostgresEntity::Schema(pg::Schema::new(
                name.to_string(),
            )));
        }
    }

    for (enum_key, legacy_enum) in sorted_objects(obj.get("enums")) {
        let (fallback_schema, fallback_name) = split_schema_key(enum_key);
        snapshot.add_entity(pg::PostgresEntity::Enum(pg::Enum::from_strings(
            str_of(legacy_enum, "schema")
                .unwrap_or(fallback_schema)
                .to_string(),
            str_of(legacy_enum, "name")
                .unwrap_or(fallback_name)
                .to_string(),
            string_list(legacy_enum, "values"),
        )));
    }

    for (seq_key, legacy_seq) in sorted_objects(obj.get("sequences")) {
        let (fallback_schema, fallback_name) = split_schema_key(seq_key);
        let mut sequence = pg::Sequence::new(
            str_of(legacy_seq, "schema")
                .unwrap_or(fallback_schema)
                .to_string(),
            str_of(legacy_seq, "name")
                .unwrap_or(fallback_name)
                .to_string(),
        );
        sequence.increment_by = scalar_string(legacy_seq, "increment").map(Cow::Owned);
        sequence.min_value = scalar_string(legacy_seq, "minValue").map(Cow::Owned);
        sequence.max_value = scalar_string(legacy_seq, "maxValue").map(Cow::Owned);
        sequence.start_with = scalar_string(legacy_seq, "startWith").map(Cow::Owned);
        sequence.cache_size = scalar_i32(legacy_seq, "cache");
        sequence.cycle = legacy_seq.get("cycle").and_then(Value::as_bool);
        snapshot.add_entity(pg::PostgresEntity::Sequence(sequence));
    }

    for (role_key, legacy_role) in sorted_objects(obj.get("roles")) {
        let mut role = pg::Role::new(str_of(legacy_role, "name").unwrap_or(role_key).to_string());
        role.create_db = legacy_role.get("createDb").and_then(Value::as_bool);
        role.create_role = legacy_role.get("createRole").and_then(Value::as_bool);
        role.inherit = legacy_role.get("inherit").and_then(Value::as_bool);
        snapshot.add_entity(pg::PostgresEntity::Role(role));
    }

    // Top-level ("linked") policies: their table is recorded in `on` as
    // `"schema"."table"`.
    for (policy_key, legacy_policy) in sorted_objects(obj.get("policies")) {
        let Some((schema, table)) = str_of(legacy_policy, "on").map(split_quoted_table_ref) else {
            // A policy without a table reference cannot be represented (and
            // cannot have existed as SQL either) — drop it.
            continue;
        };
        snapshot.add_entity(pg::PostgresEntity::Policy(policy_from_legacy(
            &schema,
            &table,
            policy_key,
            legacy_policy,
        )));
    }

    for (table_key, table) in sorted_objects(obj.get("tables")) {
        let (fallback_schema, fallback_name) = split_schema_key(table_key);
        let schema = str_of(table, "schema")
            .filter(|s| !s.is_empty())
            .unwrap_or(fallback_schema)
            .to_string();
        let table_name = str_of(table, "name").unwrap_or(fallback_name).to_string();

        let mut entity = pg::Table::new(schema.clone(), table_name.clone());
        // `false` maps to the field being absent, matching macro output.
        entity.is_rls_enabled = bool_of(table, "isRLSEnabled").then_some(true);
        snapshot.add_entity(pg::PostgresEntity::Table(entity));

        let mut flagged_pk_columns: Vec<String> = Vec::new();
        for (col_key, col) in sorted_objects(table.get("columns")) {
            let col_name = str_of(col, "name").unwrap_or(col_key).to_string();
            let mut column = pg::Column::new(
                schema.clone(),
                table_name.clone(),
                col_name.clone(),
                str_of(col, "type").unwrap_or_default().to_string(),
            );
            column.type_schema = str_of(col, "typeSchema").map(|s| Cow::Owned(s.to_string()));
            column.not_null = bool_of(col, "notNull");
            column.default = col.get("default").and_then(default_literal);
            column.generated = col
                .get("generated")
                .and_then(|g| serde_json::from_value(g.clone()).ok());
            column.identity = col
                .get("identity")
                .and_then(Value::as_object)
                .map(|identity| identity_from_legacy(&schema, identity));
            if bool_of(col, "primaryKey") {
                flagged_pk_columns.push(col_name);
            }
            snapshot.add_entity(pg::PostgresEntity::Column(column));
        }

        let composite_pks = sorted_objects(table.get("compositePrimaryKeys"));
        if let Some((cpk_key, cpk)) = composite_pks.first() {
            let legacy_name = str_of(cpk, "name")
                .filter(|s| !s.is_empty())
                .or_else(|| Some(cpk_key).filter(|s| !s.is_empty()).copied());
            let (name, name_explicit) = match legacy_name {
                Some(name) => (name.to_string(), true),
                None => (format!("{table_name}_pkey"), false),
            };
            let mut pk = pg::PrimaryKey::new(
                schema.clone(),
                table_name.clone(),
                name,
                to_cow_list(string_list(cpk, "columns")),
            );
            pk.name_explicit = name_explicit;
            snapshot.add_entity(pg::PostgresEntity::PrimaryKey(pk));
        } else if !flagged_pk_columns.is_empty() {
            snapshot.add_entity(pg::PostgresEntity::PrimaryKey(pg::PrimaryKey::new(
                schema.clone(),
                table_name.clone(),
                format!("{table_name}_pkey"),
                to_cow_list(flagged_pk_columns),
            )));
        }

        for (uq_key, uq) in sorted_objects(table.get("uniqueConstraints")) {
            let mut unique = pg::UniqueConstraint::new(
                schema.clone(),
                table_name.clone(),
                str_of(uq, "name").unwrap_or(uq_key).to_string(),
                to_cow_list(string_list(uq, "columns")),
            );
            unique.name_explicit = true;
            unique.nulls_not_distinct = bool_of(uq, "nullsNotDistinct");
            snapshot.add_entity(pg::PostgresEntity::UniqueConstraint(unique));
        }

        for (ck_key, ck) in sorted_objects(table.get("checkConstraints")) {
            snapshot.add_entity(pg::PostgresEntity::CheckConstraint(
                pg::CheckConstraint::new(
                    schema.clone(),
                    table_name.clone(),
                    str_of(ck, "name").unwrap_or(ck_key).to_string(),
                    str_of(ck, "value").unwrap_or_default().to_string(),
                ),
            ));
        }

        for (fk_key, fk) in sorted_objects(table.get("foreignKeys")) {
            let mut foreign_key = pg::ForeignKey::from_strings(
                schema.clone(),
                str_of(fk, "tableFrom").unwrap_or(&table_name).to_string(),
                str_of(fk, "name").unwrap_or(fk_key).to_string(),
                string_list(fk, "columnsFrom"),
                str_of(fk, "schemaTo").unwrap_or("public").to_string(),
                str_of(fk, "tableTo").unwrap_or_default().to_string(),
                string_list(fk, "columnsTo"),
            );
            foreign_key.name_explicit = true;
            foreign_key.on_delete = fk_action(fk, "onDelete");
            foreign_key.on_update = fk_action(fk, "onUpdate");
            snapshot.add_entity(pg::PostgresEntity::ForeignKey(foreign_key));
        }

        for (idx_key, idx) in sorted_objects(table.get("indexes")) {
            snapshot.add_entity(pg::PostgresEntity::Index(index_from_legacy(
                &schema,
                &table_name,
                idx_key,
                idx,
            )));
        }

        for (policy_key, legacy_policy) in sorted_objects(table.get("policies")) {
            snapshot.add_entity(pg::PostgresEntity::Policy(policy_from_legacy(
                &schema,
                &table_name,
                policy_key,
                legacy_policy,
            )));
        }
    }

    for (view_key, view) in sorted_objects(obj.get("views")) {
        let (fallback_schema, fallback_name) = split_schema_key(view_key);
        let mut entity = pg::View::new(
            str_of(view, "schema")
                .unwrap_or(fallback_schema)
                .to_string(),
            str_of(view, "name").unwrap_or(fallback_name).to_string(),
        );
        entity.definition = str_of(view, "definition").map(|s| Cow::Owned(s.to_string()));
        entity.materialized = bool_of(view, "materialized");
        // v7's `with` options use the same camelCase keys as the runtime
        // type; a failed decode (or an empty dict) drops the options.
        entity.with = view
            .get("with")
            .filter(|w| w.as_object().is_none_or(|o| !o.is_empty()))
            .and_then(|w| serde_json::from_value(patch_view_with_nulls(w.clone())).ok());
        entity.is_existing = bool_of(view, "isExisting");
        entity.with_no_data = view.get("withNoData").and_then(Value::as_bool);
        entity.using = str_of(view, "using").map(|s| Cow::Owned(s.to_string()));
        entity.tablespace = str_of(view, "tablespace").map(|s| Cow::Owned(s.to_string()));
        snapshot.add_entity(pg::PostgresEntity::View(entity));
    }

    let mut value = serde_json::to_value(&snapshot).unwrap_or(Value::Null);
    patch_postgres_required_nulls(&mut value);
    value
}

/// Split a legacy `"schema.name"` dictionary key into its parts. Keys
/// without a dot fall back to the `public` schema.
fn split_schema_key(key: &str) -> (&str, &str) {
    key.split_once('.').unwrap_or(("public", key))
}

/// Split a policy `on` reference of the form `"schema"."table"` (quotes
/// optional) into its parts.
fn split_quoted_table_ref(reference: &str) -> (String, String) {
    let unquote = |s: &str| s.trim().trim_matches('"').to_string();
    match reference.split_once('.') {
        Some((schema, table)) => (unquote(schema), unquote(table)),
        None => ("public".to_string(), unquote(reference)),
    }
}

fn to_cow_list(values: Vec<String>) -> Vec<Cow<'static, str>> {
    values.into_iter().map(Cow::Owned).collect()
}

fn policy_from_legacy(
    schema: &str,
    table: &str,
    key: &str,
    legacy: &Map<String, Value>,
) -> pg::Policy {
    let mut policy = pg::Policy::new(
        schema.to_string(),
        table.to_string(),
        str_of(legacy, "name").unwrap_or(key).to_string(),
    );
    policy.as_clause = str_of(legacy, "as").map(|s| Cow::Owned(s.to_string()));
    policy.for_clause = str_of(legacy, "for").map(|s| Cow::Owned(s.to_string()));
    let to_roles = string_list(legacy, "to");
    policy.to = (!to_roles.is_empty()).then(|| to_cow_list(to_roles));
    policy.using = str_of(legacy, "using").map(|s| Cow::Owned(s.to_string()));
    policy.with_check = str_of(legacy, "withCheck").map(|s| Cow::Owned(s.to_string()));
    policy
}

fn identity_from_legacy(table_schema: &str, legacy: &Map<String, Value>) -> pg::Identity {
    let name = str_of(legacy, "name").unwrap_or_default().to_string();
    let mut identity = match str_of(legacy, "type") {
        Some("byDefault") => pg::Identity::by_default(name),
        // TS only emits "always" | "byDefault"; ALWAYS is the safer default
        // for anything else.
        _ => pg::Identity::always(name),
    };
    identity.schema = Some(Cow::Owned(
        str_of(legacy, "schema")
            .filter(|s| !s.is_empty())
            .unwrap_or(table_schema)
            .to_string(),
    ));
    // drizzle-kit materializes the sequence options; carry them verbatim.
    // The differ fills the same defaults on the macro side when comparing,
    // so absent-vs-materialized stays a no-op.
    identity.increment = scalar_string(legacy, "increment").map(Cow::Owned);
    identity.min_value = scalar_string(legacy, "minValue").map(Cow::Owned);
    identity.max_value = scalar_string(legacy, "maxValue").map(Cow::Owned);
    identity.start_with = scalar_string(legacy, "startWith").map(Cow::Owned);
    identity.cache = scalar_i32(legacy, "cache");
    identity.cycle = legacy.get("cycle").and_then(Value::as_bool);
    identity
}

fn index_from_legacy(
    schema: &str,
    table: &str,
    key: &str,
    legacy: &Map<String, Value>,
) -> pg::Index {
    let columns = legacy
        .get("columns")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .filter_map(Value::as_object)
                .map(|col| {
                    let mut column = pg::IndexColumn::new(
                        str_of(col, "expression").unwrap_or_default().to_string(),
                    );
                    column.is_expression = bool_of(col, "isExpression");
                    column.asc = col.get("asc").and_then(Value::as_bool).unwrap_or(true);
                    column.nulls_first = str_of(col, "nulls")
                        .is_some_and(|nulls| nulls.eq_ignore_ascii_case("first"));
                    column.opclass = str_of(col, "opClass")
                        .filter(|s| !s.is_empty())
                        .map(|s| pg::Opclass::new(s.to_string()));
                    column
                })
                .collect()
        })
        .unwrap_or_default();

    let mut index = pg::Index::new(
        schema.to_string(),
        table.to_string(),
        str_of(legacy, "name").unwrap_or(key).to_string(),
        columns,
    );
    index.name_explicit = true;
    index.is_unique = bool_of(legacy, "isUnique");
    index.where_clause = str_of(legacy, "where").map(|s| Cow::Owned(s.to_string()));
    index.method = str_of(legacy, "method").map(|s| Cow::Owned(s.to_string()));
    index.with = index_with_from_legacy(legacy.get("with"));
    index.concurrently = bool_of(legacy, "concurrently");
    index
}

/// v7 stores index storage parameters either as a raw string or as a
/// `{key: value}` dictionary; render dictionaries as the comma-separated
/// `key=value` list `CREATE INDEX ... WITH (...)` takes.
fn index_with_from_legacy(value: Option<&Value>) -> Option<Cow<'static, str>> {
    match value? {
        Value::String(s) if !s.is_empty() => Some(Cow::Owned(s.clone())),
        Value::Object(map) if !map.is_empty() => {
            let mut entries: Vec<(&String, &Value)> = map.iter().collect();
            entries.sort_by_key(|(key, _)| key.as_str());
            let rendered = entries
                .iter()
                .map(|(key, value)| match value {
                    Value::String(s) => format!("{key}={s}"),
                    other => format!("{key}={other}"),
                })
                .collect::<Vec<_>>()
                .join(",");
            Some(Cow::Owned(rendered))
        }
        _ => None,
    }
}

/// Insert explicit `null`s for the `ViewWithOption` string fields whose serde
/// setup rejects absent fields (see [`patch_postgres_required_nulls`]).
fn patch_view_with_nulls(mut value: Value) -> Value {
    if let Some(obj) = value.as_object_mut() {
        for key in ["checkOption", "vacuumIndexCleanup"] {
            obj.entry(key).or_insert(Value::Null);
        }
    }
    value
}

/// Insert explicit `null`s for optional fields the runtime `PostgreSQL`
/// entity types cannot decode when absent.
///
/// Several fields pair `deserialize_with` with `skip_serializing_if` but no
/// serde `default` (policy `as`/`for`/`using`/`withCheck`, view
/// `using`/`tablespace` and its `with` options, identity options). serde
/// treats an absent field as a hard `missing field` error there, while an
/// explicit `null` decodes as `None` — so the emitted document carries the
/// `null`s to stay loadable via `PostgresSnapshot::from_json`.
fn patch_postgres_required_nulls(snapshot: &mut Value) {
    let Some(ddl) = snapshot.get_mut("ddl").and_then(Value::as_array_mut) else {
        return;
    };

    let ensure_nulls = |obj: &mut Map<String, Value>, keys: &[&str]| {
        for key in keys {
            obj.entry(*key).or_insert(Value::Null);
        }
    };

    for entity in ddl {
        let Some(obj) = entity.as_object_mut() else {
            continue;
        };
        match str_of(obj, "entityType")
            .unwrap_or_default()
            .to_string()
            .as_str()
        {
            "policies" => ensure_nulls(obj, &["as", "for", "using", "withCheck"]),
            "views" => {
                ensure_nulls(obj, &["using", "tablespace"]);
                if let Some(with) = obj.get_mut("with").and_then(Value::as_object_mut) {
                    ensure_nulls(with, &["checkOption", "vacuumIndexCleanup"]);
                }
            }
            "columns" => {
                if let Some(identity) = obj.get_mut("identity").and_then(Value::as_object_mut) {
                    ensure_nulls(
                        identity,
                        &["schema", "increment", "minValue", "maxValue", "startWith"],
                    );
                }
            }
            _ => {}
        }
    }
}

// =============================================================================
// Upgrade chaining
// =============================================================================

/// Determine the effective legacy version of a snapshot document from its
/// *shape*, not just its version stamp.
///
/// Historic builds of `upgrade_sqlite_v5_to_v6` / `upgrade_postgres_v6_to_v7`
/// stamped the latest version string onto documents that still had the
/// legacy object shape. The entity-array formats always carry a `ddl` array,
/// so its absence identifies those mis-stamped files and maps them back to
/// the version their shape actually corresponds to.
fn effective_version(json: &Value, dialect: Dialect) -> String {
    let version = json
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("unknown");

    if json.get("ddl").is_some_and(Value::is_array) {
        return version.to_string();
    }

    match dialect {
        Dialect::SQLite if version == SQLITE_SNAPSHOT_VERSION => "6".to_string(),
        Dialect::PostgreSQL if version == POSTGRES_SNAPSHOT_VERSION => "7".to_string(),
        _ => version.to_string(),
    }
}

/// Upgrade a snapshot to the latest version for the given dialect
#[must_use]
pub fn upgrade_to_latest(json: Value, dialect: Dialect) -> Value {
    let mut version = effective_version(&json, dialect);
    let mut current = json;

    match dialect {
        Dialect::SQLite => {
            // Chain upgrades: v5 → v6 → v7
            if version == "5" {
                current = upgrade_sqlite_v5_to_v6(current);
                version = "6".to_string();
            }
            if version == "6" {
                current = upgrade_sqlite_v6_to_v7(current);
            }
            current
        }
        Dialect::PostgreSQL => {
            // Chain upgrades: v5 → v6 → v7 → v8
            if version == "5" {
                current = upgrade_postgres_v5_to_v6(current);
                version = "6".to_string();
            }
            if version == "6" {
                current = upgrade_postgres_v6_to_v7(current);
                version = "7".to_string();
            }
            if version == "7" {
                current = upgrade_postgres_v7_to_v8(current);
            }
            current
        }
        Dialect::MySQL => {
            if version == "5" {
                current = upgrade_mysql_v5_to_v6(current);
            }
            current
        }
    }
}

/// Check if a snapshot needs upgrade using the Dialect trait
///
/// This provides type-safe version checking using the dialect marker types:
/// ```rust
/// # let _ = r####"
/// use drizzle_migrations::{Sqlite, DialectTrait};
/// if Sqlite::needs_upgrade(version) {
///     // perform upgrade
/// }
/// # "####;
/// ```
#[must_use]
pub fn needs_upgrade_for_dialect(dialect: Dialect, version: u32) -> bool {
    use crate::traits::{Dialect as DialectTrait, Mysql, Postgres, Sqlite};

    match dialect {
        Dialect::SQLite => Sqlite::needs_upgrade_from(version),
        Dialect::PostgreSQL => Postgres::needs_upgrade_from(version),
        Dialect::MySQL => Mysql::needs_upgrade_from(version),
    }
}

/// Get the latest version for a dialect using the Dialect trait
#[must_use]
pub const fn latest_version_for_dialect(dialect: Dialect) -> u32 {
    use crate::traits::{Dialect as DialectTrait, Mysql, Postgres, Sqlite, Version};

    match dialect {
        Dialect::SQLite => <Sqlite as DialectTrait>::LatestVersion::NUMBER,
        Dialect::PostgreSQL => <Postgres as DialectTrait>::LatestVersion::NUMBER,
        Dialect::MySQL => <Mysql as DialectTrait>::LatestVersion::NUMBER,
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

        // The in-shape upgrade stamps "6" — only the structural upgrade may
        // stamp the latest version.
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

        // The in-shape upgrade stamps "7" — only the structural upgrade may
        // stamp the latest version.
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
    fn test_upgrade_to_latest_chains_to_entity_format() {
        let v5 = json!({
            "version": "5",
            "dialect": "pg",
            "id": "11111111-1111-1111-1111-111111111111",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {
                "users": {
                    "name": "users",
                    "schema": "public",
                    "columns": {
                        "id": {"name": "id", "type": "integer", "primaryKey": true, "notNull": true}
                    },
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

        let latest = upgrade_to_latest(v5, Dialect::PostgreSQL);

        assert_eq!(latest["version"], POSTGRES_SNAPSHOT_VERSION);
        assert_eq!(latest["id"], "11111111-1111-1111-1111-111111111111");
        assert!(latest["ddl"].is_array());
        // The chained result must parse as the current snapshot type.
        let snapshot: PostgresSnapshot = serde_json::from_value(latest).unwrap();
        assert_eq!(snapshot.prev_ids, vec![ORIGIN_UUID.to_string()]);
        assert!(snapshot.ddl.iter().any(
            |e| matches!(e, pg::PostgresEntity::Index(idx) if idx.name == "idx" && !idx.columns.is_empty())
        ));
    }

    #[test]
    fn test_sqlite_v6_to_v7_structural() {
        let v6 = json!({
            "version": "6",
            "dialect": "sqlite",
            "id": "22222222-2222-2222-2222-222222222222",
            "prevId": "11111111-1111-1111-1111-111111111111",
            "tables": {
                "users": {
                    "name": "users",
                    "columns": {
                        "id": {
                            "name": "id",
                            "type": "integer",
                            "primaryKey": true,
                            "notNull": true,
                            "autoincrement": true
                        },
                        "email": {
                            "name": "email",
                            "type": "text",
                            "primaryKey": false,
                            "notNull": true,
                            "autoincrement": false,
                            "default": "'nobody@example.com'"
                        },
                        "score": {
                            "name": "score",
                            "type": "integer",
                            "primaryKey": false,
                            "notNull": false,
                            "autoincrement": false,
                            "default": 42
                        },
                        "email_upper": {
                            "name": "email_upper",
                            "type": "text",
                            "primaryKey": false,
                            "notNull": false,
                            "autoincrement": false,
                            "generated": {"as": "(upper(email))", "type": "virtual"}
                        }
                    },
                    "indexes": {
                        "users_email_idx": {
                            "name": "users_email_idx",
                            "columns": ["email"],
                            "isUnique": true,
                            "where": "email IS NOT NULL"
                        }
                    },
                    "foreignKeys": {},
                    "compositePrimaryKeys": {},
                    "uniqueConstraints": {
                        "users_email_unique": {
                            "name": "users_email_unique",
                            "columns": ["email"]
                        }
                    },
                    "checkConstraints": {
                        "users_score_check": {
                            "name": "users_score_check",
                            "value": "score >= 0"
                        }
                    }
                },
                "user_roles": {
                    "name": "user_roles",
                    "columns": {
                        "user_id": {"name": "user_id", "type": "integer", "primaryKey": false, "notNull": true, "autoincrement": false},
                        "role_id": {"name": "role_id", "type": "integer", "primaryKey": false, "notNull": true, "autoincrement": false}
                    },
                    "indexes": {},
                    "foreignKeys": {
                        "user_roles_user_id_users_id_fk": {
                            "name": "user_roles_user_id_users_id_fk",
                            "tableFrom": "user_roles",
                            "tableTo": "users",
                            "columnsFrom": ["user_id"],
                            "columnsTo": ["id"],
                            "onDelete": "cascade",
                            "onUpdate": "no action"
                        }
                    },
                    "compositePrimaryKeys": {
                        "user_roles_user_id_role_id_pk": {
                            "name": "user_roles_user_id_role_id_pk",
                            "columns": ["user_id", "role_id"]
                        }
                    },
                    "uniqueConstraints": {}
                }
            },
            "views": {
                "active_users": {
                    "name": "active_users",
                    "definition": "SELECT * FROM users",
                    "isExisting": false
                }
            },
            "enums": {},
            "_meta": {"tables": {}, "columns": {}}
        });

        let v7 = upgrade_sqlite_v6_to_v7(v6);
        assert_eq!(v7["version"], SQLITE_SNAPSHOT_VERSION);
        assert_eq!(v7["dialect"], "sqlite");
        assert_eq!(v7["id"], "22222222-2222-2222-2222-222222222222");
        assert_eq!(v7["prevIds"][0], "11111111-1111-1111-1111-111111111111");

        let snapshot: SQLiteSnapshot = serde_json::from_value(v7).unwrap();
        // 2 tables + 6 columns + 2 pks + 1 fk + 1 unique + 1 check + 1 index + 1 view
        assert_eq!(snapshot.ddl.len(), 15);

        let pk = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                lite::SqliteEntity::PrimaryKey(pk) if pk.table == "user_roles" => Some(pk),
                _ => None,
            })
            .expect("composite pk entity");
        assert_eq!(pk.name, "user_roles_user_id_role_id_pk");
        assert!(pk.name_explicit);
        assert_eq!(pk.columns.as_ref(), ["user_id", "role_id"]);

        let single_pk = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                lite::SqliteEntity::PrimaryKey(pk) if pk.table == "users" => Some(pk),
                _ => None,
            })
            .expect("flagged pk entity");
        assert_eq!(single_pk.name, "users_pk");
        assert!(!single_pk.name_explicit);
        assert_eq!(single_pk.columns.as_ref(), ["id"]);

        let fk = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                lite::SqliteEntity::ForeignKey(fk) => Some(fk),
                _ => None,
            })
            .expect("fk entity");
        assert_eq!(fk.name, "user_roles_user_id_users_id_fk");
        assert!(fk.name_explicit);
        assert_eq!(fk.on_delete.as_deref(), Some("CASCADE"));
        assert_eq!(fk.on_update, None, "'no action' folds to None");

        let id_col = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                lite::SqliteEntity::Column(c) if c.name == "id" => Some(c),
                _ => None,
            })
            .expect("id column");
        assert_eq!(id_col.autoincrement, Some(true));
        assert_eq!(
            id_col.primary_key, None,
            "pk lives on the PrimaryKey entity"
        );

        let generated = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                lite::SqliteEntity::Column(c) if c.name == "email_upper" => Some(c),
                _ => None,
            })
            .expect("generated column");
        let generated = generated.generated.as_ref().expect("generated config");
        assert_eq!(generated.expression, "(upper(email))");
        assert_eq!(generated.gen_type, lite::GeneratedType::Virtual);

        let score = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                lite::SqliteEntity::Column(c) if c.name == "score" => Some(c),
                _ => None,
            })
            .expect("score column");
        assert_eq!(score.default.as_deref(), Some("42"));

        let index = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                lite::SqliteEntity::Index(idx) => Some(idx),
                _ => None,
            })
            .expect("index entity");
        assert!(index.is_unique);
        assert_eq!(index.where_clause.as_deref(), Some("email IS NOT NULL"));
        assert_eq!(index.origin, lite::IndexOrigin::Manual);
        assert_eq!(index.columns[0].value, "email");
        assert!(!index.columns[0].is_expression);
    }

    #[test]
    fn test_sqlite_mis_stamped_v6_shape_still_upgrades() {
        // The old upgrade_sqlite_v5_to_v6 stamped "7" onto v6-shaped docs;
        // effective_version must map them back to "6" and convert.
        let mis_stamped = json!({
            "version": SQLITE_SNAPSHOT_VERSION,
            "dialect": "sqlite",
            "id": "33333333-3333-3333-3333-333333333333",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {
                "users": {
                    "name": "users",
                    "columns": {
                        "id": {"name": "id", "type": "integer", "primaryKey": true, "notNull": true, "autoincrement": false}
                    },
                    "indexes": {},
                    "foreignKeys": {},
                    "compositePrimaryKeys": {},
                    "uniqueConstraints": {}
                }
            },
            "views": {},
            "enums": {},
            "_meta": {"tables": {}, "columns": {}}
        });

        let upgraded = upgrade_to_latest(mis_stamped, Dialect::SQLite);
        let snapshot: SQLiteSnapshot = serde_json::from_value(upgraded).unwrap();
        assert_eq!(snapshot.ddl.len(), 3); // table + column + pk
    }

    #[test]
    fn test_upgrade_is_deterministic() {
        let v6 = json!({
            "version": "6",
            "dialect": "sqlite",
            "id": "44444444-4444-4444-4444-444444444444",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {
                "b_table": {
                    "name": "b_table",
                    "columns": {
                        "z_col": {"name": "z_col", "type": "text", "primaryKey": false, "notNull": false, "autoincrement": false},
                        "a_col": {"name": "a_col", "type": "text", "primaryKey": false, "notNull": false, "autoincrement": false}
                    },
                    "indexes": {},
                    "foreignKeys": {},
                    "compositePrimaryKeys": {},
                    "uniqueConstraints": {}
                },
                "a_table": {
                    "name": "a_table",
                    "columns": {
                        "id": {"name": "id", "type": "integer", "primaryKey": true, "notNull": true, "autoincrement": false}
                    },
                    "indexes": {},
                    "foreignKeys": {},
                    "compositePrimaryKeys": {},
                    "uniqueConstraints": {}
                }
            },
            "views": {},
            "enums": {},
            "_meta": {"tables": {}, "columns": {}}
        });

        let first = serde_json::to_string(&upgrade_sqlite_v6_to_v7(v6.clone())).unwrap();
        let second = serde_json::to_string(&upgrade_sqlite_v6_to_v7(v6)).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn test_postgres_v7_to_v8_structural() {
        let v7 = json!({
            "version": "7",
            "dialect": "postgresql",
            "id": "55555555-5555-5555-5555-555555555555",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {
                "public.users": {
                    "name": "users",
                    "schema": "public",
                    "columns": {
                        "id": {
                            "name": "id",
                            "type": "integer",
                            "primaryKey": true,
                            "notNull": true,
                            "identity": {
                                "type": "always",
                                "name": "users_id_seq",
                                "schema": "public",
                                "increment": "1",
                                "startWith": "1",
                                "minValue": "1",
                                "maxValue": "2147483647",
                                "cache": "1",
                                "cycle": false
                            }
                        },
                        "status": {
                            "name": "status",
                            "type": "status",
                            "typeSchema": "public",
                            "primaryKey": false,
                            "notNull": true,
                            "default": "'active'"
                        },
                        "email_upper": {
                            "name": "email_upper",
                            "type": "text",
                            "primaryKey": false,
                            "notNull": false,
                            "generated": {"as": "upper(email)", "type": "stored"}
                        }
                    },
                    "indexes": {
                        "users_email_idx": {
                            "name": "users_email_idx",
                            "columns": [
                                {
                                    "expression": "email",
                                    "isExpression": false,
                                    "asc": true,
                                    "nulls": "last",
                                    "opClass": "text_pattern_ops"
                                }
                            ],
                            "isUnique": true,
                            "concurrently": false,
                            "method": "btree",
                            "with": {}
                        }
                    },
                    "foreignKeys": {},
                    "compositePrimaryKeys": {},
                    "uniqueConstraints": {
                        "users_email_unique": {
                            "name": "users_email_unique",
                            "columns": ["email"],
                            "nullsNotDistinct": true
                        }
                    },
                    "policies": {
                        "users_select_policy": {
                            "name": "users_select_policy",
                            "as": "PERMISSIVE",
                            "for": "SELECT",
                            "to": ["authenticated"],
                            "using": "user_id = current_user_id()"
                        }
                    },
                    "checkConstraints": {
                        "users_score_check": {
                            "name": "users_score_check",
                            "value": "score >= 0"
                        }
                    },
                    "isRLSEnabled": true
                },
                "app.orders": {
                    "name": "orders",
                    "schema": "app",
                    "columns": {
                        "user_id": {"name": "user_id", "type": "integer", "primaryKey": false, "notNull": true},
                        "order_id": {"name": "order_id", "type": "integer", "primaryKey": false, "notNull": true}
                    },
                    "indexes": {},
                    "foreignKeys": {
                        "orders_user_id_users_id_fk": {
                            "name": "orders_user_id_users_id_fk",
                            "tableFrom": "orders",
                            "tableTo": "users",
                            "schemaTo": "public",
                            "columnsFrom": ["user_id"],
                            "columnsTo": ["id"],
                            "onDelete": "cascade",
                            "onUpdate": "no action"
                        }
                    },
                    "compositePrimaryKeys": {
                        "orders_user_id_order_id_pk": {
                            "name": "orders_user_id_order_id_pk",
                            "columns": ["user_id", "order_id"]
                        }
                    },
                    "uniqueConstraints": {},
                    "policies": {},
                    "checkConstraints": {},
                    "isRLSEnabled": false
                }
            },
            "enums": {
                "public.status": {
                    "name": "status",
                    "schema": "public",
                    "values": ["active", "inactive"]
                }
            },
            "schemas": {"app": "app"},
            "sequences": {
                "public.custom_seq": {
                    "name": "custom_seq",
                    "schema": "public",
                    "increment": "2",
                    "startWith": "10",
                    "minValue": "1",
                    "maxValue": "99999",
                    "cache": "1",
                    "cycle": false
                }
            },
            "roles": {},
            "policies": {},
            "views": {
                "public.active_users": {
                    "name": "active_users",
                    "schema": "public",
                    "definition": "SELECT * FROM users",
                    "materialized": false,
                    "isExisting": false
                }
            },
            "_meta": {"schemas": {}, "tables": {}, "columns": {}}
        });

        let v8 = upgrade_postgres_v7_to_v8(v7);
        assert_eq!(v8["version"], POSTGRES_SNAPSHOT_VERSION);
        assert_eq!(v8["id"], "55555555-5555-5555-5555-555555555555");

        let snapshot: PostgresSnapshot = serde_json::from_value(v8).unwrap();
        // 1 schema + 1 enum + 1 sequence + 2 tables + 5 columns + 2 pks
        // + 1 unique + 1 check + 1 fk + 1 index + 1 policy + 1 view
        assert_eq!(snapshot.ddl.len(), 18);

        let identity_col = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::Column(c) if c.name == "id" => Some(c),
                _ => None,
            })
            .expect("identity column");
        let identity = identity_col.identity.as_ref().expect("identity config");
        assert_eq!(identity.type_, pg::IdentityType::Always);
        assert_eq!(identity.name, "users_id_seq");
        assert_eq!(identity.max_value.as_deref(), Some("2147483647"));
        assert_eq!(identity.cache, Some(1));

        let composite_pk = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::PrimaryKey(pk) if pk.table == "orders" => Some(pk),
                _ => None,
            })
            .expect("composite pk");
        assert_eq!(composite_pk.schema, "app");
        assert_eq!(composite_pk.columns.as_ref(), ["user_id", "order_id"]);
        assert!(composite_pk.name_explicit);

        let flagged_pk = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::PrimaryKey(pk) if pk.table == "users" => Some(pk),
                _ => None,
            })
            .expect("flagged pk");
        assert_eq!(flagged_pk.name, "users_pkey");
        assert!(!flagged_pk.name_explicit);

        let fk = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::ForeignKey(fk) => Some(fk),
                _ => None,
            })
            .expect("fk entity");
        assert_eq!(fk.schema, "app");
        assert_eq!(fk.schema_to, "public");
        assert_eq!(fk.on_delete.as_deref(), Some("CASCADE"));
        assert_eq!(fk.on_update, None);

        let index = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::Index(idx) => Some(idx),
                _ => None,
            })
            .expect("index entity");
        assert!(index.is_unique);
        assert_eq!(index.method.as_deref(), Some("btree"));
        let opclass = index.columns[0].opclass.as_ref().expect("opclass");
        assert_eq!(opclass.name, "text_pattern_ops");
        assert!(!index.columns[0].nulls_first);

        let policy = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::Policy(p) => Some(p),
                _ => None,
            })
            .expect("policy entity");
        assert_eq!(policy.table, "users");
        assert_eq!(policy.for_clause.as_deref(), Some("SELECT"));
        assert_eq!(policy.using.as_deref(), Some("user_id = current_user_id()"));
        assert_eq!(policy.with_check, None);

        let unique = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::UniqueConstraint(u) => Some(u),
                _ => None,
            })
            .expect("unique entity");
        assert!(unique.nulls_not_distinct);

        let sequence = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::Sequence(s) => Some(s),
                _ => None,
            })
            .expect("sequence entity");
        assert_eq!(sequence.increment_by.as_deref(), Some("2"));
        assert_eq!(sequence.cache_size, Some(1));

        let enum_entity = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::Enum(en) => Some(en),
                _ => None,
            })
            .expect("enum entity");
        assert_eq!(enum_entity.values.as_ref(), ["active", "inactive"]);

        assert!(
            snapshot
                .ddl
                .iter()
                .any(|e| matches!(e, pg::PostgresEntity::Schema(s) if s.name == "app"))
        );
        // `public` never becomes a Schema entity.
        assert!(
            !snapshot
                .ddl
                .iter()
                .any(|e| matches!(e, pg::PostgresEntity::Schema(s) if s.name == "public"))
        );

        let table = snapshot
            .ddl
            .iter()
            .find_map(|e| match e {
                pg::PostgresEntity::Table(t) if t.name == "users" => Some(t),
                _ => None,
            })
            .expect("users table");
        assert_eq!(table.is_rls_enabled, Some(true));
    }

    #[test]
    fn mysql_v5_converts_to_loadable_v6_entities() {
        let v5 = json!({
            "version": "5",
            "dialect": "mysql",
            "id": "66666666-6666-6666-6666-666666666666",
            "prevIds": [
                "55555555-5555-5555-5555-555555555555",
                "44444444-4444-4444-4444-444444444444"
            ],
            "database": "app",
            "tables": {
                "users": {
                    "name": "users",
                    "temporary": true,
                    "engine": "InnoDB",
                    "charset": "utf8mb4",
                    "collation": "utf8mb4_0900_ai_ci",
                    "comment": "application users",
                    "options": {"rowFormat": "DYNAMIC", "autoIncrement": 42},
                    "columns": {
                        "id": {
                            "name": "id",
                            "type": "bigint unsigned",
                            "primaryKey": true,
                            "notNull": true,
                            "autoincrement": true
                        },
                        "state": {
                            "name": "state",
                            "type": "enum('draft,review','it\\'s','published')",
                            "notNull": true,
                            "default": "'published'",
                            "onUpdate": "CURRENT_TIMESTAMP"
                        },
                        "tags": {
                            "name": "tags",
                            "type": "set('one','two,three')"
                        },
                        "email": {
                            "name": "email",
                            "type": "varchar(255)"
                        },
                        "role_id": {
                            "name": "role_id",
                            "type": "int"
                        },
                        "email_lower": {
                            "name": "email_lower",
                            "type": "varchar(255)",
                            "generated": {"as": "lower(email)", "type": "virtual"}
                        }
                    },
                    "indexes": {
                        "users_email_idx": {
                            "name": "users_email_idx",
                            "columns": [
                                {"expression": "email", "isExpression": false, "length": 16, "asc": false},
                                {"expression": "lower(email)", "isExpression": true, "order": "asc"}
                            ],
                            "isUnique": true,
                            "using": "BTREE",
                            "algorithm": "INPLACE",
                            "lock": "NONE",
                            "visible": false,
                            "comment": "lookup"
                        }
                    },
                    "foreignKeys": {
                        "users_role_fk": {
                            "name": "users_role_fk",
                            "tableFrom": "users",
                            "tableTo": "roles",
                            "columnsFrom": ["role_id"],
                            "columnsTo": ["id"],
                            "onDelete": "CASCADE",
                            "onUpdate": "NO ACTION"
                        }
                    },
                    "compositePrimaryKeys": {},
                    "uniqueConstraints": {
                        "users_email_unique": {"name": "users_email_unique", "columns": ["email"]}
                    },
                    "checkConstraint": {
                        "users_state_check": {"name": "users_state_check", "value": "state <> ''", "enforced": true}
                    }
                },
                "roles": {
                    "name": "roles",
                    "columns": {
                        "tenant_id": {"name": "tenant_id", "type": "int", "primaryKey": true, "notNull": true},
                        "id": {"name": "id", "type": "int", "primaryKey": true, "notNull": true}
                    },
                    "indexes": {},
                    "foreignKeys": {},
                    "compositePrimaryKeys": {
                        "roles_tenant_id_id_pk": {
                            "name": "roles_tenant_id_id_pk",
                            "columns": ["tenant_id", "id"]
                        }
                    },
                    "uniqueConstraints": {},
                    "checkConstraints": {}
                }
            },
            "views": {
                "active_users": {
                    "name": "active_users",
                    "definition": "select * from users",
                    "algorithm": "MERGE",
                    "definer": "root@localhost",
                    "sqlSecurity": "INVOKER",
                    "checkOption": "CASCADED",
                    "charset": "utf8mb4",
                    "collation": "utf8mb4_0900_ai_ci"
                }
            }
        });

        let upgraded = upgrade_mysql_v5_to_v6(v5.clone());
        assert_eq!(upgraded["version"], MYSQL_SNAPSHOT_VERSION);
        assert_eq!(upgraded["dialect"], "mysql");
        assert_eq!(upgraded["id"], "66666666-6666-6666-6666-666666666666");
        assert_eq!(
            upgraded["prevIds"],
            json!([
                "55555555-5555-5555-5555-555555555555",
                "44444444-4444-4444-4444-444444444444"
            ])
        );

        let snapshot: MySQLSnapshot = serde_json::from_value(upgraded.clone())
            .expect("the v6 MySQL entity document must deserialize");
        assert!(MySQLDDL::try_from_entities(snapshot.ddl.clone()).is_ok());

        let state = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::Column(column) if column.name == "state" => Some(column),
                _ => None,
            })
            .expect("state column");
        assert_eq!(state.database.as_deref(), Some("app"));
        assert_eq!(state.on_update.as_deref(), Some("CURRENT_TIMESTAMP"));
        assert_eq!(
            state.inline_type,
            Some(mysql::InlineType::Enum(mysql::InlineEnum {
                values: to_cow_list(vec![
                    "draft,review".into(),
                    "it's".into(),
                    "published".into()
                ]),
            }))
        );

        let id = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::Column(column)
                    if column.name == "id" && column.table == "users" =>
                {
                    Some(column)
                }
                _ => None,
            })
            .expect("inline primary-key column");
        assert!(id.primary_key);
        assert!(id.autoincrement);

        let primary_keys: Vec<_> = snapshot
            .ddl
            .iter()
            .filter_map(|entity| match entity {
                mysql::MySQLEntity::PrimaryKey(primary_key) => Some(primary_key),
                _ => None,
            })
            .collect();
        assert_eq!(primary_keys.len(), 2, "one primary-key entity per table");
        let inline_primary_key = primary_keys
            .iter()
            .find(|primary_key| primary_key.table == "users")
            .expect("inline primary key must become a table entity");
        assert_eq!(inline_primary_key.columns, to_cow_list(vec!["id".into()]));

        let tags = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::Column(column) if column.name == "tags" => Some(column),
                _ => None,
            })
            .expect("set column");
        assert_eq!(
            tags.inline_type,
            Some(mysql::InlineType::Set(mysql::InlineEnum {
                values: to_cow_list(vec!["one".into(), "two,three".into()]),
            }))
        );

        let composite_primary_key = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::PrimaryKey(primary_key) if primary_key.table == "roles" => {
                    Some(primary_key)
                }
                _ => None,
            })
            .expect("composite primary key entity");
        assert_eq!(
            composite_primary_key.columns,
            to_cow_list(vec!["tenant_id".into(), "id".into()])
        );

        let unique = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::UniqueConstraint(unique) => Some(unique),
                _ => None,
            })
            .expect("unique entity");
        assert_eq!(unique.name, "users_email_unique");

        let check = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::CheckConstraint(check) => Some(check),
                _ => None,
            })
            .expect("singular legacy check constraint entity");
        assert_eq!(check.expression, "state <> ''");
        assert_eq!(check.enforced, Some(true));

        let table = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::Table(table) if table.name == "users" => Some(table),
                _ => None,
            })
            .expect("users table");
        assert!(table.temporary);
        assert_eq!(table.engine.as_deref(), Some("InnoDB"));
        assert_eq!(table.options[0].name, "autoIncrement");
        assert_eq!(table.options[1].name, "rowFormat");

        let generated = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::Column(column) if column.name == "email_lower" => {
                    column.generated.as_ref()
                }
                _ => None,
            })
            .expect("generated column metadata");
        assert_eq!(generated.expression, "lower(email)");
        assert_eq!(generated.generation_type, mysql::GeneratedType::Virtual);

        let index = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::Index(index) => Some(index),
                _ => None,
            })
            .expect("index entity");
        assert_eq!(index.database.as_deref(), Some("app"));
        assert!(!index.columns[0].is_expression);
        assert_eq!(index.columns[0].length, Some(16));
        assert_eq!(index.columns[0].ascending, Some(false));
        assert!(index.columns[1].is_expression);
        assert_eq!(index.columns[1].ascending, Some(true));

        let foreign_key = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::ForeignKey(foreign_key) => Some(foreign_key),
                _ => None,
            })
            .expect("foreign key entity");
        assert_eq!(
            foreign_key.on_delete,
            Some(mysql::ReferentialAction::Cascade)
        );
        assert_eq!(
            foreign_key.on_update,
            Some(mysql::ReferentialAction::NoAction)
        );

        let view = snapshot
            .ddl
            .iter()
            .find_map(|entity| match entity {
                mysql::MySQLEntity::View(view) => Some(view),
                _ => None,
            })
            .expect("view entity");
        assert_eq!(view.database.as_deref(), Some("app"));
        assert_eq!(view.algorithm, Some(mysql::ViewAlgorithm::Merge));
        assert_eq!(view.sql_security, Some(mysql::ViewSqlSecurity::Invoker));

        let again = upgrade_mysql_v5_to_v6(v5);
        assert_eq!(
            serde_json::to_string(&upgraded).unwrap(),
            serde_json::to_string(&again).unwrap(),
            "legacy dictionary order must not leak into v6 output"
        );
    }

    #[test]
    fn mysql_upgrade_to_latest_only_converts_v5_objects() {
        let v5 = json!({
            "version": "5",
            "dialect": "mysql",
            "id": "77777777-7777-7777-7777-777777777777",
            "prevId": "00000000-0000-0000-0000-000000000000",
            "tables": {},
            "views": {}
        });
        let upgraded = upgrade_to_latest(v5, Dialect::MySQL);
        let snapshot: MySQLSnapshot = serde_json::from_value(upgraded).unwrap();
        assert_eq!(snapshot.version, MYSQL_SNAPSHOT_VERSION);
        assert!(snapshot.ddl.is_empty());

        let current = serde_json::to_value(MySQLSnapshot::new()).unwrap();
        assert_eq!(upgrade_to_latest(current.clone(), Dialect::MySQL), current);
    }
}
