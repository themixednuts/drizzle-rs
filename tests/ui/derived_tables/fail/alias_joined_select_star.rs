use drizzle::core::{Cons, Nil, Scoped, SelectStar};
use drizzle::sqlite::prelude::*;
use drizzle_core::DerivedSelection;

#[SQLiteTable]
struct Users {
    #[column(primary)]
    id: i32,
}

#[SQLiteTable]
struct Posts {
    #[column(primary)]
    id: i32,
    user_id: i32,
}

type JoinedSelectStar = Scoped<SelectStar, Cons<Posts, Cons<Users, Nil>>>;

fn require_derived_selection<'a, Selection>()
where
    Selection: DerivedSelection<'a, SQLiteValue<'a>, SQLiteSchemaType, Posts>,
{
}

fn main() {
    require_derived_selection::<JoinedSelectStar>();
}
