//! PostgreSQL schema tests
//!
//! Schema creation is implicitly tested by all other tests via db.create().
//! These tests verify various schema configurations work correctly.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::eq;
use drizzle::ddl::postgres::ddl::ViewWithOptionDef;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

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

#[allow(dead_code)]
#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
}

postgres_test!(schema_simple_works, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("test")]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
});

postgres_test!(schema_with_view, ViewTestSchema, {
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
    drizzle_exec!(stmt => execute);

    let stmt = db
        .select(PgSimpleViewResult::Select)
        .from(simple_view)
        .order_by([asc(simple_view.id)]);
    let results: Vec<PgSimpleViewResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "alpha");

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
});

postgres_test!(view_alias_in_from_clause, ViewTestSchema, {
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
    drizzle_exec!(stmt => execute);

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

    let results: Vec<PgSimpleAliasResult> = drizzle_exec!(typed_stmt => all);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "alpha");

    // Keep schema value used in this test scope.
    let _ = simple_view;
});

struct SimpleAliasTag;
impl drizzle::core::Tag for SimpleAliasTag {
    const NAME: &'static str = "s_alias";
}

postgres_test!(tagged_alias_forwards_alias_metadata, SimpleSchema, {
    let tagged = Simple::alias::<SimpleAliasTag>();
    let base = Simple::new();

    assert_eq!(tagged.name(), "s_alias");
});

postgres_test!(view_definition_with_options_sql, SimpleSchema, {
    let sql = SimpleViewWithOptions::create_view_sql();
    assert!(sql.contains("CREATE MATERIALIZED VIEW"));
    assert!(sql.contains("WITH ("));
    assert!(sql.contains("security_barrier"));
    assert!(sql.contains("WITH NO DATA"));
    assert!(sql.contains("USING"));
    assert!(sql.contains("heap"));
    assert!(sql.contains("TABLESPACE"));
    assert!(sql.contains("fast_storage"));
});

#[cfg(feature = "uuid")]
postgres_test!(schema_complex_works, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("test", true, Role::User)]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test");
});

#[cfg(feature = "uuid")]
postgres_test!(schema_with_enum_works, ComplexSchema, {
    let ComplexSchema { complex, .. } = schema;

    // Insert with different enum values to verify enum type was created
    let stmt = db.insert(complex).values([
        InsertComplex::new("Admin User", true, Role::Admin),
        InsertComplex::new("Regular User", true, Role::User),
        InsertComplex::new("Mod User", true, Role::Moderator),
    ]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 3);
});

postgres_test!(schema_multiple_inserts, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Multiple separate inserts should work
    let stmt = db.insert(simple).values([InsertSimple::new("First")]);
    drizzle_exec!(stmt => execute);

    let stmt = db.insert(simple).values([InsertSimple::new("Second")]);
    drizzle_exec!(stmt => execute);

    let stmt = db.insert(simple).values([InsertSimple::new("Third")]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<SelectSimple> = drizzle_exec!(stmt => all);

    assert_eq!(results.len(), 3);
});

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

#[PostgresTable(NAME = "vq_pg_users")]
struct VqPgUser {
    #[column(serial, PRIMARY)]
    id: i32,
    name: String,
    email: String,
    active: bool,
}

#[PostgresTable(NAME = "vq_pg_posts")]
struct VqPgPost {
    #[column(serial, PRIMARY)]
    id: i32,
    title: String,
    #[column(REFERENCES = VqPgUser::id)]
    author_id: i32,
}

// Simple view with query DSL
#[PostgresView(
    query(select(VqPgUser::id, VqPgUser::name), from(VqPgUser),),
    NAME = "vq_pg_simple_view"
)]
struct VqPgSimpleView {
    id: i32,
    name: String,
}

// Filtered view
#[PostgresView(
    query(
        select(VqPgUser::id, VqPgUser::name, VqPgUser::email),
        from(VqPgUser),
        filter(eq(VqPgUser::active, true)),
    ),
    NAME = "vq_pg_active_users"
)]
struct VqPgActiveUsersView {
    id: i32,
    name: String,
    email: String,
}

// Join view
#[PostgresView(
    query(
        select(VqPgUser::id, VqPgUser::name, VqPgPost::title),
        from(VqPgUser),
        left_join(VqPgPost, eq(VqPgUser::id, VqPgPost::author_id)),
    ),
    NAME = "vq_pg_user_posts"
)]
struct VqPgUserPostsView {
    id: i32,
    name: String,
    title: String,
}

// Aggregate view with GROUP BY
#[PostgresView(
    query(
        select(VqPgUser::name, count(VqPgPost::id)),
        from(VqPgUser),
        left_join(VqPgPost, eq(VqPgUser::id, VqPgPost::author_id)),
        group_by(VqPgUser::name),
    ),
    NAME = "vq_pg_post_counts"
)]
struct VqPgPostCountsView {
    name: String,
    post_count: i32,
}

// Order + limit + offset
#[PostgresView(
    query(
        select(VqPgUser::id, VqPgUser::name),
        from(VqPgUser),
        order_by(asc(VqPgUser::name)),
        limit(10),
        offset(5),
    ),
    NAME = "vq_pg_ordered_users"
)]
struct VqPgOrderedUsersView {
    id: i32,
    name: String,
}

