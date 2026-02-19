use drizzle::core::expr::{raw_non_null, stddev_pop, stddev_samp, var_pop, var_samp, variance};
use drizzle::core::types::Int;
use drizzle::core::ExprValueType;
use drizzle::postgres::prelude::*;

fn value_type<E: ExprValueType>(_: E) -> E::ValueType
where
    E::ValueType: Default,
{
    Default::default()
}

fn main() {
    let input = raw_non_null::<PostgresValue, Int>("1");

    let _: Option<f64> = value_type(stddev_pop(input.clone()));
    let _: Option<f64> = value_type(stddev_samp(input.clone()));
    let _: Option<f64> = value_type(var_pop(input.clone()));
    let _: Option<f64> = value_type(var_samp(input.clone()));
    let _: Option<f64> = value_type(variance(input));
}
