use drizzle::core::expr::lower;
use drizzle::postgres::{builder::QueryBuilder, prelude::*};

#[PostgresTable]
struct Sources {
    name: String,
}

#[PostgresTable]
struct Targets {
    name: String,
}

#[derive(PostgresSchema)]
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
