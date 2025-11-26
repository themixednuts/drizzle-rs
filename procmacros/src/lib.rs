//! # Drizzle RS Procedural Macros
//!
//! This crate provides the procedural macros for Drizzle RS, a type-safe SQL query builder for Rust.
//!
//! ## Core Macros
//!
//! ### SQLite
//! - [`SQLiteTable`] - Define SQLite table schemas with type safety
//! - [`SQLiteEnum`] - Define enums that can be stored in SQLite
//! - [`SQLiteIndex`] - Define indexes on SQLite tables
//! - [`SQLiteSchema`] - Derive macro to group tables and indexes into a schema
//!
//! ### PostgreSQL
//! - [`PostgresTable`] - Define PostgreSQL table schemas with type safety
//! - [`PostgresEnum`] - Define enums for PostgreSQL (text, integer, or native ENUM)
//! - [`PostgresIndex`] - Define indexes on PostgreSQL tables
//! - [`PostgresSchema`] - Derive macro to group tables and indexes into a schema
//!
//! ### Shared
//! - [`FromRow`] - Derive automatic row-to-struct conversion
//! - [`sql!`] - Build SQL queries with embedded expressions
//!
//! ## Example Usage
//!
//! ```ignore
//! use drizzle::prelude::*;
//! use drizzle::rusqlite::Drizzle;
//!
//! // Define your table
//! #[SQLiteTable(name = "users")]
//! struct Users {
//!     #[integer(primary, autoincrement)]
//!     id: i32,
//!     #[text]
//!     name: String,
//!     #[text]
//!     email: Option<String>,
//! }
//!
//! // Define your schema
//! #[derive(SQLiteSchema)]
//! struct Schema {
//!     users: Users,
//! }
//!
//! // Connect and use
//! let conn = rusqlite::Connection::open_in_memory()?;
//! let (db, Schema { users }) = Drizzle::new(conn, Schema::new());
//! db.create()?;
//!
//! // Insert data
//! db.insert(users)
//!     .values([InsertUsers::new("Alice").with_email("alice@example.com")])
//!     .execute()?;
//!
//! // Query data
//! let all_users: Vec<SelectUsers> = db.select(()).from(users).all()?;
//! ```
//!
//! For more detailed documentation, see the individual macro documentation below.

extern crate proc_macro;

mod drizzle_test;
mod fromrow;
mod generators;
mod sql;
mod utils;

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "rusqlite")]
mod rusqlite;

#[cfg(feature = "postgres")]
mod postgres;

use proc_macro::TokenStream;
use syn::parse_macro_input;

/// Derive macro for creating SQLite-compatible enums.
///
/// This macro allows enums to be stored in SQLite databases as either TEXT (variant names)
/// or INTEGER (discriminant values) depending on the column attribute used.
///
/// The enum can be used with `#[text(enum)]` or `#[integer(enum)]` column attributes.
///
/// # Requirements
///
/// - Enum must have at least one variant
/// - For `#[integer(enum)]`, variants can have explicit discriminants
/// - Must derive `Default` to specify the default variant
///
/// # Examples
///
/// ## Text Storage (Variant Names)
///
/// ```
/// use drizzle::prelude::*;
///
/// #[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
/// enum UserRole {
///     #[default]
///     User,      // Stored as "User"
///     Admin,     // Stored as "Admin"
///     Moderator, // Stored as "Moderator"
/// }
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text(enum)] // Stores variant names as TEXT
///     role: UserRole,
/// }
///
/// // The enum can be converted to/from strings
/// assert_eq!(UserRole::Admin.to_string(), "Admin");
/// ```
///
/// ## Integer Storage (Discriminants)
///
/// ```
/// use drizzle::prelude::*;
///
/// #[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
/// enum Priority {
///     #[default]
///     Low = 1,    // Stored as 1
///     Medium = 5, // Stored as 5
///     High = 10,  // Stored as 10
/// }
///
/// #[SQLiteTable(name = "tasks")]
/// struct Tasks {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[integer(enum)] // Stores discriminants as INTEGER
///     priority: Priority,
/// }
///
/// // The enum can be converted to/from integers
/// let p: i64 = Priority::High.into();
/// assert_eq!(p, 10);
/// ```
///
/// ## Generated Implementations
///
/// The macro automatically implements:
/// - `std::fmt::Display` - For TEXT representation
/// - `TryFrom<i64>` - For INTEGER representation  
/// - `Into<i64>` - For INTEGER representation
/// - `From<EnumType>` for `SQLiteValue` - Database conversion
/// - `TryFrom<SQLiteValue>` for `EnumType` - Database conversion
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SQLiteEnum)]
pub fn sqlite_enum_derive(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{Data, DeriveInput, parse_macro_input};

    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Check if this is an enum or tuple struct
    match &input.data {
        Data::Enum(data) => {
            // Check if the enum has any variants
            if data.variants.is_empty() {
                return quote! {
                    compile_error!("SQLiteEnum cannot be derived for empty enums");
                }
                .into();
            }

            // Generate implementation for enum
            match crate::sqlite::r#enum::generate_enum_impl(name, data) {
                Ok(ts) => ts.into(),
                Err(e) => e.to_compile_error().into(),
            }
        }
        _ => quote! {
            compile_error!("SQLiteEnum can only be derived for enums and tuple structs");
        }
        .into(),
    }
}

