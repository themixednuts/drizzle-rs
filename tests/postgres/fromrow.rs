//! Compile-time and runtime checks for Postgres FromRow implementations.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use crate::common::schema::postgres::*;
use drizzle::core::expr::eq;
use drizzle::postgres::prelude::*;
use drizzle_macros::postgres_test;

#[derive(Debug, PostgresFromRow)]
#[from(Simple)]
struct NamedSimple {
    id: i32,
    name: String,
}

#[derive(Debug, PostgresFromRow)]
struct TupleNameId(String, i32);

postgres_test!(named_struct_maps_by_name, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    // Intentionally reverse selected column order.
    // Named structs should still decode by field name.
    let stmt = db.select((simple.name, simple.id)).from(simple);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Alice");
});

postgres_test!(tuple_struct_maps_by_index, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    // Tuple structs decode positionally, so this follows SELECT order.
    let stmt = db.select((simple.name, simple.id)).from(simple);
    let result: TupleNameId = drizzle_exec!(stmt => get);

    assert_eq!(result.0, "Alice");
    assert_eq!(result.1, 1);
});

postgres_test!(fromrow_inferred_with_select_target, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db.insert(simple).values([InsertSimple::new("Alice")]);
    drizzle_exec!(stmt => execute);

    let stmt = db.select(NamedSimple::Select).from(simple);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Alice");
});

postgres_test!(insert_returning_select_target_infers_row, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    let stmt = db
        .insert(simple)
        .values([InsertSimple::new("Alice")])
        .returning(NamedSimple::Select);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Alice");
});

postgres_test!(update_returning_select_target_infers_row, SimpleSchema, {
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
});

postgres_test!(delete_returning_select_target_infers_row, SimpleSchema, {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(db.insert(simple).values([InsertSimple::new("Alice")]) => execute);

    let stmt = db
        .delete(simple)
        .r#where(eq(simple.id, 1))
        .returning(NamedSimple::Select);
    let result: NamedSimple = drizzle_exec!(stmt => get);

    assert_eq!(result.id, 1);
    assert_eq!(result.name, "Alice");
});

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
