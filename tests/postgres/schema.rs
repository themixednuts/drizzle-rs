//! PostgreSQL schema tests
//!
//! Schema creation is implicitly tested by all other tests via db.create().
//! These tests verify various schema configurations work correctly.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::eq;
use drizzle::ddl::postgres::ddl::ViewWithOptionDef;
use drizzle::migrations::Schema as MigrationSchema;
use drizzle::postgres::prelude::*;

#[derive(Debug, PostgresFromRow)]
#[from(SimpleView)]
struct PgSimpleViewResult {
    id: i32,
    name: String,
}

#[derive(Debug, PostgresFromRow)]
struct PgSimpleAliasResult {
    id: i32,
    name: String,
}

#[PostgresView(
    NAME = "simple_view",
    DEFINITION = {
        let builder = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>();
        let SimpleSchema { simple } = SimpleSchema::new();
        builder.select((simple.id, simple.name)).from(simple)
    }
)]
struct SimpleView {
    id: i32,
    name: String,
}

#[PostgresView(
    NAME = "simple_view_mat",
    DEFINITION = {
        let builder = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>();
        let SimpleSchema { simple } = SimpleSchema::new();
        builder.select((simple.id, simple.name)).from(simple)
    },
    MATERIALIZED,
    WITH_NO_DATA
)]
struct SimpleViewMat {
    id: i32,
    name: String,
}

#[PostgresView(DEFINITION = "SELECT id FROM simple")]
struct DefaultNameView {
    id: i32,
}

#[PostgresTable]
struct PgCasingDefault {
    #[column(PRIMARY)]
    id: i32,
    created_at: String,
}

#[PostgresTable(NAME = "externalUsers")]
struct PgCasingExplicit {
    #[column(PRIMARY, name = "userId")]
    id: i32,
    #[column(name = "displayName")]
    display_name: String,
}

#[PostgresTable(NAME = "macro_snapshot_ddl")]
struct PgMacroSnapshotDdl {
    #[column(PRIMARY, identity(by_default))]
    id: i32,
    #[column(COLLATE = C)]
    name: String,
    #[column(generated(stored, "length(name)"))]
    name_len: i32,
    #[column(CHECK = "score >= 0")]
    score: i32,
    #[column(enum)]
    role: Role,
}

#[derive(PostgresSchema)]
struct PgMacroSnapshotSchema {
    table: PgMacroSnapshotDdl,
}

/// Macro-generated table comment.
#[PostgresTable(NAME = "macro_array_comments")]
struct PgMacroArrayComments {
    #[column(PRIMARY)]
    id: i32,
    /// Integer array comment.
    numbers: Vec<i32>,
    /// Text array comment.
    tags: Vec<String>,
}

#[derive(PostgresSchema)]
struct PgMacroArrayCommentSchema {
    table: PgMacroArrayComments,
}

#[derive(Debug, PostgresFromRow)]
struct PgMacroArrayCommentResult {
    id: i32,
    numbers: Vec<i32>,
    tags: Vec<String>,
}

#[PostgresTable(
    NAME = "macro_storage_attrs",
    UNLOGGED,
    INHERITS = "macro_parent",
    TABLESPACE = "fast_storage"
)]
struct PgMacroStorageAttrs {
    #[column(PRIMARY)]
    id: i32,
}

#[PostgresTable(NAME = "macro_constraints_parent")]
struct PgMacroConstraintParent {
    #[column(PRIMARY)]
    id: i32,
    tenant_id: i32,
}

#[PostgresTable(
    NAME = "macro_constraints_child",
    RLS,
    UNIQUE(
        columns(tenant_id, slug),
        name = "macro_child_tenant_slug_key",
        deferrable,
        initially_deferred
    ),
    CHECK(name = "macro_child_score_check", expr = "score >= 0"),
    FOREIGN_KEY(
        columns(tenant_id, parent_id),
        references(PgMacroConstraintParent, tenant_id, id),
        deferrable,
        initially_deferred
    )
)]
struct PgMacroConstraintChild {
    #[column(PRIMARY)]
    id: i32,
    tenant_id: i32,
    parent_id: i32,
    #[column(REFERENCES = PgMacroConstraintParent::id, DEFERRABLE, INITIALLY_DEFERRED)]
    parent_ref: i32,
    slug: String,
    #[column(default_sql = "'draft'")]
    status: String,
    score: i32,
}

