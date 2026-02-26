# Drizzle RS

A type-safe SQL query builder and ORM for Rust, inspired by Drizzle ORM.

> [!WARNING]
> This project is still evolving. Expect breaking changes.

## Getting Started

### 1. Install

Add the library and install the CLI:

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
drizzle init --dialect sqlite
```

This creates `drizzle.config.toml`. Point it at your schema and database:

```toml
dialect = "sqlite"
schema = "src/schema.rs"
out = "./drizzle"

[dbCredentials]
url = "./dev.db"
```

### 3. Define your schema

Write your schema in `src/schema.rs`. Each `#[SQLiteTable]` generates companion types for selecting, inserting, updating, and partial queries (see [Generated Models](#generated-models)).

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

Alternatively, use `drizzle new` for an interactive schema builder, or `drizzle introspect` to reverse-engineer a schema from an existing database.

### 4. Migrate

```bash
drizzle generate              # diff schema → SQL migration files
drizzle generate --name init  # name the migration
drizzle migrate               # apply pending migrations
drizzle push                  # skip migration files, apply schema diff directly
```

> `push` is useful during development. Use `generate` + `migrate` for production.

### 5. Connect & query

```rust
use drizzle::sqlite::rusqlite::Drizzle;

let conn = rusqlite::Connection::open("app.db")?;
let (db, Schema { users, posts }) = Drizzle::new(conn, Schema::new());
```

> See [`examples/rusqlite.rs`](examples/rusqlite.rs) for a full runnable example.

## CLI Reference

| Command | Description |
|---------|-------------|
| `drizzle init` | Create a new `drizzle.config.toml` |
| `drizzle new` | Interactive schema builder (`--json` for JSON input) |
| `drizzle generate` | Diff schema and emit SQL migration files (`--custom` for an empty migration) |
| `drizzle migrate` | Apply pending migrations (`--plan` to preview, `--safe` to verify first) |
| `drizzle push` | Apply schema diff directly without migration files (`--explain` for dry run) |
| `drizzle introspect` | Reverse-engineer schema from a live database (`--init` to baseline) |
| `drizzle status` | Show which migrations have been applied |
| `drizzle check` | Validate your config file |
| `drizzle export` | Print the schema as raw SQL (`--sql file.sql` to write to file) |
| `drizzle up` | Upgrade migration snapshots to the latest format |

> `drizzle pull` is an alias for `introspect`. All commands accept `-c <path>` to use a custom config file and `--db <name>` for multi-database configs.

## Generated Models

Each `#[SQLiteTable]` (or `#[PostgresTable]`) generates four companion types from your struct. Given:

```rust
#[SQLiteTable]
pub struct Users {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub name: String,
    pub email: Option<String>,
    pub age: i64,
}
```

| Model | Purpose | Fields |
|-------|---------|--------|
| `SelectUsers` | Query results | Matches the table columns exactly |
| `InsertUsers` | Insert rows | `new(name, age)` requires non-default fields; `with_email(...)` for optional ones |
| `UpdateUsers` | Update rows | `default()` starts empty; `with_age(27)` sets fields to update |
| `PartialSelectUsers` | Selective columns | All fields `Option<T>`; `with_name()` picks which columns to include |

### Insert models

`new()` takes only the required fields (columns without a default or autoincrement). Chain `with_*` methods for optional fields:

```rust
InsertUsers::new("Alex Smith", 26i64)
    .with_email("alex@example.com")
```

### Update models

Start from `default()` and set only the fields you want to change. The query won't compile unless at least one field is set:

```rust
UpdateUsers::default()
    .with_age(27)
    .with_email("new@example.com")
```

### Partial select models

Pick specific columns to return. Unselected fields come back as `None`:

```rust
let partial: Vec<PartialSelectUsers> = db
    .select(PartialSelectUsers::default().with_name().with_email())
    .from(users)
    .all()?;
```

## CRUD

```rust
use drizzle::core::expr::{eq, gt, in_subquery, min, row};
```

### Select

```rust
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

`SELECT` builders are expressions — pass them directly into comparisons and set operators:

```rust
let min_id = db.select(min(users.id)).from(users);
let newer: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where(gt(users.id, min_id))
    .all()?;

let exact_rows = db
    .select((users.id, users.name))
    .from(users)
    .r#where(eq(users.name, "Alex Smith"));

let matched: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where(in_subquery(row((users.id, users.name)), exact_rows))
    .all()?;
