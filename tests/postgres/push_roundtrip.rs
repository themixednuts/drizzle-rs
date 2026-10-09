//! `push` of an unchanged macro schema must plan no statements, and the
//! macro `create_statements` DDL must run as-is in a non-`public` schema.
//!
//! The second push's plan is computed the way `push` computes it, from an
//! introspection scoped to the test's schema (the driver's own scoped
//! introspection is private).

#![cfg(feature = "postgres-sync")]

use drizzle::postgres::prelude::*;
use drizzle_migrations::Schema as _;
use drizzle_migrations::postgres::PostgresSnapshot;
use drizzle_migrations::postgres::ddl::Schema as PgSchema;
use drizzle_migrations::postgres::introspect::{
    RawCheckInfo, RawColumnInfo, RawEnumInfo, RawForeignKeyInfo, RawIndexInfo, RawIntrospection,
    RawPolicyInfo, RawPrimaryKeyInfo, RawSequenceInfo, RawTableInfo, RawUniqueInfo, RawViewInfo,
    action_code_to_string, assemble_ddl, parse_index_columns, queries,
};
use drizzle_migrations::schema::Snapshot;

const PUSH_SCHEMA: &str = "push_roundtrip_test";

#[derive(PostgresEnum, Default, Clone, Copy, Debug, PartialEq)]
#[postgres_enum(schema = "push_roundtrip_test")]
enum PushRoundtripStatus {
    #[default]
    Active,
    Archived,
}

#[PostgresTable(name = "parents", schema = "push_roundtrip_test")]
struct PushRoundtripParent {
    #[column(primary, identity(by_default, cache = 5))]
    id: i32,
    #[column(unique)]
    code: String,
}

#[PostgresTable(
    name = "items",
    schema = "push_roundtrip_test",
    CHECK(name = "items_score_check", expr = "score >= 0 AND label <> 'bad'")
)]
struct PushRoundtripItem {
    #[column(serial, primary)]
    id: i32,
    #[column(REFERENCES = PushRoundtripParent::id)]
    parent_id: i32,
    #[column(COLLATE = C)]
    label: String,
    #[column(DEFAULT = -1)]
    balance: i32,
    #[column(DEFAULT = true)]
    active: bool,
    ratio: f32,
    score: f64,
    tags: Vec<String>,
    #[column(VARCHAR(12))]
    code: String,
    #[column(enum)]
    status: PushRoundtripStatus,
    #[column(generated(stored, "balance * 2"))]
    doubled: i32,
}

#[PostgresIndex(where = "label IS NOT NULL")]
struct PushRoundtripLabelIdx(PushRoundtripItem::label);

#[PostgresIndex(concurrent)]
struct PushRoundtripCodeIdx(PushRoundtripItem::code);

#[PostgresPolicy(
    NAME = "items_visible",
    FOR = "SELECT",
    TO("public"),
    USING = "balance > -100"
)]
struct PushRoundtripPolicy(PushRoundtripItem);

#[derive(PostgresSchema)]
struct PushRoundtripSchema {
    status: PushRoundtripStatus,
    parents: PushRoundtripParent,
    items: PushRoundtripItem,
    label_idx: PushRoundtripLabelIdx,
    code_idx: PushRoundtripCodeIdx,
    policy: PushRoundtripPolicy,
}