// Materialized view with query DSL
#[PostgresView(
    query(
        select(VqPgUser::id, VqPgUser::name, VqPgUser::email),
        from(VqPgUser),
        filter(eq(VqPgUser::active, true)),
    ),
    NAME = "vq_pg_mat_view",
    MATERIALIZED,
    WITH_NO_DATA
)]
struct VqPgMatView {
    id: i32,
    name: String,
    email: String,
}

// Complex filter view (AND/OR/IS_NULL)
#[PostgresView(
    query(
        select(VqPgUser::id, VqPgUser::name),
        from(VqPgUser),
        filter(and(
            eq(VqPgUser::active, true),
            or(gt(VqPgUser::id, 0), is_null(VqPgUser::email)),
        )),
    ),
    NAME = "vq_pg_complex_filter"
)]
struct VqPgComplexFilterView {
    id: i32,
    name: String,
}

// Having clause
#[PostgresView(
    query(
        select(VqPgUser::name, count(VqPgPost::id)),
        from(VqPgUser),
        left_join(VqPgPost, eq(VqPgUser::id, VqPgPost::author_id)),
        group_by(VqPgUser::name),
        having(gt(count(VqPgPost::id), 0)),
    ),
    NAME = "vq_pg_having_view"
)]
struct VqPgHavingView {
    name: String,
    post_count: i32,
}

#[derive(PostgresSchema)]
struct VqPgTestSchema {
    vq_pg_user: VqPgUser,
    vq_pg_post: VqPgPost,
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
    let sql = VqPgSimpleView::VIEW_DEFINITION_SQL;
    assert!(
        sql.contains("SELECT") && sql.contains("AS \"id\"") && sql.contains("AS \"name\""),
        "Expected SELECT with AS aliases, got: {sql}"
    );
    assert!(
        sql.contains("FROM \"public\".\"vq_pg_users\""),
        "Expected schema-qualified FROM clause, got: {sql}"
    );

    let ddl = VqPgSimpleView::ddl_sql();
    assert!(
        ddl.contains("CREATE VIEW \"vq_pg_simple_view\" AS SELECT"),
        "Expected CREATE VIEW DDL, got: {ddl}"
    );
}

#[test]
fn pg_view_query_filter_const_sql() {
    let sql = VqPgActiveUsersView::VIEW_DEFINITION_SQL;
    assert!(sql.contains("WHERE"), "Expected WHERE clause, got: {sql}");
    // PG uses TRUE/FALSE
    assert!(
        sql.contains("= TRUE"),
        "Expected = TRUE filter (PG dialect), got: {sql}"
    );
}

#[test]
fn pg_view_query_join_const_sql() {
    let sql = VqPgUserPostsView::VIEW_DEFINITION_SQL;
    assert!(sql.contains("LEFT JOIN"), "Expected LEFT JOIN, got: {sql}");
    assert!(
        sql.contains("\"public\".\"vq_pg_posts\""),
        "Expected schema-qualified join table, got: {sql}"
    );
}

#[test]
fn pg_view_query_aggregate_const_sql() {
    let sql = VqPgPostCountsView::VIEW_DEFINITION_SQL;
    assert!(
        sql.contains("COUNT("),
        "Expected COUNT aggregate, got: {sql}"
    );
    assert!(sql.contains("GROUP BY"), "Expected GROUP BY, got: {sql}");
}

#[test]
fn pg_view_query_order_limit_offset_const_sql() {
    let sql = VqPgOrderedUsersView::VIEW_DEFINITION_SQL;
    assert!(
        sql.contains("ORDER BY") && sql.contains("ASC"),
        "Expected ORDER BY ... ASC, got: {sql}"
    );
    assert!(
        sql.contains("LIMIT 10") && sql.contains("OFFSET 5"),
        "Expected LIMIT 10 OFFSET 5, got: {sql}"
    );
}

#[test]
fn pg_view_query_materialized_const_sql() {
    let ddl = VqPgMatView::ddl_sql();
    assert!(
        ddl.contains("CREATE MATERIALIZED VIEW"),
        "Expected CREATE MATERIALIZED VIEW, got: {ddl}"
    );
    assert!(
        ddl.contains("WITH NO DATA"),
        "Expected WITH NO DATA, got: {ddl}"
    );
    assert!(
        ddl.contains("= TRUE"),
        "Expected PG TRUE in filter, got: {ddl}"
    );
}

#[test]
fn pg_view_query_complex_filter_const_sql() {
    let sql = VqPgComplexFilterView::VIEW_DEFINITION_SQL;
    assert!(sql.contains("WHERE"), "Expected WHERE, got: {sql}");
    assert!(sql.contains("AND"), "Expected AND, got: {sql}");
    assert!(sql.contains("OR"), "Expected OR, got: {sql}");
    assert!(sql.contains("IS NULL"), "Expected IS NULL, got: {sql}");
}

#[test]
fn pg_view_query_having_const_sql() {
    let sql = VqPgHavingView::VIEW_DEFINITION_SQL;
    assert!(sql.contains("HAVING"), "Expected HAVING clause, got: {sql}");
    assert!(
        sql.contains("COUNT("),
        "Expected COUNT in HAVING, got: {sql}"
    );
}
