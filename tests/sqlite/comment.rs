#![cfg(any(feature = "rusqlite", feature = "turso", feature = "libsql"))]
//! Integration tests for the `.comment()` / `.comment_tags()` sqlcommenter
//! helper (upstream drizzle-orm Beta.19).
//!
//! These exercise the end-to-end flow: calling `.comment(...)` on a builder
//! returned by `db.select()/insert()/update()/delete()` prepends a properly
//! encoded `/* ... */` block to the generated SQL, and the resulting
//! statement still round-trips through the driver.

use crate::common::schema::sqlite::{InsertSimple, Simple, SimpleSchema};
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;

#[drizzle::test]
fn comment_select_prepends_block(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    let sql = db
        .select(())
        .from(simple)
        .comment("trace_id=abc")
        .to_sql()
        .sql();

    // A single space between the comment block and the SQL matches upstream
    // drizzle-orm output and is the conventional sqlcommenter format.
    assert_eq!(
        sql,
        r#"/*trace_id=abc*/ SELECT "simple"."id", "simple"."name" FROM "simple""#
    );
}

#[drizzle::test]
fn comment_select_with_where(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    let sql = db
        .select(())
        .from(simple)
        .r#where(eq(Simple::name, "x"))
        .comment("slow-query-warn")
        .to_sql()
        .sql();

    assert_eq!(
        sql,
        r#"/*slow-query-warn*/ SELECT "simple"."id", "simple"."name" FROM "simple" WHERE "simple"."name" = ?"#
    );
}

#[drizzle::test]
fn comment_tags_sort_and_url_encode(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    // Tags are URL-encoded (per encodeURIComponent) and sorted alphabetically.
    let sql = db
        .select(())
        .from(simple)
        .comment_tags([("route", "/users/:id"), ("action", "update")])
        .to_sql()
        .sql();

    assert_eq!(
        sql,
        r#"/*action='update',route='%2Fusers%2F%3Aid'*/ SELECT "simple"."id", "simple"."name" FROM "simple""#
    );
}

#[drizzle::test]
fn comment_sanitises_comment_terminators(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    // An attacker-controlled trace string containing `/*` / `*/` must not be
    // able to break out of the surrounding comment.
    let sql = db
        .select(())
        .from(simple)
        .comment("evil /* escape */ attempt")
        .to_sql()
        .sql();

    assert_eq!(
        sql,
        r#"/*evil / * escape * / attempt*/ SELECT "simple"."id", "simple"."name" FROM "simple""#
    );
}

#[drizzle::test]
fn comment_empty_is_noop(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    let sql = db.select(()).from(simple).comment("").to_sql().sql();
    assert_eq!(
        sql,
        r#"SELECT "simple"."id", "simple"."name" FROM "simple""#
    );

    let sql = db
        .select(())
        .from(simple)
        .comment_tags::<[(&str, &str); 0], _, _>([])
        .to_sql()
        .sql();
    assert_eq!(
        sql,
        r#"SELECT "simple"."id", "simple"."name" FROM "simple""#
    );
}

#[drizzle::test]
fn comment_on_insert_and_update_and_delete(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    // INSERT
    let insert_sql = db
        .insert(simple)
        .values([InsertSimple::new("alice")])
        .comment("ins")
        .to_sql()
        .sql();
    assert!(
        insert_sql.starts_with("/*ins*/ INSERT"),
        "expected insert SQL to start with /*ins*/ INSERT, got: {insert_sql}"
    );

    // UPDATE (uses UpdateSimple default + with_* setter)
    let update_sql = db
        .update(simple)
        .set(crate::common::schema::sqlite::UpdateSimple::default().with_name("renamed"))
        .r#where(eq(Simple::id, 1))
        .comment("upd")
        .to_sql()
        .sql();
    assert!(
        update_sql.starts_with("/*upd*/ UPDATE"),
        "expected update SQL to start with /*upd*/ UPDATE, got: {update_sql}"
    );

    // DELETE
    let delete_sql = db
        .delete(simple)
        .r#where(eq(Simple::id, 1))
        .comment("del")
        .to_sql()
        .sql();
    assert!(
        delete_sql.starts_with("/*del*/ DELETE"),
        "expected delete SQL to start with /*del*/ DELETE, got: {delete_sql}"
    );
}

#[drizzle::test]
fn comment_roundtrips_through_driver(db: &mut TestDb<SimpleSchema>, schema: SimpleSchema) {
    let SimpleSchema { simple } = schema;

    drizzle_exec!(
        db.insert(simple).values([InsertSimple::new("alice"), InsertSimple::new("bob")])
            => execute
    );

    // The comment is a valid SQL block comment, so the query must still
    // execute and return the expected rows.
    let rows: Vec<crate::common::schema::sqlite::SelectSimple> =
        drizzle_exec!(db.select(()).from(simple).comment("observability-demo") => all);
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].name, "alice");
    assert_eq!(rows[1].name, "bob");
}
