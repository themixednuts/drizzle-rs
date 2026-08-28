use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Sources {
    id: i32,
}

#[MySQLTable]
struct Targets {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: i32,
    #[column(DEFAULT_FN = String::new)]
    application_default: String,
}

#[derive(MySQLSchema)]
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