/// Define a SQLite table schema with type-safe column definitions.
///
/// This attribute macro transforms a Rust struct into a complete SQLite table definition
/// with generated types for INSERT, SELECT, and UPDATE operations.
///
/// See [SQLite CREATE TABLE documentation](https://sqlite.org/lang_createtable.html) for
/// the underlying SQL concepts.
///
/// # Table Attributes
///
/// - `name = "table_name"` - Custom table name (defaults to struct name in snake_case)
/// - `strict` - Enable [SQLite STRICT mode](https://sqlite.org/stricttables.html)  
/// - `without_rowid` - Create a [WITHOUT ROWID table](https://sqlite.org/withoutrowid.html)
///
/// # Field Attributes
///
/// ## Column Types
/// - `#[integer]` - SQLite INTEGER type
/// - `#[text]` - SQLite TEXT type
/// - `#[real]` - SQLite REAL type
/// - `#[blob]` - SQLite BLOB type
/// - `#[boolean]` - Stored as INTEGER (0/1)
///
/// ## Constraints
/// - `primary` - Primary key constraint
/// - `autoincrement` - Auto-increment (INTEGER PRIMARY KEY only)
/// - `unique` - Unique constraint
///
/// ## Defaults
/// - `default = value` - Compile-time default value
/// - `default_fn = function` - Runtime default function (called at insert time)
///
/// ## Special Types
/// - `enum` - Store enum as TEXT or INTEGER (requires `SQLiteEnum` derive)
/// - `json` - JSON serialization (requires `serde` feature)
/// - `references = Table::column` - Foreign key reference
///
/// # Examples
///
/// ## Basic Table
///
/// ```no_run
/// use drizzle::prelude::*;
/// use drizzle::rusqlite::Drizzle;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     name: String,
///     #[text(unique)]
///     email: String,
///     #[integer]
///     age: Option<i32>, // Nullable field
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     users: Users,
/// }
///
/// fn main() -> drizzle::Result<()> {
///     // Usage
///     let conn = rusqlite::Connection::open_in_memory()?;
///     let (db, Schema { users }) = Drizzle::new(conn, Schema::new());
///     db.create()?;
///
///     // Insert using generated InsertUsers type
///     db.insert(users)
///         .values([InsertUsers::new("Alice", "alice@example.com").with_age(25)])
///         .execute()?;
///
///     // Query using generated SelectUsers type
///     let all_users: Vec<SelectUsers> = db.select(()).from(users).all()?;
///     Ok(())
/// }
/// ```
///
/// ## Table with Defaults
///
/// ```
/// use drizzle::prelude::*;
///
/// #[SQLiteTable(name = "posts", strict)]
/// struct Posts {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     title: String,
///     #[text(default = "draft")]
///     status: String,
/// }
///
/// // Default value is used when not specified
/// let post = InsertPosts::new("My Title");
/// ```
///
/// ## Enums and JSON
///
/// ```ignore
/// use drizzle::prelude::*;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
/// enum Role {
///     #[default]
///     User,
///     Admin,
/// }
///
/// #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
/// struct Metadata {
///     theme: String,
/// }
///
/// #[SQLiteTable(name = "accounts")]
/// struct Accounts {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text(enum)]     // Store enum variant name as TEXT
///     role: Role,
///     #[text(json)]     // Serialize struct as JSON TEXT
///     metadata: Option<Metadata>,
/// }
/// ```
///
/// ## Foreign Key References
///
/// ```
/// use drizzle::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     name: String,
/// }
///
/// #[SQLiteTable(name = "posts")]
/// struct Posts {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[integer(references = Users::id)]  // Foreign key to users.id
///     author_id: i32,
///     #[text]
///     title: String,
/// }
/// ```
///
/// # Generated Types
///
/// For a table `Users`, the macro generates:
/// - `SelectUsers` - For SELECT operations (derives `FromRow`)
/// - `InsertUsers` - Builder for INSERT operations with `new()` and `with_*()` methods
/// - `UpdateUsers` - Builder for UPDATE operations with `set_*()` methods
///
/// # Nullability
///
/// Use `Option<T>` for nullable fields. Non-optional fields get a NOT NULL constraint:
///
/// ```
/// use drizzle::prelude::*;
///
/// #[SQLiteTable]
/// struct Example {
///     #[integer(primary, autoincrement)]
///     id: i32,               // NOT NULL, auto-generated
///     #[text]
///     name: String,          // NOT NULL (required in InsertExample::new())
///     #[text]
///     email: Option<String>, // NULL allowed (set via with_email())
/// }
///
/// // Non-optional, non-primary fields are required in new()
/// let insert = InsertExample::new("Alice").with_email("alice@example.com");
/// ```
#[cfg(feature = "sqlite")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn SQLiteTable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_result = syn::parse_macro_input!(attr as crate::sqlite::table::TableAttributes);

    match crate::sqlite::table::table_attr_macro(input, attr_result) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Attribute macro for creating SQLite indexes.
