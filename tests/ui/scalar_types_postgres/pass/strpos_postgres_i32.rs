use drizzle::core::expr::{raw_non_null, strpos};
use drizzle::core::ExprValueType;
use drizzle::postgres::prelude::*;

fn value_type<E: ExprValueType>(_: E) -> E::ValueType
where
    E::ValueType: Default,
{
    Default::default()
}

fn main() {
    let _: i32 = value_type(strpos(
        raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'hello'"),
        raw_non_null::<PostgresValue, drizzle::postgres::types::Text>("'ll'"),
    ));
}