#[PostgresPolicy(
    NAME = "macro_child_select_policy",
    AS = "RESTRICTIVE",
    FOR = "SELECT",
    TO("public", "app_user"),
    USING = "tenant_id > 0",
    WITH_CHECK = "score >= 0"
)]
struct PgMacroChildSelectPolicy(PgMacroConstraintChild);

#[derive(PostgresSchema)]
struct PgMacroFeatureSchema {
    parent: PgMacroConstraintParent,
    child: PgMacroConstraintChild,
    storage: PgMacroStorageAttrs,
    policy: PgMacroChildSelectPolicy,
}

#[PostgresTable(
    NAME = "macro_exec_parent",
    UNIQUE(columns(tenant_id, code), name = "macro_exec_parent_tenant_code_key")
)]
struct PgMacroExecParent {
    #[column(PRIMARY)]
    id: i32,
    tenant_id: i32,
    code: String,
}

#[PostgresTable(
    NAME = "macro_exec_child",
    RLS,
    UNIQUE(
        columns(tenant_id, slug),
        name = "macro_exec_child_tenant_slug_key",
        deferrable
    ),
    CHECK(name = "macro_exec_child_score_check", expr = "score >= 0"),
    FOREIGN_KEY(
        columns(tenant_id, parent_code),
        references(PgMacroExecParent, tenant_id, code),
        deferrable,
        initially_deferred
    )
)]
struct PgMacroExecChild {
    #[column(PRIMARY)]
    id: i32,
    tenant_id: i32,
    parent_code: String,
    #[column(REFERENCES = PgMacroExecParent::id, DEFERRABLE)]
    parent_id: i32,
    slug: String,
    #[column(default_sql = "'draft'")]
    status: String,
    score: i32,
}

#[PostgresPolicy(
    NAME = "macro_exec_select_policy",
    FOR = "SELECT",
    TO("public"),
    USING = "tenant_id > 0"
)]
struct PgMacroExecSelectPolicy(PgMacroExecChild);

#[derive(PostgresSchema)]
struct PgMacroExecutableFeatureSchema {
    parent: PgMacroExecParent,
    child: PgMacroExecChild,
    policy: PgMacroExecSelectPolicy,
}

#[test]
fn postgres_macro_snapshot_carries_column_ddl_metadata() {
    let snapshot = PgMacroSnapshotSchema::new().to_snapshot();
    let drizzle::migrations::Snapshot::Postgres(snapshot) = snapshot else {
        panic!("expected postgres snapshot");
    };

    let columns = snapshot
        .ddl
        .iter()
        .filter_map(|entity| match entity {
            drizzle::migrations::postgres::PostgresEntity::Column(column) => Some(column),
            _ => None,
        })
        .collect::<Vec<_>>();

    let id = columns
        .iter()
        .find(|column| column.name == "id")
        .expect("id column");
    assert!(id.identity.is_some(), "identity metadata missing");

    let name = columns
        .iter()
        .find(|column| column.name == "name")
        .expect("name column");
    assert_eq!(name.collate.as_deref(), Some("C"));

    let name_len = columns
        .iter()
        .find(|column| column.name == "name_len")
        .expect("name_len column");
    assert!(name_len.generated.is_some(), "generated metadata missing");

    let role = columns
        .iter()
        .find(|column| column.name == "role")
        .expect("role column");
    assert_eq!(role.type_schema.as_deref(), Some("public"));

    assert!(
        snapshot.ddl.iter().any(|entity| matches!(
            entity,
            drizzle::migrations::postgres::PostgresEntity::CheckConstraint(check)
                if check.name == "macro_snapshot_ddl_score_check" && check.value == "score >= 0"
        )),
        "check constraint metadata missing"
    );
}

