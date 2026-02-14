//! Test that auto-FK join (bare table) fails for tables with no FK relationship.
//! Unrelated has no FK to Parent, so the Joinable bound should fail.

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Parent {
    #[column(primary)]
    id: i32,
    name: String,
}

#[SQLiteTable]
struct Unrelated {
    #[column(primary)]
    id: i32,
    value: String,
}

fn requires_joinable<A: Joinable<B>, B>() {}

fn main() {
    // ERROR: Unrelated does not have a FK to Parent
    requires_joinable::<Unrelated, Parent>();
}
