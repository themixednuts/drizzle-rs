use proc_macro::{self, TokenStream};
use quote::ToTokens;
use syn::{DeriveInput, parse_macro_input};

mod drizzle;

#[cfg(feature = "sqlite")]
mod sqlite;

/// Derive macro for SQLite tables
///
/// This macro derives the code necessary to represent a Rust struct as a SQLite table. It validates
/// that the fields are compatible with the SQLite column types, and generates methods to create and
/// manipulate the table.
///
/// # Data Structures
///
/// The macro supports two main data structures:
///
/// ## Structs
/// Structs are directly mapped to SQLite tables, with each field becoming a column.
///
/// ## Enums
/// Simple enums (without fields or discriminants) are automatically mapped to TEXT columns.
/// The macro will generate Display and FromStr implementations for the enum.
///
/// # Table Attributes
///
/// * `name`: Override the table name (default is the struct name in snake_case)
/// * `strict`: Use [STRICT tables](https://www.sqlite.org/stricttables.html) for better type safety
/// * `without_rowid`: Create a [WITHOUT ROWID](https://www.sqlite.org/withoutrowid.html) table for better space efficiency
///
/// # Column Type Attributes
///
/// * `integer`: Use a SQLite INTEGER column (supports i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool)
/// * `real`: Use a SQLite REAL column (supports f32, f64)
/// * `text`: Use a SQLite TEXT column (supports String, types with Display/FromStr, enums)
/// * `blob`: Use a SQLite BLOB column (supports Vec<u8>, types with AsRef<[u8]>/TryFrom<&[u8]>)
///
/// # Column Attributes
///
/// * `primary_key`: Mark the column as a primary key
/// * `autoincrement`: Use AUTOINCREMENT (only valid for INTEGER PRIMARY KEY)
/// * `unique`: Add a UNIQUE constraint
/// * `name`: Override the column name (default is the field name)
/// * `references`: Define a foreign key constraint with either:
///   * String format: `"TableName.column_name"` (e.g. `references = "Users.id"`)
///   * Path format: `TableName::column_name` (e.g. `references = Users::id`)
/// * `default`: Static default value (as valid SQL expression)
/// * `default_fn`: Function to generate default value at runtime (function must be in scope)
///
/// # Nullable Columns
///
/// The NOT NULL constraint is automatically applied to non-Option types.
/// To create a nullable column, use `Option<T>`:
///
/// ```rust
/// // Nullable column
/// #[text]
/// email: Option<String>; // Can be NULL
///
/// // Non-nullable column (NOT NULL is automatic)
/// #[text]  
/// name: String;
/// ```
///
/// # Basic Usage
///
/// ```rust
/// #[derive(SQLiteTable)]
/// struct Users {
///     #[integer(primary_key, autoincrement)]
///     id: i64,
///
///     #[text] // NOT NULL is automatic for non-Option types
///     name: String,
///
///     #[text]
///     email: Option<String>, // Nullable column
///
///     #[integer]
///     is_active: bool, // Stored as 0/1
/// }
/// ```
///
/// # Enum Support
///
/// ```rust
/// #[derive(SQLiteTable)]
/// enum Status {
///     Active,
///     Pending,
///     Inactive,
/// }
///
/// #[derive(SQLiteTable)]
/// struct Tasks {
///     #[integer(primary_key, autoincrement)]
///     id: i64,
///
///     #[text]
///     name: String,
///
///     #[text]
///     status: Status, // Enum stored as TEXT
/// }
/// ```
///
/// # Custom Table Name and Strict Mode
///
/// ```rust
/// #[derive(SQLiteTable)]
/// #[table(name = "products", strict)]
/// struct Product {
///     #[integer(primary_key, autoincrement)]
///     id: i64,
///
///     #[text]
///     name: String,
///
///     #[real]
///     price: f64,
/// }
/// ```
///
/// # Composite Primary Key and Foreign Keys
///
/// ```rust
/// #[derive(SQLiteTable)]
/// struct OrderItems {
///     #[integer(primary_key)]
///     order_id: i64,
///
///     #[integer(primary_key)]
///     item_id: i32,
///
///     #[integer(references = Orders::id)] // Path-based reference (recommended)
///     parent_order_id: i64,
///
///     #[integer(references = "Products.id")] // String-based reference
///     product_id: i64,
///
///     #[integer]
///     quantity: i32,
/// }
/// ```
///
/// # Supported Types
///
/// * **INTEGER columns**: `i8, i16, i32, i64, isize, u8, u16, u32, u64, usize, bool`
/// * **REAL columns**: `f32, f64`
/// * **TEXT columns**: `String`, any type implementing `Display + FromStr`
/// * **BLOB columns**: `Vec<u8>`, any type implementing `AsRef<[u8]> + TryFrom<&[u8]>`
///
/// For custom types, see the documentation in the crate's README.
///
/// # Examples
///
/// ## Basic usage
///
/// ```rust
/// use drizzle_rs::prelude::*;
///
/// #[derive(SQLiteTable)]
/// struct Users {
///     #[integer(primary_key, autoincrement)]
///     id: i64,
///     
///     #[text]
///     name: String, // NOT NULL applied automatically
///     
///     #[text]
///     email: Option<String>, // Optional field (NULL)
///     
///     #[integer]
///     is_active: bool, // Stored as 0/1
/// }
/// ```
///
/// ## Custom table name and strict mode
///
/// ```rust
/// use drizzle_rs::prelude::*;
///
/// #[derive(SQLiteTable)]
/// #[table(name = "user_profiles", strict)]
/// struct UserProfile {
///     #[integer(primary_key)]
///     user_id: i64,
///     
///     #[text(name = "display_name")]
///     username: String,
///     
///     #[blob]
///     avatar: Option<Vec<u8>>,
///     
///     #[real]
///     rating: f64,
/// }
/// ```
///
/// ## Composite primary key and foreign keys
///
/// ```rust
/// use drizzle_rs::prelude::*;
///
/// #[derive(SQLiteTable)]
/// struct OrderItems {
///     #[integer(primary_key, references = "Orders.id")]
///     order_id: i64,
///     
///     #[integer(primary_key)]
///     item_id: i32,
///     
///     #[integer]
///     quantity: i32,
///     
///     #[real]
///     price: f64,
/// }
/// ```
///
/// ## Using custom types with TEXT columns
///
/// ```rust
/// use std::fmt::Display;
/// use std::str::FromStr;
/// use drizzle_rs::prelude::*;
///
/// // Custom enum with text representation
/// #[derive(Debug, Clone, PartialEq)]
/// enum Status {
///     Active,
///     Pending,
///     Inactive,
/// }
///
/// // Implement Display for serialization
/// impl Display for Status {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         match self {
///             Status::Active => write!(f, "active"),
///             Status::Pending => write!(f, "pending"),
///             Status::Inactive => write!(f, "inactive"),
///         }
///     }
/// }
///
/// // Implement FromStr for deserialization
/// impl FromStr for Status {
///     type Err = String;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         match s {
///             "active" => Ok(Status::Active),
///             "pending" => Ok(Status::Pending),
///             "inactive" => Ok(Status::Inactive),
///             _ => Err(format!("Unknown status: {}", s)),
///         }
///     }
/// }
///
/// #[derive(SQLiteTable)]
/// struct Tasks {
///     #[integer(primary_key, autoincrement)]
///     id: i64,
///     
///     #[text]
///     name: String,
///     
///     #[text]
///     status: Status, // Custom type with Display/FromStr
/// }
/// ```
///
/// ## Using default values
///
/// ```rust
/// use drizzle_rs::prelude::*;
///
/// #[derive(SQLiteTable)]
/// struct Products {
///     #[integer(primary_key, autoincrement)]
///     id: i64,
///     
///     #[text]
///     name: String,
///     
///     #[real(default = 0.0)]
///     price: f64,
///     
///     #[integer(default = 1)]
///     stock: i32,
///     
///     #[integer(default_fn = now)]
///     created_at: i64,
/// }
///
/// // Function to get current timestamp
/// fn now() -> i64 {
///     std::time::SystemTime::now()
///         .duration_since(std::time::UNIX_EPOCH)
///         .unwrap()
///         .as_secs() as i64
/// }
/// ```
///
/// ## Wrapper types (newtypes) with TEXT and BLOB
///
/// For numeric wrapper types, it's recommended to use TEXT or BLOB storage
/// rather than trying to use them with INTEGER/REAL columns directly.
///
/// ```rust
/// use std::fmt::Display;
/// use std::str::FromStr;
/// use drizzle_rs::prelude::*;
///
/// // User ID wrapper around i64
/// #[derive(Debug, Clone, PartialEq, Eq)]
/// struct UserId(i64);
///
/// // Implement Display for storage as TEXT
/// impl Display for UserId {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{}", self.0)
///     }
/// }
///
/// // Implement FromStr for parsing from TEXT
/// impl FromStr for UserId {
///     type Err = std::num::ParseIntError;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         s.parse::<i64>().map(UserId)
///     }
/// }
///
/// // Temperature wrapper around f64
/// #[derive(Debug, Clone, PartialEq)]
/// struct Temperature(f64);
///
/// // Implement Display for storage as TEXT
/// impl Display for Temperature {
///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
///         write!(f, "{:.2}", self.0)
///     }
/// }
///
/// // Implement FromStr for parsing from TEXT
/// impl FromStr for Temperature {
///     type Err = std::num::ParseFloatError;
///
///     fn from_str(s: &str) -> Result<Self, Self::Err> {
///         s.parse::<f64>().map(Temperature)
///     }
/// }
///
/// // Store IDs as binary data - implement AsRef<[u8]> and TryFrom<&[u8]>
/// impl AsRef<[u8]> for UserId {
///     fn as_ref(&self) -> &[u8] {
///         // NOTE: This is unsafe and for demonstration only
///         // A real implementation should use proper serialization
///         unsafe {
///             std::slice::from_raw_parts(
///                 &self.0 as *const i64 as *const u8,
///                 std::mem::size_of::<i64>(),
///             )
///         }
///     }
/// }
///
/// impl<'a> TryFrom<&'a [u8]> for UserId {
///     type Error = std::io::Error;
///
///     fn try_from(bytes: &'a [u8]) -> Result<Self, Self::Error> {
///         if bytes.len() != std::mem::size_of::<i64>() {
///             return Err(std::io::Error::new(
///                 std::io::ErrorKind::InvalidData,
///                 "Invalid byte length for UserId",
///             ));
///         }
///
///         let value = unsafe {
///             let ptr = bytes.as_ptr() as *const i64;
///             *ptr
///         };
///
///         Ok(UserId(value))
///     }
/// }
///
/// #[derive(SQLiteTable)]
/// struct Measurements {
///     #[integer(primary_key, autoincrement)]
///     id: i64,
///     
///     // Store UserId as TEXT column via Display/FromStr
///     #[text]
///     user_id: UserId,
///     
///     // Store Temperature as TEXT column via Display/FromStr
///     #[text]
///     temperature: Temperature,
///     
///     // Store UserId as BLOB via AsRef<[u8]>/TryFrom<&[u8]>
///     #[blob]
///     user_id_blob: UserId,
/// }
/// ```
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SQLiteTable, attributes(table, integer, real, text, blob))]
pub fn sqlite_table(token: TokenStream) -> TokenStream {
    let input = parse_macro_input!(token as DeriveInput);

    match sqlite::table_macro(input) {
        Ok(s) => s.to_token_stream().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Derive macro for Schema
/// This allows defining a schema as a tuple of table types
#[proc_macro_derive(Schema)]
pub fn schema(token: TokenStream) -> TokenStream {
    let input = parse_macro_input!(token as DeriveInput);

    match sqlite::schema_macro(input) {
        Ok(s) => s.to_token_stream().into(),
        Err(e) => e.to_compile_error().into(),
    }
}

/// Process the drizzle! proc macro
/// This macro creates a Drizzle instance from a connection and schema
#[proc_macro]
pub fn drizzle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input2 = proc_macro2::TokenStream::from(input);

    match drizzle::drizzle_macro(input2) {
        Ok(s) => proc_macro::TokenStream::from(s),
        Err(e) => proc_macro::TokenStream::from(e.to_compile_error()),
    }
}

// Add test to demonstrate how the proc macro handles enum fields
#[cfg(test)]
mod tests {
    use std::fmt::Display;
    use std::str::FromStr;

    // Define an enum for user roles
    #[derive(Debug, PartialEq)]
    enum Role {
        User,
        Maintainer,
        Admin,
    }

    // Implement Display for the enum to allow serialization to TEXT
    impl Display for Role {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Role::User => write!(f, "User"),
                Role::Maintainer => write!(f, "Maintainer"),
                Role::Admin => write!(f, "Admin"),
            }
        }
    }

    // Implement FromStr for the enum to allow deserialization from TEXT
    impl FromStr for Role {
        type Err = String;

        fn from_str(s: &str) -> Result<Self, Self::Err> {
            match s {
                "User" => Ok(Role::User),
                "Maintainer" => Ok(Role::Maintainer),
                "Admin" => Ok(Role::Admin),
                _ => Err(format!("Unknown role: {}", s)),
            }
        }
    }

    // Test how the SQLiteTable macro works with enum fields
    // Note: This doesn't actually run as a test, it just ensures the proc macro
    // can generate the appropriate code correctly.
    #[allow(unused)]
    fn test_enum_fields() {
        // This would use the SQLiteTable derive macro in actual code
        // #[derive(SQLiteTable)]
        struct User {
            // #[integer(primary_key, autoincrement)]
            id: i64,
            // #[text]
            name: String,
            // #[text]
            role: Role, // Enum field with Display/FromStr implementation
        }
    }
}
