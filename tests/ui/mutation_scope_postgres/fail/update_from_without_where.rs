use drizzle::postgres::prelude::*;
use drizzle::postgres::sync::Drizzle;

#[PostgresTable]
struct Users {
    #[column(primary)]
    id: i32,
    name: String,
}

#[PostgresTable]
struct Renames {
    #[column(primary)]
    id: i32,
    name: String,
}

#[derive(PostgresSchema)]
struct Schema {
    users: Users,
    renames: Renames,
}

fn run(db: &mut Drizzle<Schema>) {
    let Schema { users, renames } = Schema::new();
    // `UPDATE ... FROM` without WHERE updates every row against every source row.
    let _ = db
        .update(users)
        .set(UpdateUsers::default().with_name("x"))
        .from(renames)
        .execute();
}

fn main() {
    let _ = run;
}
