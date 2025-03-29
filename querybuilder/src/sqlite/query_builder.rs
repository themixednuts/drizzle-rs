use crate::prelude::*;
use crate::sqlite::common::SQLiteTableType;
use drivers::SQLiteValue;
use std::borrow::Cow;
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};

/// SQLite query builder factory that creates QueryBuilder instances for specific tables
/// This is the main entry point for building SQL queries using Drizzle-style API
pub struct SQLiteQueryBuilder<'a, S: Clone> {
    _schema: PhantomData<S>,
    _lifetime: PhantomData<&'a ()>,
    inner: QueryBuilder<'a, S>,
}

impl<'a, S: Clone> Deref for SQLiteQueryBuilder<'a, S> {
    type Target = QueryBuilder<'a, S>;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl<'a, S: Clone> DerefMut for SQLiteQueryBuilder<'a, S> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl<'a, S: Clone> SQLiteQueryBuilder<'a, S> {
    /// Create a new QueryBuilder factory.
    pub fn new() -> Self {
        Self {
            _schema: PhantomData,
            _lifetime: PhantomData,
            inner: QueryBuilder::new(),
        }
    }

    /// Creates a new query builder instance bound to this schema.
    pub fn query(&self) -> QueryBuilder<'a, S> {
        QueryBuilder::new()
    }

    pub fn schema() -> Self {
        Self {
            _schema: PhantomData,
            _lifetime: PhantomData,
            inner: QueryBuilder::new(),
        }
    }
}

/// Constructor with a schema type parameter
pub fn query_builder<'a, S: Clone>() -> SQLiteQueryBuilder<'a, S> {
    SQLiteQueryBuilder::new()
}

/// Helper function to create a QueryBuilder with a specific schema
pub fn schema<'a, S: Clone>() -> SQLiteQueryBuilder<'a, S> {
    SQLiteQueryBuilder::new()
}

