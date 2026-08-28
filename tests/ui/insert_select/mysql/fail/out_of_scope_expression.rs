use drizzle::core::expr::lower;
use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Sources {
    name: String,
}

#[MySQLTable]
struct Targets {
    name: String,
}

#[derive(MySQLSchema)]
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
