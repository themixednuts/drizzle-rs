use crate::traits::SQLiteTable;
use crate::values::SQLiteValue;
use drizzle_core::{SQL, SQLModel, ToSQL, Token};
use std::fmt::Debug;
use std::marker::PhantomData;

// Import the ExecutableState trait
use super::ExecutableState;

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

/// Marker for the initial state of InsertBuilder.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertInitial;

/// Marker for the state after VALUES are set.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertValuesSet;

/// Marker for the state after RETURNING clause is added.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertReturningSet;

/// Marker for the state after ON CONFLICT is set.
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertOnConflictSet;

// Const constructors for insert marker types
impl InsertInitial {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl InsertValuesSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl InsertReturningSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}
impl InsertOnConflictSet {
    #[inline]
    pub const fn new() -> Self {
        Self
    }
}

/// Conflict resolution strategies
#[derive(Debug, Clone)]
pub enum Conflict<
    'a,
    T: IntoIterator<Item: ToSQL<'a, SQLiteValue<'a>>> = Vec<SQL<'a, SQLiteValue<'a>>>,
> {
    /// Do nothing on conflict - ON CONFLICT DO NOTHING
    Ignore {
        /// Optional target columns to specify which constraint triggers the conflict
        target: Option<T>,
    },
    /// Update on conflict - ON CONFLICT DO UPDATE
    Update {
        /// Target columns that trigger the conflict
        target: T,
        /// SET clause for what to update
        set: Box<SQL<'a, SQLiteValue<'a>>>,
        /// Optional WHERE clause for the conflict target (partial indexes)
        /// This goes after the target: ON CONFLICT (col) WHERE condition
        target_where: Box<Option<SQL<'a, SQLiteValue<'a>>>>,
        /// Optional WHERE clause for the update (conditional updates)
        /// This goes after the SET: DO UPDATE SET col = val WHERE condition
        set_where: Box<Option<SQL<'a, SQLiteValue<'a>>>>,
    },
}

impl<'a> Default for Conflict<'a> {
    fn default() -> Self {
        Self::Ignore { target: None }
    }
}

impl<'a, T> Conflict<'a, T>
where
    T: IntoIterator<Item: ToSQL<'a, SQLiteValue<'a>>>,
{
    pub fn update<S, TW, SW>(
        target: T,
        set: S,
        target_where: Option<TW>,
        set_where: Option<SW>,
    ) -> Self
    where
        S: ToSQL<'a, SQLiteValue<'a>>,
        TW: ToSQL<'a, SQLiteValue<'a>>,
        SW: ToSQL<'a, SQLiteValue<'a>>,
    {
        Conflict::Update {
            target,
            set: Box::new(set.to_sql()),
            target_where: Box::new(target_where.map(|w| w.to_sql())),
            set_where: Box::new(set_where.map(|w| w.to_sql())),
        }
    }
}

// Mark states that can execute insert queries
impl ExecutableState for InsertValuesSet {}
impl ExecutableState for InsertReturningSet {}
impl ExecutableState for InsertOnConflictSet {}

//------------------------------------------------------------------------------
// InsertBuilder Definition
//------------------------------------------------------------------------------

