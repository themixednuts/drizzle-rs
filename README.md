# Drizzle RS

SQL ORM inspired by Drizzle ORM.

## Quick Start

### SQLite Example

```rust
use drizzle::prelude::*;
use drizzle::rusqlite::Drizzle;

#[SQLiteTable]
pub struct Users {
    #[integer(primary, autoincrement)]
    pub id: i32,
    #[text]
    pub name: String,
    #[text]
    pub email: Option<String>,
    #[integer]
    pub age: i32,
}

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
}

fn main() -> drizzle::Result<()> {
    let conn = rusqlite::Connection::open_in_memory()?;
    let (db, Schema { users }) = Drizzle::new(conn, Schema::new());

    // Create tables, only use on new database.
    db.create()?;

    // Insert data
    db.insert(users)
        .values([InsertUsers::new("Alice", 25).with_email("alice@example.com")])
        .execute()?;

    // Query data
    let all_users: Vec<SelectUsers> = db.select(()).from(users).all()?;
    println!("Users: {:?}", all_users);

    // Query with conditions
    let adult_users: Vec<SelectUsers> = db
        .select(())
        .from(users)
        .r#where(gte(users.age, 18))
        .all()?;

    Ok(())
}
```

### PostgreSQL Example

```rust
use drizzle::prelude::*;
use drizzle::postgres_sync::Drizzle;

#[PostgresTable]
pub struct Users {
    #[serial(primary)]
    pub id: i32,
    #[text]
    pub name: String,
    #[text]
    pub email: Option<String>,
    #[integer]
    pub age: i32,
}

#[derive(PostgresSchema)]
pub struct Schema {
    pub users: Users,
}

fn main() -> drizzle::Result<()> {
    let mut conn = postgres::Client::connect(
        "host=localhost user=postgres password=postgres dbname=test",
        postgres::NoTls
    )?;
    let (db, Schema { users }) = Drizzle::new(&mut conn, Schema::new());

    db.create()?;

    db.insert(users)
        .values([InsertUsers::new("Alice", 25).with_email("alice@example.com")])
        .execute()?;

    let all_users: Vec<SelectUsers> = db.select(()).from(users).all()?;
    println!("Users: {:?}", all_users);

    Ok(())
}
```

---

## SQLite Schema Definition

### Table Definition

The `#[SQLiteTable]` attribute macro transforms a Rust struct into a complete
SQLite table definition.

```rust
#[SQLiteTable]                              // Table name defaults to snake_case: "my_table"
#[SQLiteTable(name = "custom_name")]        // Custom table name
#[SQLiteTable(strict)]                      // SQLite STRICT mode
#[SQLiteTable(without_rowid)]               // WITHOUT ROWID table
#[SQLiteTable(name = "users", strict)]      // Combined options
```

### Column Types

| Attribute    | SQLite Type | Rust Types                                                   |
| ------------ | ----------- | ------------------------------------------------------------ |
| `#[integer]` | INTEGER     | `i8`, `i16`, `i32`, `i64`, `u8`, `u16`, `u32`, `u64`, `bool` |
| `#[text]`    | TEXT        | `String`, `&str`, enums (with `enum` flag)                   |
| `#[real]`    | REAL        | `f32`, `f64`                                                 |
| `#[blob]`    | BLOB        | `Vec<u8>`, `Uuid` (with `uuid` feature)                      |
| `#[boolean]` | INTEGER     | `bool` (stored as 0/1)                                       |

### Column Constraints

```rust
#[SQLiteTable]
pub struct Users {
    // Primary key with auto-increment
    #[integer(primary, autoincrement)]
    pub id: i32,

    // Unique constraint
    #[text(unique)]
    pub email: String,

    // Compile-time default value
    #[text(default = "active")]
    pub status: String,

    // Runtime default function
    #[blob(primary, default_fn = uuid::Uuid::new_v4)]
    pub uuid_id: Uuid,

    // Foreign key reference
    #[integer(references = Posts::id)]
    pub post_id: i32,

    // Enum stored as TEXT (variant name)
    #[text(enum)]
    pub role: UserRole,

    // Enum stored as INTEGER (discriminant)
    #[integer(enum)]
    pub priority: Priority,

    // JSON serialization (requires `serde` feature)
    #[text(json)]
    pub metadata: Option<UserMetadata>,

    #[blob(json)]
    pub config: Option<UserConfig>,
}
```

#### Constraint Reference

