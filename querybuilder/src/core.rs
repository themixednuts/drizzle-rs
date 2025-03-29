use crate::sqlite;
use drivers::SQLiteValue;
use paste::paste;
use std::{borrow::Cow, fmt::Display, ops::Add};

pub mod expressions;
pub mod schema_traits;
pub mod traits;

pub trait SQLParam: Display + Clone {}

// Core SQL struct that holds a SQL query and its parameters
// Cow<'a, str> allows for both static or dynamically generated SQL strings
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct SQL<'a, V: SQLParam>(pub(crate) Cow<'a, str>, pub(crate) Vec<V>);

impl<'a, V: SQLParam> SQL<'a, V> {
    // Create a new SQL query with parameters from a string and parameters
    pub fn new<S: Into<Cow<'a, str>>>(sql: S, params: Vec<V>) -> Self {
        Self(sql.into(), params)
    }

    // Get the SQL query string
    pub fn sql(&self) -> &str {
        self.0.as_ref()
    }

    // Get the parameters
    pub fn params(&self) -> &[V] {
        &self.1
    }

    // Consume the SQL and return the query string and parameters
    pub fn into_parts(self) -> (Cow<'a, str>, Vec<V>) {
        (self.0, self.1)
    }
}

// Implementation of ToSQL for SQL itself (identity function)
impl<'a, V: SQLParam> ToSQL<'a, V> for SQL<'a, V> {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL(self.0.clone(), self.1.clone())
    }
}

// ToSQL trait defines how to convert a value into an SQL expression
pub trait ToSQL<'a, V: SQLParam> {
    fn to_sql(&self) -> SQL<'a, V>;
}

// Implementation of ToSQL for string literals
impl<'a, V: SQLParam> ToSQL<'a, V> for &'a str {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL(Cow::Borrowed(*self), Vec::new())
    }
}

impl<'a, V: SQLParam> Display for SQL<'a, V> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}
// Trait for converting a value into a database parameter value
pub trait IntoValue<V> {
    fn into_value(self) -> V;
}

// Blanket implementation of IntoValue for any type that implements Into
impl<T, V> IntoValue<V> for T
where
    T: Into<V>,
{
    fn into_value(self) -> V {
        self.into()
    }
}

// Helper function to handle parameters in SQL queries
pub fn to_value<V, T: Into<V>>(value: T) -> V {
    value.into()
}

// SQL template for creating parameterized SQL queries
pub fn sql_template<'a, V: SQLParam>(template: &'a str, params: Vec<V>) -> SQL<'a, V> {
    SQL(Cow::Borrowed(template), params)
}

// Convenience macro for SQL expressions
#[macro_export]
macro_rules! sql {
    // Empty case
    ($sql:expr) => {
        $crate::core::SQL::new($sql, Vec::new())
    };

    // Basic case with parameters and auto conversion
    ($sql:expr, $($param:expr),*) => {{
        use $crate::core::SQL;
        let sql_with_placeholders = $sql.replace("{}", "?");

        // Use conditional compilation to support different database types
        #[cfg(feature = "sqlite")]
        {
            use $crate::sqlite::common::SQLiteValue;

            // Convert each parameter to SQLiteValue individually
            let params: Vec<SQLiteValue> = vec![
                $(SQLiteValue::from($param)),*
            ];

            SQL::new(sql_with_placeholders, params)
        }

        #[cfg(all(feature = "postgres", not(feature = "sqlite")))]
        {
            use $crate::postgres::common::PostgresValue;

            // Convert each parameter to PostgresValue individually
            let params: Vec<PostgresValue> = vec![
                $(PostgresValue::from($param)),*
            ];

            SQL::new(sql_with_placeholders, params)
        }

        #[cfg(all(feature = "mysql", not(feature = "sqlite"), not(feature = "postgres")))]
        {
            use $crate::mysql::common::MySQLValue;

            // Convert each parameter to MySQLValue individually
            let params: Vec<MySQLValue> = vec![
                $(MySQLValue::from($param)),*
            ];

            SQL::new(sql_with_placeholders, params)
        }

        // Fallback for builds with no database feature enabled
        #[cfg(not(any(feature = "sqlite", feature = "postgres", feature = "mysql")))]
        {
            compile_error!("No database feature enabled. Please enable at least one of: sqlite, postgres, mysql");
        }
    }};

    // Helper to count parameters
    (@count $param:expr) => { 1 };
    (@count $param:expr, $($rest:expr),+) => { 1 + sql!(@count $($rest),+) };
}