/// The live schema `push` compares against, scoped to `schema`.
fn introspect_schema(client: &mut postgres::Client, schema: &str) -> PostgresSnapshot {
    let schemas = vec![schema.to_string()];
    let in_scope = |name: &String| name == schema;

    let tables = client
        .query(queries::TABLES_QUERY, &[])
        .expect("tables")
        .into_iter()
        .map(|row| RawTableInfo {
            schema: row.get(0),
            name: row.get(1),
            is_rls_enabled: row.get(2),
            is_unlogged: row.get(3),
            is_temporary: row.get(4),
            tablespace: row.get(5),
            comment: row.get(6),
        })
        .filter(|t| in_scope(&t.schema))
        .collect();
    let columns = client
        .query(queries::COLUMNS_QUERY, &[])
        .expect("columns")
        .into_iter()
        .map(|row| RawColumnInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            column_type: row.get(3),
            type_schema: row.get(4),
            not_null: row.get(5),
            default_value: row.get(6),
            is_identity: row.get(7),
            identity_type: row.get(8),
            is_generated: row.get(9),
            generated_expression: row.get(10),
            generated_stored: row.get(11),
            dimensions: row.get(12),
            comment: row.get(13),
            ordinal_position: row.get(14),
        })
        .filter(|c| in_scope(&c.schema))
        .collect();
    let enums = client
        .query(queries::ENUMS_QUERY, &[])
        .expect("enums")
        .into_iter()
        .map(|row| RawEnumInfo {
            schema: row.get(0),
            name: row.get(1),
            values: row.get(2),
        })
        .filter(|e| in_scope(&e.schema))
        .collect();
    let sequences = client
        .query(queries::SEQUENCES_QUERY, &[])
        .expect("sequences")
        .into_iter()
        .map(|row| RawSequenceInfo {
            schema: row.get(0),
            name: row.get(1),
            data_type: row.get(2),
            start_value: row.get(3),
            min_value: row.get(4),
            max_value: row.get(5),
            increment: row.get(6),
            cycle: row.get(7),
            cache_value: row.get(8),
            owned_by: row.get(9),
        })
        .filter(|s| in_scope(&s.schema))
        .collect();
    let views = client
        .query(queries::VIEWS_QUERY, &[&Some(&schemas)])
        .expect("views")
        .into_iter()
        .map(|row| RawViewInfo {
            schema: row.get(0),
            name: row.get(1),
            definition: row.get(2),
            is_materialized: row.get(3),
        })
        .collect();
    let indexes = client
        .query(queries::INDEXES_QUERY_FILTERED, &[&schemas])
        .expect("indexes")
        .into_iter()
        .map(|row| RawIndexInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            is_unique: row.get(3),
            is_primary: row.get(4),
            method: row.get(5),
            columns: parse_index_columns(row.get(6)),
            where_clause: row.get(7),
            concurrent: false,
        })
        .collect();
    let foreign_keys = client
        .query(queries::FOREIGN_KEYS_QUERY, &[])
        .expect("fks")
        .into_iter()
        .map(|row| RawForeignKeyInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
            schema_to: row.get(4),
            table_to: row.get(5),
            columns_to: row.get(6),
            on_update: action_code_to_string(&row.get::<_, String>(7)),
            on_delete: action_code_to_string(&row.get::<_, String>(8)),
            deferrable: row.get(9),
            initially_deferred: row.get(10),
        })
        .filter(|f| in_scope(&f.schema))
        .collect();
    let primary_keys = client
        .query(queries::PRIMARY_KEYS_QUERY, &[])
        .expect("pks")
        .into_iter()
        .map(|row| RawPrimaryKeyInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
        })
        .filter(|p| in_scope(&p.schema))
        .collect();
    let unique_constraints = client
        .query(queries::UNIQUES_QUERY, &[])
        .expect("uniques")
        .into_iter()
        .map(|row| RawUniqueInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            columns: row.get(3),
            nulls_not_distinct: row.get(4),
            deferrable: row.get(5),
            initially_deferred: row.get(6),
        })
        .filter(|u| in_scope(&u.schema))
        .collect();
    let check_constraints = client
        .query(queries::CHECKS_QUERY_FILTERED, &[&schemas])
        .expect("checks")
        .into_iter()
        .map(|row| RawCheckInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            expression: row.get(3),
        })
        .collect();
    let policies = client
        .query(queries::POLICIES_QUERY, &[])
        .expect("policies")
        .into_iter()
        .map(|row| RawPolicyInfo {
            schema: row.get(0),
            table: row.get(1),
            name: row.get(2),
            as_clause: row.get(3),
            for_clause: row.get(4),
            to: row.get(5),
            using: row.get(6),
            with_check: row.get(7),
        })
        .filter(|p| in_scope(&p.schema))
        .collect();

    let ddl = assemble_ddl(RawIntrospection {
        schemas: vec![PgSchema::new(schema.to_string())],
        tables,
        columns,
        enums,
        sequences,
        views,
        indexes,
        foreign_keys,
        primary_keys,
        unique_constraints,
        check_constraints,
        roles: Vec::new(),
        policies,
    });
    let mut snapshot = PostgresSnapshot::new();
    for entity in ddl.to_entities() {
        snapshot.add_entity(entity);
    }
    snapshot
}