#[test]
fn postgres_macro_arrays_and_comments_reach_ddl_and_snapshot() {
    let table_sql = PgMacroArrayComments::create_table_sql();
    assert!(table_sql.contains("\"numbers\" INTEGER[] NOT NULL"));
    assert!(table_sql.contains("\"tags\" TEXT[] NOT NULL"));

    let statements = PgMacroArrayCommentSchema::new()
        .create_statements()
        .expect("create statements")
        .collect::<Vec<_>>();
    assert!(statements.iter().any(|sql| {
        sql == "COMMENT ON TABLE \"macro_array_comments\" IS 'Macro-generated table comment.';"
    }));
    assert!(statements.iter().any(|sql| {
        sql == "COMMENT ON COLUMN \"macro_array_comments\".\"numbers\" IS 'Integer array comment.';"
    }));
    assert!(statements.iter().any(|sql| {
        sql == "COMMENT ON COLUMN \"macro_array_comments\".\"tags\" IS 'Text array comment.';"
    }));

    let drizzle::migrations::Snapshot::Postgres(snapshot) =
        PgMacroArrayCommentSchema::new().to_snapshot()
    else {
        panic!("expected postgres snapshot");
    };

    assert!(snapshot.ddl.iter().any(|entity| matches!(
        entity,
        drizzle::migrations::postgres::PostgresEntity::Table(table)
            if table.name == "macro_array_comments"
                && table.comment.as_deref() == Some("Macro-generated table comment.")
    )));
    assert!(snapshot.ddl.iter().any(|entity| matches!(
        entity,
        drizzle::migrations::postgres::PostgresEntity::Column(column)
            if column.table == "macro_array_comments"
                && column.name == "numbers"
                && column.sql_type == "INTEGER"
                && column.dimensions == Some(1)
                && column.comment.as_deref() == Some("Integer array comment.")
    )));
}

#[drizzle::test]
fn postgres_macro_array_columns_roundtrip(db: &mut TestDb<PgMacroArrayCommentSchema>) {
    let PgMacroArrayCommentSchema { table } = schema;

    db.insert(table)
        .values([InsertPgMacroArrayComments::new(
            1,
            vec![1, 2, 3],
            vec!["alpha".to_string(), "beta".to_string()],
        )])
        .execute();

    let rows: Vec<PgMacroArrayCommentResult> = db.select(()).from(table).all();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].id, 1);
    assert_eq!(rows[0].numbers, vec![1, 2, 3]);
    assert_eq!(rows[0].tags, vec!["alpha".to_string(), "beta".to_string()]);
}

#[test]
fn postgres_macro_create_table_sql_carries_table_storage_and_constraints() {
    let storage_sql = PgMacroStorageAttrs::create_table_sql();
    assert!(storage_sql.starts_with("CREATE UNLOGGED TABLE \"macro_storage_attrs\""));
    assert!(storage_sql.contains("INHERITS (\"macro_parent\")"));
    assert!(storage_sql.contains("TABLESPACE \"fast_storage\""));

    let child_sql = PgMacroConstraintChild::create_table_sql();
    assert!(child_sql.contains(
        "CONSTRAINT \"macro_child_tenant_slug_key\" UNIQUE(\"tenant_id\", \"slug\") DEFERRABLE INITIALLY DEFERRED"
    ));
    assert!(child_sql.contains("CONSTRAINT \"macro_child_score_check\" CHECK (score >= 0)"));
    assert!(child_sql.contains("DEFAULT 'draft'"));
    assert!(child_sql.contains("FOREIGN KEY (\"parent_ref\") REFERENCES \"macro_constraints_parent\"(\"id\") DEFERRABLE INITIALLY DEFERRED"));
    assert!(child_sql.contains("FOREIGN KEY (\"tenant_id\", \"parent_id\") REFERENCES \"macro_constraints_parent\"(\"tenant_id\", \"id\") DEFERRABLE INITIALLY DEFERRED"));
}