// Raw SQL expression (no escaping)
#[macro_export]
macro_rules! raw {
    ($value:expr) => {
        $crate::core::SQL::new($value.to_string(), Vec::new())
    };
}

// Join SQL fragments with a separator
#[macro_export]
macro_rules! join {
    ($sep:expr, $($sql:expr),*) => {{
        let fragments = vec![$($sql),*];
        let joined = fragments.iter()
            .map(|f| f.sql())
            .collect::<Vec<_>>()
            .join($sep);
        let mut params = Vec::new();
        for f in fragments {
            params.extend(f.params().to_owned());
        }
        $crate::core::SQL(std::borrow::Cow::Owned(joined), params)
    }};
}

// Placeholder style for parameters
/// Defines different styles for SQL parameter placeholders across different dialects.
///
/// SQL dialects use different syntax for placeholders in prepared statements:
///
/// * SQLite supports `:name`, `@name`, `$name`, and `?N`
/// * PostgreSQL uses `$1`, `$2`, etc.
/// * MySQL uses `?` (anonymous) or `:name` (named)
///
/// This enum allows switching between these styles as needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum PlaceholderStyle {
    /// Named parameters with colon prefix: `:name`
    Colon,
    /// Named parameters with @ prefix: `@name`
    AtSign,
    /// Named parameters with $ prefix: `$name`
    Dollar,
    /// Indexed parameters: `?1`, `?2`
    QuestionIndex,
    /// Positional parameters (PostgreSQL style): `$1`, `$2`
    DollarIndex,
}

impl Default for PlaceholderStyle {
    fn default() -> Self {
        // Default to colon style for backward compatibility
        PlaceholderStyle::Colon
    }
}

/// Represents a named or positional parameter placeholder in SQL.
///
/// Placeholder handles different SQL dialects' parameter styles, including:
/// * Colon style (`:name`)
/// * At-sign style (`@name`)
/// * Dollar style (`$name`)
/// * Question mark with index (`?1`, `?2`)
/// * PostgreSQL-style positional parameters (`$1`, `$2`)
///
/// Use the `placeholder!` macro for easy creation with automatic style detection.
///
/// # Examples
///
/// ```
/// # use querybuilder::core::{Placeholder, PlaceholderStyle};
///
/// // Create a named parameter with colon style
/// let id_param = Placeholder::with_style("id", PlaceholderStyle::Colon);
///
/// // Create a parameter with PostgreSQL positional style
/// let pos_param = Placeholder::with_style("1", PlaceholderStyle::DollarIndex);
///
/// // For positional parameters, use convenience methods
/// let q_param = Placeholder::question_index(3); // Creates ?3
/// let pg_param = Placeholder::pg_index(4);      // Creates $4
/// ```
#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub struct Placeholder<'a> {
    pub name: &'a str,
    pub style: PlaceholderStyle,
}

impl<'a> Placeholder<'a> {
    /// Creates a new placeholder with the default style (Colon).
    ///
    /// # Arguments
    /// * `name` - The parameter name (without any prefix)
    ///
    /// # Returns
    /// A placeholder that will format as `:name` in SQL
    pub const fn new(name: &'a str) -> Self {
        Self {
            name,
            style: PlaceholderStyle::Colon, // Default for backward compatibility
        }
    }

    /// Creates a placeholder with a specific style.
    ///
    /// # Arguments
    /// * `name` - The parameter name (without any prefix)
    /// * `style` - The placeholder style to use
    ///
    /// # Returns
    /// A placeholder that will format according to the specified style
    pub const fn with_style(name: &'a str, style: PlaceholderStyle) -> Self {
        Self { name, style }
    }

    /// Creates a placeholder with PostgreSQL style ($1, $2)
    ///
    /// This is a convenience method for creating positional parameters
    /// commonly used in PostgreSQL queries.
    ///
    /// # Arguments
    /// * `index` - The parameter index (1-based)
    ///
    /// # Returns
    /// A placeholder that will format as `$index` in SQL
    ///
    /// # Example
    /// ```
    /// use querybuilder::core::{Placeholder, SQL, ToSQL};
    ///
    /// // Define a test parameter type for the example
    /// #[derive(Debug, Clone)]
    /// struct TestParam(String);
    /// impl querybuilder::core::SQLParam for TestParam {}
    /// impl std::fmt::Display for TestParam {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "{}", self.0)
    ///     }
    /// }
    ///
    /// let pg_param = Placeholder::pg_index(5);
    /// let sql: SQL<TestParam> = pg_param.to_sql();
    /// assert_eq!(sql.sql(), "$5");
    /// ```
    pub fn pg_index(index: usize) -> Placeholder<'static> {
        // Convert index to static string - this approach avoids allocation
        // for all possible index values and is more maintainable
        let index_str = Box::leak(index.to_string().into_boxed_str());

