# Drizzle RS

A type-safe SQL query builder and ORM for Rust, inspired by Drizzle ORM.

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

### 2. Initialize

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

Or use `drizzle introspect` to reverse-engineer a schema from an existing database.

### 4. Connect & query

```rust
use drizzle::sqlite::rusqlite::Drizzle;

let conn = rusqlite::Connection::open("app.db")?;
let (db, Schema { users, posts }) = Drizzle::new(conn, Schema::new());
```

> See [`examples/rusqlite.rs`](examples/rusqlite.rs) for a full runnable example.

## Migrations

Drizzle offers three ways to get schema changes into your database. Pick one — these are alternatives, not layers.

| Approach | When | Files on disk |
|---|---|---|
| CLI `drizzle migrate` | Most deploys; migrations run outside the app | Yes — committed SQL |
| Runtime `db.migrate(...)` | Apply the same files from your app at startup | Yes — committed SQL |
| `db.push(schema)` | Rapid local development | No — live diff |

### Generating migration files

```bash
drizzle generate              # diff schema -> SQL migration files
drizzle generate --name init  # name the migration
```

### Applying with the CLI

```bash
drizzle migrate
```

### Applying from your app

```rust
use drizzle::migrations::Tracking;

let migrations = drizzle::include_migrations!("./drizzle");
db.migrate(&migrations, Tracking::SQLITE)?;
```

Use `Tracking::POSTGRES` for PostgreSQL, and override the tracking table or schema when needed:

```rust
db.migrate(
    &migrations,
    Tracking::POSTGRES
        .schema("drizzle")
        .table("__drizzle_migrations"),
)?;
```

`migrate` creates the tracking schema/table if needed and skips migrations that have already been applied.

### Generating from `build.rs`

To avoid running `drizzle generate` manually, keep `./drizzle` in sync from the build script. This replaces the generation step; it does not add another migration system.

```toml
[build-dependencies]
drizzle-migrations = { git = "https://github.com/themixednuts/drizzle-rs" }
drizzle-types = { git = "https://github.com/themixednuts/drizzle-rs" }
```

```rust
use drizzle_migrations::build::{Config, Output, run};
use drizzle_types::Dialect;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::new(Dialect::SQLite)
        .file("src/schema.rs")
        .out("./drizzle");

    cfg.watch();

    match run(&cfg)? {
        Output::NoChanges => {}
        Output::Generated { tag, path, .. } => {
            println!("cargo:warning=generated migration {tag} at {}", path.display());
        }
    }

    Ok(())
}
```

For schemas split across files, chain `.file(...)` calls: `.file("src/schema/users.rs").file("src/schema/posts.rs")`.

### Push (dev only)

```rust
let schema = Schema::new();
db.push(&schema)?;
```

`push` skips migration files entirely and applies the live schema diff directly. Great for iteration, not for production.

## Generated Models

Given the schema above, each `#[SQLiteTable]` (or `#[PostgresTable]`) generates four helper types:

| Model | Purpose | Fields |
|-------|---------|--------|
| `SelectUsers` | Query results | Matches the table columns exactly |
| `InsertUsers` | Insert rows | `new(name, age)` requires non-default fields; `with_email(...)` for optional ones |
| `UpdateUsers` | Update rows | `default()` starts empty; `with_age(27)` sets fields to update |
| `PartialSelectUsers` | Selective columns | All fields `Option<T>`; `with_name()` picks which columns to include |

### Insert

`new()` takes only the required fields (columns without a default or autoincrement). Chain `with_*` for optional fields:

```rust
InsertUsers::new("Alex Smith", 26i64)
    .with_email("alex@example.com")
```

### Update

Start from `default()` and set only the fields you want to change. The query won't compile unless at least one field is set:

```rust
UpdateUsers::default()
    .with_age(27)
    .with_email("new@example.com")
```

### Partial select

Pick specific columns to return. Unselected fields come back as `None`:

```rust
let partial: Vec<PartialSelectUsers> = db
    .select(PartialSelectUsers::default().with_name().with_email())
    .from(users)
    .all()?;
```

## Querying

```rust
use drizzle::core::expr::*;
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
let names: Vec<(i64, String)> = db
    .select((users.id, users.name))
    .from(users)
    .all()?;

// Multiple conditions
let active_adults: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where(and(gt(users.age, 18), eq(users.name, "Alex Smith")))
    .all()?;

// Or
let rows: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where(eq(users.name, "Alice") | eq(users.name, "Bob"))
    .all()?;
```

#### Ordering, limiting, pagination

```rust
let rows: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .order_by(asc(users.name))
    .limit(10)
    .offset(20)
    .all()?;

// Multiple sort keys
.order_by([asc(users.name), desc(users.age)])
```

#### Group by

```rust
db.select((users.name, alias(count(users.id), "total")))
    .from(users)
    .group_by(users.name)
    .having(gt(count(users.id), 1))
    .all()?;

// Multiple group columns
db.select((users.name, users.age, alias(count(users.id), "total")))
    .from(users)
    .group_by((users.name, users.age))
    .all()?;
```

### Insert

```rust
// Single row
db.insert(users)
    .value(InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"))
    .execute()?;

// Multiple rows — all rows must set the same optional fields
db.insert(users)
    .values([
        InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"),
        InsertUsers::new("Jordan Lee", 30i64).with_email("jordan@example.com"),
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

### Joins

Use `#[derive(SQLiteFromRow)]` to map columns from multiple tables into a flat struct. `#[from(Users)]` sets the default source table for unannotated fields:

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

