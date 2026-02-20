use drizzle::core::expr::{json_agg, jsonb_agg, raw_non_null, Agg, Null, SQLExpr};
use drizzle::postgres::prelude::*;

fn main() {
    let _: SQLExpr<'static, PostgresValue, drizzle::postgres::types::Json, Null, Agg> =
        json_agg(raw_non_null::<PostgresValue, drizzle::postgres::types::Int4>("1"));
    let _: SQLExpr<'static, PostgresValue, drizzle::postgres::types::Jsonb, Null, Agg> =
        jsonb_agg(raw_non_null::<PostgresValue, drizzle::postgres::types::Int4>("1"));
}