#[test]
fn postgres_sync_second_push_of_a_macro_schema_plans_nothing() {
    let (mut db, schema) = crate::common::helpers::postgres_sync_setup::setup_empty_named_db(
        PUSH_SCHEMA,
        PushRoundtripSchema::default(),
    );

    db.push(&schema).expect("first push");

    let Snapshot::Postgres(desired) = schema.to_snapshot() else {
        panic!("expected a PostgreSQL snapshot");
    };
    let live = introspect_schema(db.conn_mut(), PUSH_SCHEMA).prepare_for_push(&desired);
    let plan = drizzle_migrations::diff(&Snapshot::Postgres(live), &Snapshot::Postgres(desired))
        .expect("diff");
    assert!(
        plan.statements.is_empty(),
        "an unchanged schema must push nothing: {:#?}",
        plan.statements
    );

    db.push(&schema).expect("second push");
}

/// Push drops a column the schema no longer has only while it holds no
/// values.
#[test]
fn postgres_sync_push_refuses_to_drop_data() {
    let (mut db, schema) = crate::common::helpers::postgres_sync_setup::setup_empty_named_db(
        PUSH_SCHEMA,
        PushRoundtripSchema::default(),
    );
    db.push(&schema).expect("first push");

    db.conn_mut()
        .batch_execute(&format!(
            "ALTER TABLE \"{PUSH_SCHEMA}\".\"parents\" ADD COLUMN \"legacy\" integer; \
             INSERT INTO \"{PUSH_SCHEMA}\".\"parents\" (\"code\", \"legacy\") VALUES ('a', 7);"
        ))
        .expect("add a column with a value");
    let error = db.push(&schema).expect_err("push must not drop values");
    assert!(
        error.to_string().contains(&format!(
            "column `{PUSH_SCHEMA}.parents.legacy` holds 1 row(s)"
        )),
        "{error}"
    );

    db.conn_mut()
        .batch_execute(&format!(
            "UPDATE \"{PUSH_SCHEMA}\".\"parents\" SET \"legacy\" = NULL"
        ))
        .expect("clear the column");
    db.push(&schema).expect("an all-NULL column is dropped");
}

const AUDIT_SCHEMA: &str = "push_audit_test";

#[derive(PostgresEnum, Default, Clone, Copy, PartialEq, Debug)]
#[postgres_enum(schema = "push_audit_test")]
enum PushAuditMood {
    #[default]
    Happy,
    Sad,
}

#[PostgresTable(schema = "push_audit_test", name = "MixedCase")]
struct PushAuditMixedCase {
    #[column(primary, identity(by_default))]
    id: i32,
    #[column(name = "DisplayName", default = "it's")]
    display_name: String,
    #[column(enum)]
    mood: PushAuditMood,
    #[column(CHECK = "score >= 0")]
    score: i32,
    #[column(generated(stored, "score * 2"))]
    double_score: i32,
    #[column(default = 5)]
    five: i32,
    #[column(default = true)]
    flag: bool,
    #[column(default = 1.5)]
    ratio: f64,
    #[column(unique)]
    email: String,
    #[column(COLLATE = C)]
    collated: String,
    #[column(varchar(20), default = "x")]
    short: String,
}

#[PostgresTable(
    schema = "push_audit_test",
    UNIQUE(columns(a, b)),
    CHECK(name = "kids_a_check", expr = "a <> 'bad'")
)]
struct PushAuditKids {
    #[column(primary)]
    id: i64,
    a: String,
    b: String,
    #[column(references = PushAuditMixedCase::id, on_delete = CASCADE)]
    parent_id: i32,
    tags: Vec<String>,
    #[column(default = "{}")]
    empty_tags: Vec<String>,
}

#[PostgresIndex(unique, where = "a IS NOT NULL")]
struct PushAuditKidsAIdx(PushAuditKids::a);

#[derive(PostgresSchema)]
struct PushAuditSchema {
    mood: PushAuditMood,
    mixed_case: PushAuditMixedCase,
    kids: PushAuditKids,
    kids_a_idx: PushAuditKidsAIdx,
}

#[test]
fn postgres_sync_second_push_of_mixed_case_quoted_schema_plans_nothing() {
    let (mut db, schema) = crate::common::helpers::postgres_sync_setup::setup_empty_named_db(
        AUDIT_SCHEMA,
        PushAuditSchema::default(),
    );

    db.push(&schema).expect("first push");

    let Snapshot::Postgres(desired) = schema.to_snapshot() else {
        panic!("expected a PostgreSQL snapshot");
    };
    let live = introspect_schema(db.conn_mut(), AUDIT_SCHEMA).prepare_for_push(&desired);
    let plan = drizzle_migrations::diff(&Snapshot::Postgres(live), &Snapshot::Postgres(desired))
        .expect("diff");
    assert!(
        plan.statements.is_empty(),
        "an unchanged schema must push nothing: {:#?}",
        plan.statements
    );
}

