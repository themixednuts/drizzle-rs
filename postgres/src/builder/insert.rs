#[cfg(not(feature = "std"))]
use crate::prelude::*;
use crate::traits::PostgresTable;
use crate::values::PostgresValue;
use core::fmt::Debug;
use core::marker::PhantomData;
use drizzle_core::{ConflictTarget, NamedConstraint, SQL, ToSQL, Token};

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

/// Marker for the state after DO UPDATE SET (before optional WHERE).
#[derive(Debug, Clone, Copy, Default)]
pub struct InsertDoUpdateSet;

// Mark states that can execute insert queries
impl ExecutableState for InsertValuesSet {}
impl ExecutableState for InsertReturningSet {}
impl ExecutableState for InsertOnConflictSet {}
impl ExecutableState for InsertDoUpdateSet {}

//------------------------------------------------------------------------------
// OnConflictBuilder
//------------------------------------------------------------------------------

/// Internal: how the conflict target was specified.
#[derive(Debug, Clone)]
enum ConflictTargetKind<'a> {
    /// ON CONFLICT (col1, col2)
    Columns(Box<SQL<'a, PostgresValue<'a>>>),
    /// ON CONFLICT ON CONSTRAINT "constraint_name"
    Constraint(&'static str),
}

/// Intermediate builder for typed ON CONFLICT clause construction (PostgreSQL).
///
/// Created by [`InsertBuilder::on_conflict()`] or
/// [`InsertBuilder::on_conflict_on_constraint()`].
/// Call [`do_nothing()`](Self::do_nothing) or [`do_update()`](Self::do_update)
/// to complete the clause.
#[derive(Debug, Clone)]
pub struct OnConflictBuilder<'a, S, T> {
    sql: SQL<'a, PostgresValue<'a>>,
    target: ConflictTargetKind<'a>,
    target_where: Option<SQL<'a, PostgresValue<'a>>>,
    schema: PhantomData<S>,
    table: PhantomData<T>,
}