| Constraint      | Description                                  | Example                                  |
| --------------- | -------------------------------------------- | ---------------------------------------- |
| `primary`       | Primary key constraint                       | `#[integer(primary)]`                    |
| `autoincrement` | Auto-incrementing (INTEGER PRIMARY KEY only) | `#[integer(primary, autoincrement)]`     |
| `unique`        | Unique constraint                            | `#[text(unique)]`                        |
| `default`       | Compile-time default value                   | `#[text(default = "value")]`             |
| `default_fn`    | Runtime default function                     | `#[blob(default_fn = Uuid::new_v4)]`     |
| `references`    | Foreign key reference                        | `#[integer(references = Table::column)]` |
| `enum`          | Store enum as TEXT or INTEGER                | `#[text(enum)]` or `#[integer(enum)]`    |
| `json`          | JSON serialization (requires `serde`)        | `#[text(json)]` or `#[blob(json)]`       |

### Enum Definition

```rust
#[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
pub enum UserRole {
    #[default]
    User,       // Stored as "User" with #[text(enum)]
    Admin,      // Stored as "Admin"
    Moderator,  // Stored as "Moderator"
}

#[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
pub enum Priority {
    Low = 1,    // Stored as 1 with #[integer(enum)]
    #[default]
    Medium = 5, // Stored as 5
    High = 10,  // Stored as 10
}
```

### Index Definition

```rust
// Simple index
#[SQLiteIndex]
pub struct UserEmailIdx(Users::email);

// Unique index
#[SQLiteIndex(unique)]
pub struct UserEmailUniqueIdx(Users::email);

// Composite index
#[SQLiteIndex]
pub struct UserNameEmailIdx(Users::name, Users::email);
```

### Schema Definition

```rust
#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
    pub user_email_idx: UserEmailIdx,
}

// Usage
let (db, Schema { users, posts, .. }) = Drizzle::new(conn, Schema::new());
db.create()?; // Creates all tables and indexes
```

---

## PostgreSQL Schema Definition

### Table Definition

The `#[PostgresTable]` attribute macro transforms a Rust struct into a
PostgreSQL table definition.

```rust
#[PostgresTable]                              // Table name defaults to snake_case
#[PostgresTable(name = "custom_name")]        // Custom table name
#[PostgresTable(unlogged)]                    // UNLOGGED table
#[PostgresTable(temporary)]                   // TEMPORARY table
#[PostgresTable(if_not_exists)]               // IF NOT EXISTS clause
```

### Column Types

| Attribute             | PostgreSQL Type  | Rust Types                      |
| --------------------- | ---------------- | ------------------------------- |
| `#[serial]`           | SERIAL           | `i32`                           |
| `#[bigserial]`        | BIGSERIAL        | `i64`                           |
| `#[smallint]`         | SMALLINT         | `i16`                           |
| `#[integer]`          | INTEGER          | `i32`                           |
| `#[bigint]`           | BIGINT           | `i64`                           |
| `#[real]`             | REAL             | `f32`                           |
| `#[double_precision]` | DOUBLE PRECISION | `f64`                           |
| `#[numeric]`          | NUMERIC          | `Decimal` (with `rust_decimal`) |
| `#[text]`             | TEXT             | `String`                        |
| `#[varchar]`          | VARCHAR          | `String`                        |
| `#[boolean]`          | BOOLEAN          | `bool`                          |
| `#[bytea]`            | BYTEA            | `Vec<u8>`                       |
| `#[uuid]`             | UUID             | `Uuid` (with `uuid` feature)    |
| `#[json]`             | JSON             | serde types (with `serde`)      |
| `#[jsonb]`            | JSONB            | serde types (with `serde`)      |
| `#[timestamp]`        | TIMESTAMP        | datetime types                  |
| `#[timestamptz]`      | TIMESTAMPTZ      | datetime types with timezone    |
| `#[date]`             | DATE             | date types                      |
| `#[time]`             | TIME             | time types                      |
| `#[r#enum(EnumType)]` | Custom ENUM      | Custom enum type                |

### Column Constraints

```rust
#[PostgresTable]
pub struct Users {
    // Auto-incrementing primary key
    #[serial(primary)]
    pub id: i32,

    // UUID primary key with default
    #[uuid(primary, default_fn = uuid::Uuid::new_v4)]
    pub uuid_id: Uuid,

    // Unique constraint
    #[text(unique)]
    pub email: String,

    // Foreign key reference
    #[integer(references = Posts::id)]
    pub post_id: i32,

    // Enum stored as TEXT
    #[text(enum)]
    pub role: UserRole,

    // Native PostgreSQL ENUM type
    #[r#enum(Priority)]
    pub priority: Priority,

    // JSON/JSONB (requires `serde` feature)
    #[jsonb]
    pub metadata: Option<serde_json::Value>,
}
```

