use crate::helpers;
use crate::traits::SQLiteTable;
use crate::values::SQLiteValue;
use drizzle_core::{SQL, SQLTable, ToSQL};
use paste::paste;
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of SelectBuilder.
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectInitial;

impl SelectInitial {
    /// Creates a new SelectInitial marker
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

/// Marker for the state after FROM clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectFromSet;

/// Marker for the state after JOIN clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectJoinSet;

/// Marker for the state after WHERE clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectWhereSet;

/// Marker for the state after GROUP BY clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectGroupSet;

/// Marker for the state after ORDER BY clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectOrderSet;

/// Marker for the state after LIMIT clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectLimitSet;

/// Marker for the state after OFFSET clause
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectOffsetSet;

/// Marker for the state after set operations (UNION/INTERSECT/EXCEPT)
#[derive(Debug, Clone, Copy, Default)]
pub struct SelectSetOpSet;

// Const constructors for all marker types
impl SelectFromSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectJoinSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectWhereSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectGroupSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectOrderSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectLimitSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectOffsetSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl SelectSetOpSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

#[doc(hidden)]
macro_rules! join_impl {
    () => {
        join_impl!(natural);
        join_impl!(natural_left);
        join_impl!(left);
        join_impl!(left_outer);
        join_impl!(natural_left_outer);
        join_impl!(natural_right);
        join_impl!(right);
        join_impl!(right_outer);
        join_impl!(natural_right_outer);
        join_impl!(natural_full);
        join_impl!(full);
        join_impl!(full_outer);
        join_impl!(natural_full_outer);
        join_impl!(inner);
        join_impl!(cross);
    };
    ($type:ident) => {
        paste! {
            pub fn [<$type _join>]<U:  SQLiteTable<'a>>(
                self,
                table: U,
                condition: impl ToSQL<'a, SQLiteValue<'a>>,
            ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
                SelectBuilder {
                    sql: self.sql.append(helpers::[<$type _join>](table, condition)),
                    schema: PhantomData,
                    state: PhantomData,
                    table: PhantomData,
                }
            }
        }
    };
}

// Mark states that can execute queries as implementing the ExecutableState trait
impl ExecutableState for SelectFromSet {}
impl ExecutableState for SelectWhereSet {}
impl ExecutableState for SelectLimitSet {}
impl ExecutableState for SelectOffsetSet {}
impl ExecutableState for SelectOrderSet {}
impl ExecutableState for SelectGroupSet {}
impl ExecutableState for SelectJoinSet {}
impl ExecutableState for SelectSetOpSet {}

//------------------------------------------------------------------------------
// SelectBuilder Definition
//------------------------------------------------------------------------------