/// Query builder operating within a specific schema `S`.
#[derive(Clone)]
pub struct QueryBuilder<'a, S: Clone> {
    _schema_phantom: PhantomData<S>,
    primary_table_name: Option<String>,
    joins: Vec<JoinClause<'a>>,
    where_clauses: Vec<Option<SQL<'a, SQLiteValue<'a>>>>,
    order_by: Vec<OrderByClause<'a>>,
    limit: Option<usize>,
    offset: Option<usize>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, S: Clone> QueryBuilder<'a, S> {
    /// Create a new query builder
    pub fn new() -> Self {
        Self {
            _schema_phantom: PhantomData,
            primary_table_name: None,
            joins: Vec::new(),
            where_clauses: Vec::new(),
            order_by: Vec::new(),
            limit: None,
            offset: None,
            _marker: PhantomData,
        }
    }

    /// Sets the primary table `T` for the query.
    pub fn from<T: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>>(&mut self) -> &mut Self
    where
        T: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>,
    {
        self.primary_table_name = Some(T::NAME.to_string());
        self
    }

    /// Build a SELECT query
    pub fn select<C: Into<Columns<'a>>>(&self, columns: C) -> Select<'a, S> {
        // TODO: Check if primary_table_name is set
        Select::new(self, columns.into())
    }

    /// Build a SELECT * query
    pub fn select_all(&self) -> Select<'a, S> {
        // TODO: Check if primary_table_name is set
        Select::new(self, Columns::All)
    }

    /// Build an INSERT query
    pub fn insert(&self) -> InsertBuilder<'a, S> {
        // TODO: Check if primary_table_name is set
        InsertBuilder::new(self)
    }

    /// Build an UPDATE query
    pub fn update(&self) -> UpdateBuilder<'a, S> {
        // TODO: Check if primary_table_name is set
        UpdateBuilder::new(self)
    }

    /// Build a DELETE query
    pub fn delete(&self) -> DeleteBuilder<'a, S> {
        // TODO: Check if primary_table_name is set
        DeleteBuilder::new(self)
    }

    /// Add a WHERE clause
    ///
    /// This method accepts any SQL condition, including those created with the and() or or()
    /// functions to create complex conditions.
    ///
    /// # Examples
    ///
    /// ```
    /// use querybuilder::prelude::*;
    /// use querybuilder::sqlite::query_builder::QueryBuilder;
    /// use querybuilder::sqlite::common::{SQLiteTableSchema, SQLiteValue};
    ///
    /// #[derive(Clone)]
    /// struct Users;
    /// impl SQLiteTableSchema for Users {
    ///     const NAME: &'static str = "users";
    ///     const TYPE: querybuilder::sqlite::common::SQLiteTableType =
    ///         querybuilder::sqlite::common::SQLiteTableType::Table;
    ///     const SQL: &'static str = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);";
    /// }
    ///
    /// // Simple condition
    /// let mut query = QueryBuilder::<Users>::new();
    /// query.r#where(eq("id", 1));
    ///
    /// // Complex AND condition
    /// let mut query = QueryBuilder::<Users>::new();
    /// let name_condition = eq("name", "John");
    /// let age_condition = eq("age", 30);
    /// let conditions = [name_condition, age_condition];
    /// let condition: SQL<SQLiteValue> = and(&conditions);
    /// query.r#where(condition);
    ///
    /// // Complex OR condition
    /// let mut query = QueryBuilder::<Users>::new();
    /// let name_john = eq("name", "John");
    /// let name_jane = eq("name", "Jane");
    /// let conditions = [name_john, name_jane];
    /// let condition: SQL<SQLiteValue> = or(&conditions);
    /// query.r#where(condition);
    ///
    /// // Nested conditions
    /// let mut query = QueryBuilder::<Users>::new();
    /// // Create an active condition where value is 1 (true)
    /// let active_cond = eq("active", 1);
    /// // Create a role condition
    /// let admin_role = eq("role", "admin");
    /// let mod_role = eq("role", "moderator");
    /// let role_conditions = [admin_role, mod_role];
    /// let role_cond: SQL<SQLiteValue> = or(&role_conditions);
    /// // Combine them with AND
    /// let all_conditions = [active_cond, role_cond];
    /// let combined: SQL<SQLiteValue> = and(&all_conditions);
    /// query.r#where(combined);
    /// ```
    ///
    /// Using macros is even simpler:
    ///
    /// ```
    /// use querybuilder::prelude::*;
    /// use querybuilder::sqlite::query_builder::QueryBuilder;
    /// use querybuilder::sqlite::common::{SQLiteTableSchema, SQLiteValue};
    ///
    /// #[derive(Clone)]
    /// struct Users;
    /// impl SQLiteTableSchema for Users {
    ///     const NAME: &'static str = "users";
    ///     const TYPE: querybuilder::sqlite::common::SQLiteTableType =
    ///         querybuilder::sqlite::common::SQLiteTableType::Table;
    ///     const SQL: &'static str = "CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT);";
    /// }
    ///
    /// let mut query = QueryBuilder::<Users>::new();
    /// let condition = and!(
    ///     eq("active", 1),
    ///     or!(
    ///         eq("role", "admin"),
    ///         eq("role", "moderator")
    ///     )
    /// );
    /// query.r#where(condition);
    /// ```
    pub fn r#where(&mut self, condition: SQL<'a, SQLiteValue<'a>>) -> &mut Self {
        self.where_clauses.push(Some(condition));
        self
    }

    /// Add an ORDER BY clause
    pub fn order_by<C: ToSQL<'a, SQLiteValue<'a>>>(
        &mut self,
        column: C,
        direction: SortDirection,
    ) -> &mut Self {
        self.order_by.push(OrderByClause {
            column: column.to_sql(),
            direction,
        });
        self
    }

    /// Add a LIMIT clause
    pub fn limit(&mut self, limit: usize) -> &mut Self {
        self.limit = Some(limit);
        self
    }

    /// Add an OFFSET clause
    pub fn offset(&mut self, offset: usize) -> &mut Self {
        self.offset = Some(offset);
        self
    }

    /// Build the WHERE clause string
    fn build_where_clause(&self) -> (String, Vec<SQLiteValue<'a>>) {
        let mut sql = String::new();
        let mut params = Vec::new();
        let mut first_clause = true;

        for condition_opt in &self.where_clauses {
            if let Some(condition) = condition_opt {
                // Check if the Option is Some
                if first_clause {
                    sql.push_str(" WHERE ");
                    first_clause = false;
                } else {
                    sql.push_str(" AND ");
                }
                // Destructure the SQL struct from the Some variant
                let SQL(condition_sql, mut condition_params) = condition.clone(); // Clone the SQL struct
                sql.push('(');
                sql.push_str(condition_sql.as_ref()); // Borrow Cow as &str
                sql.push(')');
                params.append(&mut condition_params);
            }
        }

        (sql, params)
    }

    /// Build the ORDER BY clause string
    fn build_order_by_clause(&self) -> (String, Vec<SQLiteValue<'a>>) {
        if self.order_by.is_empty() {
            return (String::new(), Vec::new());
        }

        let mut sql = String::from(" ORDER BY ");
        let mut params = Vec::new();

        for (i, order_by) in self.order_by.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }

            let SQL(column_sql, column_params) = &order_by.column; // This is already SQL, no need for to_sql()
            sql.push_str(column_sql.as_ref()); // Borrow Cow as &str
            sql.push_str(match order_by.direction {
                SortDirection::Asc => " ASC",
                SortDirection::Desc => " DESC",
            });
            // Params for order by columns might exist (e.g., order by function call)
            params.extend(column_params.clone());
        }

        (sql, params)
    }

    /// Build the LIMIT and OFFSET clause string
    fn build_limit_offset_clause(&self) -> (String, Vec<SQLiteValue<'a>>) {
        let mut sql = String::new();
        let mut params: Vec<SQLiteValue<'a>> = Vec::new();

        if let Some(limit) = self.limit {
            sql.push_str(" LIMIT ?");
            params.push(SQLiteValue::Integer(limit as i64));
        }

        if let Some(offset) = self.offset {
            sql.push_str(" OFFSET ?");
            params.push(SQLiteValue::Integer(offset as i64));
        }

        (sql, params)
    }

    /// Adds an INNER JOIN clause to the query.
    pub fn inner_join<J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>>(
        &mut self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> &mut Self
    where
        J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>,
    {
        self.joins.push(JoinClause {
            join_type: JoinType::Inner,
            table: J::NAME.to_string(),
            condition: Some(condition),
        });
        self
    }

    /// Adds a LEFT JOIN clause to the query.
    pub fn left_join<J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>>(
        &mut self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> &mut Self
    where
        J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>,
    {
        self.joins.push(JoinClause {
            join_type: JoinType::Left,
            table: J::NAME.to_string(),
            condition: Some(condition),
        });
        self
    }

    /// Adds a RIGHT JOIN clause to the query. (Note: SQLite has limited RIGHT JOIN support)
    pub fn right_join<J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>>(
        &mut self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> &mut Self
    where
        J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>,
    {
        // Add a comment warning about SQLite support if applicable
        // For now, just implement structurally
        self.joins.push(JoinClause {
            join_type: JoinType::Right,
            table: J::NAME.to_string(),
            condition: Some(condition),
        });
        self
    }

    /// Adds a FULL OUTER JOIN clause to the query. (Note: SQLite does not directly support FULL OUTER JOIN)
    pub fn full_join<J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>>(
        &mut self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> &mut Self
    where
        J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>,
    {
        // Add a comment warning about SQLite support if applicable
        // For now, just implement structurally
        self.joins.push(JoinClause {
            join_type: JoinType::Full,
            table: J::NAME.to_string(),
            condition: Some(condition),
        });
        self
    }

    /// Adds a CROSS JOIN clause to the query.
    pub fn cross_join<J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>>(&mut self) -> &mut Self
    where
        J: IsInSchema<S> + SQLSchema<'a, SQLiteTableType>,
    {
        self.joins.push(JoinClause {
            join_type: JoinType::Cross,
            table: J::NAME.to_string(),
            condition: None, // CROSS JOIN typically doesn't use ON
        });
        self
    }

    // Helper to get primary table name, panicking if not set
    fn get_primary_table_name(&self) -> &str {
        self.primary_table_name
            .as_deref()
            .expect("Primary table not set. Call .from::<Table>() first.")
    }
}

/// Columns for SELECT queries
#[derive(Clone, Default, Debug)]
pub enum Columns<'a> {
    #[default]
    All,
    List(Vec<SQL<'a, SQLiteValue<'a>>>),
}

impl<'a> Columns<'a> {
    /// Create a new Columns instance from an array of SQL expressions
    pub fn from_array<const N: usize>(columns: [SQL<'a, SQLiteValue<'a>>; N]) -> Self {
        Columns::List(columns.to_vec())
    }

    /// Convert a Columns object to a single SQL column expression
    /// This is used by the columns! macro for nested processing
    pub fn to_sql_column(&self) -> SQL<'a, SQLiteValue<'a>> {
        match self {
            Columns::All => SQL(Cow::Borrowed("*"), Vec::new()),
            Columns::List(columns) => {
                if columns.len() == 1 {
                    columns[0].clone()
                } else {
                    // For multiple columns, join them with commas
                    let (sql, params) = self.to_sql();
                    SQL(Cow::Owned(sql), params)
                }
            }
        }
    }

    fn to_sql(&self) -> (String, Vec<SQLiteValue<'a>>) {
        match self {
            Columns::All => ("*".to_string(), Vec::new()),
            Columns::List(columns) => {
                let mut sql = String::new();
                let mut params = Vec::new();

                for (i, column) in columns.iter().enumerate() {
                    if i > 0 {
                        sql.push_str(", ");
                    }

                    let SQL(column_sql, column_params) = column;
                    sql.push_str(column_sql.as_ref());
                    params.extend(column_params.clone());
                }

                (sql, params)
            }
        }
    }
}

impl<'a, C: ToSQL<'a, SQLiteValue<'a>>> From<Vec<C>> for Columns<'a> {
    fn from(columns: Vec<C>) -> Self {
        Columns::List(columns.into_iter().map(|c| c.to_sql()).collect())
    }
}

impl<'a, C: ToSQL<'a, SQLiteValue<'a>>, const N: usize> From<[C; N]> for Columns<'a> {
    fn from(columns: [C; N]) -> Self {
        let sql_columns: Vec<SQL<'a, SQLiteValue<'a>>> =
            columns.into_iter().map(|c| c.to_sql()).collect();
        Columns::List(sql_columns)
    }
}

// Instead, let's extend the Columns struct with methods to handle SQLiteColumn specifically
impl<'a> Columns<'a> {
    /// Create a Columns instance from a single SQLiteColumn
    pub fn from_sqlite_column<T: crate::sqlite::IntoSQLiteValue<'a>, Tbl>(
        column: crate::sqlite::SQLiteColumn<'a, T, Tbl>,
    ) -> Self
    where
        Tbl: Clone,
    {
        Columns::List(vec![column.to_sql()])
    }

    /// Create a Columns instance from an array of SQLiteColumns
    pub fn from_sqlite_columns<T: crate::sqlite::IntoSQLiteValue<'a>, Tbl, const N: usize>(
        columns: [crate::sqlite::SQLiteColumn<'a, T, Tbl>; N],
    ) -> Self
    where
        Tbl: Clone,
    {
        let sql_columns: Vec<SQL<'a, SQLiteValue<'a>>> =
            columns.into_iter().map(|c| c.to_sql()).collect();

        Columns::List(sql_columns)
    }
}

/// Join types for SQL joins
#[derive(Clone)]
pub enum JoinType {
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

impl JoinType {
    fn to_sql(&self) -> &'static str {
        match self {
            JoinType::Inner => "INNER JOIN",
            JoinType::Left => "LEFT JOIN",
            JoinType::Right => "RIGHT JOIN",
            JoinType::Full => "FULL JOIN",
            JoinType::Cross => "CROSS JOIN",
        }
    }
}

/// JOIN clause for SQL queries
#[derive(Clone)]
pub struct JoinClause<'a> {
    join_type: JoinType,
    table: String,
    condition: Option<SQL<'a, SQLiteValue<'a>>>,
}

