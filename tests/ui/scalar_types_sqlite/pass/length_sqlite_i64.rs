use drizzle::core::expr::{length, raw_non_null};
use drizzle::core::types::Text;
use drizzle::core::ExprValueType;
use drizzle::sqlite::prelude::*;

fn value_type<E: ExprValueType>(_: E) -> E::ValueType
where
    E::ValueType: Default,
{
    Default::default()
}

fn main() {
    let _: i64 = value_type(length(raw_non_null::<SQLiteValue, Text>("'hello'")));
}
