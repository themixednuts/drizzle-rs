use drizzle::postgres::expr::{json_get, jsonb_contains, jsonb_exists_key};
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct Doc {
    #[column(primary)]
    id: i32,
    title: String,
    #[column(json)]
    body: serde_json::Value,
    #[column(jsonb)]
    payload: serde_json::Value,
}

fn main() {
    let doc = Doc::default();
    // `->` on text.
    let _ = json_get(doc.title, "a");
    // `@>` and `?` exist only for jsonb.
    let _ = jsonb_contains(doc.body, r#"{"a":1}"#);
    let _ = jsonb_exists_key(doc.body, "a");
}
