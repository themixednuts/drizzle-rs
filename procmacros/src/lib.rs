extern crate proc_macro;

mod drizzle;
mod qb;
mod schema;

#[cfg(feature = "sqlite")]
mod sqlite;

use drizzle::DrizzleInput;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse_macro_input;

/// Process the drizzle! proc macro
/// This macro creates a Drizzle instance from a connection and schema
#[proc_macro]
pub fn drizzle(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    let input = parse_macro_input!(input as DrizzleInput);

    match drizzle::drizzle_impl(input) {
        Ok(output) => output.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Creates a schema-bound query builder factory.
///
/// Takes a list of table types (structs marked with `#[SQLiteTable]`)
/// and returns an instance of `SQLiteQueryBuilder` bound to a unique
/// schema marker type. This ensures that only tables included in the
/// macro invocation can be used with the resulting query builder.
///
/// # Example
///
/// ```rust
/// use drizzle_rs::prelude::*;
/// use drizzle_rs::qb; // Import the qb macro
/// use procmacros::SQLiteTable;
///
/// #[SQLiteTable(name = "users")]
/// struct Users { /* ... fields ... */ }
/// #[SQLiteTable(name = "posts")]
/// struct Posts { /* ... fields ... */ }
///
/// let qb = qb!([Users, Posts]);
///
/// // This works:
/// let user_query = qb.from::<Users>().select_all();
///
/// // This will fail to compile (if Category is not in qb!):
/// // let category_query = qb.from::<Category>().select_all();
/// ```
#[proc_macro]
pub fn qb(input: proc_macro::TokenStream) -> proc_macro::TokenStream {
    match qb::qb_impl(input) {
        Ok(qb) => qb.into(),
        Err(err) => err.into_compile_error().into(),
    }
}

/// Implements the SQLiteEnum trait for an enum, enabling conversion to and from SQLiteValue
///
/// This macro also implements standard Rust traits:
/// - `std::fmt::Display` - for converting to string
/// - `std::str::FromStr` - for parsing from string
///
/// # Requirements
///
/// The enum must implement the `Default` trait, which specifies a fallback value when
/// an invalid variant is encountered. This ensures there's always a valid value to return.
///
/// # Storage Behavior
///
/// - Enums with `#[repr(i8)]`, `#[repr(u8)]`, `#[repr(i32)]`, etc. will be stored as **INTEGER** values in SQLite
/// - Enums without a numeric repr attribute will be stored as **TEXT** (using variant names)
///
/// # SQLite Flexible Typing
///
/// SQLite uses flexible typing which means it won't complain about storing TEXT in INTEGER columns
/// (or vice versa) unless the table is marked STRICT. This can lead to unexpected behavior when
/// using enums with tables.
///
/// # Example
///
/// ```rust
/// #[derive(SQLiteEnum, Debug, Clone, PartialEq)]
/// enum Role {
///     User,
///     Admin,
///     Moderator,
/// }
///
/// // Must implement Default
/// impl Default for Role {
///     fn default() -> Self {
///         Self::User
///     }
/// }
/// // Stored as TEXT values: "User", "Admin", "Moderator"
///
/// #[derive(SQLiteEnum, Debug, Clone, PartialEq)]
/// #[repr(i32)]
/// enum Status {
///     Active = 1,
///     Inactive = 0,
///     Banned = -1,
/// }
///
/// // Must implement Default
/// impl Default for Status {
///     fn default() -> Self {
///         Self::Inactive
///     }
/// }
/// // Stored as INTEGER values: 1, 0, -1
/// ```
#[cfg(feature = "sqlite")]
#[proc_macro_derive(SQLiteEnum)]
pub fn sqlite_enum_derive(input: TokenStream) -> TokenStream {
    use quote::quote;
    use syn::{Data, DeriveInput, parse_macro_input};

    let input = parse_macro_input!(input as DeriveInput);
    let name = &input.ident;

    // Check if this is an enum
    let data = match &input.data {
        Data::Enum(data) => data,
        _ => {
            return quote! {
                compile_error!("SQLiteEnum can only be derived for enums");
            }
            .into();
        }
    };

    // Check if the enum has any variants
    if data.variants.is_empty() {
        return quote! {
            compile_error!("SQLiteEnum cannot be derived for empty enums");
        }
        .into();
    }

    // Check if this enum has a repr attribute
    // If it does, use INTEGER representation, otherwise use TEXT
    let has_repr = input.attrs.iter().any(|attr| attr.path().is_ident("repr"));

    // Generate implementation based on representation
    let impl_block = if has_repr {
        match crate::sqlite::r#enum::generate_integer_enum_impl(name, data) {
            Ok(ts) => ts,
            Err(e) => return e.to_compile_error().into(),
        }
    } else {
        match crate::sqlite::r#enum::generate_text_enum_impl(name, data) {
            Ok(ts) => ts,
            Err(e) => return e.to_compile_error().into(),
        }
    };

    impl_block.into()
}

/// This attribute macro helps define SQLite tables in Rust.
///
/// It can be applied to structs to generate the necessary code for working with SQLite tables.
///
/// # Arguments
///
/// * `name` - The table name to use in the database (optional, defaults to the struct name)
/// * `strict` - Enables SQLite STRICT mode for this table
/// * `without_rowid` - Creates the table without a rowid column
///
/// # Example
///
/// ```
/// #[SQLiteTable(name = "users", strict)]
/// struct User { ... }
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
