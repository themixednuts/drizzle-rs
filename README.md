# Drizzle RS

A type-safe SQL query builder / ORM-ish layer for Rust, inspired by Drizzle ORM.

> [!WARNING]
> This project is still evolving. Expect breaking changes.

## What’s in the box

- **Type-safe SQL builder**: compile-time checked expressions and query
  building.
- **Schema macros**: `#[SQLiteTable]`, `#[PostgresTable]`,
  `#[derive(SQLiteSchema)]`, etc.
- **Migrations + CLI**: generate/apply migrations and introspect schema via the
  `drizzle` binary.

## Install

### Library

Pick a database driver feature (drivers imply the corresponding dialect module):

```toml
[dependencies]
drizzle = { version = "0.1.3", features = ["rusqlite"] } # or: libsql / turso / postgres-sync / tokio-postgres
```

### CLI

Install the `drizzle` binary with the driver(s) you want:

```bash
# SQLite (sync)
cargo install drizzle-cli --locked --features rusqlite

# PostgreSQL (sync)
cargo install drizzle-cli --locked --features postgres-sync

# Turso / LibSQL (async)
cargo install drizzle-cli --locked --features turso
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
    let (db, Schema { users }) = Drizzle::new(conn, Schema::new());

    // Note: create statements are intentionally not IF NOT EXISTS (to stay compatible with migrations).
    // Use this only for a fresh DB (or rely on migrations).
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

For more complete examples (including JOIN mapping via
`#[derive(SQLiteFromRow)]`), see `examples/` and `tests/`.

## CLI quick start (migrations)

```bash
drizzle init -d sqlite          # dialects: sqlite, turso, postgresql, mysql, singlestore
drizzle generate                # generate migrations from schema changes
drizzle migrate                 # apply pending migrations
```

## Feature flags (library)

| Feature                                     | Enables                                                    |
| ------------------------------------------- | ---------------------------------------------------------- |
| `sqlite`                                    | SQLite dialect module re-exports (`drizzle::sqlite`)       |
| `postgres`                                  | PostgreSQL dialect module re-exports (`drizzle::postgres`) |
| `rusqlite`                                  | SQLite sync driver (`drizzle::sqlite::rusqlite`)           |
| `libsql`                                    | SQLite async driver (`drizzle::sqlite::libsql`)            |
| `turso`                                     | Turso/LibSQL async driver (`drizzle::sqlite::turso`)       |
| `postgres-sync`                             | PostgreSQL sync driver (`drizzle::postgres::sync`)         |
| `tokio-postgres`                            | PostgreSQL async driver (`drizzle::postgres::tokio`)       |
| `uuid`                                      | UUID support                                               |
| `serde`                                     | JSON support (serde/serde_json integration)                |
| `chrono` / `cidr` / `geo-types` / `bit-vec` | Optional PostgreSQL types                                  |
| `arrayvec`                                  | Fixed-capacity strings/arrays support                      |

## Development

- **Build**: `cargo build --all-features`
- **Test (SQLite)**: `cargo test --features "rusqlite,uuid"`
- **Test (PostgreSQL, Docker)**: `just test-pg`
- **Lint (nightly)**: `cargo clippy --all-features -- -D warnings`

More commands and repo details live in `CLAUDE.md`.

## License

MIT — see [LICENSE](LICENSE).
