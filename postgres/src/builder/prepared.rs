use crate::prelude::*;

use drizzle_core::{
    OwnedParam, Param,
    prepared::{
        OwnedPreparedStatement as CoreOwnedPreparedStatement,
        PreparedStatement as CorePreparedStatement,
    },
};

use crate::values::{OwnedPostgresValue, PostgresValue};

/// PostgreSQL-specific prepared statement wrapper.
///
/// A prepared statement represents a compiled SQL query with placeholder parameters
/// that can be executed multiple times with different parameter values. This wrapper
/// provides PostgreSQL-specific functionality while maintaining compatibility with the
/// core Drizzle prepared statement infrastructure.
///
/// ## Features
///
/// - **Parameter Binding**: Safely bind values to SQL placeholders using `$1`, `$2`, etc.
/// - **Reusable Execution**: Execute the same query multiple times efficiently
/// - **Memory Management**: Automatic handling of borrowed/owned lifetimes
/// - **Type Safety**: Compile-time verification of parameter types
///
/// ## Basic Usage
///
/// ```rust,ignore
/// use drizzle_postgres::builder::QueryBuilder;
/// use drizzle_macros::{PostgresTable, PostgresSchema};
/// use drizzle_core::{ToSQL, SQL, expr::eq};
///
/// #[PostgresTable(name = "users")]
/// struct User {
///     #[column(serial, primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[derive(PostgresSchema)]
/// struct Schema {
///     user: User,
/// }
///
/// let builder = QueryBuilder::new::<Schema>();
/// let Schema { user } = Schema::new();
///
/// // Build query with a placeholder
/// let query = builder
///     .select(user.name)
///     .from(user)
///     .r#where(eq(user.id, user.id.placeholder("id")));
///
/// // Convert to prepared statement
/// let prepared = query.prepare();
/// println!("SQL: {}", prepared);  // SELECT "users"."name" FROM "users" WHERE "users"."id" = :id
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
    pub(crate) inner: CorePreparedStatement<'a, PostgresValue<'a>>,
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
    /// # use drizzle_postgres::builder::PreparedStatement;
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
                .map(|v| OwnedPostgresValue::from(v.into_owned())),
        });

        let inner = CoreOwnedPreparedStatement {
            text_segments: self.inner.text_segments.clone(),
            params: owned_params.collect::<Box<[_]>>(),
            sql: self.inner.sql.clone(),
        };

        OwnedPreparedStatement { inner }
    }
}

/// Owned PostgreSQL prepared statement wrapper.
///
/// This is the owned counterpart to [`PreparedStatement`] that doesn't have any lifetime
/// constraints. All data is owned by this struct, making it suitable for long-term storage,
/// caching, or passing across thread boundaries.
///
/// ## Use Cases
///
/// - **Caching**: Store prepared statements in a cache for reuse
/// - **Multi-threading**: Pass prepared statements between threads (with tokio-postgres)
/// - **Long-term storage**: Keep prepared statements in application state
/// - **Query reuse**: Execute the same query with different parameters efficiently
///
/// ## Examples
///
/// ```rust,ignore
/// use drizzle_postgres::builder::{QueryBuilder, PreparedStatement, OwnedPreparedStatement};
/// use drizzle_macros::{PostgresTable, PostgresSchema};
/// use drizzle_core::{ToSQL, SQL};
///
/// #[PostgresTable(name = "users")]
/// struct User {
///     #[column(serial, primary)]
///     id: i32,
///     name: String,
/// }
///
/// #[derive(PostgresSchema)]
/// struct Schema {
///     user: User,
/// }
///
/// let builder = QueryBuilder::new::<Schema>();
/// let Schema { user } = Schema::new();
///
/// // Create a prepared statement and convert to owned
/// let prepared = builder
///     .select(user.name)
///     .from(user)
///     .prepare();
///
/// let owned: OwnedPreparedStatement = prepared.into_owned();
///
/// // Owned can be stored in a HashMap for reuse
/// // let mut cache: HashMap<String, OwnedPreparedStatement> = HashMap::new();
/// // cache.insert("select_user_name".to_string(), owned);
/// ```
///
/// ## Conversion
///
/// You can convert between borrowed and owned forms:
/// - `PreparedStatement::into_owned()` → `OwnedPreparedStatement`
/// - `OwnedPreparedStatement` → `PreparedStatement` (via `From` trait)
#[derive(Debug, Clone)]
pub struct OwnedPreparedStatement {
    pub(crate) inner: CoreOwnedPreparedStatement<crate::values::OwnedPostgresValue>,
}

impl<'a> From<PreparedStatement<'a>> for OwnedPreparedStatement {
    fn from(value: PreparedStatement<'a>) -> Self {
        let owned_params = value.inner.params.iter().map(|p| OwnedParam {
            placeholder: p.placeholder,
            value: p
                .value
                .clone()
                .map(|v| OwnedPostgresValue::from(v.into_owned())),
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
        let postgresvalue = value.inner.params.iter().map(|v| {
            Param::new(
                v.placeholder,
                v.value.clone().map(|v| Cow::Owned(PostgresValue::from(v))),
            )
        });
        let inner = CorePreparedStatement {
            text_segments: value.inner.text_segments,
            params: postgresvalue.collect::<Box<[_]>>(),
            sql: value.inner.sql,
        };
        PreparedStatement { inner }
    }
}

impl OwnedPreparedStatement {}

impl<'a> core::fmt::Display for PreparedStatement<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

impl core::fmt::Display for OwnedPreparedStatement {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}", self.inner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{SQL, prepared::prepare_render};

    #[test]
    fn test_prepare_render_basic() {
        // Test the basic prepare_render functionality for PostgreSQL
        let sql: SQL<'_, PostgresValue<'_>> = SQL::raw("SELECT * FROM users WHERE id = ")
            .append(drizzle_core::Placeholder::named("user_id").to_sql())
            .append(SQL::raw(" AND name = "))
            .append(drizzle_core::Placeholder::named("user_name").to_sql());

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
        let sql: SQL<'_, PostgresValue<'_>> = SQL::raw("SELECT COUNT(*) FROM users");
        let prepared = prepare_render(sql);

        assert_eq!(prepared.text_segments.len(), 1);
        assert_eq!(prepared.params.len(), 0);
        assert_eq!(prepared.text_segments[0], "SELECT COUNT(*) FROM users");
    }

    #[test]
    fn test_prepared_statement_display() {
        let sql: SQL<'_, PostgresValue<'_>> = SQL::raw("SELECT * FROM users")
            .append(SQL::raw(" WHERE id = "))
            .append(drizzle_core::Placeholder::named("id").to_sql());

        let prepared = prepare_render(sql);
        let display = format!("{}", prepared);

        assert!(display.contains("SELECT * FROM users"));
        assert!(display.contains("WHERE id"));
    }

    #[test]
    fn test_owned_conversion_roundtrip() {
        let sql: SQL<'_, PostgresValue<'_>> = SQL::raw("SELECT name FROM users WHERE id = ")
            .append(drizzle_core::Placeholder::named("id").to_sql());

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
