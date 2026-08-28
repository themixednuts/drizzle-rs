use drizzle::sqlite::{builder::QueryBuilder, prelude::*};

#[SQLiteTable]
struct Sources {
    id: i32,
}

#[SQLiteTable]
struct Targets {
    id: i32,
}

#[derive(SQLiteSchema)]
struct Schema {
    sources: Sources,
    targets: Targets,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { sources, targets } = Schema::new();
    let selected = builder.select(targets.id).from(sources);
    let _ = builder.insert(targets).select(selected);
}
