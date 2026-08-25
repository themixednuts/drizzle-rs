//! PostgreSQL index tests
//!
//! Note: Index creation is tested via schema creation in db.create().
//! These tests verify queries work correctly (indexes improve performance but don't change results).

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::*;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct PartialIndexAccount {
    #[column(primary, serial)]
    id: i32,
    email: String,
    archived_at: Option<String>,
}

#[PostgresIndex(unique, where = "archived_at IS NULL")]
struct ActiveAccountEmailIdx(PartialIndexAccount::email);

#[derive(PostgresSchema)]
struct PartialIndexSchema {
    accounts: PartialIndexAccount,
    active_account_email_idx: ActiveAccountEmailIdx,
}

#[PostgresTable]
struct UniqueConstraintAccount {
    #[column(primary, serial)]
    id: i32,
    #[column(unique, name = "email_address")]
    email: String,
}

#[derive(PostgresSchema)]
struct UniqueConstraintSchema {
    accounts: UniqueConstraintAccount,
}

#[test]
fn partial_unique_index_is_a_complete_conflict_target() {
    let db = drizzle::postgres::builder::QueryBuilder::new::<PartialIndexSchema>();
    let schema = PartialIndexSchema::new();
    let PartialIndexSchema {
        accounts,
        active_account_email_idx,
    } = schema;

    let statement = db
        .insert(accounts)
        .values([InsertPartialIndexAccount::new("active@example.com")])
        .on_conflict(active_account_email_idx)
        .do_nothing()
        .to_sql();

    assert_eq!(
        statement.sql(),
        r#"INSERT INTO "partial_index_account" ("email") VALUES ($1) ON CONFLICT ("email") WHERE archived_at IS NULL DO NOTHING"#
    );
}

#[test]
fn unique_column_is_a_named_constraint_target() {
    let db = drizzle::postgres::builder::QueryBuilder::new::<UniqueConstraintSchema>();
    let accounts = UniqueConstraintSchema::new().accounts;

    let statement = db
        .insert(accounts)
        .values([InsertUniqueConstraintAccount::new("active@example.com")])
        .on_conflict_on_constraint(accounts.email)
        .do_nothing()
        .to_sql();

    assert_eq!(
        statement.sql(),
        r#"INSERT INTO "unique_constraint_account" ("email_address") VALUES ($1) ON CONFLICT ON CONSTRAINT "unique_constraint_account_email_address_key" DO NOTHING"#
    );
}

// Test queries that would benefit from indexes
#[drizzle::test]
fn query_by_name_column(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([
        InsertSimple::new("Alice"),
        InsertSimple::new("Bob"),
        InsertSimple::new("Charlie"),
    ]);
    stmt.execute();

    // Query by name (would use index if one existed)
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "Bob"));
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "Bob");
}

#[cfg(feature = "uuid")]
#[drizzle::test]
fn query_by_nullable_column(db: &mut TestDb<ComplexSchema>) {
    let ComplexSchema { complex, .. } = schema;

    // Insert rows with and without email
    let stmt = db.insert(complex).values([
        InsertComplex::new("With Email", true, Role::User).with_email("test@example.com")
    ]);
    stmt.execute();

    let stmt = db
        .insert(complex)
        .values([InsertComplex::new("No Email", true, Role::User)]);
    stmt.execute();

    #[derive(Debug, PostgresFromRow)]
    struct Result {
        name: String,
    }

    // Query using email column
    let stmt = db
        .select(())
        .from(complex)
        .r#where(eq(complex.email, "test@example.com"));
    let results: Vec<Result> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "With Email");
}

#[drizzle::test]
fn query_large_dataset(db: &mut TestDb<SimpleSchema>) {
    let SimpleSchema { simple } = schema;

    // Insert many rows
    let names: Vec<String> = (0..50).map(|i| format!("User_{:03}", i)).collect();
    let rows: Vec<_> = names
        .iter()
        .map(|n| InsertSimple::new(n.as_str()))
        .collect();
    let stmt = db.insert(simple).values(rows);
    stmt.execute();

    // Query specific row (index would speed this up)
    let stmt = db
        .select((simple.id, simple.name))
        .from(simple)
        .r#where(eq(simple.name, "User_025"));
    let results: Vec<SelectSimple> = stmt.all();

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "User_025");
}
