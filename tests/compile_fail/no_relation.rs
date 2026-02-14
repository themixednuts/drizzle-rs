//! Test that Relation<T> is only satisfied when a FK actually exists.
//! Simple has no FK to Parent, so the bound should fail.

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Parent {
    #[column(primary)]
    id: i32,
    name: String,
}

#[SQLiteTable]
struct Simple {
    #[column(primary)]
    id: i32,
    value: String,
}

fn requires_relation_to_parent<T: Relation<Parent>>() {}

fn main() {
    // ERROR: Simple does not have a FK to Parent
    requires_relation_to_parent::<Simple>();
}