        Placeholder {
            name: index_str,
            style: PlaceholderStyle::DollarIndex,
        }
    }

    /// Creates a placeholder with question mark style (?1, ?2)
    ///
    /// This is a convenience method for creating positional parameters
    /// commonly used in SQLite queries.
    ///
    /// # Arguments
    /// * `index` - The parameter index (1-based)
    ///
    /// # Returns
    /// A placeholder that will format as `?index` in SQL
    ///
    /// # Example
    /// ```
    /// use querybuilder::core::{Placeholder, SQL, ToSQL};
    ///
    /// // Define a test parameter type for the example
    /// #[derive(Debug, Clone)]
    /// struct TestParam(String);
    /// impl querybuilder::core::SQLParam for TestParam {}
    /// impl std::fmt::Display for TestParam {
    ///     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    ///         write!(f, "{}", self.0)
    ///     }
    /// }
    ///
    /// let q_param = Placeholder::question_index(3);
    /// let sql: SQL<TestParam> = q_param.to_sql();
    /// assert_eq!(sql.sql(), "?3");
    /// ```
    pub fn question_index(index: usize) -> Placeholder<'static> {
        // Convert index to static string - this approach avoids allocation
        // for all possible index values and is more maintainable
        let index_str = Box::leak(index.to_string().into_boxed_str());

        Placeholder {
            name: index_str,
            style: PlaceholderStyle::QuestionIndex,
        }
    }
}

impl<'a> Display for Placeholder<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.style {
            PlaceholderStyle::Colon => write!(f, ":{}", self.name),
            PlaceholderStyle::AtSign => write!(f, "@{}", self.name),
            PlaceholderStyle::Dollar => write!(f, "${}", self.name),
            PlaceholderStyle::QuestionIndex => {
                // For numeric indices
                if let Ok(idx) = self.name.parse::<usize>() {
                    write!(f, "?{}", idx)
                } else {
                    // Fall back to default format if it's not a valid index
                    write!(f, "?{}", self.name)
                }
            }
            PlaceholderStyle::DollarIndex => {
                // For PostgreSQL style positional parameters
                if let Ok(idx) = self.name.parse::<usize>() {
                    write!(f, "${}", idx)
                } else {
                    // Fall back to default format if it's not a valid index
                    write!(f, "${}", self.name)
                }
            }
        }
    }
}

impl<'a, V: SQLParam> ToSQL<'a, V> for Placeholder<'a> {
    fn to_sql(&self) -> SQL<'a, V> {
        let sql = match self.style {
            PlaceholderStyle::Colon => format!(":{}", self.name),
            PlaceholderStyle::AtSign => format!("@{}", self.name),
            PlaceholderStyle::Dollar => format!("${}", self.name),
            PlaceholderStyle::QuestionIndex => {
                if let Ok(idx) = self.name.parse::<usize>() {
                    format!("?{}", idx)
                } else {
                    format!("?{}", self.name)
                }
            }
            PlaceholderStyle::DollarIndex => {
                if let Ok(idx) = self.name.parse::<usize>() {
                    format!("${}", idx)
                } else {
                    format!("${}", self.name)
                }
            }
        };
        SQL(sql.into(), vec![])
    }
}

/// Create a placeholder with automatic style detection.
///
/// This macro automatically detects the placeholder style based on the input prefix:
/// - `:name` -> Colon style
/// - `@name` -> AtSign style
/// - `$name` -> Dollar style
/// - `?1`, `?2`, etc. -> QuestionIndex style
/// - `$1`, `$2`, etc. -> DollarIndex style (PostgreSQL)
/// - Any other string -> Defaults to Colon style
///
/// # Examples
///
/// ```
/// use querybuilder::{placeholder, core::{SQL, ToSQL}};
///
/// // Different styles are automatically detected from strings
/// let name = placeholder!(":name");     // Colon style
/// let email = placeholder!("@email");   // AtSign style
/// let value = placeholder!("$value");   // Dollar style
/// let idx1 = placeholder!("?1");        // QuestionIndex style
/// let idx2 = placeholder!("$2");        // DollarIndex style (PostgreSQL)
/// let default = placeholder!("id");     // Default (Colon style)
/// ```
#[macro_export]
macro_rules! placeholder {
    // Original string-based detection
    ($name:expr) => {{
        use $crate::core::{Placeholder, PlaceholderStyle};

        let input = $name;

        if input.starts_with(':') {
            Placeholder::with_style(&input[1..], PlaceholderStyle::Colon)
        } else if input.starts_with('@') {
            Placeholder::with_style(&input[1..], PlaceholderStyle::AtSign)
        } else if input.starts_with('$') {
            let remainder = &input[1..];
            if remainder.chars().all(|c| c.is_ascii_digit()) {
                // If it's all digits after $, treat as PostgreSQL positional parameter
                Placeholder::with_style(remainder, PlaceholderStyle::DollarIndex)
            } else {
                // Otherwise, treat as Dollar style named parameter
                Placeholder::with_style(remainder, PlaceholderStyle::Dollar)
            }
        } else if input.starts_with('?') {
            let remainder = &input[1..];
            if remainder.chars().all(|c| c.is_ascii_digit()) {
                // If it's all digits after ?, treat as question mark positional parameter
                Placeholder::with_style(remainder, PlaceholderStyle::QuestionIndex)
            } else {
                // Otherwise, just use the input as is with default style
                Placeholder::new(input)
            }
        } else {
            // Default to colon style
            Placeholder::new(input)
        }
    }};

    // Original version with explicit style parameter
    ($name:expr, $style:expr) => {
        $crate::core::Placeholder::with_style($name, $style)
    };
}

