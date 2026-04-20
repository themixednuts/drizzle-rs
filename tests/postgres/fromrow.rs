//! Compile-time and runtime checks for Postgres FromRow implementations.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::eq;
use drizzle::postgres::prelude::*;

#[derive(Debug, PostgresFromRow)]
#[from(Simple)]
struct NamedSimple {
    id: i32,
    name: String,
}

#[derive(Debug, PostgresFromRow)]
struct TupleNameId(String, i32);

#[drizzle::test]
fn named_struct_maps_by_name(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    // Strict row-shape checks require selected column order to match struct field order.
    let stmt = db.select((simple.id, simple.name)).from(simple);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Alice");
}

#[drizzle::test]
fn tuple_struct_maps_by_index(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    // Tuple structs decode positionally, so this follows SELECT order.
    let stmt = db.select((simple.name, simple.id)).from(simple);
    let result: TupleNameId = drizzle_exec!(stmt => get);

    assert_eq!(result.0, "Alice");
    assert_eq!(result.1, 1);
}

#[drizzle::test]
fn fromrow_inferred_with_select_target(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(NamedSimple::Select).from(simple);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Alice");
}

#[drizzle::test]
fn insert_returning_select_target_infers_row(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("Alice")])
        .returning(NamedSimple::Select);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Alice");
}

#[drizzle::test]
fn update_returning_select_target_infers_row(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(db.insert(simple).values([InsertSimple::new("Alice")]) => execute);

    let stmt = db
        .update(simple)
        .set(UpdateSimple::default().with_name("Bob"))
        .r#where(eq(simple.id, 1))
        .returning(NamedSimple::Select);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Bob");
}

#[drizzle::test]
fn delete_returning_select_target_infers_row(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(db.insert(simple).values([InsertSimple::new("Alice")]) => execute);

    let stmt = db
        .delete(simple)
        .r#where(eq(simple.id, 1))
        .returning(NamedSimple::Select);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Alice");
}

#[cfg(feature = "tokio-postgres")]
mod tokio_fromrow_checks {
    use drizzle::postgres::prelude::*;
    use drizzle_macros::{PostgresFromRow, PostgresTable};

    #[PostgresTable(name = "users")]
    struct Users {
        #[column(serial, primary)]
        id: i32,
        name: String,
    }

    #[allow(dead_code)]
    #[derive(PostgresFromRow, Debug)]
    struct UserRow {
        id: i32,
        name: String,
    }

    #[allow(dead_code)]
    fn assert_tokio_fromrow<T>()
    where
        for<'a> T: TryFrom<&'a tokio_postgres::Row>,
    {
    }

    #[test]
    fn tokio_postgres_fromrow_compiles() {
        let _ = Users::default();
        assert_tokio_fromrow::<UserRow>();
    }
}
