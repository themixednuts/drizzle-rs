# Drizzle RS

A type-safe SQL query builder and ORM for Rust, inspired by Drizzle ORM.

> [!WARNING]
> This project is still evolving. Expect breaking changes.

## Contents

- [Getting Started](#getting-started)
  - [1. Install](#1-install)
  - [2. Initialize](#2-initialize)
  - [3. Define Your Schema](#3-define-your-schema)
  - [4. Connect & Query](#4-connect--query)
- [Feature Flags](#feature-flags)
- [Migrations](#migrations)
  - [Manual: Generate with the CLI](#manual-generate-with-the-cli)
  - [Automatic: Generate from build.rs](#automatic-generate-from-buildrs)
  - [Applying Migrations](#applying-migrations)
  - [Push (Dev Only)](#push-dev-only)
- [Porting from drizzle-orm (TypeScript)](#porting-from-drizzle-orm-typescript)
  - [Keep the Migration History](#keep-the-migration-history)
  - [What to Port by Hand](#what-to-port-by-hand)
  - [Schema Cheat Sheet](#schema-cheat-sheet)
- [Generated Models](#generated-models)
  - [Insert](#insert)
  - [Update](#update)
  - [JSON Columns](#json-columns)
- [Querying](#querying)
  - [Select](#select)
    - [Ordering, Limiting, Pagination](#ordering-limiting-pagination)
    - [Group By](#group-by)
  - [Insert](#insert-1)
  - [Update](#update-1)
  - [Delete](#delete)
  - [Joins](#joins)
  - [Subqueries & Set Operations](#subqueries--set-operations)
  - [Aliases](#aliases)
- [Expressions](#expressions)
  - [Type Casting](#type-casting)
- [Relational Queries](#relational-queries)
  - [Relation Names](#relation-names)
  - [Selecting Specific Columns](#selecting-specific-columns)
  - [Result Types](#result-types)
- [Transactions](#transactions)
- [Prepared Statements](#prepared-statements)
- [Seeding](#seeding)
- [PostgreSQL](#postgresql)
- [MySQL](#mysql)
- [CLI Reference](#cli-reference)
  - [Renames](#renames)
- [License](#license)

## Getting Started

### 1. Install

```toml
[dependencies]
drizzle = { version = "0.3", features = ["rusqlite"] }
rusqlite = { version = "0.39", features = ["bundled"] }
```

Pick the driver feature that matches the client your application already
uses. You create and own the connection; drizzle wraps it.

Install the CLI with the drivers it should connect through:

```bash
cargo install drizzle-cli --locked --features sqlite-all   # or postgres-all, mysql-all
```

Individual driver features (`rusqlite`, `postgres-sync`, `mysql-async`, ...)
work too. Without a driver, `generate` still works, but `migrate`, `push`, and
`introspect` stop with a "No driver available" error.

See [Feature Flags](#feature-flags) for every driver and optional column type.

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

### 3. Define Your Schema

```rust
# #[cfg(feature = "rusqlite")]
# fn main() {
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

#[SQLiteTable]
pub struct Comments {
    #[column(primary, autoincrement)]
    pub id: i64,
    pub body: String,
    #[column(references = Posts::id)]
    pub post_id: i64,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
    pub comments: Comments,
}
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

Each table also gets a module of column types named after it: `users::Name` is the type of `Users::name`, should you need to name it. The module keeps these types apart from your own, so a `User` table can have a `role: UserRole` column.

If you already have a database, run `drizzle introspect` to reverse-engineer the schema instead of writing it by hand.

### 4. Connect & Query

```rust,no_run
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::Schema;
use drizzle::sqlite::rusqlite::Drizzle;

let conn = rusqlite::Connection::open("app.db")?;
let (db, Schema { users, posts, comments }) = Drizzle::new(conn);
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

`Drizzle::new` builds the schema value itself, and the pattern that
destructures it names its type. When nothing else names it, put the type on
the call: `Drizzle::<Schema>::new(conn)` (MySQL: `Drizzle::<_, Schema>::new(conn)`).
Use `let (db, ()) = Drizzle::new(conn)` to run queries with no schema.

> [!NOTE]
> See [`examples/rusqlite.rs`](https://github.com/themixednuts/drizzle-rs/blob/main/examples/rusqlite.rs) for a full runnable example.

## Feature Flags

| Feature | What it enables |
|---------|-----------------|
| `rusqlite`, `libsql`, `turso` | SQLite drivers |
| `d1`, `durable` | Cloudflare D1 and Durable Object SQLite (`wasm32` only) |
| `postgres-sync`, `tokio-postgres` | PostgreSQL drivers |
| `hyperdrive` | Cloudflare Hyperdrive over `tokio-postgres` (`wasm32` only) |
| `aws-data-api` | AWS Aurora Serverless Data API (PostgreSQL over HTTP) |
| `mysql-sync`, `mysql-async` | MySQL drivers (`mysql` / `mysql_async`) |
| `query` | [Relational queries](#relational-queries) (`db.query(...)`) |
| `serde` | [JSON columns](#json-columns) |
| `uuid`, `chrono`, `time`, `jiff`, `rust-decimal` | Column types from those crates |
| `arrayvec`, `compact-str`, `bytes`, `smallvec-types` | Inline and zero-copy string/byte column types |
| `cidr`, `geo-types`, `bit-vec` | PostgreSQL network, geometric, and bit-string types |
| `math` | SQLite math functions (see [Expressions](#expressions)) |
| `tracing`, `profiling` | Query spans and puffin profiling scopes |

## Migrations

You have two workflows for keeping migration files in sync with your schema. Pick one — both produce the same committed SQL; the difference is whether you regenerate by hand or let `cargo` do it.

| Workflow | Generate migrations | Best for |
|---|---|---|
| **Manual** | Run `drizzle generate` yourself | Teams that want explicit control over when migrations are produced |
| **Automatic** | Regenerated when watched schema/config inputs change during `cargo build` | Solo dev or small teams who want schema and migrations to stay in lockstep |

Both workflows apply migrations the same way — either with the CLI at deploy time, or from your app at startup. For local iteration without committed files at all, see [Push (Dev Only)](#push-dev-only).

### Manual: Generate with the CLI

Run `drizzle generate` whenever you change your schema, then commit the resulting SQL files:

```bash
drizzle generate              # diff schema -> SQL migration files
drizzle generate --name init  # optional: name the migration
```

After a git merge brings in migrations generated on another branch,
`generate` (like drizzle-kit) diffs against the combined result of both
branches and records both as the new migration's parents (`prevIds`). If the
branches changed the same objects it stops with a conflict report; regenerate
one branch's migration on top of the other, or pass `--ignore-conflicts` to
diff against the newest migration only.

If a change could be a rename, `drizzle generate` asks (or, without a terminal, reads `--hints`); see [Renames](#renames).

### Automatic: Generate from `build.rs`

Add `drizzle-migrations` as a build dependency, then point it at your existing `drizzle.config.toml`. Migration files regenerate themselves whenever your schema changes — you commit them the same way as the manual workflow, you just never run `drizzle generate` by hand.

```toml
[build-dependencies]
drizzle = { version = "0.3", features = ["rusqlite"] }
drizzle-migrations = "0.3"
rusqlite = { version = "0.39", features = ["bundled"] }
```

```rust,no_run
use drizzle_migrations::build::{Config, Output, run};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::from_toml("drizzle.config.toml")?;
    cfg.watch();

    if let Output::Generated { tag, .. } = run(&cfg)? {
        println!("cargo:warning=generated migration {tag}");
    }

    Ok(())
}
```

`cfg.watch()` tells cargo to rerun `build.rs` whenever a schema file, `drizzle.config.toml`, or a referenced env var changes.

### Applying Migrations

Once migration files exist, apply them one of three ways. They all use the same SQL files and tracking table — pick whichever fits your environment.

**At deploy time, with the CLI:**

```bash
drizzle migrate
```

**At app startup, from your code:**

```text
use drizzle::migrations::Tracking;

let migrations = drizzle::include_migrations!("./drizzle");
db.migrate(&migrations, Tracking::SQLITE)?;
```

Use `Tracking::POSTGRES` for PostgreSQL and `Tracking::MYSQL` for MySQL. All
three record applied migrations in a `__drizzle_migrations` table, which
PostgreSQL keeps in a `drizzle` schema. Override the tracking table or schema
when you need to:

```text
db.migrate(
    &migrations,
    Tracking::POSTGRES
        .schema("ops")
        .table("schema_migrations"),
)?;
```

> [!NOTE]
> **PostgreSQL transaction scope differs from drizzle-orm.** drizzle-orm's
> PostgreSQL `migrate` runs every pending migration inside one transaction, so
> a failure leaves none of them applied. drizzle-rs's PostgreSQL `db.migrate`
> (`postgres-sync`, `tokio-postgres`) commits each migration in its own
> transaction together with its tracking row: when a later migration fails, the
> earlier ones from the same call stay applied, and the next call resumes at
> the failed one. `drizzle migrate` on PostgreSQL keeps drizzle-orm's single
> transaction. `CREATE/DROP INDEX CONCURRENTLY` cannot run in a transaction at
> all: `db.migrate` runs a migration containing it statement by statement
> behind a dirty marker (and `drizzle migrate` does that for every migration in
> such a run); finish an interrupted one with `drizzle migrate --repair` or
> `migrate_with_repair`.
> On SQLite, rusqlite's `db.migrate` also uses one transaction for the batch.

Migration files are split the way drizzle-orm splits them: a file with
`--> statement-breakpoint` markers runs one chunk at a time, each chunk
exactly as written (PostgreSQL and MySQL accept several statements in one
chunk; SQLite takes one statement per chunk). A hand-written file without
markers is split on top-level semicolons, honoring the dialect's quoting,
comment, and stored-program syntax.

MySQL DDL implicitly commits, so MySQL migrations do not pretend to be
transactional. The CLI takes a database-scoped advisory lock, writes a durable
dirty marker before the first statement, and marks the migration complete only
after every statement succeeds. If a migration fails or is interrupted, inspect
the partially applied DDL before retrying; automatic MySQL repair is deliberately
unsupported because the server may already have committed some statements.

**During `cargo build`, by extending the `build.rs` from above.** Set `DRIZZLE_MIGRATE=1` in your dev environment and your local database stays in lockstep with the schema:

```text
use drizzle::sqlite::rusqlite::Drizzle;
use drizzle_migrations::{MigrateOutcome, MigrationDir};

// `cfg.watch()` does not watch this flag; without this line, cargo would not
// rerun build.rs when you set or unset it.
println!("cargo:rerun-if-env-changed=DRIZZLE_MIGRATE");

if std::env::var("DRIZZLE_MIGRATE").is_ok() {
    let conn = rusqlite::Connection::open(cfg.url()?)?;
    let (db, ()) = Drizzle::new(conn);
    let migrations = MigrationDir::new(cfg.out_dir()).discover()?;

    if let MigrateOutcome::Applied { tags } = db.migrate(&migrations, cfg.tracking())? {
        println!("cargo:warning=applied {} migration(s)", tags.len());
    }
}
```

`cfg.tracking()` returns the same `Tracking` value the runtime path uses — just sourced from `drizzle.config.toml` instead of hardcoded.

`migrate` creates the tracking schema/table if needed and skips migrations that have already been applied. Without `DRIZZLE_MIGRATE`, `cargo build` only generates files and never touches the database.

### Push (Dev Only)

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
# let (mut db, _) = readme::database()?;
let schema = Schema::new();
db.push(&schema)?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

`push` skips migration files entirely and applies the live schema diff directly.

> [!CAUTION]
> `push` is for local iteration only. It bypasses the migration tracking table and offers no audit trail. Never run it against a production database.

## Porting from drizzle-orm (TypeScript)

drizzle-kit records your TypeScript schema in every migration's snapshot, so
`drizzle import` writes the Rust schema from those snapshots. It reads no
TypeScript. Point it at the folder drizzle-kit writes to (`out` in
`drizzle.config.ts`):

```bash
drizzle import ./drizzle                        # writes src/schema.rs
drizzle import ./drizzle --out src/db/schema.rs
drizzle import ./drizzle/meta/0007_snapshot.json --out -   # one snapshot, to stdout
```

It takes either migration layout drizzle-kit writes (`meta/_journal.json` from
drizzle-kit 0.x, or one folder per migration from 1.x) and uses the newest
snapshot. The dialect comes from the snapshot; `--dialect turso` imports a
SQLite snapshot for Turso. The schema goes to `--out`, else the config's
`schema`, else `src/schema.rs`, and an existing file is only replaced with
`--force`. Field names are `snake_case` unless `--casing camel` (or
`[introspect] casing` in the config) says otherwise; the SQL names are kept
with `name = "..."` wherever they differ. The command prints what it could not
express and the steps below.

### Keep the Migration History

drizzle-rs uses drizzle-kit's migration folders and tracking table, so the
migrations you already ran stay applied:

1. Point `out` in `drizzle.config.toml` at the existing folder (or pass
   `--init-config` to `drizzle import` to write a starter config that does):

   ```toml
   dialect = "postgresql"
   schema = "src/schema.rs"
   out = "./drizzle"
   ```

2. If the folder has `meta/_journal.json` (drizzle-kit 0.x), convert it to the
   folder layout with `drizzle up` (or pass `--upgrade` to `drizzle import`).
   This moves the files in place; the SQL is not changed.
3. `drizzle migrate` reads the `__drizzle_migrations` table drizzle-orm wrote
   (in the `drizzle` schema on PostgreSQL) and runs only migrations that are
   not recorded there. If `drizzle.config.ts` set `migrations.table` or
   `migrations.schema`, set the same under `[migrations]`.
4. `drizzle generate` should now report no changes. From here on, edit the
   Rust schema and generate migrations as usual.

### What to Port by Hand

Snapshots describe the database, not the TypeScript around it:

- `relations()`: drizzle-rs derives relations from foreign keys, including
  one-to-one and many-to-many, and names them (see
  [Relation Names](#relation-names)). An `author_id` column with
  `#[column(references = Users::id)]` gives `posts.author()` and
  `users.author_posts()` in the relational query API.
- `$type<T>()` and column modes (`{ mode: 'json' | 'timestamp' | 'bigint' }`):
  fields get the column's storage type. Switch to your Rust type, for example
  `#[column(json)]` with a `serde` type, or an enum deriving `SQLiteEnum`.
- `$default()`, `$defaultFn()`, `$onUpdate()`: values computed in JavaScript.
  Use `#[column(default_fn = path)]` or set them in code.
- `customType()`: the column keeps its SQL type; give it a Rust type that
  implements the dialect's column trait (`DrizzlePostgresColumn` and friends).
- PostgreSQL sequences, `numeric` and `interval` columns, and index ordering
  (`.desc()`, `.nullsFirst()`) have no Rust schema equivalent yet; the import
  warns about each one, and `drizzle generate` would drop or change them.

### Schema Cheat Sheet

| drizzle-orm | drizzle-rs |
|---|---|
| `sqliteTable('users', {...})`, `pgTable(...)`, `mysqlTable(...)` | `#[SQLiteTable]`, `#[PostgresTable]`, `#[MySQLTable]` on a struct; `name = "..."` when the table is not the struct name in `snake_case` |
| `pgSchema('auth').table(...)` | `#[PostgresTable(schema = "auth")]` |
| `text('created_at')` with a different key | a field plus `#[column(name = "created_at")]` |
| `.notNull()` / nullable | a plain field / `Option<T>` |
| `.primaryKey()` | `#[column(primary)]` |
| `primaryKey({ columns: [t.a, t.b], name })` | `#[column(primary)]` on each field; on PostgreSQL `#[PostgresTable(primary_key(name = "..."))]` |
| `integer().primaryKey({ autoIncrement: true })` | `#[column(primary, autoincrement)]` (SQLite), `#[column(primary, auto_increment)]` (MySQL) |
| `serial()`, `bigserial()` | `#[column(serial)]` on `i32`, `#[column(bigserial)]` on `i64` (PostgreSQL); `#[column(serial)]` on `u64` (MySQL) |
| `.generatedAlwaysAsIdentity({ startWith: 100 })` | `#[column(identity(always, start = 100))]`, `identity(by_default)` |
| `.default('member')`, `.defaultNow()`, `` .default(sql`...`) `` | `#[column(default = "member")]`, `#[column(default = now())]`, `#[column(default = expression)]` |
| `.unique()` | `#[column(unique)]` |
| `unique('name').on(t.a, t.b)` | `#[...Table(unique(columns(a, b), name = "name"))]` |
| `.references(() => users.id, { onDelete: 'cascade' })` | `#[column(references = Users::id, on_delete = CASCADE)]`; on PostgreSQL `fk_name = "..."` keeps a constraint name |
| `foreignKey({ columns, foreignColumns, name })` | `#[...Table(foreign_key(columns(a, b), references(Parent, x, y), on_delete = "CASCADE"))]`; `name = "..."` on PostgreSQL and MySQL |
| `index('name').on(t.a)`, `uniqueIndex(...)` | `#[SQLiteIndex] pub struct UsersEmailIdx(Users::email);` and `#[PostgresIndex(unique)]`, `#[MySQLIndex(unique)]`; `where = "..."` for partial indexes |
| ``check('name', sql`...`)`` | `#[...Table(check(name = "name", expr = "..."))]` or `#[column(check = "...")]` |
| ``.generatedAlwaysAs(sql`...`)`` | `#[column(generated(stored, "expr"))]` or `generated(virtual, "expr")` |
| `pgEnum('role', ['admin', 'member'])` | `#[derive(PostgresEnum)]` on an enum whose name and variants are the SQL type and values, and `#[column(enum)]` on the field; `#[postgres_enum(schema = "auth")]` for another schema |
| `mysqlEnum('role', [...])` | `#[derive(MySQLEnum)]` and `#[column(ENUM)]` |
| `text({ enum: [...] })` (SQLite) | `#[derive(SQLiteEnum)]` and `#[column(enum)]`, or keep a `String` |
| `json()`, `jsonb()` | `#[column(json)]` / `#[column(jsonb)]` with a `serde` type (`serde_json::Value` maps to `JSONB`) |
| `pgView('v').as(...)`, `sqliteView`, `mysqlView` | `#[PostgresView(definition = "...")]` (`materialized` for `pgMaterializedView`), `#[SQLiteView(...)]`, `#[MySQLView(...)]` on a struct with the view's columns |
| `relations(...)` | not needed; see above |
| `drizzle(client, { schema })` | `#[derive(SQLiteSchema)]` / `PostgresSchema` / `MySQLSchema` on a struct listing the tables, enums, indexes and views |

## Generated Models

Given the schema above, each `#[SQLiteTable]`, `#[PostgresTable]`, or
`#[MySQLTable]` generates four helper types:

| Model | Purpose | Fields |
|-------|---------|--------|
| `SelectUsers` | Full-row query results | One field per column, with the declared type |
| `InsertUsers` | Insert rows | `new(name, age)` requires non-default fields; `with_email(...)` for optional ones |
| `UpdateUsers` | Update rows | `default()` starts empty; `with_age(27)` sets fields to update |
| `PartialSelectUsers` | Partial-column query results | All fields `Option<T>`; populated by `db.query(users).columns(...)` (see [Relational Queries](#relational-queries)) |

### Insert

`new()` takes only the required fields (columns without a default or autoincrement). Chain `with_*` for optional fields:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::InsertUsers;
InsertUsers::new("Alex Smith", 26i64)
    .with_email("alex@example.com");
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

### Update

Start from `default()` and set only the fields you want to change. The query won't compile unless at least one field is set:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::UpdateUsers;
UpdateUsers::default()
    .with_age(27)
    .with_email("new@example.com");
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

### JSON Columns

With the `serde` feature, any `Serialize + Deserialize` type can be stored in a
JSON column (`json`; `json` or `jsonb` on PostgreSQL; `JSON` on MySQL). The
field keeps its own type in the generated models, and one payload type can back
columns in several tables. The macro implements nothing on the payload type,
and your crate does not need a `serde_json` dependency. Generated models
implement `Debug`, `Clone`, `PartialEq` and `Default` whenever every field type
does, so a payload type only needs the traits you actually use.

```rust
# #[cfg(all(feature = "rusqlite", feature = "serde"))]
# fn main() -> drizzle::Result<()> {
use drizzle::core::Json;
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
# use drizzle::sqlite::rusqlite::Drizzle;

#[derive(serde::Serialize, serde::Deserialize, Debug, Clone, PartialEq)]
pub struct Settings {
    pub theme: String,
}

#[SQLiteTable]
pub struct Profiles {
    #[column(primary)]
    pub id: i64,
    #[column(json)]
    pub settings: Settings,
    #[column(json)]
    pub tags: Vec<String>,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub profiles: Profiles,
}

# let conn = rusqlite::Connection::open_in_memory()?;
# let (db, Schema { profiles }) = Drizzle::new(conn);
# db.create()?;
let dark = Settings { theme: "dark".into() };
db.insert(profiles)
    .value(InsertProfiles::new(dark.clone(), vec!["admin".into()]))
    .execute()?;

// Compare a JSON column with a `Json(..)`-wrapped payload.
let rows: Vec<SelectProfiles> = db
    .select(())
    .from(profiles)
    .r#where(eq(profiles.settings, Json(dark.clone())))
    .all()?;
assert_eq!(rows[0].settings, dark);
# Ok(())
# }
# #[cfg(not(all(feature = "rusqlite", feature = "serde")))]
# fn main() {}
```

Selecting a JSON column on its own (`db.select(profiles.settings)`) yields
`Json<Settings>`; use `.0` or `.into_inner()` to get the payload.

## Querying

Comparison and expression functions such as `eq`, `gt`, `and`, and `count` live
in `drizzle::core::expr`. Ordering helpers such as `asc` and `desc` live in
`drizzle::core`.

### Select

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::expr::*;
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
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

// Multiple conditions — a tuple is an AND of its elements
let active_adults: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where((gt(users.age, 18), eq(users.name, "Alex Smith")))
    .all()?;

// Or
let rows: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where(eq(users.name, "Alice") | eq(users.name, "Bob"))
    .all()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

#### Combining Conditions

A tuple of conditions *is* a condition, so lists stay flat instead of nesting
`and(a, and(b, c))`. `all` and `any` combine the same lists explicitly.

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
use drizzle::core::expr::{all, any, eq, gt, is_not_null};
# use readme::*;
# let (db, Schema { users, posts, .. }) = readme::database()?;

let adults: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where((gt(users.age, 18), eq(users.name, "Alex"), is_not_null(users.email)))
    .all()?;

let staff: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where(any((eq(users.name, "Alice"), eq(users.name, "Bob"))))
    .all()?;

let posts: Vec<(i64, i64)> = db
    .select((users.id, posts.id))
    .from(users)
    .inner_join((posts, all((eq(posts.author_id, users.id), is_not_null(posts.content)))))
    .all()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

Elements may be `Option`s — `None` contributes nothing, which makes dynamic
filters composable:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::expr::{eq, gt};
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
# let name = Some("Alex");
let by_name = name.map(|n| eq(users.name, n));

// Some("Alex") => WHERE ("users"."age" > ? AND "users"."name" = ?)
// None         => WHERE ("users"."age" > ?)
let rows: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .r#where((gt(users.age, 18), by_name))
    .all()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

When every element is absent the list renders as its operator's identity:
`TRUE` for a tuple or `all` (matches everything, like an absent `WHERE`) and
`FALSE` for `any` (matches nothing, so a fully-optional `any` fails closed).

Bare tuples hold up to 8 conditions; `all`/`any` and nesting cover longer lists.

#### Ordering, Limiting, Pagination

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::{asc, desc};
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
let rows: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .order_by((asc(users.name), desc(users.age)))
    .limit(10)
    .offset(20)
    .all()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

#### Group By

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::expr::{alias, count, gt};
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
let totals: Vec<(String, i64)> = db
    .select((users.name, alias(count(users.id), "total")))
    .from(users)
    .group_by(users.name)
    .having(gt(count(users.id), 1))
    .all()?;

let totals_by_age: Vec<(String, i64, i64)> = db
    .select((users.name, users.age, alias(count(users.id), "total")))
    .from(users)
    .group_by((users.name, users.age))
    .all()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

### Insert

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
// Single row
db.insert(users)
    .value(InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"))
    .execute()?;

// Multiple rows
db.insert(users)
    .values([
        InsertUsers::new("Alex Smith", 26i64).with_email("alex@example.com"),
        InsertUsers::new("Jordan Lee", 30i64).with_email("jordan@example.com"),
    ])
    .execute()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

> [!IMPORTANT]
> In a multi-row insert, every row must set the same set of optional fields. Mixing `with_email(...)` on some rows but not others is a compile error.

### Update

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::expr::eq;
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
db.update(users)
    .set(UpdateUsers::default().with_age(27))
    .r#where(eq(users.id, 1))
    .execute()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

### Delete

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::expr::eq;
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
db.delete(users)
    .r#where(eq(users.id, 3))
    .execute()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

A `delete` or `update` without `.r#where(...)` does not compile, so a forgotten condition cannot empty or rewrite a table. To change every row on purpose, write `.r#where(true)`.

### Joins

Use `#[derive(SQLiteFromRow)]` to map columns from multiple tables into a flat struct. `#[from(Users)]` sets the default source table for unannotated fields:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
use drizzle::core::expr::eq;
use drizzle::sqlite::prelude::*;
# use readme::*;

#[derive(SQLiteFromRow, Debug)]
#[from(Users)]
struct UserWithPost {
    #[column(Users::id)]
    user_id: i64,
    name: String,
    // LEFT JOIN — every Posts column must be Option<T> in case the user has no
    // posts. A non-Option field here is a compile error.
    #[column(Posts::id)]
    post_id: Option<i64>,
    #[column(Posts::content)]
    content: Option<String>,
}

# let (db, Schema { users, posts, .. }) = readme::database()?;
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
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

### Subqueries & Set Operations

`SELECT` builders are expressions — pass them directly into comparisons or `IN`:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::expr::{eq, gt, in_subquery, min};
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
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
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

Combine queries with `union`, `union_all`, `intersect`, and `except`. `union` removes duplicates; `union_all` keeps them:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::{asc, desc};
# use drizzle::core::expr::{gte, lte};
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
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
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

### Aliases

Use a `Tag` to alias a table for self-joins:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
use drizzle::sqlite::prelude::*;
# use readme::*;

tag!(U, "u");

# let (db, _) = readme::database()?;
let u = Users::alias::<U>();
let rows: Vec<(i64,)> = db.select((u.id,)).from(u).all()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

## Expressions

Aggregate functions and common SQL expressions:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::expr::{coalesce, count, max};
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
// Aggregates
let total: (i64,) = db.select((count(users.id),)).from(users).get()?;
let oldest: (Option<i64>,) = db.select((max(users.age),)).from(users).get()?;

// Coalesce — first non-null value
let rows: Vec<(String,)> = db
    .select((coalesce(users.email, "unknown"),))
    .from(users)
    .all()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

Available in `drizzle::core::expr`:

- **Comparisons** — `eq`, `neq`, `gt`, `gte`, `lt`, `lte`
- **Boolean** — `and`, `or`, `not`, `all`, `any` (and tuples, which mean AND)
- **Aggregates** — `count`, `sum`, `avg`, `min`, `max`
- **Null handling** — `coalesce`, `is_null`, `is_not_null`
- **Strings** — `upper`, `lower`, `length`
- **Math** — `abs`, `round`, `sign`, `mod_`; `ceil`, `floor`, `trunc`, `sqrt`, `power`, `exp`, `ln`, `log`, `log10`, `log2`, `pi` (see the SQLite note below)

The ordering helpers `asc` and `desc` are in `drizzle::core`, not
`drizzle::core::expr`.

SQLite only has `ceil` through `pi` when it is compiled with
`SQLITE_ENABLE_MATH_FUNCTIONS`, so on SQLite those functions compile only with
drizzle's `math` feature. Enabling `math` is a promise about the SQLite you link:

- **rusqlite** (with `bundled`) and **libsql** compile their own SQLite and read
  `LIBSQLITE3_FLAGS` while doing so. Set
  `LIBSQLITE3_FLAGS="-DSQLITE_ENABLE_MATH_FUNCTIONS"` in the build environment,
  for example under `[env]` in `.cargo/config.toml`.
- **turso** implements these functions itself and needs no flag.

If the linked SQLite lacks them, the calls still compile under `math`, and the
query fails at runtime with `no such function`.

### Type Casting

`cast(expr, target)` takes a type marker from the dialect's `types` module
(`drizzle::sqlite::types`, `drizzle::postgres::types`, or
`drizzle::mysql::types`). The marker supplies both the SQL type name and the
result type. You can pass a SQL type name as a string instead, but a string
carries no result type, so name the marker with a turbofish:
`cast::<_, _, Real>(expr, "DOUBLE")`. SQLite and PostgreSQL only allow casts
between compatible types, such as integer to real.

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
use drizzle::core::expr::cast;
use drizzle::sqlite::types::Real;

# let (db, Schema { users, .. }) = readme::database()?;
// The marker renders `AS REAL` and types the result as a SQLite REAL (f64)
let ages: Vec<(f64,)> = db.select((cast(users.age, Real),)).from(users).all()?;

// A string only renders the SQL type name; the turbofish supplies the result type
let ages: Vec<(f64,)> = db
    .select((cast::<_, _, Real>(users.age, "DOUBLE"),))
    .from(users)
    .all()?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

## Relational Queries

Requires the `query` feature. Fetches a table with its relations in a single query — no manual joins.

Relation methods are generated from foreign keys. Given `Posts.author_id → Users.id`, `posts.author()` is the forward (many-to-one) and `users.author_posts()` the reverse (one-to-many). [Relation Names](#relation-names) explains how the names are chosen.

```rust
# #[cfg(all(feature = "rusqlite", feature = "query"))]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
let users = db.query(users)
    .with(users.author_posts())
    .find_many()?;

for user in &users {
    println!("{}: {} posts", user.name, user.author_posts.len());
}
# Ok(())
# }
# #[cfg(not(all(feature = "rusqlite", feature = "query")))]
# fn main() {}
```

`.find_first()` returns `Option<...>`:

```rust
# #[cfg(all(feature = "rusqlite", feature = "query"))]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::expr::eq;
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
let user = db.query(users)
    .with(users.author_posts())
    .r#where(eq(users.name, "Alice"))
    .find_first()?;
# Ok(())
# }
# #[cfg(not(all(feature = "rusqlite", feature = "query")))]
# fn main() {}
```

Nest relations:

```rust
# #[cfg(all(feature = "rusqlite", feature = "query"))]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
# let (db, Schema { users, posts, .. }) = readme::database()?;
let users = db.query(users)
    .with(users.author_posts().with(posts.comments()))
    .find_many()?;

println!("{} comments", users[0].author_posts[0].comments.len());
# Ok(())
# }
# #[cfg(not(all(feature = "rusqlite", feature = "query")))]
# fn main() {}
```

Filter and paginate the root query:

```rust
# #[cfg(all(feature = "rusqlite", feature = "query"))]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::asc;
# use drizzle::core::expr::gt;
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
let users = db.query(users)
    .with(users.author_posts())
    .r#where(gt(users.age, 25))
    .order_by(asc(users.name))
    .limit(10)
    .find_many()?;
# Ok(())
# }
# #[cfg(not(all(feature = "rusqlite", feature = "query")))]
# fn main() {}
```

### Relation Names

Every foreign key gives relation accessors named from the schema, so you
rarely name one yourself:

| Relation | Example | Loads | Named after |
|---|---|---|---|
| Forward, on the table with the key | `posts.author()` | the row, or an `Option` when the key is nullable | the column without `_id` (`author_id` gives `author`) |
| Reverse, on the referenced table | `users.author_posts()` | a `Vec` | the plural of the struct, after the column's role |
| One-to-one reverse, when the key alone is unique | `users.profile()` | an `Option` | the singular of the struct |
| Many-to-many, through a link table | `posts.tags()` | a `Vec` | the plural of the other column, plus the link's own name |

The struct name counts, not the SQL table name.

**Roles.** A column named after the table it references (`user_id` to
`Users`, or to `AppUsers`) plays no role, so `Posts.user_id` gives
`users.posts()`. Any other name is a role, and the reverse accessor starts
with it: `author_id` and `editor_id` give `users.author_posts()` and
`users.editor_posts()`, and a self-reference `parent_id` gives
`categories.parent_categories()`. A reverse name depends only on its own
column, so adding a foreign key never renames another accessor.

**Link tables.** A table is a link when it has exactly two foreign keys and
its rows are that pair: the pair is its primary key or a
`UNIQUE(columns(...))` constraint, or the table has no other column except a
single-column primary key. Neither key may be unique on its own, which would
make it a one-to-one. A table with two foreign keys beside columns of its
own, such as a comment with an author and a post, is an entity and gets no
many-to-many pair, so its rows are never loaded twice.

Each side of a link gets an accessor to the other, named after the other
column. Both keys may reference one table: `Fans` with `fan_id` and `idol_id`
gives `users.idols()` and `users.fans()`. A link named only after what it
links (`PostTags`, `UsersToGroups`, or `GroupMembers` with a `member_id`
column) gives plain names, `posts.tags()` and `tags.posts()`. A link with a
name of its own adds it, so two links between the same tables never clash:
`PostLikes` and `PostBookmarks` give `users.posts_via_likes()` and
`users.posts_via_bookmarks()`. The link's rows stay available as
`users.post_likes()` and `post_likes.post()`.

**Composite keys.** A table-level `foreign_key(columns(...), references(...))`
gives the same relations. Its forward accessor is the singular of the
referenced struct.

**Naming by hand.** `relation = "..."` names the reverse accessor and
`many_to_many = "..."` names a link side's many-to-many accessor; both also
work inside `foreign_key(...)`, and `many_to_many` makes any table with two
foreign keys a link. You need them only to choose another name, or when two
tables still give a third the same accessor, as a direct key and a plain link
between the same tables can (`Posts.tag_id` and `PostTags` both give
`tags.posts()`). rustc then reports the duplicate at both columns.

```rust
# #[cfg(all(feature = "rusqlite", feature = "query"))]
# fn main() -> drizzle::Result<()> {
use drizzle::sqlite::prelude::*;
# use drizzle::sqlite::rusqlite::Drizzle;

#[SQLiteTable]
pub struct Users {
    #[column(primary)]
    pub id: i64,
    // Self-reference: users.invited_by() and users.invited_by_users()
    #[column(references = Users::id)]
    pub invited_by: Option<i64>,
}

// One-to-one: users.profile() loads an Option, profiles.user() the user
#[SQLiteTable]
pub struct Profiles {
    #[column(primary)]
    pub id: i64,
    #[column(unique, references = Users::id)]
    pub user_id: i64,
    pub bio: String,
}

#[SQLiteTable]
pub struct Posts {
    #[column(primary)]
    pub id: i64,
    // posts.author() and users.author_posts()
    #[column(references = Users::id)]
    pub author_id: i64,
    // posts.editor() and users.editor_posts()
    #[column(references = Users::id)]
    pub editor_id: Option<i64>,
}

#[SQLiteTable]
pub struct Tags {
    #[column(primary)]
    pub id: i64,
}

// A plain link: posts.tags() and tags.posts()
#[SQLiteTable]
pub struct PostTags {
    #[column(references = Posts::id)]
    pub post_id: i64,
    #[column(references = Tags::id)]
    pub tag_id: i64,
}

// A link with a name and a payload of its own: users.posts_via_likes() and
// posts.users_via_likes(). Its rows stay users.post_likes().
#[SQLiteTable(UNIQUE(columns(user_id, post_id)))]
pub struct PostLikes {
    #[column(primary)]
    pub id: i64,
    #[column(references = Users::id)]
    pub user_id: i64,
    #[column(references = Posts::id)]
    pub post_id: i64,
    pub liked_at: i64,
}

// A second link between users and posts needs no names either:
// users.posts_via_bookmarks() and posts.users_via_bookmarks()
#[SQLiteTable]
pub struct PostBookmarks {
    #[column(references = Users::id)]
    pub user_id: i64,
    #[column(references = Posts::id)]
    pub post_id: i64,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub profiles: Profiles,
    pub posts: Posts,
    pub tags: Tags,
    pub post_tags: PostTags,
    pub post_likes: PostLikes,
    pub post_bookmarks: PostBookmarks,
}

# let conn = rusqlite::Connection::open_in_memory()?;
# let (db, Schema { users, profiles, posts, tags, post_tags, post_likes, .. }) = Drizzle::new(conn);
# db.create()?;
let authors = db
    .query(users)
    .with(users.author_posts())
    .with(users.editor_posts())
    .with(users.invited_by_users())
    .with(users.profile())
    .with(users.posts_via_likes())
    .with(users.posts_via_bookmarks())
    .find_many()?;

let tagged = db.query(posts).with(posts.author()).with(posts.tags()).find_many()?;
# let _ = db.query(users).with(users.invited_by()).with(users.post_likes()).find_many()?;
# let _ = db.query(profiles).with(profiles.user()).find_many()?;
# let _ = db.query(posts).with(posts.editor()).with(posts.post_tags()).find_many()?;
# let _ = db.query(tags).with(tags.posts()).find_many()?;
# let _ = db.query(posts).with(posts.users_via_likes()).with(posts.users_via_bookmarks()).find_many()?;
# let _ = db.query(post_tags).with(post_tags.post()).with(post_tags.tag()).find_many()?;
# let _ = db.query(post_likes).with(post_likes.user()).find_many()?;
# Ok(())
# }
# #[cfg(not(all(feature = "rusqlite", feature = "query")))]
# fn main() {}
```

### Selecting Specific Columns

`.columns(...)` / `.omit(...)` return `PartialSelectUsers` — same shape as `SelectUsers`, but every field is `Option<T>`:

```rust
# #[cfg(all(feature = "rusqlite", feature = "query"))]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
let users = db.query(users)
    .columns(users.columns().name().email())
    .find_many()?;

for u in &users {
    assert!(u.name.is_some());
    assert!(u.id.is_none()); // not selected
}
# Ok(())
# }
# #[cfg(not(all(feature = "rusqlite", feature = "query")))]
# fn main() {}
```

### Result Types

`.with(users.author_posts())` returns `UsersWithAuthorPosts` — base columns via deref, relation data on fields like `user.author_posts`:

```rust
# #[cfg(all(feature = "rusqlite", feature = "query"))]
# fn main() {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::UsersWithAuthorPosts;
fn print_user_posts(user: &UsersWithAuthorPosts) {
    println!("{} has {} posts", user.name, user.author_posts.len());
}
# }
# #[cfg(not(all(feature = "rusqlite", feature = "query")))]
# fn main() {}
```

## Transactions

> [!TIP]
> Transactions auto-rollback on error or panic. Return `Ok(value)` to commit, `Err(...)` to rollback. No manual cleanup needed.

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
use drizzle::sqlite::TransactionConfig;

# let (mut db, Schema { users, .. }) = readme::database()?;
db.transaction(TransactionConfig::Deferred, |tx| {
    tx.insert(users)
        .value(InsertUsers::new("Alice", 28i64))
        .execute()?;

    let all: Vec<SelectUsers> = tx.select(()).from(users).all()?;

    Ok(all.len())
})?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

Savepoints nest inside transactions — a failed savepoint rolls back without aborting the outer transaction:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
use drizzle::sqlite::TransactionConfig;
use drizzle::error::DrizzleError;

# let (mut db, Schema { users, .. }) = readme::database()?;
let count = db.transaction(TransactionConfig::Deferred, |tx| {
    tx.insert(users)
        .value(InsertUsers::new("Alice", 28i64))
        .execute()?;

    // This savepoint fails and rolls back, but the outer transaction continues
    let _ = tx.savepoint(|stx| {
        stx.insert(users)
            .value(InsertUsers::new("Bad Data", -1i64))
            .execute()?;
        Err::<(), _>(DrizzleError::Other("rollback this part".into()))
    });

    // Alice is still inserted
    tx.insert(users)
        .value(InsertUsers::new("Bob", 32i64))
        .execute()?;

    let all: Vec<SelectUsers> = tx.select(()).from(users).all()?;
    Ok(all.len())
})?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

Cloudflare D1 is the exception: the platform exposes no transaction handles, so
the D1 driver has no `transaction` method. Its `batch` method submits several
statements that D1 applies atomically.

## Prepared Statements

> [!TIP]
> Placeholders are typed by the column they came from. Binding the wrong type fails at compile time, not at runtime.

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use readme::*;
use drizzle::core::expr::eq;
use drizzle::core::SQLColumn;

# let (db, Schema { users, .. }) = readme::database()?;
let name = users.name.placeholder("name");

let find = db
    .select(())
    .from(users)
    .r#where(eq(users.name, name))
    .prepare();

let alice: Vec<SelectUsers> = find.all(db.conn(), [name.bind("Alice")])?;
let bob: Vec<SelectUsers> = find.all(db.conn(), [name.bind("Bob")])?;
// name.bind(42) — compile error: Integer is not compatible with Text
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

Placeholders work in update (and insert) models too:

```rust
# #[cfg(feature = "rusqlite")]
# fn main() -> drizzle::Result<()> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs"));
# }
# use drizzle::core::{SQLColumn, expr::eq};
# use readme::*;
# let (db, Schema { users, .. }) = readme::database()?;
let new_name = users.name.placeholder("new_name");
let target = users.id.placeholder("target");

let stmt = db
    .update(users)
    .set(UpdateUsers::default().with_name(new_name))
    .r#where(eq(users.id, target))
    .prepare();

stmt.execute(db.conn(), [new_name.bind("New Name"), target.bind(1)])?;
# Ok(())
# }
# #[cfg(not(feature = "rusqlite"))]
# fn main() {}
```

Use `.prepare().into_owned()` to convert a prepared statement into a self-contained value that can be stored or moved freely.

## Seeding

[`drizzle-seed`](https://docs.rs/drizzle-seed) fills a database with deterministic test data: the same seed gives the same rows. Values follow the column types and names (emails for `email`, timestamps for `created_at`, enum variants for enums, ...), foreign keys point at seeded parent rows, and `UNIQUE` columns get distinct values.

With the schema macros, pass the schema; tables and columns are checked at compile time:

```rust,ignore
use drizzle_seed::{SeedConfig, generators};

let schema = AppSchema::new();
for statement in SeedConfig::sqlite(&schema)
    .seed(42)
    .count(&schema.users, 100)
    .relation(&schema.users, &schema.posts, 3) // 3 posts per user
    .generator(&schema.users.age, generators::int(18..=90))
    .generate()
{
    db.execute(statement)?;
}
```

Without a Rust schema, `drizzle seed` reads the tables from the database itself:

```bash
drizzle seed --seed 42 --count users=100 --relation users:posts=3
drizzle seed --reset            # empty the seeded tables first
drizzle seed --out seed.sql     # write the SQL instead of running it
```

From Rust, build the seed schema from a live database (`db.introspect()`) or a migration's `snapshot.json` with `drizzle_seed::schema::Schema::from_snapshot` (the `migrations` feature), then configure it by name with `count_by_name`, `relation_by_name` and `generator_by_name`. `try_generate_script()` returns the whole seed as SQL with its values written inline.

## PostgreSQL

The query API above works the same way with `#[PostgresTable]`,
`#[derive(PostgresSchema)]`, `#[derive(PostgresFromRow)]`, and
`drizzle::postgres::{sync,tokio}::Drizzle`. The tokio driver's calls are
`async`, and the blocking driver runs prepared statements on `db.conn_mut()`.
Two things in the SQLite examples do not carry over: `autoincrement` (use
`serial`, `bigserial`, `smallserial`, or `identity(...)` columns) and
`TransactionConfig::Deferred`, which is a SQLite transaction mode.

PostgreSQL's `TransactionConfig::default()` uses server defaults. Its
typestated builder keeps `DEFERRABLE` on the combination where it has meaning:

```rust
# #[cfg(feature = "postgres")]
# fn main() {
use drizzle::postgres::TransactionConfig;

let config = TransactionConfig::builder()
    .serializable()
    .read_only()
    .deferrable()
    .build();
# }
# #[cfg(not(feature = "postgres"))]
# fn main() {}
```

```rust,no_run
# #[cfg(feature = "postgres-sync")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
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
let (mut db, Schema { accounts }) = Drizzle::new(client);
# Ok(())
# }
# #[cfg(not(feature = "postgres-sync"))]
# fn main() {}
```

## MySQL

MySQL uses the same `select`, `insert`, `update`, `delete`, prepared-statement,
relational-query, transaction, migration, and seed workflows as the other
dialects. Migration generation, push, and apply are available through the
Drizzle CLI, and both adapters expose `migrate(&migrations, Tracking::MYSQL)`.
MySQL applies each statement in autocommit mode because its DDL can commit
implicitly. A failed migration stays marked as interrupted until you reconcile
the partial schema and its tracking row manually. Enable
`mysql-sync` for the blocking [`mysql`](https://crates.io/crates/mysql)
client or `mysql-async` for [`mysql_async`](https://crates.io/crates/mysql_async).
The driver crates remain explicit dependencies because your application creates
and owns the connection or pool.

```toml
[dependencies]
drizzle = { version = "0.3", features = ["mysql-sync"] }
mysql = "28"
```

```rust,no_run
# #[cfg(feature = "mysql-sync")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
use drizzle::mysql::{mysql_sync::Drizzle, prelude::*};

#[MySQLTable]
struct User {
    #[column(PRIMARY, AUTO_INCREMENT)]
    id: u64,
    #[column(VARCHAR(255))]
    name: String,
}

#[derive(MySQLSchema)]
struct Schema {
    users: User,
}

let options = mysql::Opts::from_url(
    "mysql://drizzle:drizzle@127.0.0.1:3307/drizzle_test",
)?;
let connection = mysql::Conn::new(options)?;
let (mut db, Schema { users, .. }) = Drizzle::new(connection);
# Ok(())
# }
# #[cfg(not(feature = "mysql-sync"))]
# fn main() {}
```

The async adapter accepts either an owned `mysql_async::Conn` or a lazy pool:

```toml
[dependencies]
drizzle = { version = "0.3", features = ["mysql-async"] }
mysql_async = "0.37"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

```rust,no_run
# #[cfg(feature = "mysql-async")]
# #[tokio::main]
# async fn main() -> Result<(), Box<dyn std::error::Error>> {
use drizzle::mysql::{mysql_async::Drizzle, prelude::*};

let options = mysql_async::Opts::from_url(
    "mysql://drizzle:drizzle@127.0.0.1:3307/drizzle_test",
)?;
let pool = mysql_async::Pool::new(options);
let (db, ()) = Drizzle::new(pool);
// Calls on a pool-backed adapter are async and check out one connection per operation.
db.disconnect().await?;
# Ok(())
# }
# #[cfg(not(feature = "mysql-async"))]
# fn main() {}
```

Start the documented MySQL 8.4 container and run the complete adapter matrix or
the runnable blocking example:

```bash
just mysql-up
just test-mysql
cargo run --example mysql --features mysql-sync
```

`DRIZZLE_MYSQL_URL` overrides the example and test URL. First-class support
targets Oracle MySQL 8.0.31 or newer; CI runs both MySQL 8.0.31 and 8.4. MariaDB
and SingleStore compatibility are not promised. TLS configuration belongs to
the upstream `mysql`/`mysql_async` options supplied to Drizzle. The workspace
enables their native-TLS backends, and Drizzle neither disables certificate
validation nor silently changes the caller's transport policy.

Before its first typed query on a connection, the adapter sets the session time
zone to UTC and removes `NO_UNSIGNED_SUBTRACTION` and `REAL_AS_FLOAT` from the
session SQL mode. Those invariants keep temporal decoding, unsigned arithmetic,
and `REAL` columns consistent with the Rust types; using `conn_mut()` causes
them to be restored before the next Drizzle query.

Transactions use `TransactionConfig` for isolation level, access mode, and
consistent snapshots. The typestated builder only exposes `.snapshot()` after
`.repeatable_read()`. Runtime-derived values can use the enum setters instead.
The blocking adapter takes a closure synchronously; the async connection and
pool adapters expose the same transaction configuration on their async methods.
MySQL upserts use the native
`.on_duplicate_key_update(...)` builder (or `.ignore()`), not PostgreSQL's
`.on_conflict(...)` spelling.

```rust,no_run
# #[cfg(feature = "mysql-sync")]
# fn main() -> Result<(), Box<dyn std::error::Error>> {
# mod readme {
#     include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/mysql.rs"));
# }
# use readme::*;
use drizzle::mysql::TransactionConfig;

# let (mut db, Schema { users }) = readme::database()?;
let config = TransactionConfig::builder()
    .repeatable_read()
    .read_write()
    .snapshot()
    .build();

db.transaction(config, |tx| {
    tx.insert(users).value(InsertUser::new("Alice")).execute()?;
    Ok(())
})?;
# Ok(())
# }
# #[cfg(not(feature = "mysql-sync"))]
# fn main() {}
```

Transactions stay scoped to the callback. Returning `Ok` commits; returning
`Err` rolls back. Dropping or cancelling an async transaction future also
prevents the active transaction from being reused without rollback.

The type surface deliberately leaves unsupported SQL unavailable:

- MySQL mutations return `MySQLMutationResult` metadata, not SQL `RETURNING` rows.
- Full joins and partial-index predicates are rejected; MySQL does not support them.
- `.offset(n)` without an explicit limit renders a `LIMIT` of `i64::MAX`
  first, because MySQL has no standalone `OFFSET` syntax. (MySQL's manual
  suggests `18446744073709551615`, but after a `UNION` that value overflows and
  the query returns no rows.)
- String concatenation uses `concat(...)`; the builder never emits `||`, whose
  default MySQL meaning is logical OR.

See [`examples/mysql.rs`](https://github.com/themixednuts/drizzle-rs/blob/main/examples/mysql.rs) for the complete blocking example.

## CLI Reference

Most projects only need these:

| Command | Description |
|---------|-------------|
| `drizzle init` | Create `drizzle.config.toml` |
| `drizzle generate` | Diff schema and emit SQL migration files |
| `drizzle migrate` | Apply pending migrations |
| `drizzle push` | Apply schema diff directly without migration files |
| `drizzle introspect` | Reverse-engineer schema from a live database |
| `drizzle seed` | Fill a live database with deterministic test data |

Other useful commands:

| Command | Description |
|---------|-------------|
| `drizzle new` | Interactive schema builder |
| `drizzle status` | List local migration folders and whether each has a snapshot (it does not read the database; use `drizzle migrate --plan` for that) |
| `drizzle check` | Validate config |
| `drizzle export` | Print schema as raw SQL |
| `drizzle up` | Upgrade migration snapshots to the latest format |
| `drizzle import <folder>` | Write the Rust schema of a drizzle-orm (TypeScript) project from its drizzle-kit snapshots (see [Porting from drizzle-orm](#porting-from-drizzle-orm-typescript)) |

`drizzle pull` is an alias for `introspect`. Commands that read the config accept `-c <path>` for a custom config file and `--db <name>` for multi-database configs.

### Renames

When a change drops one table, column, view, or PostgreSQL schema, enum, index, or constraint (unique, check, primary key, foreign key) and adds another of the same kind (in the same schema, and for columns, indexes, and constraints the same table), the diff cannot tell a rename from a drop plus a create. `drizzle generate` and `drizzle push` never guess; they ask, the way drizzle-kit does.

**With a terminal**, they ask once per new entity that has dropped candidates, schemas first, then enums, tables, columns, unique constraints, checks, indexes, primary keys, foreign keys, and views:

```text
? Is accounts table created or renamed from another table?
> + accounts          create table
  ~ users › accounts  rename table
  ~ people › accounts rename table
```

Picking a rename uses up that candidate, and column questions use the tables' new names. `push --force` does not skip these questions; it only approves data loss.

A constraint or index whose name was derived rather than written out keeps the name it has in the database when only that derived name changes, for example `users_pkey` after `users` becomes `accounts`. As in drizzle-kit, nothing is asked or planned for it, and the new snapshot records the kept name.

**Without a terminal** (CI, scripts), answer with drizzle-kit's hints, inline or from a file:

```bash
drizzle generate --hints '[{"type":"rename","kind":"table","from":["public","users"],"to":["public","accounts"]}]'
drizzle push --hints-file hints.json
```

```json
[
  { "type": "rename", "kind": "column", "from": ["public", "accounts", "name"], "to": ["public", "accounts", "full_name"] },
  { "type": "create", "kind": "table", "entity": ["public", "audit_log"] }
]
```

`rename` turns a drop plus a create into a rename; `create` keeps them separate. Identifiers are `[name]` for schemas, `[schema, name]` for tables, views, and enums, and `[schema, table, name]` for columns, indexes, and constraints (`unique`, `check`, `primary_key`, `foreign key`). SQLite and MySQL use `public` as the schema, as drizzle-kit does. A column's table is its new name. Hints that match nothing in the current diff are ignored, so one file can be reused. If a schema, enum, table, or column question has no hint, the command changes nothing: it prints each unresolved decision with the hints that would answer it and exits with code 2. An unhinted index, constraint, or view is dropped and created, which loses no data.

The Rust API never guesses either. `drizzle_migrations::diff` and `diff_with`, `build::run`, and the drivers' `db.push` stop with `MigrationError::UnansweredRenames` when a schema, enum, table, or column may have been renamed, and the error gives the hint for each answer:

```text
cannot tell a rename from a drop plus a create, and guessing wrong loses data. Answer each question with a hint on `RenameHints` or `DiffOptions` (the `drizzle` CLI asks them interactively):
  column `users.full_name`: created, or renamed from `name`?
    renamed from `name`: .rename_column("users", "name", "full_name")
    created: .create(CreateHint::new(RenameKind::Column, "full_name").on_table("users"))
```

Pass the answer where the diff runs:

```rust,ignore
use drizzle_migrations::RenameHints;

let renames = RenameHints::new().rename_column("users", "name", "full_name");

// build.rs
let cfg = drizzle_migrations::build::Config::new(Dialect::SQLite)
    .file("src/schema.rs")
    .renames(renames.clone());

// at runtime
db.push_with(&schema, &renames)?;

// in memory
drizzle_migrations::diff_with(&prev, &next, &DiffOptions::new().with_renames(renames))?;
```

A hint that matches nothing in a later diff is ignored, so it can stay in `build.rs`. `db.push` also stops before it drops a table or column that holds rows, applying nothing, where drizzle-kit's push would ask; dropping an empty one goes ahead. An index, constraint, or view without an answer is dropped and created, which loses no data; the CLI does the same without a terminal. `drizzle_migrations::rename_questions` lists the questions, for tools that ask them their own way.

## License

MIT. See [LICENSE](https://github.com/themixednuts/drizzle-rs/blob/main/LICENSE).
