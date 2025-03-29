#![recursion_limit = "128"]

extern crate proc_macro;

mod drizzle;
mod schema;
#[cfg(feature = "sqlite")]
mod sqlite;

use proc_macro::TokenStream;
use quote::quote;
use syn::{Data, DataEnum, DeriveInput, Fields, parse_macro_input};

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

/// Derives the FromRow trait for structs, enabling automatic
/// conversion from database rows to Rust types.
///
/// # Example
///
/// ```
/// #[derive(FromRow)]
/// struct User {
///     id: i64,
///     name: String,
///     email: Option<String>,
/// }
/// ```
#[proc_macro_derive(FromRow)]
pub fn derive_from_row(input: TokenStream) -> TokenStream {
    // Parse the input tokens into a syntax tree
    let input = parse_macro_input!(input as DeriveInput);

    // Extract the struct name
    let struct_name = &input.ident;

    // Only works on structs with named fields
    let fields = match input.data {
        Data::Struct(ref data) => match data.fields {
            Fields::Named(ref fields) => &fields.named,
            _ => panic!("FromRow derive only works on structs with named fields"),
        },
        _ => panic!("FromRow derive only works on structs"),
    };

    // Generate field assignments
    let field_assignments = fields.iter().enumerate().map(|(i, field)| {
        let field_name = &field.ident;
        quote! {
            #field_name: row.get(#i)?,
        }
    });

    // Generate the FromRow implementation
    let expanded = quote! {
        impl drizzle_rs::connection::FromRow for #struct_name {
            fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
                Ok(Self {
                    #(#field_assignments)*
                })
            }
        }
    };

    // Return the generated code
    TokenStream::from(expanded)
}

/// Attribute macro for declaring SQLite tables
///
/// Example:
/// ```
/// #[SQLiteTable(name = "users", strict)]
/// struct User { ... }
/// ```
#[cfg(feature = "sqlite")]
#[allow(non_snake_case)]
#[proc_macro_attribute]
pub fn SQLiteTable(attr: TokenStream, item: TokenStream) -> TokenStream {
    let attr = proc_macro2::TokenStream::from(attr);
    let input = syn::parse_macro_input!(item as syn::DeriveInput);

    match crate::sqlite::table::table_attr_macro(input, attr) {
        Ok(tokens) => tokens.into(),
        Err(err) => err.to_compile_error().into(),
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
/// use drizzle_rs::schema; // Assuming schema macro is exported
/// use procmacros::SQLiteTable;
///
/// #[SQLiteTable(name = "users")]
/// struct Users { /* ... fields ... */ }
/// #[SQLiteTable(name = "posts")]
/// struct Posts { /* ... fields ... */ }
///
/// let qb = schema!([Users, Posts]);
///
/// // This works:
/// let user_query = qb.from::<Users>().select_all();
///
/// // This will fail to compile (if Category is not in schema!):
/// // let category_query = qb.from::<Category>().select_all();
/// ```
#[proc_macro]
pub fn schema(input: TokenStream) -> TokenStream {
    let input2 = proc_macro2::TokenStream::from(input);
    match schema::schema_macro_impl(input2) {
        Ok(ts) => ts.into(),
        Err(e) => e.to_compile_error().into(),
    }
}
