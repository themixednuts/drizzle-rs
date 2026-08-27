#[cfg(feature = "mysql-sync")]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use drizzle::core::expr::eq;
    use drizzle::mysql::{mysql_sync::Drizzle, prelude::*};
    use mysql::prelude::Queryable as _;

    #[MySQLTable(NAME = "drizzle_example_users")]
    struct User {
        #[column(PRIMARY, AUTO_INCREMENT)]
        id: u64,
        #[column(VARCHAR(255))]
        email: String,
        #[column(VARCHAR(255))]
        name: String,
    }

    #[MySQLIndex(unique)]
    struct UserEmailIndex(User::email);

    #[derive(MySQLSchema)]
    struct Schema {
        users: User,
        users_email_index: UserEmailIndex,
    }

    let url = std::env::var("DRIZZLE_MYSQL_URL")
        .unwrap_or_else(|_| "mysql://drizzle:drizzle@127.0.0.1:3307/drizzle_test".to_owned());
    let options = mysql::Opts::from_url(&url)?;
    let mut connection = mysql::Conn::new(options)?;
    connection.query_drop("DROP TABLE IF EXISTS `drizzle_example_users`")?;

    let (mut db, Schema { users, .. }) = Drizzle::new(connection, Schema::new());
    db.create()?;

    db.insert(users)
        .value(InsertUser::new("alice@example.com", "Alice"))
        .on_duplicate_key_update(UpdateUser::default().with_name("Alice"))
        .execute()?;

    db.transaction(MySQLTransactionConfig::default(), |tx| {
        tx.insert(users)
            .value(InsertUser::new("bob@example.com", "Bob"))
            .execute()?;
        Ok(())
    })?;

    let alice: SelectUser = db
        .select(())
        .from(users)
        .r#where(eq(users.email, "alice@example.com"))
        .get()?;
    assert_eq!(alice.name, "Alice");

    let all_users: Vec<SelectUser> = db.select(()).from(users).all()?;
    println!("Users: {all_users:?}");
    Ok(())
}

#[cfg(not(feature = "mysql-sync"))]
fn main() {
    println!(
        "mysql-sync feature not enabled — run with: cargo run --example mysql --features mysql-sync"
    );
}
