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
//! ```rust
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

mod drizzle;
mod fromrow;
mod qb;
mod schema;
mod sql;
mod utils;

#[cfg(feature = "sqlite")]
mod sqlite;

#[cfg(feature = "rusqlite")]
mod rusqlite;

use drizzle::DrizzleInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Initialize a Drizzle instance with database connection and table schemas.
///
/// This macro creates a type-safe Drizzle instance that provides query building
/// capabilities for the specified table schemas.
///
/// # Syntax
///
/// ```rust
/// use drizzle_rs::prelude::*;
///
/// #[SQLiteTable]
/// struct Table1 {
///     #[integer(primary)]
///     id: i32
/// }
///
/// #[SQLiteTable]
/// struct Table2 {
///     #[integer(primary)]
///     id: i32
/// }
///
/// #[SQLiteTable]
/// struct Table {
///     #[integer(primary)]
///     id: i32
/// }
///
/// # fn main() -> Result<(), drizzle_rs::error::DrizzleError> {
/// let connection1 = rusqlite::Connection::open_in_memory()?;
/// let connection2 = rusqlite::Connection::open_in_memory()?;
/// let connection3 = rusqlite::Connection::open_in_memory()?;
///
/// // Multiple tables (returns tuple)
/// let (drizzle_instance, table_handles) = drizzle!(connection1, [Table1, Table2]);
/// // Single table with array syntax (returns single table)
/// let (drizzle_instance, single_table) = drizzle!(connection2, [Table]);
/// // Single table without array syntax (returns single table)
/// let (drizzle_instance, single_table) = drizzle!(connection3, Table);
/// # Ok(())
/// # }
/// ```
///
/// # Examples
///
/// ## Single Table
/// ```rust
/// use drizzle_rs::prelude::*;
///
/// #[SQLiteTable(name = "users")]
/// struct Users {
///     #[integer(primary)]
///     id: i32,
///     #[text]
///     name: String,
/// }
///
/// # fn main() -> Result<(), drizzle_rs::error::DrizzleError> {
/// let connection1 = rusqlite::Connection::open_in_memory()?;
/// let connection2 = rusqlite::Connection::open_in_memory()?;
///
/// // Both syntaxes are equivalent for single tables:
/// let (db, users) = drizzle!(connection1, [Users]);
/// let (db, users) = drizzle!(connection2, Users);
/// # Ok(())
/// # }
/// ```
///
/// ## Multiple Tables
/// ```rust
/// use drizzle_rs::prelude::*;
/// use drizzle_rs::error::DrizzleError;
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
///     #[text]
///     title: String,
///     #[integer(references = Users::id)]
///     user_id: i32,
/// }
///
/// # fn main() -> Result<(), DrizzleError> {
/// let connection = rusqlite::Connection::open_in_memory()?;
/// let (db, (users, posts)) = drizzle!(connection, [Users, Posts]);
/// # Ok(())
/// # }
/// ```
#[proc_macro]
pub fn drizzle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DrizzleInput);

    match drizzle::drizzle_impl(input) {
        Ok(output) => output.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

#[proc_macro]
pub fn qb(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match qb::qb_impl(input) {
        Ok(qb) => qb.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

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
/// ```rust
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
/// ```rust
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
                Err(e) => return e.to_compile_error().into(),
            }
        }
        _ => {
            return quote! {
                compile_error!("SQLiteEnum can only be derived for enums and tuple structs");
            }
            .into();
        }
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
/// ```rust
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
/// ```rust
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
/// ```rust
/// use drizzle_rs::prelude::*;
/// use serde::{Serialize, Deserialize};
///
/// #[derive(SQLiteEnum, Default, Clone, PartialEq, Debug)]
/// enum Role {
///     #[default]
///     User,
///     Admin
/// }
///
/// #[derive(Serialize, Deserialize, Clone, PartialEq, Debug)]
/// struct Metadata { theme: String }
///
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
/// ```rust
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

/// A procedural macro for building SQL queries with embedded expressions.
///
/// This macro supports three different syntax forms:
/// 1. **String literal syntax**: `sql!("SELECT * FROM {table}")`
/// 2. **Token stream syntax**: `sql!(SELECT * FROM {table})` (preserves LSP hover support)
/// 3. **Printf-style syntax**: `sql!("SELECT * FROM {} WHERE {} = {}", table, column, value)`
///
/// The macro parses SQL templates and generates type-safe SQL code by:
/// - Converting literal text to `SQL::text()` calls
/// - Converting expressions in `{braces}` to `.to_sql()` calls on the expression
///
/// # Syntax Forms
///
/// ## String Literal Syntax
/// ```rust
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] struct Users { #[integer(primary)] id: i32 }
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, users) = drizzle!(conn, Users);
/// let query = sql!("SELECT * FROM {users} WHERE {users.id} = 42");
/// ```
///
/// ## Token Stream Syntax (Preserves LSP Hover)
/// ```rust
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] struct Users { #[integer(primary)] id: i32 }
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, users) = drizzle!(conn, Users);
/// let query = sql!(SELECT * FROM {users} WHERE {users.id} = 42);
/// ```
///
/// ## Printf-Style Syntax
/// ```rust
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] struct Users { #[integer(primary)] id: i32 }
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, users) = drizzle!(conn, Users);
/// let query = sql!("SELECT * FROM {} WHERE {} = {}", users, users.id, 42);
/// ```
///
/// # Examples
///
/// ## Basic Usage
/// ```rust
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] struct Users { #[integer(primary)] id: i32 }
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, users) = drizzle!(conn, Users);
/// let query = sql!("SELECT * FROM {users}");
/// // Generates: SQL::text("SELECT * FROM ").append(users.to_sql())
/// ```
///
/// ## Multiple Expressions
/// ```rust
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] struct Users { #[integer(primary)] id: i32 }
/// # #[SQLiteTable] struct Posts { #[integer(primary)] id: i32, #[integer] author: i32 }
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, (users, posts)) = drizzle!(conn, [Users, Posts]);
/// let query = sql!("SELECT * FROM {users} WHERE {users.id} = {posts.author}");
/// ```
///
/// ## Escaped Braces
/// Use `{{` and `}}` for literal braces in the SQL:
/// ```rust
/// # use drizzle_rs::{sql, prelude::*};
/// # #[SQLiteTable] struct Users { #[integer(primary)] id: i32 }
/// # #[SQLiteTable] struct Posts { #[integer(primary)] id: i32, #[integer] author: i32 }
/// # let conn = rusqlite::Connection::open_in_memory().unwrap();
/// # let (db, (users, posts)) = drizzle!(conn, [Users, Posts]);
///   let query = sql!("SELECT JSON_OBJECT('key', {{users.id}}) FROM {users}");
/// // Generates: SQL::text("SELECT JSON_OBJECT('key', {value}) FROM table")
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
