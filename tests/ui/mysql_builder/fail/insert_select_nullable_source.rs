use drizzle::mysql::{builder::QueryBuilder, prelude::*};

#[MySQLTable]
struct Users {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    name: String,
}

#[MySQLTable]
struct ImportedUsers {
    #[column(PRIMARY)]
    id: u64,
    name: Option<String>,
}

#[derive(MySQLSchema)]
struct Schema {
    users: Users,
    imported_users: ImportedUsers,
}

fn main() {
    let builder = QueryBuilder::new::<Schema>();
    let Schema {
        users,
        imported_users,
    } = Schema::new();
    let nullable = builder
        .select((imported_users.id, imported_users.name))
        .from(imported_users);
    let _ = builder.insert(users).select(nullable);
}
