//! # Drizzle RS Procedural Macros
//!
//! This crate provides the procedural macros for Drizzle RS, a type-safe SQL query builder for Rust.
//!
//! ## Core Macros
//!
//! - [`drizzle!`] - Initialize a Drizzle instance with database connection and schemas
//! - [`SQLiteTable`] - Define SQLite table schemas with type safety
//! - [`SQLiteEnum`] - Define enums that can be stored in SQLite
//! - [`FromRow`] - Derive automatic row-to-struct conversion
//!
//! ## Example Usage
//!
//! ```ignore
//! use drizzle_rs::prelude::*;
//!
//! // Define your schema
//! #[SQLiteTable(name = "users")]
//! struct Users {
//!     #[integer(primary)]
//!     id: i32,
//!     #[text]
//!     name: String,
//!     #[text]
//!     email: String,
//! }
//! ```
//!
//! For more detailed documentation, see the individual macro documentation below.

extern crate proc_macro;

mod drivers_test;
mod fromrow;
mod schema;
mod sql;
mod utils;

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "rusqlite")]
mod rusqlite;

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
/// ```ignore
/// use drizzle_rs::prelude::*;
///
/// #[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
/// enum UserRole {
///     #[default]
///     User,      // Stored as "User"
///     Admin,     // Stored as "Admin"
///     Moderator, // Stored as "Moderator"
/// }
///
/// #[SQLiteTable]
/// struct Users {
///     #[integer(primary)]
///     id: i32,
///     #[text(enum)] // Stores variant names as TEXT
///     role: UserRole,
/// }
/// ```
///
/// ## Integer Storage (Discriminants)
/// ```ignore
/// use drizzle_rs::prelude::*;
///
/// #[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
/// enum Priority {
///     #[default]
///     Low = 1,    // Stored as 1
///     Medium = 5, // Stored as 5
///     High = 10,  // Stored as 10
/// }
///
/// #[SQLiteTable]
/// struct Tasks {
///     #[integer(primary)]
///     id: i32,
///     #[integer(enum)] // Stores discriminants as INTEGER
///     priority: Priority,
/// }
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
/// - `default_fn = function` - Runtime default function
///
/// ## Special Types
/// - `enum` - Store enum as TEXT or INTEGER
/// - `json` - JSON serialization (requires serde feature)
/// - `references = Table::column` - Foreign key reference
///
/// # Examples
///
/// ## Basic Table
/// ```ignore
/// use drizzle_rs::prelude::*;
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
/// ```
///
/// ## Table with Defaults
/// ```ignore
/// use drizzle_rs::prelude::*;
///
/// #[SQLiteTable(name = "posts", strict)]
/// struct Posts {
///     #[integer(primary, autoincrement)]
///     id: i32,
///     #[text]
///     title: String,
///     #[text(default = "draft")]
///     status: String,
///     #[text(default_fn = || chrono::Utc::now().to_rfc3339())]
///     created_at: String,
/// }
/// ```
///
/// ## Enums and JSON
/// ```ignore
/// use drizzle_rs::prelude::*;
/// # #[cfg(feature = "serde")]
/// use serde::{Serialize, Deserialize};
///
/// #[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
/// enum Role {
///     #[default]
///     User,
///     Admin
/// }
///
/// # #[cfg(feature = "serde")]
/// #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
/// struct Metadata { theme: String }
///
///
/// # #[cfg(feature = "serde")]
/// #[SQLiteTable(name = "accounts")]
/// struct Accounts {
///     #[integer(primary)]
///     id: i32,
///     #[text(enum)]
///     role: Role,
///     #[text(json)]
///     metadata: Option<Metadata>,
/// }
/// ```
///
/// # Generated Types
///
/// For a table `Users`, the macro generates:
/// - `SelectUsers` - For SELECT operations
/// - `PartialSelectUsers` - For partial SELECT operations  
/// - `InsertUsers` - For INSERT operations
/// - `UpdateUsers` - For UPDATE operations
///
/// # Nullability
///
/// Use `Option<T>` for nullable fields, or `T` for NOT NULL constraints:
///
/// ```ignore
/// use drizzle_rs::prelude::*;
///
/// #[SQLiteTable]
/// struct Example {
///     #[integer(primary)]
///     id: i32,           // NOT NULL
///     #[text]
///     name: String,      // NOT NULL  
///     #[text]
///     email: Option<String>, // NULL allowed
/// }
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

