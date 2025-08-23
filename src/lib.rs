extern crate self as drizzle_rs;

mod drizzle;
mod transaction;

// Essential re-exports
pub use drizzle_core::error::Result;
pub use drizzle_macros::{FromRow, SQLSchema, sql};

// Error types
pub mod error {
    pub use drizzle_core::error::DrizzleError;
}

// Core components (dialect-agnostic)
pub mod core {
    // Core traits and types
    pub use drizzle_core::traits::*;
    pub use drizzle_core::{
        OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, SQLComparable, SQLSchemaType, ToSQL,
    };

    // Prepared statements
    pub use drizzle_core::prepared::{PreparedStatement, owned::OwnedPreparedStatement};

    // Condition expressions
    pub use drizzle_core::expressions::conditions::*;

    // Expression functions
    pub use drizzle_core::expressions::{alias, cast, r#typeof};
}

// Shared SQLite components
#[cfg(feature = "sqlite")]
pub mod sqlite {
    pub use drizzle_sqlite::builder::QueryBuilder;

    // SQLite macros
    pub use drizzle_macros::{SQLiteEnum, SQLiteIndex, SQLiteTable};

    // SQLite builders and helpers
    pub use drizzle_sqlite::builder;
    pub use drizzle_sqlite::conditions;
    pub use drizzle_sqlite::{SQLiteTransactionType, params};

    // SQLite types and traits
    pub use drizzle_sqlite::traits::{SQLiteColumn, SQLiteColumnInfo};
    pub use drizzle_sqlite::values::{InsertValue, OwnedSQLiteValue, SQLiteValue, ValueWrapper};
}

// Rusqlite driver
#[cfg(feature = "rusqlite")]
pub mod rusqlite {
    pub use crate::drizzle::sqlite::rusqlite::Drizzle;
    pub use crate::transaction::sqlite::rusqlite::Transaction;
}

// LibSQL driver
#[cfg(feature = "libsql")]
pub mod libsql {
    pub use crate::drizzle::sqlite::libsql::Drizzle;
    pub use crate::transaction::sqlite::libsql::Transaction;
}

// Turso driver
#[cfg(feature = "turso")]
pub mod turso {
    pub use crate::drizzle::sqlite::turso::Drizzle;
    pub use crate::transaction::sqlite::turso::Transaction;
}

// Placeholder for future dialects
#[cfg(feature = "postgres")]
pub mod postgres {
    // pub use querybuilder::postgres::...;
}

#[cfg(feature = "mysql")]
pub mod mysql {
    // pub use querybuilder::mysql::...;
}

/// A comprehensive prelude that brings commonly used items into scope.
///
/// This includes all shared functionality but NOT the `Drizzle` struct.
/// Users must explicitly import the driver they want:
///
/// ```ignore
/// use drizzle_rs::prelude::*;           // Shared functionality
/// use drizzle_rs::rusqlite::Drizzle;    // Explicit driver choice
/// ```
pub mod prelude {
    // Core components (traits, types, expressions)
    pub use crate::core::*;

    // Expression helpers
    pub use drizzle_core::expressions::{alias, cast, r#typeof};

    // Essential macros
    pub use drizzle_macros::{FromRow, SQLSchema};

    #[cfg(feature = "sqlite")]
    pub use crate::sqlite::*;

    #[cfg(feature = "sqlite")]
    pub use drizzle_macros::{SQLiteEnum, SQLiteIndex, SQLiteTable};

    // Future dialect support
    // #[cfg(feature = "postgres")]
    // pub use crate::postgres::*;
    // #[cfg(feature = "postgres")]
    // pub use procmacros::{PostgresEnum, PostgresTable};

    // #[cfg(feature = "mysql")]
    // pub use crate::mysql::*;
    // #[cfg(feature = "mysql")]
    // pub use procmacros::{MySQLEnum, MySQLTable};
}

#[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
#[cfg(test)]
mod tests {
    use drizzle_macros::SQLiteTable;
    use drizzle_rs::prelude::*;

    #[cfg(feature = "rusqlite")]
    use rusqlite;

    #[SQLiteTable(name = "Users")]
    pub struct User {
        #[integer(primary)]
        id: i32,
        #[text]
        name: String,
        #[text]
        email: Option<String>,
    }

    #[SQLiteTable(name = "Posts")]
    pub struct Post {
        #[integer(primary)]
        id: i32,
        #[text]
        title: String,
    }

    #[SQLiteTable(name = "Comments")]
    pub struct Comment {
        #[integer(primary)]
        id: i32,
        #[text]
        content: String,
    }

    #[derive(SQLSchema)]
    pub struct Schema {
        pub user: User,
        pub post: Post,
        pub comment: Comment,
    }

    #[test]
    fn test_schema_macro() {
        // Create a schema with the User table using schema! macro
        let Schema { user, .. } = Schema::new();
        let builder = QueryBuilder::new::<Schema>();

        let query = builder.select(user.id).from(user);
        assert_eq!(query.to_sql().sql(), r#"SELECT "Users"."id" FROM "Users""#);
    }

    #[cfg(feature = "rusqlite")]
    #[test]
    fn test_insert() {
        use drizzle_sqlite::builder::Conflict;

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let (db, Schema { user, .. }) = drizzle_rs::rusqlite::Drizzle::new(conn, Schema::new());
        db.create().expect("Should have created table");

        let result = db
            .insert(user)
            .values([InsertUser::new("test").with_name("test")])
            .on_conflict(Conflict::default())
            .execute()
            .expect("Should have inserted");

        assert_eq!(result, 1);

        let query: Vec<SelectUser> = db
            .select(())
            .from(user)
            .all()
            .expect("should have gotten all users");

        assert_eq!(query.len(), 1);
        assert_eq!(query[0].id, 1);
        assert_eq!(query[0].name, "test");
        assert_eq!(query[0].email, None);
    }

    #[test]
    fn test_placeholder_integration() {
        use drizzle_rs::core::Placeholder;

        // Test that placeholders work with the new unified SQL-based approach
        let placeholder = Placeholder::colon("test_name");
        let insert_value: InsertUser<'_, _> = InsertUser::new(placeholder);

        // Verify it's a Value variant containing SQL with the placeholder
        match &insert_value.name {
            drizzle_rs::sqlite::InsertValue::Value(wrapper) => {
                // Check that the SQL contains our placeholder
                let sql_string = wrapper.value.sql();
                assert!(sql_string.contains("test_name") || sql_string.contains("?"));
            }
            _ => panic!("Expected Value variant containing SQL"),
        }

        // Test that regular values still work
        let regular_insert: InsertUser<'_, _> = InsertUser::new("regular_value");
        match &regular_insert.name {
            drizzle_rs::sqlite::InsertValue::Value(wrapper) => {
                // Check that the SQL contains our parameter
                assert!(!wrapper.value.sql().is_empty());
            }
            _ => panic!("Expected Value variant for regular string"),
        }
    }
}
