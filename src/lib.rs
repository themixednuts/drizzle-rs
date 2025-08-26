mod drizzle;
mod transaction;

// Essential re-exports
pub use drizzle_core::error::Result;
pub use drizzle_macros::sql;

#[cfg(any(feature = "rusqlite", feature = "libsql", feature = "turso"))]
pub use drizzle_macros::FromRow;

// Error types
pub mod error {
    pub use drizzle_core::error::DrizzleError;
}

/// Core functionality
pub mod core {
    // Core traits and types
    pub use drizzle_core::traits::*;
    pub use drizzle_core::{
        OrderBy, Param, ParamBind, Placeholder, SQL, SQLChunk, SQLComparable, ToSQL,
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
    pub use drizzle_macros::{SQLiteEnum, SQLiteIndex, SQLiteSchema, SQLiteTable};

    pub use drizzle_sqlite::conditions;
    pub use drizzle_sqlite::{SQLiteTransactionType, params, pragma};

    // SQLite types and traits
    pub use drizzle_sqlite::builder;
    pub use drizzle_sqlite::common;
    pub use drizzle_sqlite::traits;
    pub use drizzle_sqlite::values;
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

// Sqlx PostgreSQL driver
// #[cfg(feature = "sqlx-postgres")]
// pub mod sqlx_postgres {
//     pub use crate::drizzle::postgres::sqlx::Drizzle;
//     pub use crate::transaction::postgres::sqlx::Transaction;
// }

// Shared PostgreSQL components
#[cfg(feature = "postgres")]
pub mod postgres {
    pub use drizzle_postgres::builder::QueryBuilder;

    // PostgreSQL macros
    pub use drizzle_macros::{PostgresEnum, PostgresIndex, PostgresSchema, PostgresTable};

    // PostgreSQL builders and helpers
    pub use drizzle_postgres::PostgresTransactionType;

    // PostgreSQL types and traits
    pub use drizzle_postgres::builder;
    pub use drizzle_postgres::common;
    pub use drizzle_postgres::traits;
    pub use drizzle_postgres::values;
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
/// use drizzle::prelude::*;           // Shared functionality
/// use drizzle::rusqlite::Drizzle;    // Explicit driver choice
/// ```
pub mod prelude {
    // Core components (traits, types, expressions)
    pub use crate::core::*;

    // Expression helpers
    pub use drizzle_core::expressions::{alias, cast, r#typeof};

    // Essential macros
    pub use drizzle_macros::FromRow;

    // #[cfg(feature = "sqlite")]
    // pub use crate::sqlite::*;

    #[cfg(feature = "sqlite")]
    pub use drizzle_macros::{SQLiteEnum, SQLiteIndex, SQLiteSchema, SQLiteTable};

    // #[cfg(feature = "postgres")]
    // pub use crate::postgres::*;

    #[cfg(feature = "postgres")]
    pub use drizzle_macros::{PostgresEnum, PostgresIndex, PostgresSchema, PostgresTable};

    // #[cfg(feature = "mysql")]
    // pub use crate::mysql::*;
    // #[cfg(feature = "mysql")]
    // pub use procmacros::{MySQLEnum, MySQLTable};
}

#[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
#[cfg(test)]
mod sqlite_tests {
    use drizzle::prelude::*;
    use drizzle_macros::SQLiteTable;

    use drizzle_sqlite::builder::QueryBuilder;
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

    #[derive(SQLiteSchema)]
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
        let (db, Schema { user, .. }) = drizzle::rusqlite::Drizzle::new(conn, Schema::new());
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
        use drizzle::core::Placeholder;

        // Test that placeholders work with the new unified SQL-based approach
        let placeholder = Placeholder::colon("test_name");
        let insert_value: InsertUser<'_, _> = InsertUser::new(placeholder);

        // Verify it's a Value variant containing SQL with the placeholder
        match &insert_value.name {
            drizzle::sqlite::values::InsertValue::Value(wrapper) => {
                // Check that the SQL contains our placeholder
                let sql_string = wrapper.value.sql();
                assert!(sql_string.contains("test_name") || sql_string.contains("?"));
            }
            _ => panic!("Expected Value variant containing SQL"),
        }

        // Test that regular values still work
        let regular_insert: InsertUser<'_, _> = InsertUser::new("regular_value");
        match &regular_insert.name {
            drizzle::sqlite::values::InsertValue::Value(wrapper) => {
                // Check that the SQL contains our parameter
                assert!(!wrapper.value.sql().is_empty());
            }
            _ => panic!("Expected Value variant for regular string"),
        }
    }
}

#[cfg(feature = "postgres")]
#[cfg(test)]
mod postgres_tests {
    use drizzle_core::{SQLColumnInfo, ToSQL};
    use drizzle_macros::{PostgresEnum, PostgresIndex, PostgresTable};

    #[derive(Debug, Clone, Default, PostgresEnum)]
    pub enum Status {
        #[default]
        Active,
        Inactive,
    }

    #[derive(Debug, Clone, Default, PostgresEnum)]
    pub enum Priority {
        #[default]
        Low,
        Medium,
        High,
    }

    #[PostgresTable(name = "users")]
    pub struct User {
        #[serial(primary)]
        id: i32,
        #[text]
        name: String,
        #[text]
        email: Option<String>,
        #[text(enum)]
        status: Status,
        #[r#enum(Priority)]
        priority: Priority,
    }

    #[PostgresTable(name = "posts")]
    pub struct Post {
        #[serial(primary)]
        id: i32,
        #[text]
        title: String,
        #[boolean]
        published: bool,
    }

    #[PostgresIndex(unique)]
    pub struct UserEmailIdx(User::email);

    #[test]
    fn test_postgres_table_creation() {
        let user = User::new();
        assert_eq!(user.id.name(), "id");
        assert_eq!(user.name.name(), "name");
        assert_eq!(user.email.name(), "email");
    }

    #[test]
    fn test_postgres_index_creation() {
        let idx = UserEmailIdx::new();
        assert_eq!(idx.index_name(), "user_email_idx");
        assert!(idx.create_index_sql().contains("CREATE UNIQUE INDEX"));
    }

    #[test]
    fn test_postgres_enum_values() {
        use drizzle_core::ToSQL;

        // Test text-based enum (stored as TEXT)
        let status = Status::Active;
        let status_sql = status.to_sql();
        println!("Text enum SQL: {}", status_sql);

        // Test native PostgreSQL enum (stored as native ENUM)
        let priority = Priority::High;
        let priority_sql = priority.to_sql();
        println!("Native enum SQL: {}", priority_sql);

        // Verify they generate different types of SQL
        // Text enum should be a text parameter
        // Native enum should be an enum parameter
        assert!(!status_sql.to_string().is_empty());
        assert!(!priority_sql.to_string().is_empty());
    }

    #[test]
    fn test_postgress_stuff() {
        use drizzle_core::ToSQL;
        let sql = InsertUser::new("name", Status::Active, Priority::Low).with_email("test@email");
    }
}
