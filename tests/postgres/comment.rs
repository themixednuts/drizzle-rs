//! SQL-generation tests for `.comment()` / `.comment_tags()` sqlcommenter
//! helpers on the Postgres builder.
//!
//! These only assert the emitted SQL string so they don't require a running
//! database — they run under any postgres-enabled feature configuration.

#![cfg(feature = "postgres")]

use crate::common::schema::postgres::{Simple, SimpleSchema};
use drizzle::core::expr::eq;
use drizzle::postgres::builder::QueryBuilder;
use drizzle::postgres::prelude::*;

#[test]
fn comment_on_select() {
    let builder = QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema { simple } = SimpleSchema::new();

    let sql = builder
        .select(())
        .from(simple)
        .comment("trace_id=abc")
        .to_sql()
        .sql();

    assert_eq!(
        sql,
        r#"/*trace_id=abc*/ SELECT "simple"."id", "simple"."name" FROM "simple""#
    );
}

#[test]
fn comment_sanitises_block_terminators() {
    let builder = QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema { simple } = SimpleSchema::new();

    // Embedded `/*` / `*/` must not be able to close the surrounding block.
    let sql = builder
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

#[test]
fn comment_tags_sort_and_url_encode() {
    let builder = QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema { simple } = SimpleSchema::new();

    let sql = builder
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

#[test]
fn comment_on_update_and_delete() {
    let builder = QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema { simple } = SimpleSchema::new();

    let update_sql = builder
        .update(simple)
        .set(crate::common::schema::postgres::UpdateSimple::default().with_name("renamed"))
        .r#where(eq(Simple::id, 1))
        .comment("upd")
        .to_sql()
        .sql();
    assert!(
        update_sql.starts_with("/*upd*/ UPDATE"),
        "expected update SQL to start with /*upd*/ UPDATE, got: {update_sql}"
    );

    let delete_sql = builder
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

#[test]
fn empty_comment_is_noop() {
    let builder = QueryBuilder::new::<SimpleSchema>();
    let SimpleSchema { simple } = SimpleSchema::new();

    let sql = builder.select(()).from(simple).comment("").to_sql().sql();
    assert_eq!(
        sql,
        r#"SELECT "simple"."id", "simple"."name" FROM "simple""#
    );

    let sql = builder
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
