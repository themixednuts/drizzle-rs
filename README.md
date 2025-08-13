# Drizzle RS

A type-safe SQL query builder for Rust inspired by Drizzle ORM.

## Features

- **Type Safety**: Compile-time guarantees for SQL queries and schema definitions
- **SQLite Support**: Multiple SQLite drivers (rusqlite, turso, libsql)
- **Schema-First**: Define your database schema with Rust structs and derive macros
- **Query Builder**: Fluent API for building complex SQL queries
- **Zero-Cost Abstractions**: Minimal runtime overhead with compile-time optimizations

## Quick Start

```rust
use drizzle_rs::prelude::*;

// Define your schema
#[SQLiteTable(name = "users")]
struct Users {
    #[integer(primary)]
    id: i32,
    #[text]
    name: String,
    #[text]
    email: String,
    #[integer]
    age: Option<i32>, // Optional field - nullable in database
}

// Use the schema
let (db, users) = drizzle!(connection, Users);

// Insert data - use ::new() for required fields
let inserted = db.insert(users)
    .values([
        InsertUsers::new("Alice", "alice@example.com"), // Required fields
        InsertUsers::new("Bob", "bob@example.com").with_age(25), // Optional fields via .with_*
    ])
    .execute()?;

// Query data
let all_users: Vec<SelectUsers> = db
    .select(())
    .from(users)
    .all()?;

let user: SelectUsers = db
    .select(())
    .from(users)
    .where_(users.email.eq("alice@example.com"))
    .get()?;

// Partial selection with FromRow
#[derive(FromRow, Debug)]
struct UserName {
    name: String,
}

let names: Vec<UserName> = db
    .select(users.name)
    .from(users)
    .all()?;
```

## Schema Definition

### Table Attributes

```rust
#[SQLiteTable] // Basic table
#[SQLiteTable(name = "custom_name")] // Custom table name
#[SQLiteTable(strict)] // SQLite STRICT mode
#[SQLiteTable(without_rowid)] // WITHOUT ROWID table
#[SQLiteTable(name = "users", strict, without_rowid)] // Combined
```

### Field Attributes

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

### Nullability

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
    #[integer]
    age: Option<i32>,  // NULL allowed - optional field
}

// Insert usage
InsertExample::new(1, "John".to_string()) // Required fields only
    .with_email("john@example.com".to_string()) // Optional fields
    .with_age(25)
```

## Advanced Features

### UUID Support

Choose between binary storage (BLOB) or string storage (TEXT):

```rust
use uuid::Uuid;

#[SQLiteTable(name = "posts")]
struct Posts {
    // Option 1: Store UUID as binary (16 bytes) - more efficient
    #[blob(primary, default_fn = Uuid::new_v4)]
    id: Uuid,
    
    // Option 2: Store UUID as string (36 characters) - human readable
    // #[text(primary, default_fn = || Uuid::new_v4().to_string())]
    // id: String,
    
    #[text]
    title: String,
}
```

### Enums

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

### JSON Fields

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

## SQLite Drivers

```toml
[dependencies]
# For local SQLite files (stable)
drizzle_rs = { version = "0.1", features = ["rusqlite"] }

# For libSQL (local + remote)
drizzle_rs = { version = "0.1", features = ["libsql"] }

# For Turso (alpha driver)
drizzle_rs = { version = "0.1", features = ["turso"] }

# With additional features
drizzle_rs = { version = "0.1", features = ["rusqlite", "uuid", "serde"] }
```

## Benefits

- **Compile-time error checking** - catch schema mismatches before runtime
- **IDE autocompletion** for columns, tables, and generated types
- **Refactoring safety** - rename columns and get compile errors where they're used
- **Type-safe queries** - no string-based SQL, all generated from typed constructs
- **Zero runtime overhead** - compile-time code generation with minimal abstractions
- **Flexible nullability** - use `Option<T>` vs `T` to control NULL constraints

## License

MIT