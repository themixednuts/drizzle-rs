//! Structural upgrade round-trips: legacy TS drizzle-kit snapshots must
//! convert to the current entity-array formats, and the converted documents
//! must load back through the typed snapshot APIs (including from disk via
//! `Snapshot::load`).

use drizzle_migrations::postgres::PostgresSnapshot;
use drizzle_migrations::schema::Snapshot;
use drizzle_migrations::sqlite::SQLiteSnapshot;
use drizzle_migrations::upgrade::upgrade_to_latest;
use drizzle_migrations::version::{POSTGRES_SNAPSHOT_VERSION, SQLITE_SNAPSHOT_VERSION};
use drizzle_types::Dialect;
use serde_json::{Value, json};

/// A realistic TS drizzle-kit sqlite v6 snapshot: composite PK, FK with
/// actions, generated column, unique + check constraints, partial unique
/// index, view.
fn sqlite_v6_fixture() -> Value {
    json!({
        "version": "6",
        "dialect": "sqlite",
        "id": "8c7cbc46-8a3b-42f7-96b4-fc2af4a9d001",
        "prevId": "00000000-0000-0000-0000-000000000000",
        "tables": {
            "users": {
                "name": "users",
                "columns": {
                    "id": {"name": "id", "type": "integer", "primaryKey": true, "notNull": true, "autoincrement": true},
                    "email": {"name": "email", "type": "text", "primaryKey": false, "notNull": true, "autoincrement": false},
                    "score": {"name": "score", "type": "integer", "primaryKey": false, "notNull": false, "autoincrement": false, "default": 0},
                    "email_upper": {
                        "name": "email_upper",
                        "type": "text",
                        "primaryKey": false,
                        "notNull": false,
                        "autoincrement": false,
                        "generated": {"as": "(upper(email))", "type": "stored"}
                    }
                },
                "indexes": {
                    "users_email_idx": {"name": "users_email_idx", "columns": ["email"], "isUnique": true, "where": "email IS NOT NULL"}
                },
                "foreignKeys": {},
                "compositePrimaryKeys": {},
                "uniqueConstraints": {
                    "users_email_unique": {"name": "users_email_unique", "columns": ["email"]}
                },
                "checkConstraints": {
                    "users_score_check": {"name": "users_score_check", "value": "\"users\".\"score\" >= 0"}
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
                    "user_roles_user_id_role_id_pk": {"name": "user_roles_user_id_role_id_pk", "columns": ["user_id", "role_id"]}
                },
                "uniqueConstraints": {}
            }
        },
        "views": {
            "active_users": {"name": "active_users", "definition": "select * from users", "isExisting": false}
        },
        "enums": {},
        "_meta": {"tables": {}, "columns": {}}
    })
}

/// A realistic TS drizzle-kit postgres v7 snapshot: enum, identity column,
/// index with opClass, policy (one without `using`), standalone sequence,
/// non-public schema, materialized view.
fn postgres_v7_fixture() -> Value {
    json!({
        "version": "7",
        "dialect": "postgresql",
        "id": "9f0e11f8-30f2-4f14-8f0e-aa41f7fbe002",
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
                            "type": "byDefault",
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
                    "email": {"name": "email", "type": "text", "primaryKey": false, "notNull": true},
                    "status": {
                        "name": "status",
                        "type": "status",
                        "typeSchema": "public",
                        "primaryKey": false,
                        "notNull": true,
                        "default": "'active'"
                    }
                },
                "indexes": {
                    "users_email_idx": {
                        "name": "users_email_idx",
                        "columns": [
                            {"expression": "email", "isExpression": false, "asc": true, "nulls": "last", "opClass": "text_pattern_ops"}
                        ],
                        "isUnique": true,
                        "concurrently": false,
                        "method": "btree",
                        "with": {}
                    }
                },
                "foreignKeys": {},
                "compositePrimaryKeys": {},
                "uniqueConstraints": {},
                "policies": {
                    "users_select": {
                        "name": "users_select",
                        "as": "PERMISSIVE",
                        "for": "SELECT",
                        "to": ["authenticated"],
                        "using": "id = current_user_id()"
                    },
                    "users_all": {
                        // No `using`/`withCheck` — the emitted document must
                        // still deserialize (explicit nulls are patched in).
                        "name": "users_all",
                        "as": "PERMISSIVE",
                        "for": "ALL",
                        "to": ["public"]
                    }
                },
                "checkConstraints": {},
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
                        "onDelete": "restrict",
                        "onUpdate": "no action"
                    }
                },
                "compositePrimaryKeys": {
                    "orders_user_id_order_id_pk": {"name": "orders_user_id_order_id_pk", "columns": ["user_id", "order_id"]}
                },
                "uniqueConstraints": {},
                "policies": {},
                "checkConstraints": {},
                "isRLSEnabled": false
            }
        },
        "enums": {
            "public.status": {"name": "status", "schema": "public", "values": ["active", "inactive"]}
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
            "public.user_stats": {
                "name": "user_stats",
                "schema": "public",
                "definition": "select count(*) from users",
                "materialized": true,
                "withNoData": false,
                "isExisting": false
            }
        },
        "_meta": {"schemas": {}, "tables": {}, "columns": {}}
    })
}