### Subqueries & set operations

`SELECT` builders are expressions — pass them directly into comparisons or `IN`:

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
    .r#where(in_subquery((users.id, users.name), exact_rows))
    .all()?;
```

Combine queries with `union`, `union_all`, `intersect`, and `except`. `union` removes duplicates; `union_all` keeps them:

```rust
let results: Vec<(String,)> = db
    .select((users.name,))
    .from(users)
    .r#where(lte(users.age, 25))
    .union(
        db.select((users.name,))
          .from(users)
          .r#where(gte(users.age, 30))
    )
    .order_by(asc(users.name))
    .all()?;
```

### Aliases

Use a `Tag` to alias a table for self-joins:

```rust
use drizzle::sqlite::prelude::*;

tag!(U, "u");

let u = Users::alias::<U>();
let rows: Vec<(i64,)> = db.select((u.id,)).from(u).all()?;
```

## Expressions

Aggregate functions and common SQL expressions:

```rust
// Aggregates
let total: (i64,) = db.select((count(users.id),)).from(users).get()?;
let oldest: (Option<i64>,) = db.select((max(users.age),)).from(users).get()?;

// Coalesce — first non-null value
let rows: Vec<(String,)> = db
    .select((coalesce(users.email, "unknown"),))
    .from(users)
    .all()?;
```

Available in `drizzle::core::expr`: `count`, `sum`, `avg`, `min`, `max`, `coalesce`, `abs`, `upper`, `lower`, `length`, and more.

### Type casting

Each dialect provides cast target markers for use with `cast()`. Pass a string when you need a custom SQL type name.

```rust
use drizzle::core::expr::cast;

// SQLite
let age = cast(json_age, drizzle::sqlite::types::Integer);

// PostgreSQL
let age = cast(user.age, drizzle::postgres::types::Int4);
```

## Relational Queries

Requires the `query` feature. Fetches a table with its relations in a single query — no manual joins.

Relation methods are auto-generated from `#[column(references = ...)]` foreign keys. Given `Posts.author_id → Users.id`, calling `users.posts()` returns the reverse (one-to-many) relation and `posts.author()` returns the forward (many-to-one) relation.

```rust
// Users with their posts
let users = db.query(users)
    .with(users.posts())
    .find_many()?;

for user in &users {
    println!("{}: {} posts", user.name, user.posts().len());
}
```

`.find_first()` returns `Option<QueryRow<...>>` instead of `Vec`:

```rust
let user = db.query(users)
    .with(users.posts())
    .r#where(eq(users.name, "Alice"))
    .find_first()?;
```

Relations nest — fetch users with their posts and each post's comments:

```rust
let users = db.query(users)
    .with(users.posts().with(posts.comments()))
    .find_many()?;

let first_post = &users[0].posts()[0];
println!("{} comments", first_post.comments().len());
```

Multiple relations on the same table:

```rust
let users = db.query(users)
    .with(users.posts())
    .with(users.invited_by())
    .find_many()?;
```

Supports `where`, `order_by`, `limit`, and `offset` on the root query:

```rust
let users = db.query(users)
    .with(users.posts())
    .r#where(gt(users.age, 25))
    .order_by(asc(users.name))
    .limit(10)
    .find_many()?;
```

Each table generates convenient type aliases for use in function signatures:

```rust
use schema::{UsersQueryRow, UsersWithPosts, QueryUsersPosts};

fn print_user_posts(user: &UsersQueryRow<UsersWithPosts>) {
    println!("{} has {} posts", user.name, user.posts().len());
}
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

Use `.prepare().into_owned()` to convert a prepared statement into a self-contained value that can be stored or moved freely.

## PostgreSQL

Everything above works with `#[PostgresTable]`, `#[derive(PostgresSchema)]`, and `drizzle::postgres::{sync,tokio}::Drizzle`. Transactions take `PostgresTransactionType` (e.g. `ReadCommitted`, `Serializable`) in place of `SQLiteTransactionType`.

```rust
use drizzle::postgres::prelude::*;
use drizzle::postgres::sync::Drizzle;

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

let client = postgres::Client::connect(
    "host=localhost user=postgres password=postgres dbname=drizzle_test",
    postgres::NoTls,
)?;
let (mut db, Schema { accounts }) = Drizzle::new(client, Schema::new());
```

## CLI Reference

Most projects only need these:

| Command | Description |
|---------|-------------|
| `drizzle init` | Create `drizzle.config.toml` |
| `drizzle generate` | Diff schema and emit SQL migration files |
| `drizzle migrate` | Apply pending migrations |
| `drizzle push` | Apply schema diff directly without migration files |
| `drizzle introspect` | Reverse-engineer schema from a live database |

Other useful commands:

| Command | Description |
|---------|-------------|
| `drizzle new` | Interactive schema builder |
| `drizzle status` | Show applied migrations |
| `drizzle check` | Validate config |
| `drizzle export` | Print schema as raw SQL |
| `drizzle up` | Upgrade migration snapshots to the latest format |

`drizzle pull` is an alias for `introspect`. All commands accept `-c <path>` for a custom config file and `--db <name>` for multi-database configs.

## License

MIT — see [LICENSE](LICENSE).