```

### Insert

```rust
// Single row
db.insert(users)
    .value(InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"))
    .execute()?;

// Multiple rows
db.insert(users)
    .values([
        InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"),
        InsertUsers::new("Jordan Lee", 30i64),
    ])
    .execute()?;
```

### Update

```rust
db.update(users)
    .set(UpdateUsers::default().with_age(27))
    .r#where(eq(users.id, 1))
    .execute()?;
```

### Delete

```rust
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
        .value(InsertUsers::new("Alice", 28i64))
        .execute()?;

    let all: Vec<SelectUsers> = tx.select(()).from(users).all()?;

    Ok(all.len())
})?;
```

Savepoints nest inside transactions — a failed savepoint rolls back without aborting the outer transaction:

```rust
use drizzle::sqlite::connection::SQLiteTransactionType;
use drizzle::core::error::DrizzleError;

let count = db.transaction(SQLiteTransactionType::Deferred, |tx| {
    tx.insert(users)
        .value(InsertUsers::new("Alice", 28i64))
        .execute()?;

    // This savepoint fails and rolls back, but the outer transaction continues
    let _ = tx.savepoint(|stx| {
        stx.insert(users)
            .value(InsertUsers::new("Bad Data", -1i64))
            .execute()?;
        Err(DrizzleError::Other("rollback this part".into()))
    });

    // Alice is still inserted
    tx.insert(users)
        .value(InsertUsers::new("Bob", 32i64))
        .execute()?;

    let all: Vec<SelectUsers> = tx.select(()).from(users).all()?;
    Ok(all.len())
})?;
```

> PostgreSQL uses `PostgresTransactionType` (e.g. `ReadCommitted`, `Serializable`) instead.

## Prepared Statements

Placeholders are created from columns — wrong bind types fail at compile time.

```rust
use drizzle::core::expr::eq;

let name = users.name.placeholder("name");

let find = db
    .select(())
    .from(users)
    .r#where(eq(users.name, name))
    .prepare();

let alice: Vec<SelectUsers> = find.all(db.conn(), [name.bind("Alice")])?;
let bob: Vec<SelectUsers> = find.all(db.conn(), [name.bind("Bob")])?;
// name.bind(42) — compile error: Integer is not compatible with Text
```

Placeholders work in update (and insert) models too:

```rust
let new_name = users.name.placeholder("new_name");
let target = users.id.placeholder("target");

let stmt = db
    .update(users)
    .set(UpdateUsers::default().with_name(new_name))
    .r#where(eq(users.id, target))
    .prepare();

stmt.execute(db.conn(), [new_name.bind("New Name"), target.bind(1)])?;
```

> Use `.prepare().into_owned()` to convert a prepared statement into a self-contained value that can be stored or moved freely.

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

## Aliases

Use a `Tag` to create compile-time-safe aliases for self-joins and CTEs.
The `tag!` macro defines one in a single line:

```rust
use drizzle::sqlite::prelude::*;

tag!(U, "u");

let u = Users::alias::<U>();
let rows: Vec<(i64,)> = db.select((u.id,)).from(u).all()?;
```

Every alias or CTE is created through a `Tag` type (`alias::<Tag>()` / `into_cte::<Tag>()`),
which embeds its SQL name at compile time.

## Cast Targets

Each dialect provides cast target markers for use with `cast()`:

```rust
use drizzle::core::expr::cast;

// SQLite
let age = cast(json_age, drizzle::sqlite::types::Integer);

// PostgreSQL
let age = cast(user.age, drizzle::postgres::types::Int4);
```

Use a string cast target only when you need a custom SQL type name.

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

Same pattern as SQLite, but with Postgres macros.

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
        .value(InsertAccounts::new("Acme"))
        .execute()?;

    let rows: Vec<SelectAccounts> = db.select(()).from(accounts).all()?;
    println!("accounts: {rows:?}");
    Ok(())
}
```

> For async, use `drizzle::postgres::tokio::Drizzle` with `tokio_postgres::connect`.

## Runtime Migrations

Apply migrations at startup without the CLI. Load your migration folder into a `MigrationSet` and call `db.migrate()`.

```rust
use drizzle::migrations::MigrationSet;
use drizzle::Dialect;

let migrations = MigrationSet::from_dir("./drizzle", Dialect::Sqlite)?;
db.migrate(&migrations)?;
```

`migrate` creates the internal bookkeeping table on first run and skips migrations that have already been applied.

### Push

For development, `push` skips migration files entirely — it introspects the live database, diffs it against your schema, and applies the changes directly.

```rust
let schema = Schema::new();
db.push(&schema)?;
```

> `push` is intended for rapid iteration. Use `migrate` with versioned migration files for production deployments.

## License

MIT — see [LICENSE](LICENSE).
