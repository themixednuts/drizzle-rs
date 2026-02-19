use drizzle::core::expr::{bool_and, bool_or, raw_non_null};
use drizzle::core::types::Bool;
use drizzle::core::ExprValueType;
use drizzle::postgres::prelude::*;

fn value_type<E: ExprValueType>(_: E) -> E::ValueType
where
    E::ValueType: Default,
{
    Default::default()
}

fn main() {
    let expr = raw_non_null::<PostgresValue, Bool>("true");

    let _: Option<bool> = value_type(bool_and(expr.clone()));
    let _: Option<bool> = value_type(bool_or(expr));
}