#### Constraint Reference

| Constraint   | Description                   | Example                                  |
| ------------ | ----------------------------- | ---------------------------------------- |
| `primary`    | Primary key constraint        | `#[serial(primary)]`                     |
| `unique`     | Unique constraint             | `#[text(unique)]`                        |
| `default`    | Compile-time default value    | `#[text(default = "value")]`             |
| `default_fn` | Runtime default function      | `#[uuid(default_fn = Uuid::new_v4)]`     |
| `references` | Foreign key reference         | `#[integer(references = Table::column)]` |
| `enum`       | Store enum as TEXT or INTEGER | `#[text(enum)]` or `#[integer(enum)]`    |
| `json`       | JSON serialization            | `#[text(json)]`                          |

### Enum Definition

PostgreSQL supports both text-based enums and native ENUM types:

```rust
#[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
pub enum UserRole {
    #[default]
    User,
    Admin,
    Moderator,
}

#[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
pub enum Priority {
    Low = 1,
    #[default]
    Medium = 5,
    High = 10,
}

#[PostgresTable]
pub struct Tasks {
    #[serial(primary)]
    pub id: i32,

    // Store as TEXT: "User", "Admin", etc.
    #[text(enum)]
    pub role: UserRole,

    // Native PostgreSQL ENUM type
    #[r#enum(Priority)]
    pub priority: Priority,
}
```

### Index Definition

```rust
#[PostgresIndex]
pub struct UserEmailIdx(Users::email);

#[PostgresIndex(unique)]
pub struct UserEmailUniqueIdx(Users::email);

#[PostgresIndex]
pub struct UserNameEmailIdx(Users::name, Users::email);
```

### Schema Definition

For PostgreSQL, enums used with `#[r#enum(...)]` must be included in the schema:

```rust
#[derive(PostgresSchema)]
pub struct Schema {
    // Enums must be listed before tables that use them
    pub priority: Priority,
    pub role: UserRole,
    // Tables
    pub users: Users,
    pub tasks: Tasks,
    // Indexes
    pub user_email_idx: UserEmailIdx,
}
```

---

## Naming Conventions

By default, table and column names are converted to `snake_case`:

```rust
#[SQLiteTable]           // Table name: "my_users"
pub struct MyUsers {
    #[integer(primary)]
    pub userId: i32,     // Column name: "userId" (field name as-is)
}
```

Use the `name` attribute to customize:

```rust
#[SQLiteTable(name = "users")]
pub struct MyUsers {
    #[integer(primary)]
    pub id: i32,
}
```

---

## Nullability

Nullability is controlled by Rust's type system:

```rust
#[SQLiteTable(name = "example")]
pub struct Example {
    #[integer(primary)]
    pub id: i32,           // NOT NULL - required

    #[text]
    pub name: String,      // NOT NULL - required in InsertExample::new()

    #[text]
    pub email: Option<String>, // NULL allowed - set via .with_email()
}
```

---

## Generated Types

For a table named `Users`, the macro generates:

| Type          | Purpose              | Usage                                      |
| ------------- | -------------------- | ------------------------------------------ |
| `SelectUsers` | SELECT query results | `Vec<SelectUsers>`                         |
| `InsertUsers` | INSERT operations    | `InsertUsers::new(...).with_optional(...)` |
| `UpdateUsers` | UPDATE operations    | `UpdateUsers::default().with_field(...)`   |

### Insert Model

```rust
// Required fields go in new()
// Optional fields and fields with defaults use .with_*()
let insert = InsertUsers::new("Alice", 25)
    .with_email("alice@example.com")
    .with_role(UserRole::Admin);

db.insert(users).values([insert]).execute()?;

// Batch insert
db.insert(users)
    .values([
        InsertUsers::new("Alice", 25),
        InsertUsers::new("Bob", 30),
        InsertUsers::new("Charlie", 28),
    ])
    .execute()?;
```

### Update Model

```rust
let update = UpdateUsers::default()
    .with_name("Alice Smith")
    .with_age(26);

db.update(users)
    .set(update)
    .r#where(eq(users.id, 1))
    .execute()?;
```

---

## Query Building

### SELECT Queries