/// Builds an INSERT query specifically for SQLite.
///
/// `InsertBuilder` provides a type-safe, fluent API for constructing INSERT statements
/// with support for conflict resolution, batch inserts, and returning clauses.
///
/// ## Type Parameters
///
/// - `Schema`: The database schema type, ensuring only valid tables can be referenced
/// - `State`: The current builder state, enforcing proper query construction order
/// - `Table`: The table being inserted into
///
/// ## Query Building Flow
///
/// 1. Start with `QueryBuilder::insert(table)` to specify the target table
/// 2. Add `values()` to specify what data to insert
/// 3. Optionally add conflict resolution with `on_conflict()`
/// 4. Optionally add a `returning()` clause
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
/// #     pub use drizzle_types as ddl;
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
/// // Basic INSERT
/// let query = builder
///     .insert(user)
///     .values([InsertUser::new("Alice")]);
/// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" ("name") VALUES (?)"#);
///
/// // Batch INSERT
/// let query = builder
///     .insert(user)
///     .values([
///         InsertUser::new("Alice").with_email("alice@example.com"),
///         InsertUser::new("Bob").with_email("bob@example.com"),
///     ]);
/// ```
///
/// ## Conflict Resolution
///
/// SQLite supports various conflict resolution strategies:
///
/// ```rust,no_run
/// # mod drizzle {
/// #     pub mod core { pub use drizzle_core::*; }
/// #     pub mod error { pub use drizzle_core::error::*; }
/// #     pub mod types { pub use drizzle_types::*; }
/// #     pub mod migrations { pub use drizzle_migrations::*; }
/// #     pub use drizzle_types::Dialect;
/// #     pub use drizzle_types as ddl;
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
/// # use drizzle::sqlite::builder::{QueryBuilder, insert::Conflict};
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// // Ignore conflicts (ON CONFLICT DO NOTHING)
/// let query = builder
///     .insert(user)
///     .values([InsertUser::new("Alice")])
///     .on_conflict(Conflict::default());
/// ```
pub type InsertBuilder<'a, Schema, State, Table> = super::QueryBuilder<'a, Schema, State, Table>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> InsertBuilder<'a, Schema, InsertInitial, Table>
where
    Table: SQLiteTable<'a>,
{
    /// Specifies the values to insert into the table.
    ///
    /// This method accepts an iterable of insert value objects generated by the
    /// SQLiteTable macro (e.g., `InsertUser`). You can insert single values or
    /// multiple values for batch operations.
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
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, email: Option<String> }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Single insert
    /// let query = builder
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice")]);
    /// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" ("name") VALUES (?)"#);
    ///
    /// // Batch insert (all values must have the same fields set)
    /// let query = builder
    ///     .insert(user)
    ///     .values([
    ///         InsertUser::new("Alice").with_email("alice@example.com"),
    ///         InsertUser::new("Bob").with_email("bob@example.com"),
    ///     ]);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name", "email") VALUES (?, ?), (?, ?)"#
    /// );
    /// ```
    #[inline]
    pub fn values<I, T>(self, values: I) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        I: IntoIterator<Item = Table::Insert<T>>,
        Table::Insert<T>: SQLModel<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::values::<'a, Table, T>(values);
        InsertBuilder {
            sql: self.sql.append(sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-VALUES Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertValuesSet, T> {
    /// Adds conflict resolution to handle constraint violations.
    ///
    /// SQLite supports various conflict resolution strategies when inserting data
    /// that would violate unique constraints or primary keys. This method allows
    /// you to specify how to handle such conflicts.
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
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::{QueryBuilder, insert::Conflict};
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, email: Option<String> }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// // Ignore conflicts (do nothing)
    /// let query = builder
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice")])
    ///     .on_conflict(Conflict::default());
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name") VALUES (?) ON CONFLICT DO NOTHING"#
    /// );
    /// ```
    pub fn on_conflict<TI>(
        self,
        conflict: Conflict<'a, TI>,
    ) -> InsertBuilder<'a, S, InsertOnConflictSet, T>
    where
        TI: IntoIterator,
        TI::Item: ToSQL<'a, SQLiteValue<'a>>,
    {
        let conflict_sql = match conflict {
            Conflict::Ignore { target } => {
                if let Some(target_iter) = target {
                    let cols = SQL::join(
                        target_iter.into_iter().map(|item| item.to_sql()),
                        Token::COMMA,
                    );
                    SQL::from_iter([Token::ON, Token::CONFLICT, Token::LPAREN])
                        .append(cols)
                        .push(Token::RPAREN)
                        .push(Token::DO)
                        .push(Token::NOTHING)
                } else {
                    SQL::from_iter([Token::ON, Token::CONFLICT, Token::DO, Token::NOTHING])
                }
            }
            Conflict::Update {
                target,
                set,
                target_where,
                set_where,
            } => {
                let target_cols =
                    SQL::join(target.into_iter().map(|item| item.to_sql()), Token::COMMA);
                let mut sql = SQL::from_iter([Token::ON, Token::CONFLICT, Token::LPAREN])
                    .append(target_cols)
                    .push(Token::RPAREN);

                // Add target WHERE clause (for partial indexes)
                if let Some(target_where) = *target_where {
                    sql = sql.push(Token::WHERE).append(target_where);
                }

                sql = sql
                    .push(Token::DO)
                    .push(Token::UPDATE)
                    .push(Token::SET)
                    .append(*set);

                // Add set WHERE clause (for conditional updates)
                if let Some(set_where) = *set_where {
                    sql = sql.push(Token::WHERE).append(set_where);
                }

                sql
            }
        };

        InsertBuilder {
            sql: self.sql.append(conflict_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }

    /// Adds a RETURNING clause and transitions to ReturningSet state
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> InsertBuilder<'a, S, InsertReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-ON CONFLICT Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertOnConflictSet, T> {
    /// Adds a RETURNING clause after ON CONFLICT
    #[inline]
    pub fn returning(
        self,
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> InsertBuilder<'a, S, InsertReturningSet, T> {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
        }
    }
}