// =============================================================================
// create_statements in a non-public schema
// =============================================================================

const DDL_SCHEMA: &str = "macro_ddl_schema_test";

#[derive(PostgresEnum, Default, Clone, Copy, Debug, PartialEq)]
#[postgres_enum(schema = "macro_ddl_schema_test")]
enum MacroDdlMood {
    #[default]
    Happy,
    Sad,
}

#[PostgresTable(
    name = "parent",
    schema = "macro_ddl_schema_test",
    UNIQUE(columns(tenant, id), name = "parent_tenant_id_key")
)]
struct MacroDdlParent {
    #[column(primary)]
    id: i32,
    tenant: i32,
}

#[PostgresTable(
    name = "child",
    schema = "macro_ddl_schema_test",
    UNIQUE(columns(tenant, parent_id), name = "child_tenant_parent_key"),
    FOREIGN_KEY(columns(tenant, parent_id), references(MacroDdlParent, tenant, id))
)]
struct MacroDdlChild {
    #[column(primary)]
    id: i32,
    #[column(REFERENCES = MacroDdlParent::id)]
    parent_id: i32,
    tenant: i32,
    #[column(enum)]
    mood: MacroDdlMood,
}

#[derive(PostgresSchema)]
struct MacroDdlSchema {
    mood: MacroDdlMood,
    parent: MacroDdlParent,
    child: MacroDdlChild,
}

#[test]
fn postgres_macro_create_statements_run_in_a_non_public_schema() {
    let statements: Vec<String> = MacroDdlSchema::default()
        .create_statements()
        .expect("create statements")
        .collect();

    assert_eq!(statements[0], format!("CREATE SCHEMA \"{DDL_SCHEMA}\";"));
    assert_eq!(
        statements[1],
        format!("CREATE TYPE \"{DDL_SCHEMA}\".\"MacroDdlMood\" AS ENUM ('Happy', 'Sad')")
    );
    let child = statements
        .iter()
        .find(|sql| sql.contains("TABLE \"macro_ddl_schema_test\".\"child\""))
        .expect("child table");
    assert!(
        child.contains(&format!(
            "\"mood\" \"{DDL_SCHEMA}\".\"MacroDdlMood\" NOT NULL"
        )),
        "{child}"
    );
    assert!(
        child.contains(&format!("REFERENCES \"{DDL_SCHEMA}\".\"parent\"(\"id\")")),
        "{child}"
    );
    assert!(
        child.contains(&format!(
            "REFERENCES \"{DDL_SCHEMA}\".\"parent\"(\"tenant\", \"id\")"
        )),
        "{child}"
    );

    let mut client = postgres::Client::connect(
        &std::env::var("DATABASE_URL").unwrap_or_else(|_| {
            "host=localhost user=postgres password=postgres dbname=drizzle_test".to_string()
        }),
        postgres::NoTls,
    )
    .expect("connect");
    client
        .batch_execute(&format!("DROP SCHEMA IF EXISTS \"{DDL_SCHEMA}\" CASCADE;"))
        .expect("clean up");
    let result = statements
        .iter()
        .try_for_each(|sql| client.batch_execute(sql).map_err(|e| (sql.clone(), e)));
    let mood: Option<String> = result.as_ref().ok().map(|()| {
        client
            .query_one(
                "SELECT t.typname::text FROM pg_type t JOIN pg_namespace n ON n.oid = t.typnamespace \
                 WHERE n.nspname = $1 AND t.typname = 'MacroDdlMood'",
                &[&DDL_SCHEMA],
            )
            .expect("enum type keeps its case")
            .get(0)
    });
    client
        .batch_execute(&format!("DROP SCHEMA IF EXISTS \"{DDL_SCHEMA}\" CASCADE;"))
        .expect("clean up");
    if let Err((sql, error)) = result {
        panic!("{sql}\nfailed: {error:?}");
    }
    assert_eq!(mood.as_deref(), Some("MacroDdlMood"));
}
