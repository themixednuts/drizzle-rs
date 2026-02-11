# Drizzle RS

A type-safe SQL query builder / ORM-ish layer for Rust, inspired by Drizzle ORM.

> [!WARNING]
> This project is still evolving. Expect breaking changes.

## Install

### Library

Pick a database driver feature (drivers imply the corresponding dialect module):

```toml
[dependencies]
drizzle = { version = "0.1.5", features = ["rusqlite"] } # or: libsql / turso / postgres-sync / tokio-postgres
```

### CLI

Install the `drizzle` binary:

```bash
cargo install drizzle-cli --locked --all-features
```

## Define your schema (`schema.rs`)

Convention: keep all table definitions in a dedicated `schema.rs` module. Each `#[SQLiteTable]` generates `Select*`, `Insert*`, and `Update*` companion types.

```rust
// schema.rs
use drizzle::sqlite::prelude::*;

#[SQLiteTable]
pub struct Users {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub age: i64,
}

#[SQLiteTable]
pub struct Posts {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub title: String,
    pub content: Option<String>,
    #[column(references = Users::id)]
    pub author_id: i64,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
}
```

> See [`examples/rusqlite.rs`](examples/rusqlite.rs) for a full runnable example.

## Connect

```rust
use drizzle::sqlite::rusqlite::Drizzle;

let conn = rusqlite::Connection::open("app.db")?;
let (db, Schema { users, posts }) = Drizzle::new(conn, Schema::new());

// Create tables (no IF NOT EXISTS — use only on a fresh database)
db.create()?;
```

## CRUD

### Insert

```rust
db.insert(users)
    .values([
        InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"),
        InsertUsers::new("Jordan Lee", 30i64),
    ])
    .execute()?;
```

### Select

```rust
use drizzle::core::expr::eq;

// All rows
let all: Vec<SelectUsers> = db.select(()).from(users).all()?;

// Single row with filter
let user: SelectUsers = db
    .select(())
    .from(users)
    .r#where(eq(users.name, "Alex Smith"))
    .get()?;

// Specific columns
let names: Vec<(String,)> = db
    .select((users.name,))
    .from(users)
    .all()?;
```

### Update

```rust
use drizzle::core::expr::eq;

db.update(users)
    .set(UpdateUsers::default().with_age(27))
    .r#where(eq(users.id, 1))
    .execute()?;
```

### Delete

```rust
use drizzle::core::expr::eq;

db.delete(users)
    .r#where(eq(users.id, 1))
    .execute()?;
```

## Joins

Use `#[derive(SQLiteFromRow)]` to map columns from multiple tables into a flat struct:

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
    #[column(Posts::content)]
    content: Option<String>,
}

let rows: Vec<UserWithPost> = db
    .select(UserWithPost::default())
    .from(users)
    .left_join(posts, eq(users.id, posts.author_id))
    .all()?;
```

## PostgreSQL

Use the same `schema.rs` pattern as SQLite, but with Postgres macros.

```rust
// schema.rs
use drizzle::postgres::prelude::*;

#[PostgresTable]
pub struct Accounts {
    #[column(serial, primary)]
    pub id: i32,
    pub name: String,
}

#[derive(PostgresSchema)]
pub struct Schema {
    pub accounts: Accounts,
}
```

```rust
// main.rs
mod schema;

use drizzle::postgres::sync::Drizzle;
use schema::*;

fn main() -> drizzle::Result<()> {
    let client = postgres::Client::connect(
        "host=localhost user=postgres password=postgres dbname=drizzle_test",
        postgres::NoTls,
    )?;
    let (mut db, Schema { accounts }) = Drizzle::new(client, Schema::new());
    db.create()?;

    db.insert(accounts)
        .values([InsertAccounts::new("Acme")])
        .execute()?;

    let rows: Vec<SelectAccounts> = db.select(()).from(accounts).all()?;
    println!("accounts: {rows:?}");
    Ok(())
}
```

> For async, use `drizzle::postgres::tokio::Drizzle` with `tokio_postgres::connect`.

## CLI (migrations)

```bash
drizzle init -d sqlite          # dialects: sqlite, turso, postgresql, mysql, singlestore
drizzle generate                # generate migrations from schema changes
drizzle migrate                 # apply pending migrations
```

## License

MIT — see [LICENSE](LICENSE).