/// Sort direction for ORDER BY clauses
#[derive(Clone, Copy)]
pub enum SortDirection {
    Asc,
    Desc,
}

/// ORDER BY clause for SQL queries
#[derive(Clone)]
struct OrderByClause<'a> {
    column: SQL<'a, SQLiteValue<'a>>,
    direction: SortDirection,
}

/// SELECT query builder for untyped results
#[derive(Clone)]
pub struct Select<'a, S: Clone> {
    query_builder: QueryBuilder<'a, S>,
    columns: Columns<'a>,
}

impl<'a, S: Clone> Select<'a, S> {
    /// Create a new SELECT builder
    fn new(query_builder: &QueryBuilder<'a, S>, columns: Columns<'a>) -> Self {
        Self {
            query_builder: query_builder.clone(),
            columns,
        }
    }

    /// Add a WHERE clause
    pub fn r#where(mut self, condition: SQL<'a, SQLiteValue<'a>>) -> Self {
        self.query_builder.r#where(condition);
        self
    }

    /// Build the SQL query
    pub fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        let mut sql = String::from("SELECT ");
        let mut params = Vec::new();

        // Add columns
        let (columns_sql, mut columns_params) = self.columns.to_sql();
        sql.push_str(&columns_sql);
        params.append(&mut columns_params);

        // Add FROM clause
        let table_name = self.query_builder.get_primary_table_name();
        sql.push_str(&format!(" FROM {}", table_name));

        // Add JOIN clauses if any
        for join in &self.query_builder.joins {
            sql.push_str(&format!(" {} {}", join.join_type.to_sql(), join.table));
            if let Some(condition) = &join.condition {
                let SQL(cond_sql, mut cond_params) = condition.clone(); // Clone the SQL struct
                sql.push_str(&format!(" ON {}", cond_sql.as_ref())); // Borrow Cow as &str inside format!
                params.append(&mut cond_params);
            }
        }

        // Add WHERE clause
        let (where_sql, mut where_params) = self.query_builder.build_where_clause();
        sql.push_str(&where_sql);
        params.append(&mut where_params);

        // Add ORDER BY clause
        let (order_by_sql, mut order_by_params) = self.query_builder.build_order_by_clause();
        sql.push_str(&order_by_sql);
        params.append(&mut order_by_params);

        // Add LIMIT and OFFSET clauses
        let (limit_offset_sql, mut limit_offset_params) =
            self.query_builder.build_limit_offset_clause();
        sql.push_str(&limit_offset_sql);
        params.append(&mut limit_offset_params);

        SQL(Cow::Owned(sql), params)
    }
}