```rust
// Select all columns
let all_users: Vec<SelectUsers> = db.select(()).from(users).all()?;

// Select specific columns
let names: Vec<String> = db.select(users.name).from(users).all()?;

// Select multiple columns
let results = db.select((users.id, users.name)).from(users).all()?;

// Get single row (returns error if no rows found)
let user: SelectUsers = db.select(()).from(users).get()?;
```

### WHERE Conditions

```rust
use drizzle::prelude::*;

// Equality
db.select(()).from(users).r#where(eq(users.id, 1)).all()?;

// Comparison operators
db.select(()).from(users).r#where(gt(users.age, 18)).all()?;
db.select(()).from(users).r#where(gte(users.age, 18)).all()?;
db.select(()).from(users).r#where(lt(users.age, 65)).all()?;
db.select(()).from(users).r#where(lte(users.age, 65)).all()?;
db.select(()).from(users).r#where(ne(users.status, "inactive")).all()?;

// IS NULL / IS NOT NULL
db.select(()).from(users).r#where(is_null(users.email)).all()?;
db.select(()).from(users).r#where(is_not_null(users.email)).all()?;

// LIKE
db.select(()).from(users).r#where(like(users.name, "%Alice%")).all()?;

// IN
db.select(()).from(users).r#where(r#in(users.id, [1, 2, 3])).all()?;

// Combining conditions with AND/OR
db.select(())
    .from(users)
    .r#where(and(
        gte(users.age, 18),
        eq(users.status, "active")
    ))
    .all()?;

db.select(())
    .from(users)
    .r#where(or(
        eq(users.role, "admin"),
        eq(users.role, "moderator")
    ))
    .all()?;
```

### JOIN Queries

```rust
#[derive(SQLiteFromRow, Default, Debug)]
struct UserPost {
    #[column(Users::id)]
    user_id: i32,
    #[column(Users::name)]
    user_name: String,
    #[column(Posts::id)]
    post_id: i32,
    #[column(Posts::title)]
    title: String,
}

// INNER JOIN
let results: Vec<UserPost> = db
    .select(UserPost::default())
    .from(users)
    .inner_join(posts, eq(users.id, posts.user_id))
    .all()?;

// LEFT JOIN
let results: Vec<UserPost> = db
    .select(UserPost::default())
    .from(users)
    .left_join(posts, eq(users.id, posts.user_id))
    .all()?;

// RIGHT JOIN (PostgreSQL)
let results: Vec<UserPost> = db
    .select(UserPost::default())
    .from(users)
    .right_join(posts, eq(users.id, posts.user_id))
    .all()?;
```

### ORDER BY and LIMIT

```rust
db.select(())
    .from(users)
    .order_by(users.name, OrderBy::Asc)
    .limit(10)
    .offset(20)
    .all()?;
```

### DELETE Queries

```rust
db.delete(users)
    .r#where(eq(users.id, 1))
    .execute()?;
```

---

## FromRow Derive

The `FromRow` derive macro generates row-to-struct conversion:

### SQLite

```rust
#[derive(SQLiteFromRow, Debug, Default)]
struct User {
    id: i32,
    name: String,
    email: Option<String>,
}

// With column mapping for JOINs
#[derive(SQLiteFromRow, Debug, Default)]
struct UserPost {
    #[column(Users::id)]
    user_id: i32,
    #[column(Posts::id)]
    post_id: i32,
    name: String,
}

// Tuple structs
#[derive(SQLiteFromRow, Default)]
struct Count(i64);
```

### PostgreSQL

```rust
#[derive(PostgresFromRow, Debug, Default)]
struct User {
    id: i32,
    name: String,
    email: Option<String>,
}

#[derive(PostgresFromRow, Debug, Default)]
struct UserPost {
    #[column(Users::id)]
    user_id: i32,
    #[column(Posts::id)]
    post_id: i32,
}
```

---

## Transactions

### SQLite (Sync)

```rust
use drizzle::sqlite::SQLiteTransactionType;

db.transaction(SQLiteTransactionType::Deferred, |tx| {
    tx.insert(users)
        .values([InsertUsers::new("Alice", 25)])
        .execute()?;

    tx.update(users)
        .set(UpdateUsers::default().with_age(26))
        .r#where(eq(users.name, "Alice"))
        .execute()?;

    Ok(())
})?;
```

Transaction types: `Deferred`, `Immediate`, `Exclusive`

### SQLite (Async - libsql/turso)

