use drizzle::core::{Cons, Nil, Scoped, SelectCols};
use drizzle::sqlite::prelude::*;
use drizzle_core::row::{MarkerAggValidFor, ScalarCheck, ScopeHere};

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
    email: String,
}

type InvalidGroupedSelection = Scoped<SelectCols<(users::Name,)>, Cons<Users, Nil>>;
type GroupedColumns = Cons<users::Email, Nil>;

fn require_valid_group<Selection>()
where
    Selection: MarkerAggValidFor<GroupedColumns, (ScalarCheck<ScopeHere>,)>,
{
}

fn main() {
    require_valid_group::<InvalidGroupedSelection>();
}
