use drizzle::core::expr::{json_agg, jsonb_agg, raw_non_null, Agg, Null, SQLExpr};
use drizzle::core::types::{Int, Json, Jsonb};
use drizzle::postgres::prelude::*;

fn main() {
    let _: SQLExpr<'static, PostgresValue, Json, Null, Agg> =
        json_agg(raw_non_null::<PostgresValue, Int>("1"));
    let _: SQLExpr<'static, PostgresValue, Jsonb, Null, Agg> =
        jsonb_agg(raw_non_null::<PostgresValue, Int>("1"));
}
