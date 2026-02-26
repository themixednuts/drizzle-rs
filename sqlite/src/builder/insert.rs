use crate::traits::SQLiteTable;
use crate::values::SQLiteValue;
use core::fmt::Debug;
use core::marker::PhantomData;
use drizzle_core::{ConflictTarget, SQL, SQLModel, ToSQL, Token};

// Import the ExecutableState trait
use super::ExecutableState;

#[inline]
fn append_sql<'a>(
    mut base: SQL<'a, SQLiteValue<'a>>,
    fragment: SQL<'a, SQLiteValue<'a>>,
) -> SQL<'a, SQLiteValue<'a>> {
    base.append_mut(fragment);
    base
}

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

/// Intermediate builder for typed ON CONFLICT clause construction.
///
/// Created by [`InsertBuilder::on_conflict()`]. Call [`do_nothing()`](Self::do_nothing)
/// or [`do_update()`](Self::do_update) to complete the clause.
#[derive(Debug, Clone)]
pub struct OnConflictBuilder<'a, S, T> {
    sql: SQL<'a, SQLiteValue<'a>>,
    target_sql: SQL<'a, SQLiteValue<'a>>,
    target_where: Option<SQL<'a, SQLiteValue<'a>>>,
    schema: PhantomData<S>,
    table: PhantomData<T>,
}

impl<'a, S, T> OnConflictBuilder<'a, S, T> {
    /// Adds a WHERE clause to the conflict target for partial index matching.
    ///
    /// Generates: `ON CONFLICT (col) WHERE condition DO ...`
    pub fn r#where<E>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        self.target_where = Some(condition.to_sql());
        self
    }

    /// Splits into (base insert SQL, conflict target SQL prefix).
    fn into_parts(self) -> (SQL<'a, SQLiteValue<'a>>, SQL<'a, SQLiteValue<'a>>) {
        let mut target = SQL::from_iter([Token::ON, Token::CONFLICT, Token::LPAREN])
            .append(self.target_sql)
            .push(Token::RPAREN);
        if let Some(tw) = self.target_where {
            target = target.push(Token::WHERE).append(tw);
        }
        (self.sql, target)
    }

    /// Resolves the conflict by doing nothing (ignoring the conflicting row).
    ///
    /// Generates: `ON CONFLICT (col1, col2) DO NOTHING`
    pub fn do_nothing(self) -> InsertBuilder<'a, S, InsertOnConflictSet, T> {
        let (sql, target) = self.into_parts();
        InsertBuilder {
            sql: append_sql(sql, target.push(Token::DO).push(Token::NOTHING)),
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
    /// which generates the SET clause assignments.
    ///
    /// Generates: `ON CONFLICT (col1, col2) DO UPDATE SET ...`
    ///
    /// Chain `.r#where(condition)` to add a conditional update filter.
    pub fn do_update(
        self,
        set: impl ToSQL<'a, SQLiteValue<'a>>,
    ) -> InsertBuilder<'a, S, InsertDoUpdateSet, T> {
        let (sql, target) = self.into_parts();
        let conflict = target
            .push(Token::DO)
            .push(Token::UPDATE)
            .push(Token::SET)
            .append(set.to_sql());
        InsertBuilder {
            sql: append_sql(sql, conflict),
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

/// Builds an INSERT query specifically for SQLite.
///
/// Provides a type-safe, fluent API for constructing INSERT statements
/// with support for typed conflict resolution, batch inserts, and returning clauses.
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
/// 3. Optionally add conflict resolution with `on_conflict(target).do_nothing()` or `.do_update(set)`
/// 4. Optionally add a `returning()` clause
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
    Table: SQLiteTable<'a>,
{
    /// Specifies a single row to insert into the table.
    ///
    /// Accepts an insert value object generated by the SQLiteTable macro
    /// (e.g., `InsertUser`).
    #[inline]
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Table::Insert<T>: SQLModel<'a, SQLiteValue<'a>>,
    {
        self.values([value])
    }

    /// Specifies the values to insert into the table.
    ///
    /// Accepts an iterable of insert value objects generated by the
    /// SQLiteTable macro (e.g., `InsertUser`).
    #[inline]
    pub fn values<I, T>(self, values: I) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        I: IntoIterator<Item = Table::Insert<T>>,
        Table::Insert<T>: SQLModel<'a, SQLiteValue<'a>>,
    {
        let sql = crate::helpers::values::<'a, Table, T>(values);
        InsertBuilder {
            sql: append_sql(self.sql, sql),
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
    /// Begins a typed ON CONFLICT clause targeting a specific constraint.
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
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
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
    ///     #[column(unique)]
    ///     email: Option<String>,
    /// }
    ///
    /// #[derive(SQLiteSchema)]
    /// struct Schema {
    ///     user: User,
    /// }
    ///
    /// let builder = QueryBuilder::new::<Schema>();
    /// let schema = Schema::new();
    /// let user = schema.user;
    ///
    /// // Target a specific column (requires PK or unique constraint)
    /// builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict(user.id).do_nothing();
    ///
    /// // Target with DO UPDATE
    /// builder.insert(user).values([InsertUser::new("Alice")])
    ///     .on_conflict(user.email).do_update(UpdateUser::default().with_name("updated"));
    /// ```
    pub fn on_conflict<C: ConflictTarget<T>>(self, target: C) -> OnConflictBuilder<'a, S, T> {
        let columns = target.conflict_columns();
        let target_sql = SQL::join(columns.iter().map(|c| SQL::ident(*c)), Token::COMMA);
        OnConflictBuilder {
            sql: self.sql,
            target_sql,
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
            sql: append_sql(self.sql, conflict_sql),
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
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: append_sql(self.sql, returning_sql),
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
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: append_sql(self.sql, returning_sql),
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
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
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
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let returning_sql = crate::helpers::returning(columns);
        InsertBuilder {
            sql: append_sql(self.sql, returning_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
        }
    }
}
