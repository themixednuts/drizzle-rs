use drizzle::core::expr::sum;
use drizzle::core::ExprValueType;
use drizzle::postgres::prelude::*;

#[PostgresTable]
struct User {
    #[column(primary)]
    id: i32,
    age: i32,
}

fn value_type<E: ExprValueType>(_: E) -> E::ValueType
where
    E::ValueType: Default,
{
    Default::default()
}

fn main() {
    let user = User::default();
    let _: Option<i32> = value_type(sum(user.age));
}
