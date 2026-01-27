//! PostgreSQL schema tests
//!
//! Schema creation is implicitly tested by all other tests via db.create().
//! These tests verify various schema configurations work correctly.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::ddl::postgres::ddl::ViewWithOptionDef;
use drizzle::postgres::prelude::*;
use drizzle_core::OrderBy;
use drizzle_macros::postgres_test;

#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
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

#[cfg(feature = "uuid")]
#[derive(Debug, PostgresFromRow)]
struct PgComplexResult {
    id: uuid::Uuid,
    name: String,
}

postgres_test!(schema_simple_works, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("test")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

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
    drizzle_exec!(stmt.execute());

    let stmt = db
        .select((simple_view.id, simple_view.name))
        .from(simple_view)
        .order_by([OrderBy::asc(simple_view.id)]);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "alpha");

    assert_eq!(DefaultNameView::VIEW_NAME, "default_name_view");
    assert_eq!(default_name_view.name(), "default_name_view");

    let statements = schema.create_statements();
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
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

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
    drizzle_exec!(stmt.execute());

    let stmt = db.select(()).from(complex);
    let results: Vec<PgComplexResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 3);
});

postgres_test!(schema_multiple_inserts, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Multiple separate inserts should work
    let stmt = db.insert(simple).values([InsertSimple::new("First")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.insert(simple).values([InsertSimple::new("Second")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.insert(simple).values([InsertSimple::new("Third")]);
    drizzle_exec!(stmt.execute());

    let stmt = db.select((simple.id, simple.name)).from(simple);
    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt.all());

    assert_eq!(results.len(), 3);
});
