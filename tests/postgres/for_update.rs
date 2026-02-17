//! PostgreSQL FOR UPDATE/SHARE row locking tests
//!
//! Tests for PostgreSQL-specific row locking clauses.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[allow(dead_code)]
#[derive(Debug, PostgresFromRow)]
struct PgSimpleResult {
    id: i32,
    name: String,
}

// Test SQL generation for FOR UPDATE clause
postgres_test!(for_update_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_update();

    let sql = stmt.to_sql().sql();

    assert!(
        sql.contains("FOR UPDATE"),
        "Expected FOR UPDATE in SQL: {}",
        sql
    );
});

// Test SQL generation for FOR SHARE clause
postgres_test!(for_share_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_share();

    let sql = stmt.to_sql().sql();

    assert!(
        sql.contains("FOR") && sql.contains("SHARE"),
        "Expected FOR SHARE in SQL: {}",
        sql
    );
});

// Test SQL generation for FOR NO KEY UPDATE clause
postgres_test!(for_no_key_update_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_no_key_update();

    let sql = stmt.to_sql().sql();

    assert!(
        sql.contains("FOR NO KEY UPDATE"),
        "Expected FOR NO KEY UPDATE in SQL: {}",
        sql
    );
});

// Test SQL generation for FOR KEY SHARE clause
postgres_test!(for_key_share_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_key_share();

    let sql = stmt.to_sql().sql();

    assert!(
        sql.contains("FOR KEY") && sql.contains("SHARE"),
        "Expected FOR KEY SHARE in SQL: {}",
        sql
    );
});

// Test SQL generation for FOR UPDATE with NOWAIT
postgres_test!(for_update_nowait_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_update()
        .nowait();

    let sql = stmt.to_sql().sql();

    assert!(
        sql.contains("FOR UPDATE"),
        "Expected FOR UPDATE in SQL: {}",
        sql
    );
    assert!(sql.contains("NOWAIT"), "Expected NOWAIT in SQL: {}", sql);
});

// Test SQL generation for FOR UPDATE with SKIP LOCKED
postgres_test!(for_update_skip_locked_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_update()
        .skip_locked();

    let sql = stmt.to_sql().sql();

    assert!(
        sql.contains("FOR UPDATE"),
        "Expected FOR UPDATE in SQL: {}",
        sql
    );
    assert!(
        sql.contains("SKIP LOCKED"),
        "Expected SKIP LOCKED in SQL: {}",
        sql
    );
});

// Test SQL generation for FOR UPDATE OF table
postgres_test!(for_update_of_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_update_of(simple);

    let sql = stmt.to_sql().sql();

    assert!(
        sql.contains("FOR UPDATE OF"),
        "Expected FOR UPDATE OF in SQL: {}",
        sql
    );
    // Verify unqualified table name is used (per beta-12 fix)
    assert!(
        sql.contains(r#"OF "simple""#),
        "Expected unqualified table name in SQL: {}",
        sql
    );
});

// Test SQL generation for FOR SHARE OF table
postgres_test!(for_share_of_sql_generation, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_share_of(simple);

    let sql = stmt.to_sql().sql();

    assert!(
        sql.contains("FOR") && sql.contains("SHARE OF"),
        "Expected FOR SHARE OF in SQL: {}",
        sql
    );
    // Verify unqualified table name is used (per beta-12 fix)
    assert!(
        sql.contains(r#"OF "simple""#),
        "Expected unqualified table name in SQL: {}",
        sql
    );
});

// Test FOR UPDATE from different states (FROM, WHERE, ORDER BY)
postgres_test!(for_update_from_different_states, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // From SelectFromSet
    let stmt = db.select(()).from(simple).for_update();
    let sql = stmt.to_sql().sql();
    assert!(sql.contains("FOR UPDATE"));

    // From SelectWhereSet
    let stmt = db
        .select(())
        .from(simple)
        .r#where(eq(simple.id, 1))
        .for_update();
    let sql = stmt.to_sql().sql();
    assert!(sql.contains("FOR UPDATE"));

    // From SelectOrderSet
    let stmt = db
        .select(())
        .from(simple)
        .order_by([drizzle_core::asc(simple.id)])
        .for_update();
    let sql = stmt.to_sql().sql();
    assert!(sql.contains("FOR UPDATE"));

    // From SelectLimitSet
    let stmt = db.select(()).from(simple).limit(10).for_update();
    let sql = stmt.to_sql().sql();
    assert!(sql.contains("FOR UPDATE"));
});

// Test actual execution of FOR UPDATE (locks rows)
postgres_test!(for_update_execution, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    // Insert test data
    let stmt = db.insert(simple).values([InsertSimple::new("test_lock")]);
    drizzle_exec!(stmt => execute);

    // Execute SELECT FOR UPDATE
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "test_lock"))
        .for_update();

    let results: Vec<PgSimpleResult> = drizzle_exec!(stmt => all_as);

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "test_lock");
});
