use drizzle::sqlite::builder::QueryBuilder;
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
struct Users {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = Teams::id)]
    current_team_id: Option<i32>,
}

#[SQLiteTable]
struct Teams {
    #[column(PRIMARY)]
    id: i32,
    #[column(REFERENCES = Users::id)]
    owner_id: Option<i32>,
}

#[derive(SQLiteSchema)]
struct Schema {
    users: Users,
    teams: Teams,
}

fn main() {
    let qb = QueryBuilder::new::<Schema>();
    let Schema { users, teams } = Schema::new();
    // Users and teams have keys to each other: which one is the ON clause?
    let _ = qb.select(users.id).from(users).join(teams);
}
