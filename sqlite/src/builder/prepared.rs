use std::borrow::Cow;

use drizzle_core::{
    prepared::{
        OwnedPreparedStatement as CoreOwnedPreparedStatement,
        PreparedStatement as CorePreparedStatement,
    },
    OwnedParam, Param,
};

use crate::values::{OwnedSQLiteValue, SQLiteValue};

/// SQLite-specific prepared statement wrapper.
///
/// A prepared statement represents a compiled SQL query with placeholder parameters
/// that can be executed multiple times with different parameter values. This wrapper
/// provides SQLite-specific functionality while maintaining compatibility with the
/// core Drizzle prepared statement infrastructure.
///
/// ## Features
///
/// - **Parameter Binding**: Safely bind values to SQL placeholders
/// - **Reusable Execution**: Execute the same query multiple times efficiently  
/// - **Memory Management**: Automatic handling of borrowed/owned lifetimes
/// - **Type Safety**: Compile-time verification of parameter types
///
/// ## Basic Usage
///
/// ```rust,ignore
/// use drizzle_sqlite::builder::QueryBuilder;
/// use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// use drizzle_core::{ToSQL, expr::eq};
///
/// #[SQLiteTable(name = "users")]
/// struct User {
///     #[integer(primary)]
///     id: i32,
///     #[text]
///     name: String,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     user: User,
/// }
///
/// let builder = QueryBuilder::new::<Schema>();
/// let Schema { user } = Schema::new();
///
/// // Build query that will become a prepared statement
/// let query = builder
///     .select(user.name)
///     .from(user)
///     .r#where(eq(user.id, drizzle_core::Placeholder::anonymous()));
///
/// // Convert to prepared statement (this would typically be done by the driver)
/// let sql = query.to_sql();
/// println!("SQL: {}", sql.sql());  // "SELECT "users"."name" FROM "users" WHERE "users"."id" = ?"
/// ```
///
/// ## Lifetime Management
///
/// The prepared statement can be converted between borrowed and owned forms:
///
/// - `PreparedStatement<'a>` - Borrows data with lifetime 'a
/// - `OwnedPreparedStatement` - Owns all data, no lifetime constraints
///
/// This allows for flexible usage patterns depending on whether you need to
/// store the prepared statement long-term or use it immediately.
#[derive(Debug, Clone)]
pub struct PreparedStatement<'a> {
    pub inner: CorePreparedStatement<'a, SQLiteValue<'a>>,
}

impl<'a> PreparedStatement<'a> {
    /// Converts this borrowed prepared statement into an owned one.
    ///
    /// This method clones all the internal data to create an `OwnedPreparedStatement`
    /// that doesn't have any lifetime constraints. This is useful when you need to
    /// store the prepared statement beyond the lifetime of the original query builder.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// # use drizzle_sqlite::builder::PreparedStatement;
    /// # let prepared_statement: PreparedStatement = todo!(); // Would come from driver
    /// // Convert borrowed to owned for long-term storage
    /// let owned = prepared_statement.into_owned();
    ///
    /// // Now `owned` can be stored without lifetime constraints
    /// ```
    pub fn into_owned(&self) -> OwnedPreparedStatement {
        let owned_params = self.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedSQLiteValue::from(v.into_owned())),
        });

        let inner = CoreOwnedPreparedStatement {
            text_segments: self.inner.text_segments.clone(),
            params: owned_params.collect::<Box<[_]>>(),
            sql: self.inner.sql.clone(),
        };

        OwnedPreparedStatement { inner }
    }
}

