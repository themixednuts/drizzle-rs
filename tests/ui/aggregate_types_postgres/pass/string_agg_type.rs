use drizzle::core::expr::{raw_non_null, string_agg};
use drizzle::core::types::Text;
use drizzle::core::ExprValueType;
use drizzle::postgres::prelude::*;

fn value_type<E: ExprValueType>(_: E) -> E::ValueType
where
    E::ValueType: Default,
{
    Default::default()
}

fn main() {
    let expr = raw_non_null::<PostgresValue, Text>("'a'");
    let _: Option<String> = value_type(string_agg(expr, ","));
}
