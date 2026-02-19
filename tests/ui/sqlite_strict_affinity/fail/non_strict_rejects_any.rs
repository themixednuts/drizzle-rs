use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct NonStrictAny {
    #[column(primary)]
    id: i32,
    #[column(any)]
    payload: String,
}

fn main() {
    let _ = NonStrictAny::default();
}
