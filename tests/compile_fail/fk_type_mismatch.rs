//! Test that FK column type mismatch is caught at compile time.
//! The source column is Text (String) but the target column is Int (i32).

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Parent {
    #[column(primary)]
    id: i32,
    name: String,
}

#[SQLiteTable]
struct Child {
    #[column(primary)]
    id: i32,
    #[column(references = Parent::id)]
    parent_ref: String, // ERROR: Text vs Int type mismatch
}

fn main() {}
