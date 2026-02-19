# Drizzle RS

A type-safe SQL query builder / ORM-ish layer for Rust, inspired by Drizzle ORM.

> [!WARNING]
> This project is still evolving. Expect breaking changes.

## Getting Started

### 1. Install

```toml
[dependencies]
drizzle = { git = "https://github.com/themixednuts/drizzle-rs", features = ["rusqlite"] }
# drivers: rusqlite | libsql | turso | postgres-sync | tokio-postgres
```

```bash
cargo install drizzle-cli --git https://github.com/themixednuts/drizzle-rs --locked --all-features
```

### 2. Initialize & configure

```bash
drizzle init -d sqlite    # creates drizzle.config.toml
```

```toml
# drizzle.config.toml
dialect = "sqlite"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
url = "./dev.db"
```

### 3. Define your schema

**New project** — write your schema in `src/schema.rs`. Each `#[SQLiteTable]` generates `Select*`, `Insert*`, and `Update*` companion types.

```rust
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

**Interactive wizard** — build a schema step-by-step with guided prompts:

```bash
drizzle new                   # interactive schema builder
drizzle new --json            # read schema definition from stdin as JSON
drizzle new --json --from schema.json   # read JSON from a file
drizzle new --export-json schema.json   # export schema as JSON after building
drizzle new --schema-help     # print the expected JSON shape and exit
```

**Existing database** — pull the schema from a live database instead:

```bash
drizzle introspect            # generates a schema snapshot from the database
drizzle introspect --init     # also initializes migration metadata as a baseline
```

> `drizzle pull` is an alias for `introspect`. Use `--tablesFilter` to include/exclude tables by glob pattern.

### 4. Generate & apply migrations

```bash
drizzle generate    # diff schema → SQL migration files
drizzle migrate     # apply pending migrations
```

### 5. Connect & query

```rust
use drizzle::sqlite::rusqlite::Drizzle;

let conn = rusqlite::Connection::open("app.db")?;
let (db, Schema { users, posts }) = Drizzle::new(conn, Schema::new());
```

> See [`examples/rusqlite.rs`](examples/rusqlite.rs) for a full runnable example.

## CRUD

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

### Insert

```rust
db.insert(users)
    .values([
        InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"),
        InsertUsers::new("Jordan Lee", 30i64),
    ])
    .execute()?;
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

## Transactions

Transactions auto-rollback on error or panic. Return `Ok(value)` to commit, `Err(...)` to rollback.

```rust
use drizzle::sqlite::connection::SQLiteTransactionType;

db.transaction(SQLiteTransactionType::Deferred, |tx| {
    tx.insert(users)
        .values([InsertUsers::new("Alice", 28i64)])
        .execute()?;

    let all: Vec<SelectUsers> = tx.select(()).from(users).all()?;

    Ok(all.len())
})?;
```

Savepoints nest inside transactions — a failed savepoint rolls back without aborting the outer transaction:

```rust
use drizzle::core::error::DrizzleError;

let count = db.transaction(SQLiteTransactionType::Deferred, |tx| {
    tx.insert(users)
        .values([InsertUsers::new("Alice", 28i64)])
        .execute()?;

    // This savepoint fails and rolls back, but the outer transaction continues
    let _ = tx.savepoint(|stx| {
        stx.insert(users)
            .values([InsertUsers::new("Bad Data", -1i64)])
            .execute()?;
        Err(DrizzleError::Other("rollback this part".into()))
    });

    // Alice is still inserted
    tx.insert(users)
        .values([InsertUsers::new("Bob", 32i64)])
        .execute()?;

    let all: Vec<SelectUsers> = tx.select(()).from(users).all()?;
    Ok(all.len())
})?;
```

> PostgreSQL uses `PostgresTransactionType` (e.g. `ReadCommitted`, `Serializable`) instead.

## Joins

Use `#[derive(SQLiteFromRow)]` to map columns from multiple tables into a flat struct.
`#[from(Users)]` sets the default source table for unannotated fields:

```rust
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;

#[derive(SQLiteFromRow, Debug)]
#[from(Users)]
struct UserWithPost {
    #[column(Users::id)]
    user_id: i64,
    name: String,
    #[column(Posts::id)]
    post_id: i64,
    #[column(Posts::content)]
    content: Option<String>,
}

// Explicit ON condition
let rows: Vec<UserWithPost> = db
    .select(UserWithPost::Select)
    .from(users)
    .left_join((posts, eq(users.id, posts.author_id)))
    .all()?;

// Auto-FK: derives the ON condition from #[column(references = ...)]
let rows: Vec<UserWithPost> = db
    .select(UserWithPost::Select)
    .from(users)
    .left_join(posts)
    .all()?;
```

## Typed Aliases (`Tag`)

Use a `Tag` to create compile-time-safe aliases for self-joins, CTEs, and
typed `Select` models.

```rust
use drizzle::sqlite::prelude::*;

struct U;
impl drizzle::core::Tag for U {
    const NAME: &'static str = "u";
}

let u = Users::alias::<U>();
let rows: Vec<(i64,)> = db.select((u.id,)).from(u).all()?;
```

`alias_named(...)` is still available for runtime names (used internally by
query builders), but metadata traits like `columns()` return base table static
metadata in that mode.

## Order By / Limit / Offset

```rust
let rows: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .order_by([asc(users.name)])
    .limit(10)
    .offset(20)
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

## License

MIT — see [LICENSE](LICENSE).
