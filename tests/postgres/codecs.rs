//! PostgreSQL value normalization tests.

#![cfg(any(feature = "postgres-sync", feature = "tokio-postgres"))]

use std::borrow::Cow;

use crate::common::schema::postgres::SimpleSchema;
use drizzle::core::SQL;
use drizzle::postgres::prelude::*;
use drizzle::postgres::values::PostgresValue;

#[derive(Debug, PostgresFromRow)]
struct CodecArrayEcho(Vec<i32>, Vec<bool>, Vec<String>, Vec<Vec<u8>>);

#[derive(Debug, PostgresFromRow)]
struct CodecNullableArrayEcho(Vec<Option<i32>>, Vec<Option<String>>);

#[cfg(feature = "serde")]
#[derive(Debug, PostgresFromRow)]
struct CodecJsonArrayEcho(Vec<serde_json::Value>);

#[drizzle::test]
fn postgres_array_codecs_bind_as_native_arrays(db: &mut TestDb<SimpleSchema>) {
    let query = SQL::raw("SELECT ")
        .append(SQL::param(PostgresValue::from(vec![1_i32, 2, 3])))
        .append(SQL::raw("::int4[], "))
        .append(SQL::param(PostgresValue::from(vec![true, false, true])))
        .append(SQL::raw("::bool[], "))
        .append(SQL::param(PostgresValue::from(vec![
            "alpha",
            "comma,value",
            "quote\"value",
        ])))
        .append(SQL::raw("::text[], "))
        .append(SQL::param(PostgresValue::Array(vec![
            PostgresValue::Bytea(Cow::Owned(vec![0, 1, 2])),
            PostgresValue::Bytea(Cow::Owned(vec![255, 254])),
        ])))
        .append(SQL::raw("::bytea[]"));

    let rows: Vec<CodecArrayEcho> = result!(db.all(query)).unwrap();
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    assert_eq!(row.0, vec![1, 2, 3]);
    assert_eq!(row.1, vec![true, false, true]);
    assert_eq!(
        row.2,
        vec![
            "alpha".to_owned(),
            "comma,value".to_owned(),
            "quote\"value".to_owned()
        ]
    );
    assert_eq!(row.3, vec![vec![0, 1, 2], vec![255, 254]]);
}

#[drizzle::test]
fn postgres_array_codecs_preserve_null_elements(db: &mut TestDb<SimpleSchema>) {
    let query = SQL::raw("SELECT ")
        .append(SQL::param(PostgresValue::Array(vec![
            PostgresValue::Integer(1),
            PostgresValue::Null,
            PostgresValue::Integer(3),
        ])))
        .append(SQL::raw("::int4[], "))
        .append(SQL::param(PostgresValue::Array(vec![
            PostgresValue::Text(Cow::Borrowed("alpha")),
            PostgresValue::Null,
            PostgresValue::Text(Cow::Borrowed("omega")),
        ])))
        .append(SQL::raw("::text[]"));

    let rows: Vec<CodecNullableArrayEcho> = result!(db.all(query)).unwrap();
    assert_eq!(rows.len(), 1);

    let row = &rows[0];
    assert_eq!(row.0, vec![Some(1), None, Some(3)]);
    assert_eq!(
        row.1,
        vec![Some("alpha".to_owned()), None, Some("omega".to_owned())]
    );
}

#[cfg(feature = "serde")]
#[drizzle::test]
fn postgres_array_codecs_bind_json_arrays(db: &mut TestDb<SimpleSchema>) {
    let query = SQL::raw("SELECT ")
        .append(SQL::param(PostgresValue::Array(vec![
            PostgresValue::Jsonb(serde_json::json!({ "kind": "object", "n": 1 })),
            PostgresValue::Jsonb(serde_json::json!(["array", 2])),
        ])))
        .append(SQL::raw("::jsonb[]"));

    let rows: Vec<CodecJsonArrayEcho> = result!(db.all(query)).unwrap();
    assert_eq!(rows.len(), 1);

    assert_eq!(
        rows[0].0,
        vec![
            serde_json::json!({ "kind": "object", "n": 1 }),
            serde_json::json!(["array", 2]),
        ]
    );
}
