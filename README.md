# Drizzle RS

A type-safe SQL query builder / ORM-ish layer for Rust, inspired by Drizzle ORM.

> [!WARNING]
> This project is still evolving. Expect breaking changes.

## Install

### Library

Pick a database driver feature (drivers imply the corresponding dialect module):

```toml
[dependencies]
drizzle = { version = "0.1.4", features = ["rusqlite"] } # or: libsql / turso / postgres-sync / tokio-postgres
```

### CLI

Install the `drizzle` binary:

```bash
cargo install drizzle-cli --locked --all-features
```

## Quick start (SQLite + rusqlite)

```rust
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    pub name: String,
    pub age: i64,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
}

fn main() -> drizzle::Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    let (mut db, Schema { users }) = Drizzle::new(conn, Schema::new());
    db.create()?;

    db.insert(users)
        .values([InsertUsers::new("Alex Smith", 26i64)])
        .execute()?;

    let row: SelectUsers = db
        .select(())
        .from(users)
        .r#where(eq(users.name, "Alex Smith"))
        .get()?;

    println!("user: {row:?}");
    Ok(())
}
```

> [!NOTE]
> `db.create()` statements are intentionally not `IF NOT EXISTS` so migrations can
> own schema creation. Use it only for fresh databases.

## Query patterns (SQLite)

### Join mapping with `SQLiteFromRow`

Assuming a `Posts` table with `user_id` referencing `Users::id`:

```rust
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;

#[derive(SQLiteFromRow, Default, Debug)]
struct UserWithPost {
    #[column(Users::id)]
    user_id: i64,
    #[column(Users::name)]
    name: String,
    #[column(Posts::id)]
    post_id: i64,
    #[column(Posts::context)]
    context: Option<String>,
}

let row: UserWithPost = db
    .select(UserWithPost::default())
    .from(users)
    .left_join(posts, eq(users.id, posts.user_id))
    .get()?;
```

### Updates

```rust
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;

let stmt = db
    .update(users)
    .set(UpdateUsers::default().with_age(27))
    .r#where(eq(users.id, 1));
stmt.execute()?;
```

## Quick start (PostgreSQL, sync)

```rust
use drizzle::postgres::prelude::*;
use drizzle::postgres::sync::Drizzle;

#[PostgresTable]
struct Accounts {
    #[column(serial, primary)]
    id: i32,
    name: String,
}

#[derive(PostgresSchema)]
struct Schema {
    accounts: Accounts,
}

fn main() -> drizzle::Result<()> {
    let client = postgres::Client::connect(
        "host=localhost user=postgres dbname=drizzle_test",
        postgres::NoTls,
    )?;
    let (mut db, Schema { accounts }) = Drizzle::new(client, Schema::new());
    // Async variant: drizzle::postgres::tokio::Drizzle with tokio_postgres::connect.
    db.create()?;

    db.insert(accounts)
        .values([InsertAccounts::new("Acme")])
        .execute()?;

    let rows: Vec<SelectAccounts> = db.select(()).from(accounts).all()?;
    println!("accounts: {rows:?}");
    Ok(())
}
```

## CLI quick start (migrations)

```bash
drizzle init -d sqlite          # dialects: sqlite, turso, postgresql, mysql, singlestore
drizzle generate                # generate migrations from schema changes
drizzle migrate                 # apply pending migrations
```
## License

MIT — see [LICENSE](LICENSE).