/// Attribute macro for creating SQLite indexes
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

/// Automatically implements `TryFrom<&Row<'_>>` for structs using field name-based column access.
///
/// This derive macro generates a `TryFrom` implementation that maps struct fields to rusqlite
/// columns using the field names directly (e.g., `name: row.get("name")?`).
///
/// # Example
///
/// ```ignore
/// #[derive(FromRow, Debug)]
/// struct User {
///     id: i32,
///     name: String,
///     email: Option<String>,
/// }
///
/// // Generated implementation:
/// impl TryFrom<&Row<'_>> for User {
///     type Error = rusqlite::Error;
///     fn try_from(row: &Row<'_>) -> Result<Self, Self::Error> {
///         Ok(Self {
///             id: row.get("id")?,
///             name: row.get("name")?,
///             email: row.get("email")?,
///         })
///     }
/// }
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
/// # Examples
///
/// ```ignore
/// use drizzle_rs::prelude::*;
///
/// #[SQLiteTable]
/// struct Users {
///     #[integer(primary)]
///     id: i32,
///     #[text]
///     email: String,
/// }
///
/// #[SQLiteIndex(unique)]
/// struct UserEmailIdx(Users::email);
///
/// #[derive(SQLSchema)]
/// struct AppSchema {
///     users: Users,
///     user_email_idx: UserEmailIdx,
/// }
///
/// # #[cfg(any(feature = "libsql", feature = "turso"))]
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # #[cfg(feature = "libsql")]
/// # let connection = rt.block_on(async { libsql::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "turso")]
/// # let connection = rt.block_on(async { turso::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "rusqlite")]
/// # let connection = rusqlite::Connection::open_in_memory().unwrap();
///
/// // Usage
/// let (db, schema) = drizzle!(connection, AppSchema);
/// # #[cfg(any(feature = "libsql", feature = "turso"))]
/// # rt.block_on(async { db.create().await }).unwrap();
/// # #[cfg(feature = "rusqlite")]
/// # db.create().unwrap(); // Creates tables, then indexes
/// ```
#[proc_macro_derive(SQLSchema)]
pub fn schema_derive(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as syn::DeriveInput);

    match crate::schema::generate_schema_derive_impl(input) {
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
/// ```ignore
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] pub struct Users { #[integer(primary)] pub id: i32 }
/// # #[derive(SQLSchema)] pub struct UserSchema { pub users: Users }
/// # #[cfg(any(feature = "libsql", feature = "turso"))]
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # #[cfg(feature = "libsql")]
/// # let conn = rt.block_on(async { libsql::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "turso")]
/// # let conn = rt.block_on(async { turso::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "rusqlite")]
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, UserSchema { users }) = drizzle!(conn, UserSchema);
/// let query = sql!("SELECT * FROM {users} WHERE {users.id} = 42");
/// ```
///
/// ## Printf-Style Syntax
/// ```ignore
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] pub struct Users { #[integer(primary)] pub id: i32 }
/// # #[derive(SQLSchema)] pub struct UserSchema { pub users: Users }
/// # #[cfg(any(feature = "libsql", feature = "turso"))]
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # #[cfg(feature = "libsql")]
/// # let conn = rt.block_on(async { libsql::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "turso")]
/// # let conn = rt.block_on(async { turso::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "rusqlite")]
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, UserSchema { users }) = drizzle!(conn, UserSchema);
/// let query = sql!("SELECT * FROM {} WHERE {} = {}", users, users.id, 42);
/// ```
///
/// # Examples
///
/// ## Basic Usage
/// ```ignore
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] pub struct Users { #[integer(primary)] pub id: i32 }
/// # #[derive(SQLSchema)] pub struct UserSchema { pub users: Users }
/// # #[cfg(any(feature = "libsql", feature = "turso"))]
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # #[cfg(feature = "libsql")]
/// # let conn = rt.block_on(async { libsql::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "turso")]
/// # let conn = rt.block_on(async { turso::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "rusqlite")]
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, UserSchema { users }) = drizzle!(conn, UserSchema);
/// let query = sql!("SELECT * FROM {users}");
/// // Generates: SQL::text("SELECT * FROM ").append(users.to_sql())
/// ```
///
/// ## Multiple Expressions
/// ```ignore
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] pub struct Users { #[integer(primary)] pub id: i32 }
/// # #[derive(SQLSchema)] pub struct UserSchema { pub users: Users }
/// # #[SQLiteTable] pub struct Posts { #[integer(primary)] pub id: i32, #[integer] pub author: i32 }
/// # #[derive(SQLSchema)] pub struct BlogSchema { pub users: Users, pub posts: Posts }
/// # #[cfg(any(feature = "libsql", feature = "turso"))]
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # #[cfg(feature = "libsql")]
/// # let conn = rt.block_on(async { libsql::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "turso")]
/// # let conn = rt.block_on(async { turso::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "rusqlite")]
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, BlogSchema { users, posts }) = drizzle!(conn, BlogSchema);
/// let query = sql!("SELECT * FROM {users} WHERE {users.id} = {posts.author}");
/// ```
///
/// ## Escaped Braces
/// Use `{{` and `}}` for literal braces in the SQL:
/// ```ignore
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] pub struct Users { #[integer(primary)] pub id: i32 }
/// # #[derive(SQLSchema)] pub struct UserSchema { pub users: Users }
/// # #[cfg(any(feature = "libsql", feature = "turso"))]
/// # let rt = tokio::runtime::Runtime::new().unwrap();
/// # #[cfg(feature = "libsql")]
/// # let conn = rt.block_on(async { libsql::Builder::new_local(":memory:").build().await.unwrap().connect() }).unwrap();
/// # #[cfg(feature = "turso")]
/// # let conn = rt.block_on(async { turso::Builder::new_local(":memory:").build().await.unwrap().connect().await }).unwrap();
/// # #[cfg(feature = "rusqlite")]
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, UserSchema { users }) = drizzle!(conn, UserSchema);
/// let query = sql!("SELECT JSON_OBJECT('key', {{literal}}) FROM {users}");
/// // Generates: SQL::text("SELECT JSON_OBJECT('key', {literal}) FROM table")
/// ```
///
/// # Requirements
///
/// All expressions within `{braces}` must implement `ToSQL<'a, V>` trait.
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
/// drivers_test!(test_name, SchemaType, {
///     // Test body - uses `db` and `schema` variables
///     let table = schema.my_table;
///     let result = drizzle_exec!(db.insert(table).values([data]).execute());
///     assert_eq!(result, 1);
/// });
/// ```
///
/// # Generated Functions
///
/// For a test named `my_test`, this generates:
/// - `my_test_rusqlite()` - Sync test for rusqlite
/// - `my_test_libsql()` - Async test for libsql  
/// - `my_test_turso()` - Async test for turso
///
/// # Available Macros in Test Body
///
/// - `drizzle_exec!(operation)` - Execute operation with proper async/sync handling
/// - `drizzle_try!(operation)` - Try operation with proper async/sync handling
/// - `drizzle_tx!(tx_type, { body })` - Execute transaction with proper async/sync handling
///
/// # Variables Available in Test Body
///
/// - `db` - The Drizzle instance for the current driver
/// - `schema` - The schema instance with all tables
/// - `tx` - The transaction instance (within drizzle_tx! blocks)
#[proc_macro]
pub fn drizzle_test(input: TokenStream) -> TokenStream {
    crate::drivers_test::drivers_test_impl(input)
}