/// Type-parameterized SELECT builder
#[derive(Clone)]
pub struct SelectBuilder<'a, T: SQLSchema<'a, SQLiteTableType> + Clone + 'a> {
    query_builder: QueryBuilder<'a, T>,
    columns: Columns<'a>,
}

impl<'a, T: SQLSchema<'a, SQLiteTableType> + Clone + 'a> SelectBuilder<'a, T> {
    /// Add a WHERE clause
    pub fn r#where(mut self, condition: SQL<'a, SQLiteValue<'a>>) -> Self {
        self.query_builder.r#where(condition);
        self
    }

    /// Build the SQL query
    pub fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        let mut sql = String::from("SELECT ");
        let mut params = Vec::new();

        // Add columns
        let (columns_sql, mut columns_params) = self.columns.to_sql();
        sql.push_str(&columns_sql);
        params.append(&mut columns_params);

        // Add FROM clause
        sql.push_str(&format!(" FROM {}", T::NAME));

        // Add JOIN clauses if any

        // Add WHERE clause
        let (where_sql, mut where_params) = self.query_builder.build_where_clause();
        sql.push_str(&where_sql);
        params.append(&mut where_params);

        // Add ORDER BY clause
        let (order_by_sql, mut order_by_params) = self.query_builder.build_order_by_clause();
        sql.push_str(&order_by_sql);
        params.append(&mut order_by_params);

        // Add LIMIT and OFFSET clauses
        let (limit_offset_sql, mut limit_offset_params) =
            self.query_builder.build_limit_offset_clause();
        sql.push_str(&limit_offset_sql);
        params.append(&mut limit_offset_params);

        SQL(Cow::Owned(sql), params)
    }

    /// Execute the query and return a typed result
    ///
    /// This method would normally interact with a database driver to execute the query
    /// and return the results as a collection of typed records.
    ///
    /// For the purpose of this example, we're just showing the interface.
    pub fn execute(&self) -> Result<Vec<T>, &'static str> {
        // In a real implementation, we would:
        // 1. Execute the SQL query
        // 2. Convert the results to the expected record type
        // 3. Return the collection

        // For now, we'll just return an error to indicate this is a placeholder
        Err("Not implemented: This method would execute the query and return typed results")
    }

    /// Get a single record from the query
    ///
    /// Returns the first record that matches the query, or None if no records match.
    pub fn get(&self) -> Result<Option<T>, &'static str> {
        // Similar to execute, but returns at most one record
        Err("Not implemented: This method would execute the query and return a single typed result")
    }
}