/// Owned SQLite prepared statement wrapper.
///
/// This is the owned counterpart to [`PreparedStatement`] that doesn't have any lifetime
/// constraints. All data is owned by this struct, making it suitable for long-term storage,
/// caching, or passing across thread boundaries.
///
/// ## Use Cases
///
/// - **Caching**: Store prepared statements in a cache for reuse
/// - **Multi-threading**: Pass prepared statements between threads
/// - **Long-term storage**: Keep prepared statements in application state
/// - **Serialization**: Convert to/from persistent storage (when serialization is implemented)
///
/// ## Examples
///
/// ```rust,ignore
/// use drizzle_sqlite::builder::{QueryBuilder, PreparedStatement, OwnedPreparedStatement};
/// use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// use drizzle_core::ToSQL;
///
/// #[SQLiteTable(name = "users")]
/// struct User {
///     #[integer(primary)]
///     id: i32,
///     #[text]
///     name: String,
/// }
///
/// #[derive(SQLiteSchema)]
/// struct Schema {
///     user: User,
/// }
///
/// let builder = QueryBuilder::new::<Schema>();
/// let Schema { user } = Schema::new();
///
/// // Create a prepared statement and convert to owned
/// let query = builder.select(user.name).from(user);
/// let sql = query.to_sql();
///
/// // In practice, this conversion would be handled by the database driver
/// // let prepared: PreparedStatement = driver.prepare(sql)?;
/// // let owned: OwnedPreparedStatement = prepared.into_owned();
/// ```
///
/// ## Conversion
///
/// You can convert between borrowed and owned forms:
/// - `PreparedStatement::into_owned()` → `OwnedPreparedStatement`
/// - `OwnedPreparedStatement` → `PreparedStatement` (via `From` trait)
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub inner: CoreOwnedPreparedStatement<crate::values::OwnedSQLiteValue>,
}
impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        let owned_params = value.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedSQLiteValue::from(v.into_owned())),
        });
        let inner = CoreOwnedPreparedStatement {
            text_segments: value.inner.text_segments,
            params: owned_params.collect::<Box<[_]>>(),
            sql: value.inner.sql,
        };
        Self { inner }
    }
}

impl From<OwnedPreparedStatement> for PreparedStatement<'_> {
    fn from(value: OwnedPreparedStatement) -> Self {
        let sqlitevalue = value.inner.params.iter().map(|v| {
            Param::new(
                v.placeholder,
                v.value.clone().map(|v| Cow::Owned(SQLiteValue::from(v))),
            )
        });
        let inner = CorePreparedStatement {
            text_segments: value.inner.text_segments,
            params: sqlitevalue.collect::<Box<[_]>>(),
            sql: value.inner.sql,
        };
        PreparedStatement { inner }
    }
}

impl OwnedPreparedStatement {}

impl<'a> std::fmt::Display for PreparedStatement<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl std::fmt::Display for OwnedPreparedStatement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::SQLiteValue;
    use drizzle_core::{prepared::prepare_render, SQL};

    #[test]
    fn test_prepare_render_basic() {
        // Test the basic prepare_render functionality for SQLite
        let sql: SQL<'_, SQLiteValue<'_>> = SQL::raw("SELECT * FROM users WHERE id = ")
            .append(SQL::placeholder("user_id"))
            .append(SQL::raw(" AND name = "))
            .append(SQL::placeholder("user_name"));

        let prepared = prepare_render(sql);

        // Should have 3 text segments: before first param, between params, after last param
        assert_eq!(prepared.text_segments.len(), 3);
        assert_eq!(prepared.params.len(), 2);

        // Verify text segments contain expected content
        assert!(prepared.text_segments[0].contains("SELECT * FROM users WHERE id"));
        assert!(prepared.text_segments[1].contains("AND name"));
    }

    #[test]
    fn test_prepare_with_no_parameters() {
        // Test preparing SQL with no parameters
        let sql: SQL<'_, SQLiteValue<'_>> = SQL::raw("SELECT COUNT(*) FROM users");
        let prepared = prepare_render(sql);

        assert_eq!(prepared.text_segments.len(), 1);
        assert_eq!(prepared.params.len(), 0);
        assert_eq!(prepared.text_segments[0], "SELECT COUNT(*) FROM users");
    }

    #[test]
    fn test_prepared_statement_display() {
        let sql: SQL<'_, SQLiteValue<'_>> = SQL::raw("SELECT * FROM users")
            .append(SQL::raw(" WHERE id = "))
            .append(SQL::placeholder("id"));

        let prepared = prepare_render(sql);
        let display = format!("{}", prepared);

        assert!(display.contains("SELECT * FROM users"));
        assert!(display.contains("WHERE id"));
    }

    #[test]
    fn test_owned_conversion_roundtrip() {
        let sql: SQL<'_, SQLiteValue<'_>> =
            SQL::raw("SELECT name FROM users WHERE id = ").append(SQL::placeholder("id"));

        let prepared = prepare_render(sql);
        let core_prepared = PreparedStatement { inner: prepared };

        // Convert to owned
        let owned = core_prepared.into_owned();

        // Convert back to borrowed
        let borrowed: PreparedStatement<'_> = owned.into();

        // Verify structure is preserved
        assert_eq!(borrowed.inner.text_segments.len(), 2);
        assert_eq!(borrowed.inner.params.len(), 1);
    }
}
