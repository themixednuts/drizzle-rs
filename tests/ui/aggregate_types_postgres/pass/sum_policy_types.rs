use drizzle::core::expr::sum;
use drizzle::core::ExprValueType;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
    score: f32,
}

fn value_type<E: ExprValueType>(_: E) -> E::ValueType
where
    E::ValueType: Default,
{
    Default::default()
}

fn main() {
    let user = User::default();
    let _: Option<i64> = value_type(sum(user.age));
    let _: Option<f64> = value_type(sum(user.score));
}