/// INSERT query builder
#[derive(Clone)]
pub struct InsertBuilder<'a, S: Clone> {
    query_builder: QueryBuilder<'a, S>,
    values: Vec<(String, SQLiteValue<'a>)>,
    returning_columns: Option<Columns<'a>>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, S: Clone> InsertBuilder<'a, S> {
    fn new(query_builder: &QueryBuilder<'a, S>) -> Self {
        Self {
            query_builder: query_builder.clone(),
            values: Vec::new(),
            returning_columns: None,
            _marker: PhantomData,
        }
    }

    /// Add a column-value pair to the INSERT query
    pub fn value<C: ToSQL<'a, SQLiteValue<'a>>, V: Into<SQLiteValue<'a>>>(
        mut self,
        column: C,
        value: V,
    ) -> Self {
        let sql_col = column.to_sql();
        // Assuming the column itself doesn't introduce parameters that need handling here.
        // We just need the column name string.
        self.values.push((sql_col.0.into_owned(), value.into()));
        self
    }

    /// Add multiple column-value pairs to the INSERT query
    /// Note: This method signature might need adjustment if Vec<(C,V)> is difficult with ToSQL
    /// Keeping it simple for now, might need separate method or macro later.
    pub fn values<C: ToSQL<'a, SQLiteValue<'a>>, V: Into<SQLiteValue<'a>>>(
        mut self,
        values: Vec<(C, V)>,
    ) -> Self {
        for (column, value) in values {
            let sql_col = column.to_sql();
            self.values.push((sql_col.0.into_owned(), value.into()));
        }
        self
    }

    /// Specify columns to return after the INSERT operation.
    pub fn returning(mut self, columns: impl Into<Columns<'a>>) -> Self {
        self.returning_columns = Some(columns.into());
        self
    }

    /// Build the SQL query
    pub fn to_sql(&self) -> SQL<'_, SQLiteValue<'_>> {
        if self.values.is_empty() || self.query_builder.primary_table_name.is_none() {
            return SQL(Cow::Borrowed(""), Vec::new());
        }

        let table_name = self.query_builder.get_primary_table_name();
        let mut columns = Vec::new();
        let mut placeholders = Vec::new();
        let mut params = Vec::new();

        for (column, value) in &self.values {
            columns.push(column.clone());
            placeholders.push("?".to_string());
            params.push(value.clone());
        }

        let mut sql = format!(
            "INSERT INTO {} ({}) VALUES ({})",
            table_name,
            columns.join(", "),
            placeholders.join(", ")
        );

        // Append RETURNING clause if specified
        if let Some(returning_cols) = &self.returning_columns {
            let (returning_sql, returning_params) = returning_cols.to_sql();
            // RETURNING clause does not add parameters itself
            if !returning_params.is_empty() {
                // This case should ideally not happen if columns are just names/all
                // Handle potentially complex expressions if needed, though returning often uses simple columns.
                eprintln!("Warning: Parameters in RETURNING clause are currently ignored.");
            }
            sql.push_str(&format!(" RETURNING {}", returning_sql));
        }

        SQL(Cow::Owned(sql), params)
    }
}

/// UPDATE query builder
#[derive(Clone)]
pub struct UpdateBuilder<'a, S: Clone> {
    query_builder: QueryBuilder<'a, S>,
    values: Vec<(String, SQLiteValue<'a>)>,
    returning_columns: Option<Columns<'a>>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, S: Clone> UpdateBuilder<'a, S> {
    fn new(query_builder: &QueryBuilder<'a, S>) -> Self {
        Self {
            query_builder: query_builder.clone(),
            values: Vec::new(),
            returning_columns: None,
            _marker: PhantomData,
        }
    }

    /// Set a column to a value in the UPDATE query
    pub fn set<C: ToSQL<'a, SQLiteValue<'a>>, V: Into<SQLiteValue<'a>>>(
        mut self,
        column: C,
        value: V,
    ) -> Self {
        let sql_col = column.to_sql();
        // Assuming the column itself doesn't introduce parameters that need handling here.
        self.values.push((sql_col.0.into_owned(), value.into()));
        self
    }

    /// Add a WHERE clause
    pub fn r#where(mut self, condition: SQL<'a, SQLiteValue<'a>>) -> Self {
        self.query_builder.r#where(condition);
        self
    }

    /// Specify columns to return after the UPDATE operation.
    pub fn returning(mut self, columns: impl Into<Columns<'a>>) -> Self {
        self.returning_columns = Some(columns.into());
        self
    }

    /// Build the SQL query
    pub fn to_sql(&self) -> SQL<'_, SQLiteValue<'_>> {
        if self.values.is_empty() || self.query_builder.primary_table_name.is_none() {
            return SQL(Cow::Owned(String::new()), Vec::new());
        }

        let table_name = self.query_builder.get_primary_table_name();
        let mut sql = format!("UPDATE {} SET ", table_name);
        let mut params = Vec::new();

        // Add SET values
        for (i, (column, value)) in self.values.iter().enumerate() {
            if i > 0 {
                sql.push_str(", ");
            }
            sql.push_str(&format!("{} = ?", column));
            params.push(value.clone());
        }

        // Add WHERE clause
        let (where_sql, mut where_params) = self.query_builder.build_where_clause();
        sql.push_str(&where_sql);
        params.append(&mut where_params);

        // Append RETURNING clause if specified
        if let Some(returning_cols) = &self.returning_columns {
            let (returning_sql, returning_params) = returning_cols.to_sql();
            if !returning_params.is_empty() {
                eprintln!("Warning: Parameters in RETURNING clause are currently ignored.");
            }
            sql.push_str(&format!(" RETURNING {}", returning_sql));
        }

        SQL(Cow::Owned(sql), params)
    }
}

