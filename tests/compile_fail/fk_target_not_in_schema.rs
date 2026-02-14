//! Test that a schema missing a FK target table is caught at compile time.
//! Child references Parent, but the schema only contains Child.

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
    parent_id: Option<i32>,
}

// ERROR: Child has FK to Parent, but Parent is not in the schema.
#[derive(SQLiteSchema)]
struct BadSchema {
    child: Child,
}

fn main() {}