impl<'a, S, T> OnConflictBuilder<'a, S, T> {
    /// Adds a WHERE clause to the conflict target for partial index matching.
    ///
    /// Generates: `ON CONFLICT (col) WHERE condition DO ...`
    ///
    /// Note: WHERE is only meaningful for column-based targets, not for
    /// `ON CONFLICT ON CONSTRAINT` targets.
    pub fn r#where<E>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        self.target_where = Some(condition.to_sql());
        self
    }

    /// Splits into (base insert SQL, conflict target SQL prefix).
    fn into_parts(self) -> (SQL<'a, PostgresValue<'a>>, SQL<'a, PostgresValue<'a>>) {
        let target = match self.target {
            ConflictTargetKind::Columns(cols) => {
                let mut t = SQL::from_iter([Token::ON, Token::CONFLICT, Token::LPAREN])
                    .append(*cols)
                    .push(Token::RPAREN);
                if let Some(tw) = self.target_where {
                    t = t.push(Token::WHERE).append(tw);
                }
                t
            }
            ConflictTargetKind::Constraint(name) => {
                SQL::from_iter([Token::ON, Token::CONFLICT, Token::ON, Token::CONSTRAINT])
                    .append(SQL::ident(name))
            }
        };
        (self.sql, target)
    }

    /// Resolves the conflict by doing nothing (ignoring the conflicting row).
    ///
    /// Generates: `ON CONFLICT (col1, col2) DO NOTHING`
    pub fn do_nothing(self) -> InsertBuilder<'a, S, InsertOnConflictSet, T> {
        let (sql, target) = self.into_parts();
        InsertBuilder {
            sql: sql.append(target.push(Token::DO).push(Token::NOTHING)),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Resolves the conflict by updating the existing row.
    ///
    /// The `set` parameter accepts any `ToSQL` value, typically an `UpdateModel`
    /// which generates the SET clause assignments. Use `EXCLUDED.column` to
    /// reference the proposed insert values.
    ///
    /// Generates: `ON CONFLICT (col1, col2) DO UPDATE SET ...`
    ///
    /// Chain `.r#where(condition)` to add a conditional update filter.
    pub fn do_update(
        self,
        set: impl ToSQL<'a, PostgresValue<'a>>,
    ) -> InsertBuilder<'a, S, InsertDoUpdateSet, T> {
        let (sql, target) = self.into_parts();
        let conflict = target
            .push(Token::DO)
            .push(Token::UPDATE)
            .push(Token::SET)
            .append(set.to_sql());
        InsertBuilder {
            sql: sql.append(conflict),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// InsertBuilder Definition
//------------------------------------------------------------------------------

/// Builds an INSERT query specifically for PostgreSQL.
///
/// Provides a type-safe, fluent API for constructing INSERT statements
/// with support for typed conflict resolution, batch inserts, and returning clauses.
///
/// ## Type Parameters
///
/// - `Schema`: The database schema type, ensuring only valid tables can be referenced
/// - `State`: The current builder state, enforcing proper query construction order
/// - `Table`: The table being inserted into
pub type InsertBuilder<'a, Schema, State, Table, Marker = (), Row = ()> =
    super::QueryBuilder<'a, Schema, State, Table, Marker, Row>;

type ReturningMarker<Table, Columns> = drizzle_core::Scoped<
    <Columns as drizzle_core::IntoSelectTarget>::Marker,
    drizzle_core::Cons<Table, drizzle_core::Nil>,
>;

type ReturningRow<Table, Columns> =
    <<Columns as drizzle_core::IntoSelectTarget>::Marker as drizzle_core::ResolveRow<Table>>::Row;

type ReturningBuilder<'a, S, T, Columns> = InsertBuilder<
    'a,
    S,
    InsertReturningSet,
    T,
    ReturningMarker<T, Columns>,
    ReturningRow<T, Columns>,
>;

//------------------------------------------------------------------------------
// Initial State Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table> InsertBuilder<'a, Schema, InsertInitial, Table>
where
    Table: PostgresTable<'a>,
{
    /// Specifies a single row to insert. Shorthand for `.values([row])`.
    #[inline]
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table> {
        self.values([value])
    }

    /// Specifies multiple rows to insert.
    #[inline]
    pub fn values<I, T>(self, values: I) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        I: IntoIterator<Item = Table::Insert<T>>,
    {
        let sql = crate::helpers::values::<'a, Table, T>(values);
        InsertBuilder {
            sql: self.sql.append(sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-VALUES Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertValuesSet, T> {
    /// Begins a typed ON CONFLICT clause targeting specific columns.
    ///
    /// The target must implement `ConflictTarget<T>`, which is auto-generated for
    /// primary key columns, unique columns, and unique indexes.
    ///
    /// Returns an [`OnConflictBuilder`] to specify `do_nothing()` or `do_update()`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresColumnInfo, PostgresTable, PostgresTableInfo};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// use drizzle::postgres::prelude::*;
    /// use drizzle::postgres::builder::QueryBuilder;
    ///
    /// #[PostgresTable(name = "users")]
    /// struct User {
    ///     #[column(serial, primary)]
    ///     id: i32,
    ///     name: String,
    ///     #[column(unique)]
    ///     email: Option<String>,
    /// }
    ///
    /// #[PostgresIndex(unique)]
    /// struct UserEmailIdx(User::email);
    ///
    /// #[derive(PostgresSchema)]
    /// struct Schema {
    ///     user: User,
    ///     user_email_idx: UserEmailIdx,
    /// }
    ///
    /// let builder = QueryBuilder::new::<Schema>();
    /// let schema = Schema::new();
    /// let user = schema.user;
    ///
    /// // Target a specific column
    /// builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict(user.id).do_nothing();
    ///
    /// // Target with DO UPDATE using EXCLUDED
    /// builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict(user.email).do_update(UpdateUser::default().with_name("updated"));
    ///
    /// // Target a unique index
    /// builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict(schema.user_email_idx).do_nothing();
    /// ```
    pub fn on_conflict<C: ConflictTarget<T>>(self, target: C) -> OnConflictBuilder<'a, S, T> {
        let columns = target.conflict_columns();
        let target_sql = SQL::join(columns.iter().map(|c| SQL::ident(*c)), Token::COMMA);
        OnConflictBuilder {
            sql: self.sql,
            target: ConflictTargetKind::Columns(Box::new(target_sql)),
            target_where: None,
            schema: PhantomData,
            table: PhantomData,
        }
    }

    /// Begins a typed ON CONFLICT ON CONSTRAINT clause (PostgreSQL-only).
    ///
    /// The target must implement `NamedConstraint<T>`, which is auto-generated
    /// for unique indexes.
    ///
    /// Returns an [`OnConflictBuilder`] to specify `do_nothing()` or `do_update()`.
    ///
    /// # Examples
    ///
    /// ```rust,no_run
    /// # mod drizzle {
    /// #     pub mod core { pub use drizzle_core::*; }
    /// #     pub mod error { pub use drizzle_core::error::*; }
    /// #     pub mod types { pub use drizzle_types::*; }
    /// #     pub mod migrations { pub use drizzle_migrations::*; }
    /// #     pub use drizzle_types::Dialect;
    /// #     pub use drizzle_types as ddl;
    /// #     pub mod postgres {
    /// #         pub mod values { pub use drizzle_postgres::values::*; }
    /// #         pub mod traits { pub use drizzle_postgres::traits::*; }
    /// #         pub mod common { pub use drizzle_postgres::common::*; }
    /// #         pub mod attrs { pub use drizzle_postgres::attrs::*; }
    /// #         pub mod builder { pub use drizzle_postgres::builder::*; }
    /// #         pub mod helpers { pub use drizzle_postgres::helpers::*; }
    /// #         pub mod expr { pub use drizzle_postgres::expr::*; }
    /// #         pub mod types { pub use drizzle_postgres::types::*; }
    /// #         pub struct Row;
    /// #         impl Row {
    /// #             pub fn get<'a, I, T>(&'a self, _: I) -> T { unimplemented!() }
    /// #             pub fn try_get<'a, I, T>(&'a self, _: I) -> Result<T, Box<dyn std::error::Error + Sync + Send>> { unimplemented!() }
    /// #         }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{PostgresTable, PostgresSchema, PostgresIndex};
    /// #             pub use drizzle_postgres::attrs::*;
    /// #             pub use drizzle_postgres::common::PostgresSchemaType;
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresColumnInfo, PostgresTable, PostgresTableInfo};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// use drizzle::postgres::prelude::*;
    /// use drizzle::postgres::builder::QueryBuilder;
    ///
    /// #[PostgresTable(name = "users")]
    /// struct User {
    ///     #[column(serial, primary)]
    ///     id: i32,
    ///     name: String,
    ///     #[column(unique)]
    ///     email: Option<String>,
    /// }
    ///
    /// #[PostgresIndex(unique)]
    /// struct UserEmailIdx(User::email);
    ///
    /// #[derive(PostgresSchema)]
    /// struct Schema {
    ///     user: User,
    ///     user_email_idx: UserEmailIdx,
    /// }
    ///
    /// let builder = QueryBuilder::new::<Schema>();
    /// let schema = Schema::new();
    ///
    /// builder.insert(schema.user).values([InsertUser::new("Alice")])
    ///     .on_conflict_on_constraint(schema.user_email_idx).do_nothing();
    /// ```
    pub fn on_conflict_on_constraint<C: NamedConstraint<T>>(
        self,
        target: C,
    ) -> OnConflictBuilder<'a, S, T> {
        OnConflictBuilder {
            sql: self.sql,
            target: ConflictTargetKind::Constraint(target.constraint_name()),
            target_where: None,
            schema: PhantomData,
            table: PhantomData,
        }
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    ///
    /// This matches any constraint violation.
    pub fn on_conflict_do_nothing(self) -> InsertBuilder<'a, S, InsertOnConflictSet, T> {
        let conflict_sql = SQL::from_iter([Token::ON, Token::CONFLICT, Token::DO, Token::NOTHING]);
        InsertBuilder {
            sql: self.sql.append(conflict_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds a RETURNING clause and transitions to ReturningSet state
    #[inline]
    pub fn returning<Columns>(self, columns: Columns) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-ON CONFLICT Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertOnConflictSet, T> {
    /// Adds a RETURNING clause after ON CONFLICT
    #[inline]
    pub fn returning<Columns>(self, columns: Columns) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// Post-DO UPDATE SET Implementation
//------------------------------------------------------------------------------

impl<'a, S, T> InsertBuilder<'a, S, InsertDoUpdateSet, T> {
    /// Adds a WHERE clause to the DO UPDATE SET clause.
    ///
    /// Generates: `ON CONFLICT (col) DO UPDATE SET ... WHERE condition`
    pub fn r#where<E>(self, condition: E) -> InsertBuilder<'a, S, InsertOnConflictSet, T>
    where
        E: drizzle_core::expr::Expr<'a, PostgresValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let sql = self.sql.push(Token::WHERE).append(condition.to_sql());
        InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }

    /// Adds a RETURNING clause after DO UPDATE SET
    #[inline]
    pub fn returning<Columns>(self, columns: Columns) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: ToSQL<'a, PostgresValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: self.sql.append(returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use drizzle_core::{SQL, ToSQL};

    #[test]
    fn test_insert_builder_creation() {
        let builder = InsertBuilder::<(), InsertInitial, ()> {
            sql: SQL::raw("INSERT INTO test"),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "INSERT INTO test");
    }
}