#[test]
fn postgres_macro_snapshot_carries_table_constraints_rls_policy_and_defaults() {
    let snapshot = PgMacroFeatureSchema::new().to_snapshot();
    let drizzle::migrations::Snapshot::Postgres(snapshot) = snapshot else {
        panic!("expected postgres snapshot");
    };

    let storage = snapshot
        .ddl
        .iter()
        .find_map(|entity| match entity {
            drizzle::migrations::postgres::PostgresEntity::Table(table)
                if table.name == "macro_storage_attrs" =>
            {
                Some(table)
            }
            _ => None,
        })
        .expect("storage table");
    assert_eq!(storage.is_unlogged, Some(true));
    assert_eq!(storage.inherits.as_deref(), Some("macro_parent"));
    assert_eq!(storage.tablespace.as_deref(), Some("fast_storage"));

    let child = snapshot
        .ddl
        .iter()
        .find_map(|entity| match entity {
            drizzle::migrations::postgres::PostgresEntity::Table(table)
                if table.name == "macro_constraints_child" =>
            {
                Some(table)
            }
            _ => None,
        })
        .expect("child table");
    assert_eq!(child.is_rls_enabled, Some(true));

    assert!(snapshot.ddl.iter().any(|entity| matches!(
        entity,
        drizzle::migrations::postgres::PostgresEntity::Column(column)
            if column.table == "macro_constraints_child"
                && column.name == "status"
                && column.default.as_deref() == Some("'draft'")
    )));
    assert!(snapshot.ddl.iter().any(|entity| matches!(
        entity,
        drizzle::migrations::postgres::PostgresEntity::UniqueConstraint(unique)
            if unique.name == "macro_child_tenant_slug_key"
                && unique.deferrable
                && unique.initially_deferred
    )));
    assert!(snapshot.ddl.iter().any(|entity| matches!(
        entity,
        drizzle::migrations::postgres::PostgresEntity::CheckConstraint(check)
            if check.name == "macro_child_score_check" && check.value == "score >= 0"
    )));
    assert!(snapshot.ddl.iter().any(|entity| matches!(
        entity,
        drizzle::migrations::postgres::PostgresEntity::ForeignKey(fk)
            if fk.name == "macro_constraints_child_parent_ref_fkey"
                && fk.deferrable
                && fk.initially_deferred
    )));
    assert!(snapshot.ddl.iter().any(|entity| matches!(
        entity,
        drizzle::migrations::postgres::PostgresEntity::Policy(policy)
            if policy.name == "macro_child_select_policy"
                && policy.as_clause.as_deref() == Some("RESTRICTIVE")
                && policy.for_clause.as_deref() == Some("SELECT")
                && policy.using.as_deref() == Some("tenant_id > 0")
                && policy.with_check.as_deref() == Some("score >= 0")
    )));
}

#[test]
fn postgres_macro_create_statements_emit_rls_and_policy() {
    let statements = PgMacroFeatureSchema::new()
        .create_statements()
        .expect("create statements")
        .collect::<Vec<_>>();

    assert!(statements.iter().any(|sql| {
        sql == "ALTER TABLE \"macro_constraints_child\" ENABLE ROW LEVEL SECURITY;"
    }));
    assert!(statements.iter().any(|sql| {
        sql.contains("CREATE POLICY \"macro_child_select_policy\"")
            && sql.contains(" AS RESTRICTIVE FOR SELECT TO PUBLIC, \"app_user\"")
            && sql.contains("USING (tenant_id > 0)")
            && sql.contains("WITH CHECK (score >= 0)")
    }));
}

#[drizzle::test]
fn postgres_macro_feature_ddl_executes(db: &mut TestDb<PgMacroExecutableFeatureSchema>) {
    let _ = db;
    assert!(
        schema
            .create_statements()
            .expect("create statements")
            .any(|sql| sql.contains("CREATE POLICY \"macro_exec_select_policy\""))
    );
}

#[PostgresView(EXISTING, NAME = "existing_simple_view")]
struct ExistingSimpleView {
    id: i32,
}

