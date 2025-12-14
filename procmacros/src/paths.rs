//! Centralized path definitions for generated code.
//!
//! This module provides fully-qualified paths for all types and traits used in macro-generated code.
//! Using `drizzle::` prefix (without leading `::`) allows doc tests in subcrates to create a
//! `mod drizzle { ... }` shim that re-exports from the current crate, avoiding circular dependencies.

use proc_macro2::TokenStream;
use quote::quote;

// =============================================================================
// STANDARD LIBRARY
// =============================================================================

pub mod std {
    use super::*;

    pub fn option() -> TokenStream {
        quote!(::std::option::Option)
    }

    pub fn result() -> TokenStream {
        quote!(::std::result::Result)
    }

    pub fn vec() -> TokenStream {
        quote!(::std::vec::Vec)
    }

    pub fn string() -> TokenStream {
        quote!(::std::string::String)
    }

    pub fn cow() -> TokenStream {
        quote!(::std::borrow::Cow)
    }

    pub fn phantom_data() -> TokenStream {
        quote!(::std::marker::PhantomData)
    }

    pub fn into() -> TokenStream {
        quote!(::std::convert::Into)
    }

    pub fn try_from() -> TokenStream {
        quote!(::std::convert::TryFrom)
    }
}

// =============================================================================
// CORE TYPES AND TRAITS
// =============================================================================

/// Core traits from drizzle::core
pub mod core {
    use super::*;

    pub fn sql() -> TokenStream {
        quote!(drizzle::core::SQL)
    }

    pub fn sql_column_info() -> TokenStream {
        quote!(drizzle::core::SQLColumnInfo)
    }

    pub fn sql_table_info() -> TokenStream {
        quote!(drizzle::core::SQLTableInfo)
    }

    pub fn sql_column() -> TokenStream {
        quote!(drizzle::core::SQLColumn)
    }

    pub fn sql_table() -> TokenStream {
        quote!(drizzle::core::SQLTable)
    }

    pub fn sql_schema() -> TokenStream {
        quote!(drizzle::core::SQLSchema)
    }

    pub fn sql_model() -> TokenStream {
        quote!(drizzle::core::SQLModel)
    }

    pub fn sql_partial() -> TokenStream {
        quote!(drizzle::core::SQLPartial)
    }

    pub fn sql_index() -> TokenStream {
        quote!(drizzle::core::SQLIndex)
    }

    pub fn sql_index_info() -> TokenStream {
        quote!(drizzle::core::SQLIndexInfo)
    }

    pub fn to_sql() -> TokenStream {
        quote!(drizzle::core::ToSQL)
    }

    pub fn sql_comparable() -> TokenStream {
        quote!(drizzle::core::SQLComparable)
    }

    pub fn order_by() -> TokenStream {
        quote!(drizzle::core::OrderBy)
    }

    pub fn sql_param() -> TokenStream {
        quote!(drizzle::core::SQLParam)
    }

    pub fn param() -> TokenStream {
        quote!(drizzle::core::Param)
    }

    pub fn param_bind() -> TokenStream {
        quote!(drizzle::core::ParamBind)
    }

    pub fn token() -> TokenStream {
        quote!(drizzle::core::Token)
    }

    pub fn drizzle_error() -> TokenStream {
        quote!(drizzle::error::DrizzleError)
    }

    pub fn sql_schema_impl() -> TokenStream {
        quote!(drizzle::core::SQLSchemaImpl)
    }

    pub fn sql_enum_info() -> TokenStream {
        quote!(drizzle::core::SQLEnumInfo)
    }
}

// =============================================================================
// SQLITE TYPES AND TRAITS
// =============================================================================

#[cfg(feature = "sqlite")]
pub mod sqlite {
    use super::*;

    pub fn sqlite_value() -> TokenStream {
        quote!(drizzle::sqlite::values::SQLiteValue)
    }

    pub fn sqlite_insert_value() -> TokenStream {
        quote!(drizzle::sqlite::values::SQLiteInsertValue)
    }

    pub fn value_wrapper() -> TokenStream {
        quote!(drizzle::sqlite::values::ValueWrapper)
    }

    pub fn sqlite_schema_type() -> TokenStream {
        quote!(drizzle::sqlite::common::SQLiteSchemaType)
    }

    pub fn sqlite_table() -> TokenStream {
        quote!(drizzle::sqlite::traits::SQLiteTable)
    }

    pub fn sqlite_table_info() -> TokenStream {
        quote!(drizzle::sqlite::traits::SQLiteTableInfo)
    }

    pub fn sqlite_column() -> TokenStream {
        quote!(drizzle::sqlite::traits::SQLiteColumn)
    }

    pub fn sqlite_column_info() -> TokenStream {
        quote!(drizzle::sqlite::traits::SQLiteColumnInfo)
    }

    pub fn from_sqlite_value() -> TokenStream {
        quote!(drizzle::sqlite::traits::FromSQLiteValue)
    }

    pub fn drizzle_row() -> TokenStream {
        quote!(drizzle::sqlite::traits::DrizzleRow)
    }

    pub fn expression() -> TokenStream {
        quote!(drizzle::sqlite::expression)
    }

    pub fn column_marker() -> TokenStream {
        quote!(drizzle::sqlite::attrs::ColumnMarker)
    }
}

// =============================================================================
// POSTGRES TYPES AND TRAITS
// =============================================================================

#[cfg(feature = "postgres")]
pub mod postgres {
    use super::*;

    pub fn postgres_value() -> TokenStream {
        quote!(drizzle::postgres::values::PostgresValue)
    }

    pub fn postgres_insert_value() -> TokenStream {
        quote!(drizzle::postgres::values::PostgresInsertValue)
    }

    pub fn value_wrapper() -> TokenStream {
        quote!(drizzle::postgres::values::ValueWrapper)
    }

    pub fn postgres_schema_type() -> TokenStream {
        quote!(drizzle::postgres::common::PostgresSchemaType)
    }

    pub fn postgres_table() -> TokenStream {
        quote!(drizzle::postgres::traits::PostgresTable)
    }

    pub fn postgres_table_info() -> TokenStream {
        quote!(drizzle::postgres::traits::PostgresTableInfo)
    }

    pub fn postgres_column() -> TokenStream {
        quote!(drizzle::postgres::traits::PostgresColumn)
    }

    pub fn postgres_column_info() -> TokenStream {
        quote!(drizzle::postgres::traits::PostgresColumnInfo)
    }

    pub fn from_postgres_value() -> TokenStream {
        quote!(drizzle::postgres::traits::FromPostgresValue)
    }

    pub fn drizzle_row() -> TokenStream {
        quote!(drizzle::postgres::DrizzleRow)
    }

    pub fn column_marker() -> TokenStream {
        quote!(drizzle::postgres::attrs::ColumnMarker)
    }
}