/// DELETE query builder
#[derive(Clone)]
pub struct DeleteBuilder<'a, S: Clone> {
    query_builder: QueryBuilder<'a, S>,
    returning_columns: Option<Columns<'a>>,
    _marker: PhantomData<&'a ()>,
}

impl<'a, S: Clone> DeleteBuilder<'a, S> {
    fn new(query_builder: &QueryBuilder<'a, S>) -> Self {
        Self {
            query_builder: query_builder.clone(),
            returning_columns: None,
            _marker: PhantomData,
        }
    }

    /// Add a WHERE clause
    pub fn r#where(mut self, condition: SQL<'a, SQLiteValue<'a>>) -> Self {
        self.query_builder.r#where(condition);
        self
    }

    /// Specify columns to return after the DELETE operation.
    pub fn returning(mut self, columns: impl Into<Columns<'a>>) -> Self {
        self.returning_columns = Some(columns.into());
        self
    }

    /// Build the SQL query
    pub fn to_sql(&self) -> SQL<'_, SQLiteValue<'_>> {
        let table_name = self.query_builder.get_primary_table_name();
        let mut sql = format!("DELETE FROM {}", table_name);
        let mut params = Vec::new();

        // Add WHERE clause
        let (where_sql, mut where_params) = self.query_builder.build_where_clause();
        sql.push_str(&where_sql);
        params.append(&mut where_params);

        // Append RETURNING clause if specified
        if let Some(returning_cols) = &self.returning_columns {
            let (returning_sql, returning_params) = returning_cols.to_sql();
            if !returning_params.is_empty() {
                eprintln!("Warning: Parameters in RETURNING clause are currently ignored.");
            }
            sql.push_str(&format!(" RETURNING {}", returning_sql));
        }

        SQL(Cow::Owned(sql), params)
    }
}

/// Convenience function to alias a column for SQL queries.
///
/// # Arguments
/// * `col` - The column to alias (can be a string or SQLiteColumn)
/// * `alias` - The alias to use for the column in the SELECT statement
///
/// # Returns
/// A SQL expression representing "column AS alias"
pub fn alias<'a, C: ToSQL<'a, SQLiteValue<'a>>>(col: C, alias: &str) -> SQL<'a, SQLiteValue<'a>> {
    let SQL(sql, params) = col.to_sql();
    SQL(Cow::Owned(format!("{} AS {}", sql, alias)), params)
}

