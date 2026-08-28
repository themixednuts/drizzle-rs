use drizzle::sqlite::{builder::QueryBuilder, prelude::*};

#[SQLiteTable]
struct Sources {
    id: i32,
    name: String,
}

#[SQLiteTable]
struct Targets {
    id: i32,
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
    let selected = builder
        .select((sources.id, sources.name))
        .from(sources)
        .group_by(sources.name);
    let _ = builder.insert(targets).select(selected);
}
