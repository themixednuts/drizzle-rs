//! Unit tests for the `PostgreSQL` relational query SQL shape.
//!
//! `PostgreSQL` evaluates SELECT-list subqueries for every row the plan
//! produces before LIMIT/OFFSET discard it, so paginated relational queries
//! must push the base scan into a derived table. These tests pin that shape.

use drizzle_core::query::{RelCardinality, RenderedRelation, build_query_sql};
use drizzle_core::{ColumnRef, PaginationArg, SQL, SQLChunk};

use crate::values::PostgresValue;

type PgSql = SQL<'static, PostgresValue<'static>>;

fn details_relation() -> RenderedRelation<'static, PostgresValue<'static>> {
    RenderedRelation {
        table_name: "order_details",
        column_names: vec!["quantity", "order_id"],
        blob_columns: &[],
        fk_columns: &[("order_id", "id")],
        cardinality: RelCardinality::Many,
        rel_name: "details",
        where_sql: SQL::empty(),
        order_by_sql: SQL::empty(),
        limit: None,
        offset: None,
        nested: Vec::new(),
        junction: None,
    }
}

fn order_by_id() -> PgSql {
    let mut sql = PgSql::empty();
    sql.push_mut(SQLChunk::Column(ColumnRef::sql("orders", "id").into()));
    sql.push_mut(SQLChunk::Raw(" ASC".into()));
    sql
}

const DETAILS_SUBQUERY: &str = r#"(SELECT COALESCE(json_agg(json_build_object('quantity', "t1"."quantity", 'order_id', "t1"."order_id")), '[]'::json) FROM "order_details" AS "t1" WHERE "t1"."order_id" = "t0"."id")::text AS "__rel_details""#;

#[test]
fn unpaginated_relation_keeps_flat_shape() {
    let sql = build_query_sql(
        "orders",
        &["id", "name"],
        &[],
        vec![details_relation()],
        SQL::empty(),
        SQL::empty(),
        None,
        None,
        false,
    );

    assert_eq!(
        sql.sql(),
        format!(r#"SELECT "t0"."id", "t0"."name", {DETAILS_SUBQUERY} FROM "orders" AS "t0""#)
    );
}

#[test]
fn paginated_relation_wraps_base_scan_in_derived_table() {
    let sql = build_query_sql(
        "orders",
        &["id", "name"],
        &[],
        vec![details_relation()],
        SQL::empty(),
        order_by_id(),
        Some(50usize.into_pagination_sql()),
        Some(10usize.into_pagination_sql()),
        false,
    );

    assert_eq!(
        sql.sql(),
        format!(
            r#"SELECT "t0"."id", "t0"."name", {DETAILS_SUBQUERY} FROM (SELECT "t0"."id", "t0"."name" FROM "orders" AS "t0" ORDER BY "t0"."id" ASC LIMIT $1 OFFSET $2) AS "t0" ORDER BY "t0"."id" ASC"#
        )
    );
}

#[test]
fn paginated_partial_selection_exposes_fk_and_order_columns() {
    let sql = build_query_sql(
        "orders",
        &["name"],
        &[],
        vec![details_relation()],
        SQL::empty(),
        order_by_id(),
        Some(50usize.into_pagination_sql()),
        None,
        true,
    );

    // The derived table must expose "id" (relation FK / ORDER BY column) even
    // though the selection only asked for "name".
    assert_eq!(
        sql.sql(),
        format!(
            r#"SELECT json_build_object('name', "t0"."name")::text AS "__base", {DETAILS_SUBQUERY} FROM (SELECT "t0"."name", "t0"."id" FROM "orders" AS "t0" ORDER BY "t0"."id" ASC LIMIT $1) AS "t0" ORDER BY "t0"."id" ASC"#
        )
    );
}

#[test]
fn pagination_without_relations_keeps_flat_shape() {
    let sql = build_query_sql::<PostgresValue<'static>>(
        "orders",
        &["id", "name"],
        &[],
        Vec::new(),
        SQL::empty(),
        order_by_id(),
        Some(50usize.into_pagination_sql()),
        None,
        false,
    );

    assert_eq!(
        sql.sql(),
        r#"SELECT "t0"."id", "t0"."name" FROM "orders" AS "t0" ORDER BY "t0"."id" ASC LIMIT $1"#
    );
}