/// Re-export the columns macro at the module level
pub use crate::columns;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::expressions::conditions::eq;
    use procmacros::schema;
    use uuid::Uuid;

    #[procmacros::SQLiteTable(name = "test_table")]
    pub struct TestTable {
        #[integer(primary, autoincrement)]
        id: i64,
        #[text]
        name: String,
    }

    #[procmacros::SQLiteTable(name = "users")]
    pub struct User {
        #[text(primary, default_fn = Uuid::new_v4)]
        id: Uuid,
        #[text]
        email: String,
        #[integer(default = 1)]
        is_active: bool,
        #[text]
        data: String,
    }

    #[procmacros::SQLiteTable(name = "posts")]
    pub struct Post {
        #[text(primary, default_fn = Uuid::new_v4)]
        id: Uuid,
        #[text(references = User::id)]
        user_id: Uuid,
        #[text]
        title: String,
    }

    #[procmacros::SQLiteTable(name = "comments")]
    pub struct Comments {
        #[text(primary)]
        id: String,
        #[text(references = Post::id)]
        post_id: String,
        #[text]
        content: String,
    }

    // Define Category table here so it's in scope for all tests
    #[procmacros::SQLiteTable(name = "categories")]
    #[derive(Clone)]
    struct Category {
        id: i32,
        name: String,
    }

    #[test]
    fn test_columns_with_sqlite_columns() {
        // Use type annotation for the schema
        let mut qb_factory = schema!([User]);
        let query = qb_factory
            .from::<User>()
            .select(columns!(User::id, User::email, User::is_active, User::data))
            .r#where(eq(User::id, "test-id"));

        let sql = query.to_sql();

        assert!(
            sql.0
                .contains("SELECT id, email, is_active, data FROM users WHERE id =")
        );
        assert!(
            sql.0.contains("test-id") || sql.0.contains("?"),
            "SQL should include parameter value or placeholder"
        );

        if sql.0.contains("?") {
            assert_eq!(sql.1.len(), 1, "Should have one parameter");

            if let SQLiteValue::Text(text) = &sql.1[0] {
                assert_eq!(text, "test-id");
            } else {
                panic!("Expected TEXT parameter");
            }
        }
    }

    #[test]
    fn test_inner_join() {
        // Define schema with multiple tables
        let mut qb_factory = schema!([User, Post]);
        let join_condition = eq(User::id, Post::user_id);

        // Build the query configuration first
        let query_builder = qb_factory.from::<User>();
        query_builder
            // Use inner_join instead of join(JoinType::Inner, ...)
            .inner_join::<Post>(join_condition)
            .r#where(eq(User::email, "test@example.com")); // eq(Column, Value)

        // Finalize with select
        let select_query = query_builder.select(columns!(User::email, Post::title));

        let sql = select_query.to_sql();

        assert_eq!(
            sql.0.to_lowercase(),
            // The eq(User::id, Post::user_id) should generate "id = user_id" directly
            "select email, title from users inner join posts on id = user_id where email = ?"
                .to_lowercase()
        );
        assert_eq!(sql.1.len(), 1); // Only the parameter from the WHERE clause
        assert_eq!(
            sql.1[0],
            SQLiteValue::Text(Cow::Borrowed("test@example.com"))
        );
    }

    #[test]
    fn test_left_join() {
        // Define schema with multiple tables
        let mut qb_factory = schema!([User, Post]);
        let join_condition = eq(User::id, Post::user_id);

        // Build the query configuration first
        let query_builder = qb_factory.from::<User>();
        query_builder
            // Use left_join instead of join(JoinType::Left, ...)
            .left_join::<Post>(join_condition)
            .r#where(eq(Post::title, "My First Post")); // Condition on joined table column

        // Finalize with select
        let select_query = query_builder.select_all(); // Select all from the primary table (User)

        let sql = select_query.to_sql();

        assert_eq!(
            sql.0.to_lowercase(),
            // The eq(User::id, Post::user_id) should generate "id = user_id" directly
            "select * from users left join posts on id = user_id where title = ?".to_lowercase()
        );
        assert_eq!(sql.1.len(), 1); // Only the parameter from the WHERE clause
        assert_eq!(sql.1[0], SQLiteValue::Text(Cow::Borrowed("My First Post")));
    }

    #[test]
    fn test_multi_table_schema() {
        // Test with 3 tables in schema
        let mut qb = schema!([User, Post, Comments]);

        // Should be able to query from any of the tables in the schema
        let _user_query = qb.from::<User>().select_all();
        let _post_query = qb.from::<Post>().select_all();
        let _comment_query = qb.from::<Comments>().select_all();

        // And should be able to join between them using new methods
        let query = qb
            .from::<Post>()
            .inner_join::<User>(eq(Post::user_id, User::id))
            .left_join::<Comments>(eq(Post::id, Comments::post_id))
            .select(columns!(Post::title, User::email))
            .to_sql();

        assert!(
            query
                .0
                .to_lowercase()
                .contains("from posts inner join users on user_id = id")
        );
        assert!(
            query
                .0
                .to_lowercase()
                .contains("left join comments on id = post_id")
        );
    }

    #[test]
    fn test_columns_aliasing() {
        let mut qb_factory = schema!([User]);
        // Test with SQLiteColumn using as_ method
        let cols = columns!(User::id.as_("user_id"));
        let (sql_string, _) = cols.to_sql();
        assert_eq!(sql_string, "id AS user_id");

        // Test with SQLiteColumn with alias function
        let cols = columns!(alias(User::email, "contact_email"));
        let (sql_string, _) = cols.to_sql();
        assert_eq!(sql_string, "email AS contact_email");

        // Test with SQLiteColumn with and without aliasing
        let query = qb_factory.from::<User>().select(columns!(
            User::id.as_("user_id"),
            User::email,
            alias(User::is_active, "status")
        ));

        let sql = query.to_sql();
        // Check the generated SQL string directly for the expected parts
        assert!(sql.0.contains("id AS user_id"), "SQL: {}", sql.0);
        assert!(sql.0.contains(", email,"), "SQL: {}", sql.0); // Be more specific about email
        assert!(sql.0.contains("is_active AS status"), "SQL: {}", sql.0);
    }

    #[test]
    fn test_comprehensive_query_with_aliases() {
        let mut qb_factory = schema!([User]);
        // Build the query in steps
        let query_builder = qb_factory.from::<User>();

        // Add WHERE clause
        query_builder.r#where(eq(User::is_active, true));

        // Add ORDER BY clause
        query_builder.order_by(User::id, SortDirection::Asc);

        // Add LIMIT clause
        query_builder.limit(10);

        // Create SELECT with column aliases
        let select_query = query_builder.select(columns!(
            User::id.as_("user_id"),
            alias(User::email, "contact_email"),
            alias(
                SQL(
                    Cow::Owned(format!(
                        "CASE WHEN {} = 1 THEN 'Active' ELSE 'Inactive' END",
                        User::is_active.name
                    )),
                    vec![]
                ),
                "status_text"
            ),
            User::data
        ));

        let sql = select_query.to_sql();

        // Check that all our column aliases appear in the final SQL
        assert!(sql.0.contains("id AS user_id"));
        assert!(sql.0.contains("email AS contact_email"));
        assert!(
            sql.0.contains(
                "CASE WHEN is_active = 1 THEN 'Active' ELSE 'Inactive' END AS status_text"
            )
        );

        // Check other parts of the query
        assert!(sql.0.contains("FROM users"));
        assert!(sql.0.contains("WHERE is_active = ?"));
        assert!(sql.0.contains("ORDER BY id ASC"));
        assert!(sql.0.contains("LIMIT ?"));

        // Check parameters
        assert_eq!(sql.1.len(), 2); // is_active param and limit param

        // Check limit parameter (second parameter)
        if let SQLiteValue::Integer(i) = sql.1[1] {
            assert_eq!(i, 10); // Limit parameter
        } else {
            panic!("Expected INTEGER for LIMIT parameter");
        }
    }

    #[test]
    fn test_schema_type_safety() {
        // This schema includes TestTable and Comments
        let mut qb = schema!([TestTable, Comments]);

        // Querying TestTable (in schema) should work
        let _test_table_query = qb.from::<TestTable>().select_all();

        // Querying Comments (in schema) should work
        let _comments_query = qb.from::<Comments>().select_all();

        let _tests_comments_query = qb
            .from::<TestTable>()
            .inner_join::<Comments>(eq(TestTable::id, Comments::post_id))
            .select_all();

        // Category is now defined outside this test

        // The following lines *should* fail to compile if uncommented,
        // because Category is not part of TestSchema.

        // Attempting to query Category (NOT in schema) - COMPILE ERROR EXPECTED
        // let _category_query = qb.from::<Category>().select_all(); // UNCOMMENTED

        // Attempting to join with Category (NOT in schema) - COMPILE ERROR EXPECTED
        // Note: We need a schema that *includes* TestTable to test the join.
        // let qb_join_test = schema!([TestTable]); // Schema with just TestTable
        // let _join_query = qb_join_test
        //     .from::<TestTable>() // UNCOMMENTED
        //     // Use left_join
        //     .left_join::<Category>(eq(TestTable::id, Category::id)) // UNCOMMENTED
        //     .select_all();
    }

    #[test]
    fn test_insert_returning() {
        let mut qb = schema!([User]);
        let user_email = "insert_returning@example.com";
        let insert_query = qb
            .from::<User>()
            .insert()
            .value(User::email, user_email)
            .value(User::is_active, true)
            .value(User::data, "some data")
            .returning(columns!(User::id, User::email));

        let sql = insert_query.to_sql();

        assert!(sql.0.to_lowercase().contains("insert into users"));
        assert!(sql.0.to_lowercase().contains("(email, is_active, data)"));
        assert!(sql.0.to_lowercase().contains("values (?, ?, ?)"));
        assert!(sql.0.to_lowercase().contains("returning id, email"));

        assert_eq!(sql.1.len(), 3);
        assert_eq!(sql.1[0], SQLiteValue::Text(Cow::Borrowed(user_email)));
        assert_eq!(sql.1[1], SQLiteValue::Integer(1)); // true is 1
        assert_eq!(sql.1[2], SQLiteValue::Text(Cow::Borrowed("some data")));
    }

    #[test]
    fn test_update_returning() {
        let mut qb = schema!([User]);
        let user_email = "update_returning@example.com";
        let update_query = qb
            .from::<User>()
            .update()
            .set(User::is_active, false)
            .set(User::data, "updated data")
            .r#where(eq(User::email, user_email))
            .returning(columns!(User::id, User::is_active));

        let sql = update_query.to_sql();

        assert!(
            sql.0
                .to_lowercase()
                .contains("update users set is_active = ?, data = ?")
        );
        assert!(sql.0.to_lowercase().contains("where email = ?"));
        assert!(sql.0.to_lowercase().contains("returning id, is_active"));

        assert_eq!(sql.1.len(), 3);
        assert_eq!(sql.1[0], SQLiteValue::Integer(0)); // false is 0
        assert_eq!(sql.1[1], SQLiteValue::Text(Cow::Borrowed("updated data")));
        assert_eq!(sql.1[2], SQLiteValue::Text(Cow::Borrowed(user_email)));
    }

    #[test]
    fn test_delete_returning() {
        let mut qb = schema!([User]);
        let user_email = "delete_returning@example.com";
        let delete_query = qb
            .from::<User>()
            .delete()
            .r#where(eq(User::email, user_email))
            .returning(columns!(User::id)); // Return just the ID

        let sql = delete_query.to_sql();

        assert!(
            sql.0
                .to_lowercase()
                .contains("delete from users where email = ?")
        );
        assert!(sql.0.to_lowercase().contains("returning id"));

        assert_eq!(sql.1.len(), 1);
        assert_eq!(sql.1[0], SQLiteValue::Text(Cow::Borrowed(user_email)));
    }
}
