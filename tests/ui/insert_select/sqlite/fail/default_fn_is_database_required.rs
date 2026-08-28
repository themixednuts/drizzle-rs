use drizzle::sqlite::{builder::QueryBuilder, prelude::*};

#[SQLiteTable]
struct Sources {
    id: i32,
}

#[SQLiteTable]
struct Targets {
    #[column(primary)]
    id: i32,
    #[column(default_fn = String::new)]
    application_default: String,
}

#[derive(SQLiteSchema)]
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