// SQL utilities
pub mod sql_utils {
    use super::*;

    // Creates an identifier SQL
    pub fn ident<'a, V: SQLParam>(name: &'a str) -> SQL<'a, V> {
        SQL(Cow::Borrowed(name), Vec::new())
    }

    // Creates a parameter with placeholder
    pub fn param<'a, V: SQLParam>(value: V) -> SQL<'a, V> {
        SQL(Cow::Borrowed("?"), vec![value])
    }

    // Converts a value to the appropriate SQLiteValue type
    // This is used by the sql! macro for string interpolation
    #[cfg(feature = "sqlite")]
    pub fn convert_to_value<T>(value: T) -> drivers::SQLiteValue<'static>
    where
        T: Into<drivers::SQLiteValue<'static>>,
    {
        value.into()
    }
}

// ToSQL implementations for common types

// Implementation for i64
impl<'a, V: SQLParam + From<i64>> ToSQL<'a, V> for i64 {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL(Cow::Borrowed("?"), vec![(*self).into()])
    }
}

// Implementation for String
impl<'a, V: SQLParam + From<String>> ToSQL<'a, V> for String {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL(Cow::Borrowed("?"), vec![self.clone().into()])
    }
}

// Implementation for Option<T> where T: ToSQL<'a, V>
impl<'a, T, V: SQLParam> ToSQL<'a, V> for Option<T>
where
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match self {
            Some(value) => value.to_sql(),
            None => SQL(Cow::Borrowed("NULL"), Vec::new()),
        }
    }
}

// UUID support
#[cfg(feature = "uuid")]
impl<'a, V: SQLParam + From<uuid::Uuid>> ToSQL<'a, V> for uuid::Uuid {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL(Cow::Borrowed("?"), vec![(*self).into()])
    }
}

