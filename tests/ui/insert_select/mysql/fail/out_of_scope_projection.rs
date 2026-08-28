use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Sources {
    id: i32,
}

#[MySQLTable]
struct Targets {
    id: i32,
}

#[derive(MySQLSchema)]
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