#[test]
fn sqlite_v6_converts_and_reloads() {
    let upgraded = upgrade_to_latest(sqlite_v6_fixture(), Dialect::SQLite);
    assert_eq!(upgraded["version"], SQLITE_SNAPSHOT_VERSION);
    assert_eq!(upgraded["dialect"], "sqlite");
    assert_eq!(upgraded["id"], "8c7cbc46-8a3b-42f7-96b4-fc2af4a9d001");
    assert_eq!(
        upgraded["prevIds"][0],
        "00000000-0000-0000-0000-000000000000"
    );

    let snapshot = SQLiteSnapshot::from_json(&serde_json::to_string(&upgraded).expect("serialize"))
        .expect("converted sqlite snapshot must deserialize");
    // 2 tables + 6 columns + 2 pks + 1 fk + 1 index + 1 unique + 1 check + 1 view
    assert_eq!(snapshot.ddl.len(), 15);
}

#[test]
fn postgres_v7_converts_and_reloads() {
    let upgraded = upgrade_to_latest(postgres_v7_fixture(), Dialect::PostgreSQL);
    assert_eq!(upgraded["version"], POSTGRES_SNAPSHOT_VERSION);
    assert_eq!(upgraded["id"], "9f0e11f8-30f2-4f14-8f0e-aa41f7fbe002");

    let snapshot =
        PostgresSnapshot::from_json(&serde_json::to_string(&upgraded).expect("serialize"))
            .expect("converted postgres snapshot must deserialize");
    // 1 schema + 1 enum + 1 sequence + 2 tables + 5 columns + 2 pks + 1 fk
    // + 1 index + 2 policies + 1 view
    assert_eq!(snapshot.ddl.len(), 17);
}

#[test]
fn converted_snapshots_load_from_disk() {
    let dir = tempfile::tempdir().expect("tempdir");

    let sqlite_path = dir.path().join("sqlite_snapshot.json");
    let upgraded = upgrade_to_latest(sqlite_v6_fixture(), Dialect::SQLite);
    std::fs::write(
        &sqlite_path,
        serde_json::to_string_pretty(&upgraded).expect("serialize"),
    )
    .expect("write sqlite snapshot");
    let loaded = Snapshot::load(&sqlite_path, Dialect::SQLite).expect("load sqlite snapshot");
    assert!(!loaded.is_empty());
    assert_eq!(loaded.id(), "8c7cbc46-8a3b-42f7-96b4-fc2af4a9d001");

    let pg_path = dir.path().join("postgres_snapshot.json");
    let upgraded = upgrade_to_latest(postgres_v7_fixture(), Dialect::PostgreSQL);
    std::fs::write(
        &pg_path,
        serde_json::to_string_pretty(&upgraded).expect("serialize"),
    )
    .expect("write postgres snapshot");
    let loaded = Snapshot::load(&pg_path, Dialect::PostgreSQL).expect("load postgres snapshot");
    assert!(!loaded.is_empty());
    assert_eq!(loaded.id(), "9f0e11f8-30f2-4f14-8f0e-aa41f7fbe002");
}

#[test]
fn conversion_is_deterministic() {
    let first =
        serde_json::to_string_pretty(&upgrade_to_latest(sqlite_v6_fixture(), Dialect::SQLite))
            .expect("serialize");
    let second =
        serde_json::to_string_pretty(&upgrade_to_latest(sqlite_v6_fixture(), Dialect::SQLite))
            .expect("serialize");
    assert_eq!(first, second);

    let first = serde_json::to_string_pretty(&upgrade_to_latest(
        postgres_v7_fixture(),
        Dialect::PostgreSQL,
    ))
    .expect("serialize");
    let second = serde_json::to_string_pretty(&upgrade_to_latest(
        postgres_v7_fixture(),
        Dialect::PostgreSQL,
    ))
    .expect("serialize");
    assert_eq!(first, second);
}

#[test]
fn already_current_snapshots_pass_through_unchanged() {
    let current = serde_json::to_value(SQLiteSnapshot::new()).expect("serialize");
    let upgraded = upgrade_to_latest(current.clone(), Dialect::SQLite);
    assert_eq!(current, upgraded);

    let current = serde_json::to_value(PostgresSnapshot::new()).expect("serialize");
    let upgraded = upgrade_to_latest(current.clone(), Dialect::PostgreSQL);
    assert_eq!(current, upgraded);
}
