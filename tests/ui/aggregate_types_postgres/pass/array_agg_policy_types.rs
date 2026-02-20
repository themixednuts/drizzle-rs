use drizzle::core::expr::{array_agg, raw_non_null};
use drizzle::core::ExprValueType;
use drizzle::postgres::prelude::*;

fn value_type<E: ExprValueType>(_: E) -> E::ValueType
where
    E::ValueType: Default,
{
    Default::default()
}

fn main() {
    let _: Option<Vec<i32>> = value_type(array_agg(raw_non_null::<PostgresValue, drizzle::postgres::types::Int4>("1")));
    let _: Option<Vec<String>> = value_type(array_agg(raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'x'")));
}
