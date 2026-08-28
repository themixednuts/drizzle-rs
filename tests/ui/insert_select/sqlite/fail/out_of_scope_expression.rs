use drizzle::core::expr::lower;
use drizzle::sqlite::{builder::QueryBuilder, prelude::*};

#[SQLiteTable]
struct Sources {
    name: String,
}

#[SQLiteTable]
struct Targets {
    name: String,
}

#[derive(SQLiteSchema)]
struct Schema {
    sources: Sources,
    targets: Targets,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { sources, targets } = Schema::new();
    let selected = builder.select(lower(targets.name)).from(sources);
    let _ = builder.insert(targets).select(selected);
}