#[PostgresView(
    NAME = "simple_view_opts",
    DEFINITION = "SELECT id, name FROM simple",
    MATERIALIZED,
    WITH = ViewWithOptionDef::new().security_barrier(),
    WITH_NO_DATA,
    USING = "heap",
    TABLESPACE = "fast_storage"
)]
struct SimpleViewWithOptions {
    id: i32,
    name: String,
}

#[derive(PostgresSchema)]
struct ViewTestSchema {
    simple: Simple,
    simple_view: SimpleView,
    simple_view_mat: SimpleViewMat,
    default_name_view: DefaultNameView,
    existing_simple_view: ExistingSimpleView,
}

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
}

#[drizzle::test]
fn schema_simple_works(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("test")]);
    stmt.execute();

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
}

#[drizzle::test]
fn schema_with_view(db: &mut TestDb<ViewTestSchema>) {
    let ViewTestSchema {
        simple,
        simple_view,
        simple_view_mat: _,
        default_name_view,
        existing_simple_view: _,
    } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("alpha"), InsertSimple::new("beta")]);
    stmt.execute();

    let stmt = db
        .select(PgSimpleViewResult::Select)
        .from(simple_view)
        .order_by([asc(simple_view.id)]);
    let results: Vec<PgSimpleViewResult> = stmt.all();

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].id, 1);
    assert_eq!(results[0].name, "alpha");
    assert_eq!(results[1].id, 2);

    assert_eq!(DefaultNameView::VIEW_NAME, "default_name_view");
    assert_eq!(default_name_view.name(), "default_name_view");

    let statements: Vec<_> = schema
        .create_statements()
        .expect("create statements")
        .collect();
    assert!(
        statements.iter().any(|sql| sql.contains("CREATE VIEW")),
        "Expected CREATE VIEW statement"
    );
    assert!(
        statements
            .iter()
            .any(|sql| sql.contains("CREATE MATERIALIZED VIEW")),
        "Expected CREATE MATERIALIZED VIEW statement"
    );
    assert!(
        statements
            .iter()
            .any(|sql| sql.contains("default_name_view")),
        "Expected default name view statement"
    );
    assert!(
        !statements
            .iter()
            .any(|sql| sql.contains("existing_simple_view")),
        "Existing view should not be created"
    );
}

#[drizzle::test]
fn view_alias_in_from_clause(db: &mut TestDb<ViewTestSchema>) {
    let ViewTestSchema {
        simple,
        simple_view,
        simple_view_mat: _,
        default_name_view: _,
        existing_simple_view: _,
    } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("alpha"), InsertSimple::new("beta")]);
    stmt.execute();

    struct SvTag;
    impl drizzle::core::Tag for SvTag {
        const NAME: &'static str = "sv";
    }

    let sv = SimpleView::alias::<SvTag>();
    let stmt = db
        .select((sv.id, sv.name))
        .from(sv)
        .r#where(eq(sv.name, "alpha"))
        .order_by([asc(sv.id)]);

    let sql = stmt.to_sql().sql();
    assert!(sql.contains("FROM \"simple_view\" AS \"sv\""));
    assert!(sql.contains("\"sv\".\"name\""));

    let sv2 = SimpleView::alias::<SvTag>();
    let typed_stmt = db
        .select(PgSimpleAliasResult::Select)
        .from(sv2)
        .r#where(eq(sv2.name, "alpha"))
        .order_by([asc(sv2.id)]);

    let results: Vec<PgSimpleAliasResult> = typed_stmt.all();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, 1);
    assert_eq!(results[0].name, "alpha");

    // Keep schema value used in this test scope.
    let _ = simple_view;
}

struct SimpleAliasTag;
impl drizzle::core::Tag for SimpleAliasTag {
    const NAME: &'static str = "s_alias";
}

#[drizzle::test]
fn tagged_alias_forwards_alias_metadata(db: &mut TestDb<SimpleSchema>) {
    let tagged = Simple::alias::<SimpleAliasTag>();
    let _base = Simple::new();

    assert_eq!(tagged.name(), "s_alias");
}