///
/// This macro generates SQLite-specific index definitions for columns in your tables.
/// Indexes improve query performance when filtering or sorting by the indexed columns.
///
/// # Attributes
///
/// - `unique` - Create a unique index (enforces uniqueness constraint)
/// - No attributes for standard index
///
/// # Examples
///
/// ## Unique Index
///
/// ```
/// use drizzle::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     email: String,
/// }
///
/// #[SQLiteIndex(unique)]
/// struct UserEmailIdx(Users::email);
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     users: Users,
///     user_email_idx: UserEmailIdx,
/// }
/// ```
///
/// ## Composite Index
///
/// Index on multiple columns:
///
/// ```
/// use drizzle::prelude::*;
///
/// #[SQLiteTable(name = "posts")]
/// struct Posts {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[integer]
///     author_id: i32,
///     #[text]
///     status: String,
/// }
///
/// #[SQLiteIndex]
/// struct PostAuthorStatusIdx(Posts::author_id, Posts::status);
/// ```
///
/// ## Standard (Non-Unique) Index
///
/// ```
/// use drizzle::prelude::*;
///
/// #[SQLiteTable(name = "logs")]
/// struct Logs {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     created_at: String,
/// }
///
/// #[SQLiteIndex]
/// struct LogsCreatedAtIdx(Logs::created_at);
/// ```
#[cfg(feature = "sqlite")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn SQLiteIndex(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_input = syn::parse_macro_input!(attr as crate::sqlite::index::IndexAttributes);

    match crate::sqlite::index::sqlite_index_attr_macro(attr_input, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Automatically implements row-to-struct conversion for database result types.
///
/// This derive macro generates `TryFrom` implementations for all enabled SQLite database
/// drivers, allowing seamless conversion from database rows to Rust structs.
///
/// # Supported Drivers
///
/// Implementations are generated based on enabled features:
/// - **rusqlite** - `TryFrom<&rusqlite::Row<'_>>` (sync)
/// - **libsql** - `TryFrom<&libsql::Row>` (async)
/// - **turso** - `TryFrom<&turso::Row>` (async)
///
/// # Supported Types
///
/// The macro automatically handles type conversion for:
///
/// | Rust Type | SQLite Type | Notes |
/// |-----------|-------------|-------|
/// | `i8`, `i16`, `i32`, `i64` | INTEGER | Auto-converts from i64 |
/// | `u8`, `u16`, `u32`, `u64` | INTEGER | Auto-converts from i64 |
/// | `f32`, `f64` | REAL | Auto-converts from f64 |
/// | `bool` | INTEGER | 0 = false, non-zero = true |
/// | `String` | TEXT | |
/// | `Vec<u8>` | BLOB | |
/// | `uuid::Uuid` | BLOB | Requires `uuid` feature |
/// | `Option<T>` | Any | Nullable columns |
///
/// # Field Attributes
///
/// - `#[column(Table::field)]` - Map to a specific table column (useful for JOINs)
/// - `#[json]` - Deserialize JSON from TEXT column (requires `serde` feature, libsql/turso only)
/// - No attribute - Maps to column with same name as the field
///
/// # Struct Types
///
/// Both named structs and tuple structs are supported:
/// - Named structs map fields by column name
/// - Tuple structs map fields by column index (0-based)
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use drizzle::prelude::*;
///
/// #[derive(FromRow, Debug, Default)]
/// struct User {
///     id: i32,
///     name: String,
///     email: Option<String>,  // Nullable column
///     active: bool,           // INTEGER 0/1 -> bool
/// }
/// ```
///
/// ## Custom Column Mapping (for JOINs)
///
/// When joining tables with columns of the same name, use `#[column(...)]` to
/// specify which table's column to use:
///
/// ```
/// use drizzle::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary)]
///     id: i32,
///     #[text]
///     name: String,
/// }
///
/// #[SQLiteTable(name = "posts")]
/// struct Posts {
///     #[integer(primary)]
///     id: i32,
///     #[integer(references = Users::id)]
///     user_id: i32,
///     #[text]
///     title: String,
/// }
///
/// #[derive(FromRow, Debug, Default)]
/// struct UserPost {
///     #[column(Users::id)]     // Explicitly use users.id
///     user_id: i32,
///     #[column(Users::name)]
///     user_name: String,
///     #[column(Posts::id)]     // Explicitly use posts.id
///     post_id: i32,
///     #[column(Posts::title)]
///     title: String,
/// }
/// ```
///
/// ## Tuple Structs
///
/// For simple single-column or multi-column results:
///
/// ```
/// use drizzle::prelude::*;
///
/// // Single column result
/// #[derive(FromRow, Default)]
/// struct Count(i64);
///
/// // Multiple columns by index
/// #[derive(FromRow, Default)]
/// struct IdAndName(i32, String);
/// ```
///
/// ## With UUID (requires `uuid` feature)
///
/// ```ignore
/// use drizzle::prelude::*;
/// use uuid::Uuid;
///
/// #[derive(FromRow, Debug)]
/// struct UserWithId {
///     id: Uuid,        // Stored as BLOB (16 bytes)
///     name: String,
/// }
/// ```
///
/// ## With JSON (requires `serde` feature, libsql/turso)
///
/// ```ignore
/// use drizzle::prelude::*;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Debug)]
/// struct Profile {
///     bio: String,
///     website: Option<String>,
/// }
///
/// #[derive(FromRow, Debug)]
/// struct UserWithProfile {
///     id: i32,
///     name: String,
///     #[json]  // Deserialize from JSON TEXT
///     profile: Profile,
/// }
/// ```
///
/// ## Tuple Structs
///
/// ```ignore
/// use drizzle::prelude::*;
///
/// #[derive(FromRow, Default)]
/// struct NameOnly(String);
///
/// let names: Vec<NameOnly> = db.select(users.name).from(users).all()?;
/// ```
#[proc_macro_derive(FromRow, attributes(column))]
pub fn from_row_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::fromrow::generate_from_row_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Derive macro for creating schema structures that manage tables and indexes.
///
/// This macro analyzes struct fields to automatically detect tables and indexes,
/// then generates methods to create all database objects in the correct order.
///
/// The schema provides:
/// - `Schema::new()` - Creates a new schema instance with all tables and indexes
/// - Integration with `Drizzle::new()` for database operations
/// - Automatic table and index creation via `db.create()`
///
/// # Examples
///
/// ## Basic Schema
///
/// ```no_run
/// use drizzle::prelude::*;
/// use drizzle::rusqlite::Drizzle;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     email: String,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     users: Users,
/// }
///
/// fn main() -> drizzle::Result<()> {
///     // Create connection and schema
///     let conn = rusqlite::Connection::open_in_memory()?;
///     let (db, Schema { users }) = Drizzle::new(conn, Schema::new());
///
///     // Create all tables
///     db.create()?;
///
///     // Use the schema
///     db.insert(users)
///         .values([InsertUsers::new("alice@example.com")])
///         .execute()?;
///     Ok(())
/// }
/// ```
///
/// ## Schema with Indexes
///
/// ```no_run
/// use drizzle::prelude::*;
/// use drizzle::rusqlite::Drizzle;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     email: String,
///     #[text]
///     name: String,
/// }
///
/// #[SQLiteIndex(unique)]
/// struct UserEmailIdx(Users::email);
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     users: Users,
///     user_email_idx: UserEmailIdx,
/// }
///
/// fn main() -> drizzle::Result<()> {
///     let conn = rusqlite::Connection::open_in_memory()?;
///     let (db, schema) = Drizzle::new(conn, Schema::new());
///
///     // Creates tables first, then indexes
///     db.create()?;
///     Ok(())
/// }
/// ```
///
/// ## Async Drivers (libsql, turso)
///
/// ```ignore
/// use drizzle::prelude::*;
/// use drizzle::libsql::Drizzle;  // or drizzle::turso::Drizzle
///
/// #[SQLiteTable]
/// struct Users {
///     #[integer(primary)]
///     id: i32,
///     #[text]
///     name: String,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     users: Users,
/// }
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
///     let db_builder = libsql::Builder::new_local(":memory:").build().await?;
///     let conn = db_builder.connect()?;
///     let (db, Schema { users }) = Drizzle::new(conn, Schema::new());
///
///     // Async create
///     db.create().await?;
///
///     // Async operations
///     db.insert(users)
///         .values([InsertUsers::new("Alice")])
///         .execute()
///         .await?;
///
///     Ok(())
/// }
/// ```
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SQLiteSchema)]
pub fn sqlite_schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::sqlite::schema::generate_schema_derive_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

