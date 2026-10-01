use drizzle::postgres::expr::{
    PgArray, array_contains, json_get, json_get_text, jsonb_contained, jsonb_contains,
};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Doc {
    #[column(primary)]
    id: i32,
    tags: Vec<String>,
    #[column(json)]
    body: serde_json::Value,
    #[column(jsonb)]
    payload: serde_json::Value,
}

fn main() {
    let doc = Doc::default();
    let _ = array_contains(doc.tags, PgArray(vec!["rust"]));
    let _ = array_contains(doc.tags, doc.tags);
    let _ = json_get_text(doc.body, "a");
    // `->` on jsonb stays jsonb, so jsonb operators chain.
    let _ = jsonb_contained(json_get(doc.payload, "meta"), r#"{"level":3}"#);
    let _ = jsonb_contains(doc.payload, json_get(doc.payload, "meta"));
}
