# Drizzle RS

A type-safe SQL query builder for Rust inspired by Drizzle ORM.

## Schema Setup

First, create a `schema.rs` file to define your database tables. All schema
items must be `pub` for proper destructuring:

```rust
use drizzle::prelude::*;
use uuid::Uuid;

#[derive(SQLiteSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
    pub user_email_idx: UserEmailIdx,
    pub user_email_username_idx: UserEmailUsernameIdx,
}

#[SQLiteTable(name = "users")]
pub struct Users {
    #[blob(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
    #[text]
    pub name: String,
    #[text]
    pub email: Option<String>,
    #[integer]
    pub age: u64,
}

#[SQLiteTable(name = "posts")]
pub struct Posts {
    #[blob(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
    #[blob(references = Users::id)]
    pub user_id: Uuid,
    #[text]
    pub context: Option<String>,
}

// Index definitions
#[SQLiteIndex(unique)]
pub struct UserEmailUsernameIdx(Users::email, Users::name);

#[SQLiteIndex]
pub struct UserEmailIdx(Users::email);
```

### UUID Storage Options

Choose between binary (BLOB) or string (TEXT) storage:

```rust
// Binary storage (16 bytes) - more efficient
#[blob(primary, default_fn = Uuid::new_v4)]
pub id: Uuid,

// String storage (36 characters) - human readable
#[text(primary, default_fn = || Uuid::new_v4)]
pub id: Uuid,
```

### Indexes

Indexes are defined as separate structs and included in your schema. They
reference table columns using the `Table::column` syntax:

```rust
use drizzle::prelude::*;

// Unique compound index on email and name
#[SQLiteIndex(unique)]
pub struct UserEmailUsernameIdx(Users::email, Users::name);

// Simple index on email column
#[SQLiteIndex]
pub struct UserEmailIdx(Users::email);
```

The indexes are automatically created when you call `db.create()` and must be
included as fields in your schema struct.

## Basic Usage

In your `main.rs`, use the schema without feature flags:

```rust
mod schema;

use drizzle::prelude::*;
use drizzle::rusqlite::Drizzle;
use rusqlite::Connection;
use uuid::Uuid;

use crate::schema::{InsertPosts, InsertUsers, Posts, Schema, SelectPosts, SelectUsers, Users};

fn main() -> drizzle::Result<()> {
    let conn = Connection::open_in_memory()?;
    let (mut db, Schema { users, posts, .. }) = Drizzle::new(conn, Schema::new());

    // Create tables (only on fresh database)
    db.create()?;

    let id = Uuid::new_v4();

    // Insert data
    db.insert(users)
        .values([InsertUsers::new("Alex Smith", 26).with_id(id)])
        .execute()?;

    db.insert(posts)
        .values([InsertPosts::new(id).with_context("just testing")])
        .execute()?;

    // Query data
    let user_rows: Vec<SelectUsers> = db.select(()).from(users).all()?;
    let post_rows: Vec<SelectPosts> = db.select(()).from(posts).all()?;

    println!("Users: {:?}", user_rows);
    println!("Posts: {:?}", post_rows);

    // JOIN queries with custom result struct
    #[derive(FromRow, Default, Debug)]
    struct JoinedResult {
        #[column(Users::id)]
        id: Uuid,
        #[column(Posts::id)]
        post_id: Uuid,
        name: String,
        age: u64,
    }

    let row: JoinedResult = db
        .select(JoinedResult::default())
        .from(users)
        .left_join(posts, eq(users.id, posts.user_id))
        .get()?;

    Ok(())
}
```

## Insert Models

```rust
// Always use new() as it forces you at compile time to input required fields
InsertUsers::new("John Doe", 25)
    .with_email("john@example.com") // Optional fields or fields with defaults via .with_*
```

> [!WARNING]\
> Avoid using `InsertUsers::default()`, as it will fail at runtime if required
> fields are not provided.

The `.values()` method automatically batches inserts of the same type:

```rust
// Same insert model type - will batch
db.insert(users)
    .values([
        InsertUsers::new("Alice", 30),
        InsertUsers::new("Bob", 25),
    ])
    .execute()?;

// compile time failure
db.insert(users)
    .values([
        InsertUsers::new("Alice", 30),
        InsertUsers::new("Bob", 25).with_email("bob@example.com"),
    ])
    .execute()?;
```

## Transactions

For multiple different operations or when you need ACID guarantees, use
transactions:

```rust
use drizzle::sqlite::SQLiteTransactionType;

// sync drivers
db.transaction(SQLiteTransactionType::Deferred, |tx| {
    // Insert users
    tx.insert(users)
        .values([InsertUsers::new("Alice", 30)])
        .execute()?;

    // Insert posts
    tx.insert(posts)
        .values([InsertPosts::new(user_id)])
        .execute()?;

    // Update data
    tx.update(users)
        .set(UpdateUsers::default().with_age(31))
        .r#where(eq(users.name, "Alice"))
        .execute()?;

    Ok(())
})?;

// async drivers - api is wip as I think pinning here is gross.
db.transaction(SQLiteTransactionType::Deferred, |tx| Box::pin(async move {
    // Insert users
    tx.insert(users)
        .values([InsertUsers::new("Alice", 30)])
        .execute()
        .await?;

    // Insert posts
    tx.insert(posts)
        .values([InsertPosts::new(user_id)])
        .execute()
        .await?;

    // Update data
    tx.update(users)
        .set(UpdateUsers::default().with_age(31))
        .r#where(eq(users.name, "Alice"))
        .execute()
        .await?;

    Ok(())
})).await?;
```