#[test]
fn postgres_casing_is_schema_definition_scoped() {
    let default = PgCasingDefault::new();
    assert_eq!(default.name(), "pg_casing_default");
    assert_eq!(default.created_at.name(), "created_at");

    let explicit = PgCasingExplicit::new();
    assert_eq!(explicit.name(), "externalUsers");
    assert_eq!(explicit.id.name(), "userId");
    assert_eq!(explicit.display_name.name(), "displayName");

    let sql = drizzle::postgres::builder::QueryBuilder::new::<SimpleSchema>()
        .select((default.id, default.created_at))
        .from(default)
        .to_sql()
        .sql();
    assert!(sql.contains(r#""pg_casing_default"."created_at""#));
}

#[drizzle::test]
fn view_definition_with_options_sql(db: &mut TestDb<SimpleSchema>) {
    let sql = SimpleViewWithOptions::create_view_sql();
    assert!(sql.contains("CREATE MATERIALIZED VIEW"));
    assert!(sql.contains("WITH ("));
    assert!(sql.contains("security_barrier"));
    assert!(sql.contains("WITH NO DATA"));
    assert!(sql.contains("USING"));
    assert!(sql.contains("heap"));
    assert!(sql.contains("TABLESPACE"));
    assert!(sql.contains("fast_storage"));
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn schema_complex_works(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("test", true, Role::User)]);
    stmt.execute();

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_ne!(results[0].id, uuid::Uuid::nil());
    assert_eq!(results[0].name, "test");
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn schema_with_enum_works(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex, .. } = schema;

    // Insert with different enum values to verify enum type was created
    let stmt = db.insert(complex).values([
        InsertComplex::new("Admin User", true, Role::Admin),
        InsertComplex::new("Regular User", true, Role::User),
        InsertComplex::new("Mod User", true, Role::Moderator),
    ]);
    stmt.execute();

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = stmt.all();

    assert_eq!(results.len(), 3);
}

#[drizzle::test]
fn schema_multiple_inserts(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Multiple separate inserts should work
    let stmt = db.insert(simple).values([InsertSimple::new("First")]);
    stmt.execute();

    let stmt = db.insert(simple).values([InsertSimple::new("Second")]);
    stmt.execute();

    let stmt = db.insert(simple).values([InsertSimple::new("Third")]);
    stmt.execute();

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 3);
}

#[PostgresTable(NAME = "cycle_a")]
struct PgCycleA {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = PgCycleB::id)]
    b_id: i32,
}

#[PostgresTable(NAME = "cycle_b")]
struct PgCycleB {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = PgCycleC::id)]
    c_id: i32,
}

#[PostgresTable(NAME = "cycle_c")]
struct PgCycleC {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = PgCycleA::id)]
    a_id: i32,
}

#[derive(PostgresSchema)]
struct PgCycleSchema {
    a: PgCycleA,
    b: PgCycleB,
    c: PgCycleC,
}

#[test]
fn postgres_cycle_reports_structured_error() {
    let schema = PgCycleSchema::new();
    let err = match schema.create_statements() {
        Ok(_) => panic!("expected cycle detection error"),
        Err(err) => err,
    };
    assert!(
        err.to_string()
            .contains("Cyclic table dependency detected in PostgresSchema"),
        "unexpected error: {err}"
    );
}

#[PostgresTable(NAME = "dup_table")]
struct PgDuplicateTableOne {
    #[column(PRIMARY)]
    id: i32,
}

#[PostgresTable(NAME = "dup_table")]
struct PgDuplicateTableTwo {
    #[column(PRIMARY)]
    id: i32,
}

#[derive(PostgresSchema)]
struct PgDuplicateTableSchema {
    first: PgDuplicateTableOne,
    second: PgDuplicateTableTwo,
}

#[test]
fn postgres_duplicate_table_reports_error() {
    let schema = PgDuplicateTableSchema::new();
    let err = match schema.create_statements() {
        Ok(_) => panic!("expected duplicate table error"),
        Err(err) => err,
    };
    assert!(
        err.to_string()
            .contains("Duplicate table names detected in PostgresSchema"),
        "unexpected error: {err}"
    );
}