#[cfg(feature = "postgres")]
#[proc_macro_derive(PostgresSchema)]
pub fn postgres_schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::postgres::generate_postgres_schema_derive_impl(input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// A procedural macro for building SQL queries with embedded expressions.
///
/// This macro supports two different syntax forms:
/// 1. **String literal syntax**: `sql!("SELECT * FROM {table}")`
/// 2. **Printf-style syntax**: `sql!("SELECT * FROM {} WHERE {} = {}", table, column, value)`
///
/// The macro parses SQL templates and generates type-safe SQL code by:
/// - Converting literal text to `SQL::text()` calls
/// - Converting expressions in `{braces}` to `.to_sql()` calls on the expression
///
/// # Syntax Forms
///
/// ## String Literal Syntax
///
/// Embed expressions directly in the SQL string using `{expression}`:
///
/// ```ignore
/// use drizzle::{sql, prelude::*};
/// use drizzle::rusqlite::Drizzle;
///
/// #[SQLiteTable(name = "users")]
/// pub struct Users {
///     #[integer(primary)]
///     pub id: i32,
///     #[text]
///     pub name: String,
/// }
///
/// #[derive(SQLiteSchema)]
/// pub struct Schema { pub users: Users }
///
/// let conn = rusqlite::Connection::open_in_memory()?;
/// let (db, Schema { users }) = Drizzle::new(conn, Schema::new());
///
/// let query = sql!("SELECT * FROM {users} WHERE {users.id} = 42");
/// ```
///
/// ## Printf-Style Syntax
///
/// Use `{}` placeholders with arguments after the string:
///
/// ```ignore
/// use drizzle::{sql, prelude::*};
/// use drizzle::rusqlite::Drizzle;
///
/// #[SQLiteTable(name = "users")]
/// pub struct Users {
///     #[integer(primary)]
///     pub id: i32,
/// }
///
/// #[derive(SQLiteSchema)]
/// pub struct Schema { pub users: Users }
///
/// let conn = rusqlite::Connection::open_in_memory()?;
/// let (db, Schema { users }) = Drizzle::new(conn, Schema::new());
///
/// let query = sql!("SELECT * FROM {} WHERE {} = {}", users, users.id, 42);
/// ```
///
/// # Examples
///
/// ## Basic Usage
///
/// ```
/// use drizzle::{sql, prelude::*};
///
/// #[SQLiteTable(name = "users")]
/// pub struct Users {
///     #[integer(primary)]
///     pub id: i32,
/// }
///
/// let users = Users::new();
/// let query = sql!("SELECT * FROM {users}");
/// // Generates: SQL::text("SELECT * FROM ").append(users.to_sql())
/// ```
///
/// ## Multiple Expressions
///
/// ```
/// use drizzle::{sql, prelude::*};
///
/// #[SQLiteTable(name = "users")]
/// pub struct Users {
///     #[integer(primary)]
///     pub id: i32,
/// }
///
/// #[SQLiteTable(name = "posts")]
/// pub struct Posts {
///     #[integer(primary)]
///     pub id: i32,
///     #[integer]
///     pub author_id: i32,
/// }
///
/// let users = Users::new();
/// let posts = Posts::new();
/// let query = sql!("SELECT * FROM {users} WHERE {users.id} = {posts.author_id}");
/// ```
///
/// ## Escaped Braces
///
/// Use `{{` and `}}` for literal braces in the SQL:
///
/// ```
/// use drizzle::{sql, prelude::*};
///
/// #[SQLiteTable(name = "users")]
/// pub struct Users {
///     #[integer(primary)]
///     pub id: i32,
/// }
///
/// let users = Users::new();
/// let query = sql!("SELECT JSON_OBJECT('key', {{literal}}) FROM {users}");
/// // Generates: SQL::text("SELECT JSON_OBJECT('key', {literal}) FROM ").append(users.to_sql())
/// ```
///
/// # Requirements
///
/// All expressions within `{braces}` must implement the `ToSQL` trait.
#[proc_macro]
pub fn sql(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as crate::sql::SqlInput);

    match crate::sql::sql_impl(input) {
        Ok(output) => output.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Generates test functions for all enabled SQLite drivers.
///
/// This macro creates separate test functions for rusqlite, libsql, and turso drivers,
/// each with proper async/sync handling and driver-specific setup.
///
/// # Syntax
///
/// ```ignore
/// drizzle_test!(test_name, SchemaType, {
///     // Test body - uses `db` and `schema` variables
///     let SchemaType { my_table } = schema;
///     let result = drizzle_exec!(db.insert(my_table).values([data]).execute());
///     assert_eq!(result, 1);
/// });
/// ```
///
/// # Generated Functions
///
/// For a test named `my_test`, this generates:
/// - `my_test_rusqlite()` - Sync test for rusqlite (when `rusqlite` feature enabled)
/// - `my_test_libsql()` - Async test for libsql (when `libsql` feature enabled)
/// - `my_test_turso()` - Async test for turso (when `turso` feature enabled)
///
/// # Available Macros in Test Body
///
/// - `drizzle_exec!(operation)` - Execute operation with proper async/sync handling
/// - `drizzle_try!(operation)` - Try operation, returns early on error
/// - `drizzle_tx!(tx_type, { body })` - Execute transaction with proper async/sync handling
///
/// # Variables Available in Test Body
///
/// - `db` - The Drizzle instance for the current driver
/// - `schema` - The schema instance with all tables
/// - `tx` - The transaction instance (within `drizzle_tx!` blocks)
///
/// # Example
///
/// ```ignore
/// use drizzle::prelude::*;
/// use drizzle_macros::drizzle_test;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     name: String,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct TestSchema {
///     users: Users,
/// }
///
/// drizzle_test!(insert_and_select, TestSchema, {
///     let TestSchema { users } = schema;
///
///     // Insert a user
///     drizzle_exec!(db.insert(users)
///         .values([InsertUsers::new("Alice")])
///         .execute());
///
///     // Select all users
///     let results: Vec<SelectUsers> = drizzle_exec!(
///         db.select(()).from(users).all()
///     );
///
///     assert_eq!(results.len(), 1);
///     assert_eq!(results[0].name, "Alice");
/// });
/// ```
#[proc_macro]
pub fn drizzle_test(input: TokenStream) -> TokenStream {
    crate::drizzle_test::drizzle_test_impl(input)
}

/// Derive macro for creating PostgreSQL-compatible enums.
///
/// This macro allows enums to be stored in PostgreSQL databases in three ways:
/// - **TEXT** - Store variant names as text (`#[text(enum)]`)
/// - **INTEGER** - Store discriminant values as integers (`#[integer(enum)]`)
/// - **Native ENUM** - Use PostgreSQL's native ENUM type (`#[enum(EnumType)]`)
///
/// # Requirements
///
/// - Enum must have at least one variant
/// - For `#[integer(enum)]`, variants can have explicit discriminants
/// - Must derive `Default` to specify the default variant
///
/// # Examples
///
/// ## Text Storage (Variant Names)
///
/// ```ignore
/// use drizzle::prelude::*;
///
/// #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
/// enum UserRole {
///     #[default]
///     User,      // Stored as "User"
///     Admin,     // Stored as "Admin"
///     Moderator, // Stored as "Moderator"
/// }
///
/// #[PostgresTable(name = "users")]
/// struct Users {
///     #[serial(primary)]
///     id: i32,
///     #[text(enum)]  // Stores variant names as TEXT
///     role: UserRole,
/// }
/// ```
///
/// ## Integer Storage (Discriminants)
///
/// ```ignore
/// use drizzle::prelude::*;
///
/// #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
/// enum Priority {
///     #[default]
///     Low = 1,    // Stored as 1
///     Medium = 5, // Stored as 5
///     High = 10,  // Stored as 10
/// }
///
/// #[PostgresTable(name = "tasks")]
/// struct Tasks {
///     #[serial(primary)]
///     id: i32,
///     #[integer(enum)]  // Stores discriminants as INTEGER
///     priority: Priority,
/// }
/// ```
///
/// ## Native PostgreSQL ENUM Type
///
/// PostgreSQL supports native ENUM types which are more efficient and type-safe:
///
/// ```ignore
/// use drizzle::prelude::*;
///
/// #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
/// enum Color {
///     #[default]
///     Red,
///     Green,
///     Blue,
/// }
///
/// #[PostgresTable(name = "items")]
/// struct Items {
///     #[serial(primary)]
///     id: i32,
///     #[r#enum(Color)]  // Uses PostgreSQL native ENUM type
///     color: Color,
/// }
/// ```
///
/// ## Generated Implementations
///
/// The macro automatically implements:
/// - `std::fmt::Display` - For TEXT representation
/// - `TryFrom<i64>` - For INTEGER representation
/// - `Into<i64>` - For INTEGER representation
/// - `From<EnumType>` for `PostgresValue` - Database conversion
/// - `TryFrom<PostgresValue>` for `EnumType` - Database conversion
#[cfg(feature = "postgres")]
#[proc_macro_derive(PostgresEnum)]
pub fn postgres_enum_derive(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{Data, DeriveInput, parse_macro_input};

    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Check if this is an enum
    match &input.data {
        Data::Enum(data) => {
            // Check if the enum has any variants
            if data.variants.is_empty() {
                return quote! {
                    compile_error!("PostgresEnum cannot be derived for empty enums");
                }
                .into();
            }

            // Generate implementation for enum
            match crate::postgres::r#enum::generate_enum_impl(name, data) {
                Ok(ts) => ts.into(),
                Err(e) => e.to_compile_error().into(),
            }
        }
        _ => quote! {
            compile_error!("PostgresEnum can only be derived for enums");
        }
        .into(),
    }
}

/// Define a PostgreSQL table schema with type-safe column definitions.
///
/// This attribute macro transforms a Rust struct into a complete PostgreSQL table definition
/// with generated types for INSERT, SELECT, and UPDATE operations.
///
/// See [PostgreSQL CREATE TABLE documentation](https://www.postgresql.org/docs/current/sql-createtable.html) for
/// the underlying SQL concepts.
///
/// # Table Attributes
///
/// - `name = "table_name"` - Custom table name (defaults to struct name in snake_case)
/// - `unlogged` - Create UNLOGGED table for better performance  
/// - `temporary` - Create TEMPORARY table
/// - `if_not_exists` - Add IF NOT EXISTS clause
///
/// # Field Attributes
///
/// ## Column Types
/// - `#[integer]` - PostgreSQL INTEGER type
/// - `#[bigint]` - PostgreSQL BIGINT type
/// - `#[smallint]` - PostgreSQL SMALLINT type
/// - `#[serial]` - PostgreSQL SERIAL type (auto-increment)
/// - `#[bigserial]` - PostgreSQL BIGSERIAL type
/// - `#[text]` - PostgreSQL TEXT type
/// - `#[varchar(n)]` - PostgreSQL VARCHAR(n) type
/// - `#[real]` - PostgreSQL REAL type
/// - `#[double_precision]` - PostgreSQL DOUBLE PRECISION type
/// - `#[boolean]` - PostgreSQL BOOLEAN type
/// - `#[bytea]` - PostgreSQL BYTEA type (binary data)
/// - `#[uuid]` - PostgreSQL UUID type (requires `uuid` feature)
/// - `#[json]` - PostgreSQL JSON type (requires `serde` feature)
/// - `#[jsonb]` - PostgreSQL JSONB type (requires `serde` feature)
/// - `#[enum(MyEnum)]` - PostgreSQL native ENUM type
///
/// ## Constraints
/// - `primary` - Primary key constraint
/// - `unique` - Unique constraint
///
/// ## Defaults
/// - `default = value` - Compile-time default value
/// - `default_fn = function` - Runtime default function
///
/// ## Special Types
/// - `enum` - Store enum as TEXT or INTEGER (`#[text(enum)]` or `#[integer(enum)]`)
/// - `json` - JSON serialization (`#[text(json)]` or `#[jsonb]`)
/// - `references = Table::column` - Foreign key reference
///
/// # Examples
///
/// ## Basic Table
///
/// ```ignore
/// use drizzle::prelude::*;
///
/// #[PostgresTable(name = "users")]
/// struct Users {
///     #[serial(primary)]
///     id: i32,
///     #[text]
///     name: String,
///     #[text(unique)]
///     email: String,
///     #[integer]
///     age: Option<i32>,  // Nullable field
/// }
///
/// #[derive(PostgresSchema)]
/// struct Schema {
///     users: Users,
/// }
/// ```
///
/// ## Enums (Text, Integer, and Native)
///
/// ```ignore
/// use drizzle::prelude::*;
///
/// #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
/// enum Status {
///     #[default]
///     Draft,
///     Published,
///     Archived,
/// }
///
/// #[derive(PostgresEnum, Default, Clone, PartialEq, Debug)]
/// enum Priority {
///     #[default]
///     Low,
///     Medium,
///     High,
/// }
///
/// #[PostgresTable(name = "posts")]
/// struct Posts {
///     #[serial(primary)]
///     id: i32,
///     #[text]
///     title: String,
///     #[text(enum)]       // Store as TEXT: "Draft", "Published", etc.
///     status: Status,
///     #[r#enum(Priority)] // Native PostgreSQL ENUM type
///     priority: Priority,
/// }
/// ```
///
/// ## JSON and JSONB
///
/// ```ignore
/// use drizzle::prelude::*;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(Serialize, Deserialize, Clone, PartialEq, Debug, Default)]
/// struct Metadata {
///     theme: String,
///     notifications: bool,
/// }
///
/// #[PostgresTable(name = "settings")]
/// struct Settings {
///     #[serial(primary)]
///     id: i32,
///     #[jsonb]  // Binary JSON for faster queries
///     config: Metadata,
///     #[json]   // Standard JSON
///     raw_data: Option<serde_json::Value>,
/// }
/// ```
///
/// # Generated Types
///
/// For a table `Users`, the macro generates:
/// - `SelectUsers` - For SELECT operations (derives `FromRow`)
/// - `InsertUsers` - Builder for INSERT operations with `new()` and `with_*()` methods
/// - `UpdateUsers` - Builder for UPDATE operations with `set_*()` methods
///
/// # Nullability
///
/// Use `Option<T>` for nullable fields. Non-optional fields get a NOT NULL constraint.
#[cfg(feature = "postgres")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PostgresTable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_result = syn::parse_macro_input!(attr as crate::postgres::table::TableAttributes);

    match crate::postgres::table::table_attr_macro(input, attr_result) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}

