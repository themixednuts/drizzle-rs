use drizzle::postgres::{builder::QueryBuilder, prelude::*};

#[PostgresTable]
struct Sources {
    id: i32,
}

#[PostgresTable]
struct Targets {
    #[column(primary)]
    id: i32,
    #[column(default_fn = String::new)]
    application_default: String,
}

#[derive(PostgresSchema)]
struct Schema {
    sources: Sources,
    targets: Targets,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema { sources, targets } = Schema::new();
    let selected = builder.select(sources.id).from(sources);
    let _ = builder.insert(targets).columns(targets.id).select(selected);
}