#[PostgresTable(NAME = "dup_idx_table")]
struct PgDuplicateIndexTable {
    #[column(PRIMARY)]
    id: i32,
    email: String,
}

#[PostgresIndex]
struct PgDuplicateIndex(PgDuplicateIndexTable::email);

#[derive(PostgresSchema)]
struct PgDuplicateIndexSchema {
    table: PgDuplicateIndexTable,
    idx1: PgDuplicateIndex,
    idx2: PgDuplicateIndex,
}

#[test]
fn postgres_duplicate_index_reports_error() {
    let schema = PgDuplicateIndexSchema::new();
    let err = match schema.create_statements() {
        Ok(_) => panic!("expected duplicate index error"),
        Err(err) => err,
    };
    assert!(
        err.to_string().contains(
            "Duplicate index 'pg_duplicate_index' on table 'public.dup_idx_table' in PostgresSchema"
        ),
        "unexpected error: {err}"
    );
}

// =============================================================================
// View query DSL tests (PostgreSQL)
// =============================================================================

#[cfg(feature = "uuid")]
mod view_query {
    use super::*;
    use crate::common::schema::postgres::{Complex, Post, Role};
    use uuid::Uuid;

    #[PostgresView(
        query(select(Complex::id, Complex::name), from(Complex)),
        NAME = "vq_pg_simple_view"
    )]
    struct VqPgSimpleView {
        id: Uuid,
        name: String,
    }

    #[PostgresView(
        query(
            select(Complex::id, Complex::name, Complex::email),
            from(Complex),
            filter(eq(Complex::active, true)),
        ),
        NAME = "vq_pg_active_users"
    )]
    struct VqPgActiveUsersView {
        id: Uuid,
        name: String,
        email: Option<String>,
    }

    #[PostgresView(
        query(
            select(Complex::id, Complex::name, Post::title),
            from(Complex),
            left_join(Post, eq(Complex::id, Post::author_id)),
        ),
        NAME = "vq_pg_user_posts"
    )]
    struct VqPgUserPostsView {
        id: Uuid,
        name: String,
        title: Option<String>,
    }

    #[PostgresView(
        query(
            select(Complex::name, count(Post::id)),
            from(Complex),
            left_join(Post, eq(Complex::id, Post::author_id)),
            group_by(Complex::name),
        ),
        NAME = "vq_pg_post_counts"
    )]
    struct VqPgPostCountsView {
        name: String,
        post_count: i32,
    }

    #[PostgresView(
        query(
            select(Complex::id, Complex::name),
            from(Complex),
            order_by(asc(Complex::name)),
            limit(10),
            offset(5),
        ),
        NAME = "vq_pg_ordered_users"
    )]
    struct VqPgOrderedUsersView {
        id: Uuid,
        name: String,
    }

    #[PostgresView(
        query(
            select(Complex::id, Complex::name, Complex::email),
            from(Complex),
            filter(eq(Complex::active, true)),
        ),
        NAME = "vq_pg_mat_view",
        MATERIALIZED,
        WITH_NO_DATA
    )]
    struct VqPgMatView {
        id: Uuid,
        name: String,
        email: Option<String>,
    }

    #[PostgresView(
        query(
            select(Complex::id, Complex::name),
            from(Complex),
            filter(and(
                eq(Complex::active, true),
                or(gt(Complex::age, 0), is_null(Complex::email)),
            )),
        ),
        NAME = "vq_pg_complex_filter"
    )]
    struct VqPgComplexFilterView {
        id: Uuid,
        name: String,
    }

    #[PostgresView(
        query(
            select(Complex::name, count(Post::id)),
            from(Complex),
            left_join(Post, eq(Complex::id, Post::author_id)),
            group_by(Complex::name),
            having(gt(count(Post::id), 0)),
        ),
        NAME = "vq_pg_having_view"
    )]
    struct VqPgHavingView {
        name: String,
        post_count: i32,
    }

    #[derive(PostgresSchema)]
    struct VqPgTestSchema {
        role: Role,
        complex: Complex,
        post: Post,
        vq_pg_simple_view: VqPgSimpleView,
        vq_pg_active_users: VqPgActiveUsersView,
        vq_pg_user_posts: VqPgUserPostsView,
        vq_pg_post_counts: VqPgPostCountsView,
        vq_pg_ordered_users: VqPgOrderedUsersView,
        vq_pg_mat_view: VqPgMatView,
        vq_pg_complex_filter: VqPgComplexFilterView,
        vq_pg_having_view: VqPgHavingView,
    }

    #[test]
    fn pg_view_query_simple_const_sql() {
        assert_eq!(
            VqPgSimpleView::VIEW_DEFINITION_SQL,
            r#"SELECT "public"."complex"."id" AS "id", "public"."complex"."name" AS "name" FROM "public"."complex""#
        );
        assert_eq!(
            VqPgSimpleView::ddl_sql(),
            r#"CREATE VIEW "vq_pg_simple_view" AS SELECT "public"."complex"."id" AS "id", "public"."complex"."name" AS "name" FROM "public"."complex""#
        );
    }

    #[test]
    fn pg_view_query_filter_const_sql() {
        assert_eq!(
            VqPgActiveUsersView::VIEW_DEFINITION_SQL,
            r#"SELECT "public"."complex"."id" AS "id", "public"."complex"."name" AS "name", "public"."complex"."email" AS "email" FROM "public"."complex" WHERE "public"."complex"."active" = TRUE"#
        );
    }

    #[test]
    fn pg_view_query_join_const_sql() {
        assert_eq!(
            VqPgUserPostsView::VIEW_DEFINITION_SQL,
            r#"SELECT "public"."complex"."id" AS "id", "public"."complex"."name" AS "name", "public"."post"."title" AS "title" FROM "public"."complex" LEFT JOIN "public"."post" ON "public"."complex"."id" = "public"."post"."author_id""#
        );
    }

    #[test]
    fn pg_view_query_aggregate_const_sql() {
        assert_eq!(
            VqPgPostCountsView::VIEW_DEFINITION_SQL,
            r#"SELECT "public"."complex"."name" AS "name", COUNT("public"."post"."id") AS "post_count" FROM "public"."complex" LEFT JOIN "public"."post" ON "public"."complex"."id" = "public"."post"."author_id" GROUP BY "public"."complex"."name""#
        );
    }

    #[test]
    fn pg_view_query_order_limit_offset_const_sql() {
        assert_eq!(
            VqPgOrderedUsersView::VIEW_DEFINITION_SQL,
            r#"SELECT "public"."complex"."id" AS "id", "public"."complex"."name" AS "name" FROM "public"."complex" ORDER BY "public"."complex"."name" ASC LIMIT 10 OFFSET 5"#
        );
    }

    #[test]
    fn pg_view_query_materialized_const_sql() {
        assert_eq!(
            VqPgMatView::ddl_sql(),
            r#"CREATE MATERIALIZED VIEW "vq_pg_mat_view" AS SELECT "public"."complex"."id" AS "id", "public"."complex"."name" AS "name", "public"."complex"."email" AS "email" FROM "public"."complex" WHERE "public"."complex"."active" = TRUE WITH NO DATA"#
        );
    }

    #[test]
    fn pg_view_query_complex_filter_const_sql() {
        assert_eq!(
            VqPgComplexFilterView::VIEW_DEFINITION_SQL,
            r#"SELECT "public"."complex"."id" AS "id", "public"."complex"."name" AS "name" FROM "public"."complex" WHERE ("public"."complex"."active" = TRUE AND ("public"."complex"."age" > 0 OR "public"."complex"."email" IS NULL))"#
        );
    }

    #[test]
    fn pg_view_query_having_const_sql() {
        assert_eq!(
            VqPgHavingView::VIEW_DEFINITION_SQL,
            r#"SELECT "public"."complex"."name" AS "name", COUNT("public"."post"."id") AS "post_count" FROM "public"."complex" LEFT JOIN "public"."post" ON "public"."complex"."id" = "public"."post"."author_id" GROUP BY "public"."complex"."name" HAVING COUNT("public"."post"."id") > 0"#
        );
    }
}
