//! Test that HasConstraint<ForeignKeyK> is only satisfied when the table has a FK.
//! Simple has no FK, so the bound should fail.

use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Simple {
    #[column(primary)]
    id: i32,
    value: String,
}

fn requires_fk_constraint<T: HasConstraint<ForeignKeyK>>() {}

fn main() {
    // ERROR: Simple has no foreign key constraint
    requires_fk_constraint::<Simple>();
}
