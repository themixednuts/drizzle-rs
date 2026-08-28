use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Sources {
    id: i32,
    name: String,
}

#[MySQLTable]
struct Targets {
    id: i32,
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
    let selected = builder
        .select((sources.id, sources.name))
        .from(sources)
        .group_by(sources.name);
    let _ = builder.insert(targets).select(selected);
}
