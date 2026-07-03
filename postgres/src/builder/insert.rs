use crate::traits::PostgresTable;
use crate::values::PostgresValue;
use core::marker::PhantomData;
use drizzle_core::builder::{
    OnConflictBuilder as CoreOnConflictBuilder, OnConflictOutput, PostgresConflictTarget,
};
use drizzle_core::{ConflictTarget, NamedConstraint, SQL, ToSQL, Token};

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{
    InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
};

//------------------------------------------------------------------------------
// OnConflictBuilder
//------------------------------------------------------------------------------

/// Intermediate builder for typed ON CONFLICT clause construction (`PostgreSQL`).
///
/// Created by [`InsertBuilder::on_conflict()`] or
/// [`InsertBuilder::on_conflict_on_constraint()`].
/// Call [`do_nothing()`](Self::do_nothing) or [`do_update()`](Self::do_update)
/// to complete the clause.
pub type OnConflictBuilder<'a, S, T> = CoreOnConflictBuilder<
    'a,
    PostgresValue<'a>,
    S,
    T,
    PostgresConflictTarget<'a, PostgresValue<'a>>,
    PostgresOnConflictOutput,
>;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct PostgresOnConflictOutput;

impl<'a, S, T> OnConflictOutput<'a, PostgresValue<'a>, S, T> for PostgresOnConflictOutput {
    type OnConflictSet = InsertBuilder<'a, S, InsertOnConflictSet, T>;
    type DoUpdateSet = InsertBuilder<'a, S, InsertDoUpdateSet, T>;

    fn on_conflict(sql: SQL<'a, PostgresValue<'a>>) -> Self::OnConflictSet {
        InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    fn do_update(sql: SQL<'a, PostgresValue<'a>>) -> Self::DoUpdateSet {
        InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// InsertBuilder Definition
//------------------------------------------------------------------------------

/// Builds an INSERT query specifically for `PostgreSQL`.
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
            grouped: PhantomData,
        }
    }

    /// Inserts rows produced by a SELECT query without an explicit column list.
    ///
    /// The SELECT output must provide every table column in declaration order.
    #[inline]
    pub fn select<Q>(self, query: Q) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Q: ToSQL<'a, PostgresValue<'a>>,
    {
        InsertBuilder {
            sql: self.sql.append(query.into_sql()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
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
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
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
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// fn main() {
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
    /// }
    /// ```
    pub fn on_conflict<C: ConflictTarget<T>>(self, target: C) -> OnConflictBuilder<'a, S, T> {
        let columns = target.conflict_columns();
        let target_sql = SQL::join(columns.iter().map(|c| SQL::ident(*c)), Token::COMMA);
        OnConflictBuilder::new(self.sql, PostgresConflictTarget::columns(target_sql))
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
    /// ```rust
    /// # extern crate self as drizzle;
    /// # mod _drizzle {
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
    /// #             pub use drizzle_postgres::traits::{PostgresColumn, PostgresTable};
    /// #             pub use drizzle_postgres::values::{PostgresInsertValue, PostgresUpdateValue, PostgresValue};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// fn main() {
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
    /// }
    /// ```
    pub fn on_conflict_on_constraint<C: NamedConstraint<T>>(
        self,
        target: C,
    ) -> OnConflictBuilder<'a, S, T> {
        OnConflictBuilder::new(
            self.sql,
            PostgresConflictTarget::constraint(target.constraint_name()),
        )
    }

    /// Shorthand for `ON CONFLICT DO NOTHING` without specifying a target.
    ///
    /// This matches any constraint violation.
    #[must_use]
    pub fn on_conflict_do_nothing(self) -> InsertBuilder<'a, S, InsertOnConflictSet, T> {
        let conflict_sql = SQL::from_iter([Token::ON, Token::CONFLICT, Token::DO, Token::NOTHING]);
        InsertBuilder {
            sql: self.sql.append(conflict_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds a RETURNING clause and transitions to `ReturningSet` state
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
            grouped: PhantomData,
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
            grouped: PhantomData,
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
        let sql = self
            .sql
            .push(Token::WHERE)
            .append(condition.into_expr_sql());
        InsertBuilder {
            sql,
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
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
            grouped: PhantomData,
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
            grouped: PhantomData,
        };

        assert_eq!(builder.to_sql().sql(), "INSERT INTO test");
    }
}
