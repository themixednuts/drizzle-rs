use drizzle::postgres::expr::jsonb_contains;
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
    // The containment operand must be JSON or JSON text.
    let _ = jsonb_contains(doc.payload, 1i32);
    let _ = jsonb_contains(doc.payload, doc.id);
}
