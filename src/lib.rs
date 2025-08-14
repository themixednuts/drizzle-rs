extern crate self as drizzle_rs;

mod drizzle;
pub use drizzle_core::error::Result;
pub use procmacros::{drizzle, qb};

pub mod error {
    pub use drizzle_core::error::DrizzleError;
}

// Core components (dialect-agnostic)
pub mod core {
    // Core traits and types from core crate
    pub use drizzle_core::traits::*;
    pub use drizzle_core::{Param, ParamBind, SQL, SQLComparable, ToSQL};

    // Core expression functions & macros
    pub use drizzle_core::SQLSchemaType;
    pub use drizzle_core::expressions::conditions::*;
}

// SQLite specific components
#[cfg(feature = "sqlite")]
pub mod sqlite {
    pub use super::drizzle::sqlite::Drizzle;
    pub use ::procmacros::SQLiteIndex;
    pub use sqlite::builder::QueryBuilder;

    // SQLite specific types, columns, etc. from sqlite crate
    pub use sqlite::builder;
    pub use sqlite::conditions;
    pub use sqlite::traits::{SQLiteColumn, SQLiteColumnInfo};
    pub use sqlite::values::{InsertValue, SQLiteValue};
    pub use sqlite::{SQLiteTransactionType, params};

    // Re-export rusqlite specific functionality when the feature is enabled
    #[cfg(feature = "rusqlite")]
    pub use ::rusqlite;

    // Re-export libsql specific functionality when the feature is enabled
    #[cfg(feature = "libsql")]
    pub use ::libsql;

    // Re-export libsql-rusqlite specific functionality when the feature is enabled
    #[cfg(feature = "libsql-rusqlite")]
    pub use ::libsql_rusqlite;
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
pub mod prelude {
    // Core components (traits, types, expressions)
    pub use crate::core::*; // Includes core traits, SQL, SQLParam, conditions::*, core macros

    // Export QueryBuilder types from core crate
    pub use drizzle_core::expressions::alias;

    #[cfg(feature = "sqlite")]
    pub use super::drizzle::sqlite::Drizzle;
    #[cfg(feature = "sqlite")]
    pub use sqlite::builder::QueryBuilder;

    // Proc Macros (essential for schema definition)
    pub use procmacros::{FromRow, drizzle, qb};

    // Dialect-specific components (gated)
    #[cfg(feature = "sqlite")]
    pub use crate::sqlite::*; // Includes SQLiteColumn, SQLiteValue, etc.

    #[cfg(feature = "sqlite")]
    pub use procmacros::{SQLiteEnum, SQLiteIndex, SQLiteTable}; // SQLite specific macros

    // #[cfg(feature = "postgres")]
    // pub use crate::postgres::*;
    // #[cfg(feature = "postgres")] pub use procmacros::{PostgresEnum, PostgresTable};

    // #[cfg(feature = "mysql")]
    // pub use crate::mysql::*;
    // #[cfg(feature = "mysql")] pub use procmacros::{MySQLEnum, MySQLTable};
}

#[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
#[cfg(test)]
mod tests {
    use drizzle_rs::prelude::*;
    use procmacros::{SQLiteTable, drizzle, qb};

    #[cfg(feature = "rusqlite")]
    use rusqlite;

    #[SQLiteTable(name = "Users")]
    struct User {
        #[integer(primary)]
        id: i32,
        #[text]
        name: String,
        #[text]
        email: Option<String>,
    }

    #[SQLiteTable(name = "Posts")]
    struct Post {
        #[integer(primary)]
        id: i32,
        #[text]
        title: String,
    }

    #[SQLiteTable(name = "Comments")]
    struct Comment {
        #[integer(primary)]
        id: i32,
        #[text]
        content: String,
    }

    #[test]
    fn test_schema_macro() {
        // Create a schema with the User table using schema! macro
        let (builder, (user, ..)) = qb!([User, Post]);

        let query = builder.select(user.id).from(user);
        assert_eq!(query.to_sql().sql(), r#"SELECT "Users"."id" FROM "Users""#);
    }

    #[cfg(feature = "rusqlite")]
    #[test]
    fn test_insert() {
        use sqlite::builder::Conflict;

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        let (db, user) = drizzle!(conn, [User]);
        db.execute(User::SQL).expect("Should have created table");

        let result = db
            .insert(user)
            .values([InsertUser::default().with_name("test")])
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
}
