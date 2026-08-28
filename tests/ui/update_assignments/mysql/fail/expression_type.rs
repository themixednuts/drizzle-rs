use drizzle::{
    core::expr::lower,
    mysql::prelude::*,
};

#[MySQLTable]
struct Users {
    visits: i64,
    name: String,
    optional_visits: Option<i64>,
}

fn main() {
    let users = Users::new();

    let _ = UpdateUsers::default().with_visits(lower(users.name));
    let _ = UpdateUsers::default().with_visits(users.optional_visits + 1_i64);
}