/// Builds a SELECT query specifically for SQLite.
///
/// `SelectBuilder` provides a type-safe, fluent API for constructing SELECT statements
/// with compile-time verification of query structure and table relationships.
///
/// ## Type Parameters
///
/// - `Schema`: The database schema type, ensuring only valid tables can be referenced
/// - `State`: The current builder state, enforcing proper query construction order
/// - `Table`: The primary table being queried (when applicable)
///
/// ## Query Building Flow
///
/// 1. Start with `QueryBuilder::select()` to specify columns
/// 2. Add `from()` to specify the source table
/// 3. Optionally add joins, conditions, grouping, ordering, and limits
///
/// ## Basic Usage
///
/// ```rust
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// use drizzle::sqlite::prelude::*;
/// use drizzle::sqlite::builder::QueryBuilder;
///
/// #[SQLiteTable(name = "users")]
/// struct User {
///     #[column(primary)]
///     id: i32,
///     name: String,
///     email: Option<String>,
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
/// // Basic SELECT
/// let query = builder.select(user.name).from(user);
/// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
///
/// // SELECT with WHERE clause
/// use drizzle::core::expr::gt;
/// let query = builder
///     .select((user.id, user.name))
///     .from(user)
///     .r#where(gt(user.id, 10));
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"SELECT "users"."id", "users"."name" FROM "users" WHERE "users"."id" > ?"#
/// );
/// ```
///
/// ## Advanced Queries
///
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::core::expr::eq;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[SQLiteTable(name = "posts")] struct Post { #[column(primary)] id: i32, user_id: i32, title: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User, post: Post }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user, post } = Schema::new();
/// let query = builder
///     .select((user.name, post.title))
///     .from(user)
///     .join(post, eq(user.id, post.user_id));
/// ```
///
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub mod sqlite {
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # use drizzle::core::OrderBy;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .select(user.name)
///     .from(user)
///     .order_by(OrderBy::asc(user.name))
///     .limit(10);
/// ```
pub type SelectBuilder<'a, Schema, State, Table = ()> =
    super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, S> SelectBuilder<'a, S, SelectInitial> {
    /// Specifies the table or subquery to select FROM.
    ///
    /// This method transitions the builder from the initial state to the FROM state,
    /// enabling subsequent WHERE, JOIN, ORDER BY, and other clauses.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Select from a table
    /// let query = builder.select(user.name).from(user);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
    /// ```
    #[inline]
    pub fn from<T>(self, query: T) -> SelectBuilder<'a, S, SelectFromSet, T>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let sql = self.sql.append(helpers::from(query));
        SelectBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-FROM State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectFromSet, T> {
    /// Adds an INNER JOIN clause to the query.
    ///
    /// Joins another table to the current query using the specified condition.
    /// The joined table must be part of the schema and the condition should
    /// relate columns from both tables.
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::eq;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[SQLiteTable(name = "posts")] struct Post { #[column(primary)] id: i32, user_id: i32, title: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User, post: Post }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user, post } = Schema::new();
    /// let query = builder
    ///     .select((user.name, post.title))
    ///     .from(user)
    ///     .join(post, eq(user.id, post.user_id));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name", "posts"."title" FROM "users" JOIN "posts" ON "users"."id" = "posts"."user_id""#
    /// );
    /// ```
    #[inline]
    pub fn join<U: SQLiteTable<'a>>(
        self,
        table: U,
        condition: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::join(table, condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    join_impl!();

    /// Adds a WHERE clause to filter query results.
    ///
    /// This method applies conditions to filter the rows returned by the query.
    /// You can use various condition functions from `drizzle::core::expr::conditions`.
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::core::expr::{gt, and, eq};
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, age: Option<i32> }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Single condition
    /// let query = builder
    ///     .select(user.name)
    ///     .from(user)
    ///     .r#where(gt(user.id, 10));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"SELECT "users"."name" FROM "users" WHERE "users"."id" > ?"#
    /// );
    ///
    /// // Multiple conditions
    /// let query = builder
    ///     .select(user.name)
    ///     .from(user)
    ///     .r#where(and([gt(user.id, 10), eq(user.name, "Alice")]));
    /// ```
    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectWhereSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::r#where(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a GROUP BY clause to the query
    pub fn group_by(
        self,
        expressions: Vec<SQL<'a, SQLiteValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Limits the number of rows returned
    #[inline]
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Sets the offset for the query results
    #[inline]
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Sorts the query results
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    ///
    /// The returned `CTEView` provides typed access to the table's columns through
    /// an aliased table instance, allowing you to reference CTE columns in subsequent queries.
    ///
    /// # Type Parameters
    ///
    /// The `T` (Table) type parameter from `.from(table)` determines the aliased type,
    /// enabling type-safe field access on the returned CTE.
    ///
    /// # Examples
    ///
    /// ```rust
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub mod sqlite {
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Create a CTE from a select query
    /// let active_users = builder
    ///     .select((user.id, user.name))
    ///     .from(user)
    ///     .as_cte("active_users");
    ///
    /// // Use the CTE with typed field access
    /// let query = builder
    ///     .with(&active_users)
    ///     .select(active_users.name)  // Deref gives access to aliased table fields
    ///     .from(&active_users);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"WITH active_users AS (SELECT "users"."id", "users"."name" FROM "users") SELECT "active_users"."name" FROM "active_users""#
    /// );
    /// ```
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        Self,
    >
    where
        T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
    {
        super::CTEView::new(
            <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::alias(name),
            name,
            self,
        )
    }
}

//------------------------------------------------------------------------------
// Post-JOIN State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectJoinSet, T> {
    /// Adds a WHERE condition after a JOIN
    #[inline]
    pub fn r#where(
        self,
        condition: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectWhereSet, T> {
        SelectBuilder {
            sql: self.sql.append(crate::helpers::r#where(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
    /// Sorts the query results
    #[inline]
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
    /// Adds a JOIN clause to the query
    #[inline]
    pub fn join<U: SQLiteTable<'a>>(
        self,
        table: U,
        condition: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectJoinSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::join(table, condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
    join_impl!();
}

impl<'a, S, T> SelectBuilder<'a, S, SelectJoinSet, T>
where
    T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        Self,
    > {
        super::CTEView::new(
            <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::alias(name),
            name,
            self,
        )
    }
}

//------------------------------------------------------------------------------
// Post-WHERE State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectWhereSet, T> {
    /// Adds a GROUP BY clause after a WHERE
    pub fn group_by(
        self,
        expressions: Vec<SQL<'a, SQLiteValue<'a>>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::group_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds an ORDER BY clause after a WHERE
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a LIMIT clause after a WHERE
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, S, T> SelectBuilder<'a, S, SelectWhereSet, T>
where
    T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        Self,
    > {
        super::CTEView::new(
            <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::alias(name),
            name,
            self,
        )
    }
}

//------------------------------------------------------------------------------
// Post-GROUP BY State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectGroupSet, T> {
    /// Adds a HAVING clause after GROUP BY
    pub fn having(
        self,
        condition: SQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectGroupSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::having(condition)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds an ORDER BY clause after GROUP BY
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, S, T> SelectBuilder<'a, S, SelectGroupSet, T>
where
    T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        Self,
    > {
        super::CTEView::new(
            <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::alias(name),
            name,
            self,
        )
    }
}

//------------------------------------------------------------------------------
// Post-ORDER BY State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectOrderSet, T> {
    /// Adds a LIMIT clause after ORDER BY
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        let sql = helpers::limit(limit);
        SelectBuilder {
            sql: self.sql.append(sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, S, T> SelectBuilder<'a, S, SelectOrderSet, T>
where
    T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        Self,
    > {
        super::CTEView::new(
            <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::alias(name),
            name,
            self,
        )
    }
}

//------------------------------------------------------------------------------
// Post-LIMIT State Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> SelectBuilder<'a, S, SelectLimitSet, T> {
    /// Adds an OFFSET clause after LIMIT
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, S, T> SelectBuilder<'a, S, SelectLimitSet, T>
where
    T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        Self,
    > {
        super::CTEView::new(
            <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::alias(name),
            name,
            self,
        )
    }
}

impl<'a, S, T> SelectBuilder<'a, S, SelectOffsetSet, T>
where
    T: SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Converts this SELECT query into a CTE (Common Table Expression) with the given name.
    #[inline]
    pub fn as_cte(
        self,
        name: &'static str,
    ) -> super::CTEView<
        'a,
        <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::Aliased,
        Self,
    > {
        super::CTEView::new(
            <T as SQLTable<'a, crate::common::SQLiteSchemaType, SQLiteValue<'a>>>::alias(name),
            name,
            self,
        )
    }
}

//------------------------------------------------------------------------------
// Set operation support (UNION / INTERSECT / EXCEPT)
//------------------------------------------------------------------------------

impl<'a, S, State, T> SelectBuilder<'a, S, State, T>
where
    State: ExecutableState,
{
    /// Combines this query with another using UNION.
    pub fn union(
        self,
        other: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::union(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using UNION ALL.
    pub fn union_all(
        self,
        other: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::union_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using INTERSECT.
    pub fn intersect(
        self,
        other: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::intersect(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using INTERSECT ALL.
    pub fn intersect_all(
        self,
        other: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::intersect_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using EXCEPT.
    pub fn except(
        self,
        other: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::except(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Combines this query with another using EXCEPT ALL.
    pub fn except_all(
        self,
        other: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> SelectBuilder<'a, S, SelectSetOpSet, T> {
        SelectBuilder {
            sql: helpers::except_all(self.sql, other),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

impl<'a, S, T> SelectBuilder<'a, S, SelectSetOpSet, T> {
    /// Sorts the results of a set operation.
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> SelectBuilder<'a, S, SelectOrderSet, T>
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
    {
        SelectBuilder {
            sql: self.sql.append(helpers::order_by(expressions)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Limits the results of a set operation.
    pub fn limit(self, limit: usize) -> SelectBuilder<'a, S, SelectLimitSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::limit(limit)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Offsets the results of a set operation.
    pub fn offset(self, offset: usize) -> SelectBuilder<'a, S, SelectOffsetSet, T> {
        SelectBuilder {
            sql: self.sql.append(helpers::offset(offset)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}