```rust
db.transaction(SQLiteTransactionType::Deferred, |tx| Box::pin(async move {
    tx.insert(users)
        .values([InsertUsers::new("Alice", 25)])
        .execute()
        .await?;

    Ok(())
})).await?;
```

### PostgreSQL

```rust
use drizzle::postgres::PostgresTransactionType;

db.transaction(PostgresTransactionType::ReadCommitted, |tx| {
    tx.insert(users)
        .values([InsertUsers::new("Alice", 25)])
        .execute()?;

    Ok(())
})?;
```

---

## SQLite PRAGMA Support

```rust
use drizzle::sqlite::pragma::{Pragma, JournalMode, Synchronous};
use drizzle::core::ToSQL;

// Enable foreign keys
db.execute(Pragma::foreign_keys(true))?;

// Set WAL mode for better concurrency
db.execute(Pragma::journal_mode(JournalMode::Wal))?;

// Configure synchronous mode
db.execute(Pragma::Synchronous(Synchronous::Normal))?;

// Introspection
db.execute(Pragma::table_info("users"))?;
db.execute(Pragma::integrity_check(None))?;
```

---

## UUID Support

Enable the `uuid` feature for UUID support:

```toml
[dependencies]
drizzle = { version = "0.1", features = ["rusqlite", "uuid"] }
uuid = { version = "1.18", features = ["v4"] }
```

### SQLite (BLOB storage - recommended)

```rust
use uuid::Uuid;

#[SQLiteTable(name = "users")]
pub struct Users {
    #[blob(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,  // 16 bytes binary storage
}
```

### SQLite (TEXT storage)

```rust
#[SQLiteTable(name = "users")]
pub struct Users {
    #[text(primary, default_fn = || Uuid::new_v4())]
    pub id: Uuid,  // 36 character string storage
}
```

### PostgreSQL (native UUID type)

```rust
#[PostgresTable(name = "users")]
pub struct Users {
    #[uuid(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
}
```

---

## JSON Support

Enable the `serde` feature for JSON support:

```toml
[dependencies]
drizzle = { version = "0.1", features = ["rusqlite", "serde"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
```

### SQLite

```rust
#[derive(Serialize, Deserialize, Clone, Default)]
struct UserMetadata {
    theme: String,
    notifications: bool,
}

#[SQLiteTable(name = "users")]
pub struct Users {
    #[integer(primary)]
    pub id: i32,

    #[text(json)]  // JSON stored as TEXT
    pub metadata: Option<UserMetadata>,

    #[blob(json)]  // JSON stored as BLOB
    pub config: Option<UserMetadata>,
}
```

### PostgreSQL

```rust
#[PostgresTable(name = "users")]
pub struct Users {
    #[serial(primary)]
    pub id: i32,

    #[json]   // Standard JSON
    pub metadata: Option<serde_json::Value>,

    #[jsonb]  // Binary JSON (faster queries)
    pub config: Option<serde_json::Value>,
}
```

---

## Migrations

Embed migrations at compile time for runtime execution:

```rust
use drizzle::prelude::*;

const MIGRATIONS: EmbeddedMigrations = include_migrations!("./drizzle");

fn main() -> drizzle::Result<()> {
    let conn = rusqlite::Connection::open("app.db")?;
    let (db, schema) = Drizzle::new(conn, Schema::new());

    // Apply embedded migrations
    let applied = db.migrate(&MIGRATIONS)?;
    println!("Applied {} migrations", applied);

    Ok(())
}
```

---

## Raw SQL

Use the `sql!` macro for raw SQL with type safety:

```rust
use drizzle::sql;

// Embedded expressions
let query = sql!("SELECT * FROM {users} WHERE {users.id} = 42");

// Printf-style syntax
let query = sql!("SELECT * FROM {} WHERE {} = {}", users, users.id, 42);
```

---

## Feature Flags

| Feature          | Description                           |
| ---------------- | ------------------------------------- |
| `rusqlite`       | SQLite via rusqlite (sync)            |
| `libsql`         | SQLite via libsql (async)             |
| `turso`          | SQLite via turso (async)              |
| `postgres-sync`  | PostgreSQL via postgres crate (sync)  |
| `tokio-postgres` | PostgreSQL via tokio-postgres (async) |
| `uuid`           | UUID type support                     |
| `serde`          | JSON serialization support            |
| `chrono`         | Date/time types for PostgreSQL        |
| `arrayvec`       | Fixed-capacity strings and arrays     |

---

## License

MIT License - see [LICENSE](LICENSE) for details.