/// Attribute macro for creating PostgreSQL indexes.
///
/// This macro generates PostgreSQL-specific index definitions with support for
/// various PostgreSQL index features.
///
/// # Attributes
///
/// - `unique` - Create a unique index
/// - No attributes for standard index
///
/// # Examples
///
/// ## Unique Index
///
/// ```ignore
/// use drizzle::prelude::*;
///
/// #[PostgresTable(name = "users")]
/// struct Users {
///     #[serial(primary)]
///     id: i32,
///     #[text]
///     email: String,
/// }
///
/// #[PostgresIndex(unique)]
/// struct UserEmailIdx(Users::email);
///
/// #[derive(PostgresSchema)]
/// struct Schema {
///     users: Users,
///     user_email_idx: UserEmailIdx,
/// }
/// ```
///
/// ## Composite Index
///
/// ```ignore
/// use drizzle::prelude::*;
///
/// #[PostgresTable(name = "users")]
/// struct Users {
///     #[serial(primary)]
///     id: i32,
///     #[text]
///     email: String,
///     #[integer]
///     organization_id: i32,
/// }
///
/// #[PostgresIndex(unique)]
/// struct UserOrgIdx(Users::email, Users::organization_id);
/// ```
#[cfg(feature = "postgres")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn PostgresIndex(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = syn::parse_macro_input!(item as syn::DeriveInput);
    let attr_input = syn::parse_macro_input!(attr as crate::postgres::index::IndexAttributes);

    match crate::postgres::index::postgres_index_attr_macro(attr_input, input) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
    }
}
