// Core types and modules
pub mod core;
pub mod sqlite;

// Re-export common types at the crate root for easy access
pub use core::{
    IntoValue, SQL, SQLParam, ToSQL, expressions::conditions::*, placeholders, traits::*,
};
pub use core::{Placeholder, PlaceholderStyle};

// The procmacros attributes are imported directly in files that need them,
// not re-exported here to avoid conflicts with existing macros

// Macros with #[macro_export] are already available directly at the crate root

/// Macro for creating a list of columns for SELECT queries
///
/// This macro makes it easier to select specific columns in a query.
/// It accepts any number of column expressions and creates a Columns
/// instance with them. Columns can be either string literals or SQLiteColumn
/// instances.
///
/// # Examples
///
/// ```
/// use querybuilder::prelude::*;
/// use querybuilder::columns;
///
/// // Select specific columns using string literals
/// let query = QueryBuilder::<Users>::new();
/// let select = query.select(columns!("id", "name", "email"));
///
/// // Use with column aliases (renaming)
/// // let select = query.select(columns!(
/// //     alias("id", "user_id"),         // String literal with alias
/// //     Users::email.as_("user_email")  // SQLiteColumn with as_ method
/// // ));
/// ```
#[macro_export]
macro_rules! columns {
    // Base case with no columns
    () => {
        $crate::sqlite::query_builder::Columns::List(vec![])
    };

    // Case with a single column name (string literal or SQLiteColumn)
    ($col:expr) => {
        {
            use $crate::core::ToSQL;
            $crate::sqlite::query_builder::Columns::List(vec![$col.to_sql()])
        }
    };

    // Case with multiple expressions
    ($($col:expr),+ $(,)?) => {
        {
            use $crate::core::ToSQL;
            $crate::sqlite::query_builder::Columns::List(vec![
                $($col.to_sql()),+
            ])
        }
    };
}

// Create a unified prelude that brings everything together
pub mod prelude {
    // Re-export core functionality
    pub use crate::core::*;
    // pub use crate::sqlite::prelude::*; // Removed redundant sqlite prelude export here

    // Removed misleading re-export of SQLiteValue
    // pub use crate::sqlite::common::SQLiteValue;

    // Re-export core traits
    pub use crate::core::traits::*;

    // Re-export schema validation trait
    pub use crate::core::schema_traits::*;

    // Re-export condition functions
    pub use crate::core::expressions::conditions::*;

    // Re-export macros from the crate root
    pub use crate::{and, columns, or};

    // SQLite-specific exports
    #[cfg(feature = "sqlite")]
    pub use crate::sqlite::prelude::*; // This should contain the needed sqlite specifics

    // Column aliasing helper
    #[cfg(feature = "sqlite")]
    pub use crate::sqlite::query_builder::alias;
}

// No need to re-export sqlite::prelude here as it's already included in the main prelude

#[cfg(test)]
mod tests {
    #[cfg(feature = "sqlite")]
    use crate::sqlite::common::SQLiteValue;

    #[test]
    #[cfg(feature = "sqlite")]
    fn test_sqlite_value_conversions() {
        // Test integer conversion
        let int_val: SQLiteValue<'_> = 42.into();
        if let SQLiteValue::Integer(i) = int_val {
            assert_eq!(i, 42);
        } else {
            panic!("Expected Integer variant");
        }

        // Test string conversion
        let str_val: SQLiteValue<'_> = "hello".into();
        if let SQLiteValue::Text(t) = str_val {
            assert_eq!(t, "hello");
        } else {
            panic!("Expected Text variant");
        }

        // Test Option conversion (Some)
        let some_val: SQLiteValue<'_> = Some("test").into();
        if let SQLiteValue::Text(t) = some_val {
            assert_eq!(t, "test");
        } else {
            panic!("Expected Text variant");
        }

        // Test Option conversion (None)
        let none_val: SQLiteValue<'_> = Option::<String>::None.into();
        if let SQLiteValue::Null = none_val {
            // This is expected
        } else {
            panic!("Expected Null variant");
        }
    }
}