#[cfg(feature = "uuid")]
impl<'a, V: SQLParam + From<uuid::Uuid>> ToSQL<'a, V> for &uuid::Uuid {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL(Cow::Borrowed("?"), vec![(**self).into()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct TestParam(String);

    impl SQLParam for TestParam {}

    impl Display for TestParam {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    #[test]
    fn test_basic_sql_creation() {
        let sql: SQL<TestParam> = SQL(Cow::Owned("SELECT * FROM users".to_string()), vec![]);
        assert_eq!(sql.0, "SELECT * FROM users");
        assert_eq!(sql.1.len(), 0);
    }

    #[test]
    fn test_sql_combine() {
        let sql1: SQL<TestParam> =
            SQL(Cow::Owned("SELECT * FROM users WHERE ".to_string()), vec![]);
        let sql2: SQL<TestParam> = SQL(
            Cow::Owned("id = ?".to_string()),
            vec![TestParam("1".to_string())],
        );

        // Use the Add trait
        let combined = sql1 + sql2;

        assert_eq!(combined.0, "SELECT * FROM users WHERE id = ?");
        assert_eq!(combined.1.len(), 1);
    }

    #[test]
    fn test_placeholder_style_auto_detection() {
        // Colon style
        let p1 = placeholder!(":name");
        let sql1: SQL<TestParam> = p1.to_sql();
        assert_eq!(sql1.0.as_ref(), ":name");

        // AtSign style
        let p2 = placeholder!("@email");
        let sql2: SQL<TestParam> = p2.to_sql();
        assert_eq!(sql2.0.as_ref(), "@email");

        // Dollar style
        let p3 = placeholder!("$value");
        let sql3: SQL<TestParam> = p3.to_sql();
        assert_eq!(sql3.0.as_ref(), "$value");

        // QuestionIndex style
        let p4 = placeholder!("?1");
        let sql4: SQL<TestParam> = p4.to_sql();
        assert_eq!(sql4.0.as_ref(), "?1");

        // DollarIndex style (PostgreSQL)
        let p5 = placeholder!("$2");
        let sql5: SQL<TestParam> = p5.to_sql();
        assert_eq!(sql5.0.as_ref(), "$2");

        // Default to Colon style
        let p6 = placeholder!("id");
        let sql6: SQL<TestParam> = p6.to_sql();
        assert_eq!(sql6.0.as_ref(), ":id");
    }

    #[test]
    fn test_placeholder_with_explicit_style() {
        // Explicit style specification
        let p1 = placeholder!("name", PlaceholderStyle::AtSign);
        let sql1: SQL<TestParam> = p1.to_sql();
        assert_eq!(sql1.0.as_ref(), "@name");

        let p2 = placeholder!("value", PlaceholderStyle::Dollar);
        let sql2: SQL<TestParam> = p2.to_sql();
        assert_eq!(sql2.0.as_ref(), "$value");

        let p3 = placeholder!("1", PlaceholderStyle::QuestionIndex);
        let sql3: SQL<TestParam> = p3.to_sql();
        assert_eq!(sql3.0.as_ref(), "?1");

        let p4 = placeholder!("2", PlaceholderStyle::DollarIndex);
        let sql4: SQL<TestParam> = p4.to_sql();
        assert_eq!(sql4.0.as_ref(), "$2");
    }
}

/// Helper functions for creating dialect-specific placeholders
///
/// This module provides specialized functions for creating placeholder values
/// with different styles for various SQL dialects.
///
/// # Examples
///
/// ```
/// use querybuilder::core::placeholders;
/// use querybuilder::core::{Placeholder, SQL, ToSQL};
///
/// // Create different placeholder types
/// let name_param = placeholders::colon("name");                // :name
/// let email_param = placeholders::at_sign("email");            // @email
/// let value_param = placeholders::dollar("value");             // $value
///
/// // Positional parameters
/// let q_param = placeholders::question_index::<TestParam>(2);    // ?2
/// let pg_param = placeholders::postgres_index::<TestParam>(3);   // $3
///
/// # struct TestParam(String);
/// # impl querybuilder::core::SQLParam for TestParam {}
/// # impl std::fmt::Display for TestParam {
/// #     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
/// #         write!(f, "{}", self.0)
/// #     }
/// # }
/// # impl Clone for TestParam {
/// #     fn clone(&self) -> Self {
/// #         Self(self.0.clone())
/// #     }
/// # }
/// ```
pub mod placeholders {
    use super::{Placeholder, PlaceholderStyle, SQL, SQLParam};
    use std::borrow::Cow;

    /// Create a placeholder with colon style (:name)
    pub fn colon(name: &str) -> Placeholder<'_> {
        Placeholder::with_style(name, PlaceholderStyle::Colon)
    }

    /// Create a placeholder with at-sign style (@name)
    pub fn at_sign(name: &str) -> Placeholder<'_> {
        Placeholder::with_style(name, PlaceholderStyle::AtSign)
    }

    /// Create a placeholder with dollar style ($name)
    pub fn dollar(name: &str) -> Placeholder<'_> {
        Placeholder::with_style(name, PlaceholderStyle::Dollar)
    }

    /// Create a placeholder with question mark and index style (?N)
    /// Returns formatted SQL directly to avoid lifetime issues
    pub fn question_index<'a, V: SQLParam>(index: usize) -> SQL<'a, V> {
        // Directly format with the index - simpler and works for all index values
        SQL(Cow::Owned(format!("?{}", index)), vec![])
    }

    /// Create a placeholder with PostgreSQL dollar and index style ($N)
    /// Returns formatted SQL directly to avoid lifetime issues
    pub fn postgres_index<'a, V: SQLParam>(index: usize) -> SQL<'a, V> {
        // Directly format with the index - simpler and works for all index values
        SQL(Cow::Owned(format!("${}", index)), vec![])
    }
}

// Implement Add trait for SQL
impl<'a, V: SQLParam> Add for SQL<'a, V> {
    type Output = SQL<'a, V>;

    fn add(self, rhs: Self) -> Self::Output {
        SQL(Cow::Owned(format!("{}{}", self.0, rhs.0)), {
            let mut params = self.1;
            params.extend(rhs.1);
            params
        })
    }
}
