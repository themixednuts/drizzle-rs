# Drizzle RS

A type-safe SQL query builder for Rust inspired by Drizzle ORM.

## Schema Setup

First, create a `schema.rs` file to define your database tables. All schema items must be `pub` for proper destructuring:

```rust
use drizzle_rs::{SQLSchema, sqlite::SQLiteTable};
use uuid::Uuid;

#[derive(SQLSchema)]
pub struct Schema {
    pub users: Users,
    pub posts: Posts,
}

#[SQLiteTable(name = "users")]
pub struct Users {
    #[blob(primary, default_fn = Uuid::new_v4)]
    pub id: Uuid,
    #[text]
    pub name: String,
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

Create indexes as separate structs:

```rust
use drizzle_rs::sqlite::{SQLiteTable, SQLiteIndex};
#[SQLiteTable]
struct User {
    #[integer(primary)]
    id: i32,
    #[text]
    email: String,
    #[text]
    username: String,
}

#[SQLiteIndex(unique)]
struct UserEmailUsernameIdx(User::email, User::username);

#[SQLiteIndex]
struct UserEmailIdx(User::email);
```

## Basic Usage

In your `main.rs`, use the schema without feature flags:

```rust
mod schema;

use drizzle_rs::{core::eq, drizzle};
use procmacros::FromRow;
use rusqlite::Connection;
use uuid::Uuid;

use crate::schema::{InsertPosts, InsertUsers, Posts, Schema, SelectPosts, SelectUsers, Users};

             // drizzle_rs::error::Result<()>
fn main() -> Result<(), drizzle_rs::error::DrizzleError> {
    let conn = Connection::open_in_memory()?;
    let (db, Schema { users, posts }) = drizzle!(conn, Schema);
    
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
    .with_email("john@example.com") // Optional fields via .with_*
```
> [!WARNING]  
> Avoid using `InsertUsers::default()`, as it will fail at runtime if required fields are not provided.

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

For multiple different operations or when you need ACID guarantees, use transactions:

```rust
use drizzle_rs::sqlite::SQLiteTransactionType;

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
```

Transaction types:
- `SQLiteTransactionType::Deferred` - Default, begins when first read/write
- `SQLiteTransactionType::Immediate` - Begins immediately with write lock
- `SQLiteTransactionType::Exclusive` - Exclusive access to database

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