For more details on transaction types, see the
[SQLite Transaction Documentation](https://www.sqlite.org/lang_transaction.html).

## Table Attributes

```rust
#[SQLiteTable] // Basic table
#[SQLiteTable(name = "custom_name")] // Custom table name
#[SQLiteTable(strict)] // SQLite STRICT mode
#[SQLiteTable(without_rowid)] // WITHOUT ROWID table
#[SQLiteTable(name = "users", strict, without_rowid)] // Combined
```

## Field Attributes

```rust
// Column types
#[integer] // INTEGER column
#[text]    // TEXT column
#[real]    // REAL column
#[blob]    // BLOB column
#[boolean] // Stored as INTEGER (0/1)

// Constraints
#[integer(primary)]              // Primary key
#[integer(primary, autoincrement)] // Auto-incrementing primary key
#[text(unique)]                  // Unique constraint
#[text(primary)]                 // Text primary key

// Default values
#[text(default = "hello")]             // Compile-time default
#[integer(default = 42)]               // Compile-time default
#[text(default_fn = String::new)]      // Runtime default function

// Special types
#[text(enum)]    // Store enum as TEXT
#[integer(enum)] // Store enum as INTEGER
#[text(json)]    // JSON serialized to TEXT
#[blob(json)]    // JSON serialized to BLOB

// Foreign keys
#[integer(references = Users::id)] // Foreign key reference
```

## Nullability

Nullability is controlled by Rust's type system:

```rust
#[SQLiteTable(name = "example")]
struct Example {
    #[integer(primary)]
    id: i32,           // NOT NULL - required field
    #[text]
    name: String,      // NOT NULL - required field
    #[text]
    email: Option<String>, // NULL allowed - optional field
}
```

## Enums

```rust
#[derive(SQLiteEnum, Default)]
enum UserRole {
    #[default]
    User,
    Admin,
    Moderator,
}

#[SQLiteTable(name = "users")]
struct Users {
    #[integer(primary)]
    id: i32,
    #[text(enum)] // Stored as TEXT: "User", "Admin", "Moderator"
    role: UserRole,
}
```

## JSON Fields

```rust
#[derive(serde::Serialize, serde::Deserialize)]
struct UserMetadata {
    preferences: Vec<String>,
    theme: String,
}

#[SQLiteTable(name = "users")]
struct Users {
    #[integer(primary)]
    id: i32,
    #[text(json)] // JSON stored as TEXT
    metadata: Option<UserMetadata>,
    #[blob(json)] // JSON stored as BLOB (binary)
    config: Option<UserMetadata>,
}
```

## FromRow Derive Macro

The `FromRow` derive macro automatically generates `TryFrom<&Row>`
implementations for converting database rows into your structs.

### Basic Usage

```rust
use drizzle::prelude::*;

#[derive(FromRow, Debug)]
struct User {
    id: i32,
    name: String,
    email: String,
}

// Query returns User structs directly
let users: Vec<User> = db.select(()).from(users_table).all()?;
```

### Column Mapping

Use the `#[column]` attribute to map struct fields to specific table columns:

```rust
#[derive(FromRow, Debug)]
struct UserInfo {
    #[column(Users::id)]
    user_id: i32,
    #[column(Users::name)]
    full_name: String,
    #[column(Users::email)]
    email_address: String,
}

// Use in SELECT queries with custom column mapping
// You can collect into your favorite container
let info: Vec<UserInfo> = db
    .select(UserInfo::default())
    .from(users)
    .all()?;
```

### JOIN Queries

Perfect for handling JOIN query results:

```rust
#[derive(FromRow, Debug)]
struct UserPost {
    #[column(Users::id)]
    user_id: i32,
    #[column(Users::name)]
    user_name: String,
    #[column(Posts::id)]
    post_id: i32,
    #[column(Posts::title)]
    post_title: String,
}

let results: Vec<UserPost> = db
    .select(UserPost::default())
    .from(users)
    .inner_join(posts, eq(users.id, posts.user_id))
    .all()?;
```

### Tuple Structs

Also works with tuple structs for simple cases:

```rust
#[derive(FromRow, Debug)]
struct UserName(String);

let names: Vec<UserName> = db
    .select(users.name)
    .from(users)
    .all()?;
```

The macro automatically handles:

- Type conversions between SQLite types and Rust types
- Optional fields (`Option<T>`)
- All supported column types (integers, text, blobs, JSON, enums)

## SQLite PRAGMA Support

```rust
use drizzle::sqlite::pragma::{Pragma, JournalMode};

// Type-safe pragma statements
let pragma = Pragma::foreign_keys(true);
// Execute with drizzle
db.execute(pragma)?;
```
