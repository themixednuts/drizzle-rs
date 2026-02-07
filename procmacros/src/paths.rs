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

    pub fn sql_view() -> TokenStream {
        quote!(drizzle::core::SQLView)
    }

    pub fn sql_view_info() -> TokenStream {
        quote!(drizzle::core::SQLViewInfo)
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

    pub fn impl_try_from_int() -> TokenStream {
        quote!(drizzle::core::impl_try_from_int)
    }

    /// Path to the types module (DataType markers)
    pub fn types() -> TokenStream {
        quote!(drizzle::core::types)
    }

    /// Path to the expr module (Expr trait and markers)
    pub fn expr() -> TokenStream {
        quote!(drizzle::core::expr)
    }

    /// Path to the ToSQL trait
    pub fn to_sql_trait() -> TokenStream {
        quote!(drizzle::core::ToSQL)
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

    pub fn sqlite_update_value() -> TokenStream {
        quote!(drizzle::sqlite::values::SQLiteUpdateValue)
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

    pub fn expressions() -> TokenStream {
        quote!(drizzle::sqlite::expressions)
    }

    pub fn column_marker() -> TokenStream {
        quote!(drizzle::sqlite::attrs::ColumnMarker)
    }
}

// =============================================================================
// DDL TYPES (from drizzle_types, exposed as drizzle::ddl)
// =============================================================================

/// DDL type paths - these point to drizzle::ddl (re-exported from drizzle_types)
pub mod ddl {
    pub mod sqlite {
        use proc_macro2::TokenStream;
        use quote::quote;

        pub fn table_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::TableDef)
        }

        pub fn column_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ColumnDef)
        }

        pub fn primary_key_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::PrimaryKeyDef)
        }

        pub fn foreign_key_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ForeignKeyDef)
        }

        pub fn unique_constraint_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::UniqueConstraintDef)
        }

        pub fn view() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::View)
        }

        pub fn view_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ViewDef)
        }

        pub fn index_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::IndexDef)
        }

        pub fn index_column_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::IndexColumnDef)
        }

        pub fn referential_action() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ReferentialAction)
        }

        pub fn table_sql() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::sql::TableSql)
        }
    }

    pub mod postgres {
        use proc_macro2::TokenStream;
        use quote::quote;

        pub fn table_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::TableDef)
        }

        pub fn column_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ColumnDef)
        }

        pub fn primary_key_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::PrimaryKeyDef)
        }

        pub fn foreign_key_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ForeignKeyDef)
        }

        pub fn unique_constraint_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::UniqueConstraintDef)
        }

        pub fn view() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::View)
        }

        pub fn view_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ViewDef)
        }

        pub fn index_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::IndexDef)
        }

        pub fn index_column_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::IndexColumnDef)
        }

        pub fn identity_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::IdentityDef)
        }

        pub fn referential_action() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ReferentialAction)
        }

        pub fn table_sql() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::sql::TableSql)
        }

        pub fn view_with_option_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ViewWithOptionDef)
        }
    }
}

// =============================================================================
// MIGRATIONS TYPES AND TRAITS
// =============================================================================

pub mod migrations {
    use super::*;

    pub fn schema() -> TokenStream {
        quote!(drizzle::migrations::Schema)
    }

    pub fn dialect() -> TokenStream {
        quote!(drizzle::Dialect)
    }

    pub fn snapshot() -> TokenStream {
        quote!(drizzle::migrations::Snapshot)
    }

    // SQLite DDL types (from drizzle::ddl)
    pub mod sqlite {
        use super::*;

        pub fn snapshot() -> TokenStream {
            quote!(drizzle::migrations::sqlite::SQLiteSnapshot)
        }

        pub fn entity() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::SqliteEntity)
        }

        pub fn table() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::Table)
        }

        pub fn table_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::TableDef)
        }

        pub fn column() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::Column)
        }

        pub fn column_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ColumnDef)
        }

        pub fn index() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::Index)
        }

        pub fn index_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::IndexDef)
        }

        pub fn index_column() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::IndexColumn)
        }

        pub fn index_column_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::IndexColumnDef)
        }

        pub fn index_origin() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::IndexOrigin)
        }

        pub fn primary_key() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::PrimaryKey)
        }

        pub fn primary_key_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::PrimaryKeyDef)
        }

        pub fn unique_constraint() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::UniqueConstraint)
        }

        pub fn unique_constraint_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::UniqueConstraintDef)
        }

        pub fn view() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::View)
        }

        pub fn view_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ViewDef)
        }

        pub fn foreign_key() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ForeignKey)
        }

        pub fn foreign_key_def() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ForeignKeyDef)
        }

        pub fn referential_action() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::ReferentialAction)
        }

        pub fn table_sql() -> TokenStream {
            quote!(drizzle::ddl::sqlite::ddl::TableSql)
        }
    }

    // PostgreSQL DDL types (from drizzle::ddl)
    pub mod postgres {
        use super::*;

        pub fn snapshot() -> TokenStream {
            quote!(drizzle::migrations::postgres::PostgresSnapshot)
        }

        pub fn entity() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::PostgresEntity)
        }

        pub fn schema_entity() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::Schema)
        }

        pub fn table() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::Table)
        }

        pub fn table_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::TableDef)
        }

        pub fn column() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::Column)
        }

        pub fn column_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ColumnDef)
        }

        pub fn identity() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::Identity)
        }

        pub fn identity_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::IdentityDef)
        }

        pub fn index() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::Index)
        }

        pub fn index_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::IndexDef)
        }

        pub fn primary_key() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::PrimaryKey)
        }

        pub fn primary_key_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::PrimaryKeyDef)
        }

        pub fn unique_constraint() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::UniqueConstraint)
        }

        pub fn unique_constraint_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::UniqueConstraintDef)
        }

        pub fn view() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::View)
        }

        pub fn view_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ViewDef)
        }

        pub fn foreign_key() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ForeignKey)
        }

        pub fn foreign_key_def() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ForeignKeyDef)
        }

        pub fn enum_type() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::Enum)
        }

        pub fn referential_action() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::ReferentialAction)
        }

        pub fn table_sql() -> TokenStream {
            quote!(drizzle::ddl::postgres::ddl::TableSql)
        }
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

    pub fn postgres_update_value() -> TokenStream {
        quote!(drizzle::postgres::values::PostgresUpdateValue)
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
        quote!(drizzle::postgres::traits::DrizzleRow)
    }

    pub fn row() -> TokenStream {
        quote!(drizzle::postgres::Row)
    }

    pub fn column_marker() -> TokenStream {
        quote!(drizzle::postgres::attrs::ColumnMarker)
    }
}
