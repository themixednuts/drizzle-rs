use crate::traits::SQLiteTable;
use crate::values::SQLiteValue;
use core::marker::PhantomData;
use drizzle_core::builder::{
    ConflictColumnsTarget, OnConflictBuilder as CoreOnConflictBuilder, OnConflictOutput,
};
use drizzle_core::{
    ConflictTarget, InsertSelectCompatible, InsertSelectTable, InsertTargetColumns,
    PartialInsertSelectCompatible, SQL, SQLModel, ToSQL, Token,
};

use super::select::{CompletedSelect, IntoSelectQuery};

//------------------------------------------------------------------------------
// Type State Markers
//------------------------------------------------------------------------------

pub use drizzle_core::builder::{
    InsertColumnsSet, InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet,
    InsertValuesSet,
};

//------------------------------------------------------------------------------
// OnConflictBuilder
//------------------------------------------------------------------------------

/// An `ON CONFLICT (target)` clause waiting for its action.
///
/// Created by [`InsertBuilder::on_conflict`]. Finish it with `do_nothing()`
/// or `do_update(..)`.
pub type OnConflictBuilder<'a, S, T> = CoreOnConflictBuilder<
    'a,
    SQLiteValue<'a>,
    S,
    T,
    ConflictColumnsTarget<'a, SQLiteValue<'a>>,
    SQLiteOnConflictOutput,
>;

#[doc(hidden)]
#[derive(Debug, Clone, Copy, Default)]
pub struct SQLiteOnConflictOutput;

impl<'a, S, T> OnConflictOutput<'a, SQLiteValue<'a>, S, T> for SQLiteOnConflictOutput {
    type OnConflictSet = InsertBuilder<'a, S, InsertOnConflictSet, T>;
    type DoUpdateSet = InsertBuilder<'a, S, InsertDoUpdateSet, T>;

    fn on_conflict(sql: SQL<'a, SQLiteValue<'a>>) -> Self::OnConflictSet {
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

    fn do_update(sql: SQL<'a, SQLiteValue<'a>>) -> Self::DoUpdateSet {
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

/// An INSERT query being built for `SQLite`.
///
/// This is [`QueryBuilder`](super::QueryBuilder) in one of the `Insert*`
/// states. Start it with [`QueryBuilder::insert`](super::QueryBuilder::insert).
///
/// # Clause order
///
/// 1. A row source: [`values`](Self::values) or [`value`](Self::value),
///    [`select`](Self::select), or [`columns`](Self::columns) followed by
///    `select`.
/// 2. Optionally a conflict clause: [`on_conflict`](Self::on_conflict)
///    followed by `do_nothing()` or `do_update(..)` (and optionally `where`),
///    or [`on_conflict_do_nothing`](Self::on_conflict_do_nothing).
/// 3. Optionally [`returning`](Self::returning).
///
/// Nothing can follow `returning`.
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
/// #         #[cfg(feature = "rusqlite")]
/// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
/// #         #[cfg(feature = "libsql")]
/// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
/// #         #[cfg(feature = "turso")]
/// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
/// #         pub mod prelude {
/// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
/// #             pub use drizzle_sqlite::{*, attrs::*};
/// #             pub use drizzle_core::*;
/// #         }
/// #     }
/// # }
/// # use drizzle::sqlite::prelude::*;
/// # use drizzle::sqlite::builder::QueryBuilder;
/// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, #[column(unique)] email: Option<String> }
/// # #[derive(SQLiteSchema)] struct Schema { user: User }
/// # let builder = QueryBuilder::new::<Schema>();
/// # let Schema { user } = Schema::new();
/// let query = builder
///     .insert(user)
///     .values([InsertUser::new("Alice"), InsertUser::new("Bob")])
///     .on_conflict_do_nothing()
///     .returning(user.id);
/// assert_eq!(
///     query.to_sql().sql(),
///     r#"INSERT INTO "users" ("name") VALUES (?), (?) ON CONFLICT DO NOTHING RETURNING "users"."id""#
/// );
/// ```
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
    /// Inserts one row. Same as `values([value])`.
    ///
    /// `value` is the table's generated insert model (for example
    /// `InsertUser`).
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

    /// Inserts one or more rows.
    ///
    /// Each item is the table's generated insert model (for example
    /// `InsertUser`). All rows must set the same columns: the insert model's
    /// type tracks which columns are set, so rows built with different
    /// setters do not type-check together. If no column is set, this renders
    /// `DEFAULT VALUES` for one row (several such rows insert `NULL` into
    /// `rowid`, which a `WITHOUT ROWID` table rejects).
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
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # use drizzle::sqlite::prelude::*;
    /// # use drizzle::sqlite::builder::QueryBuilder;
    /// # #[SQLiteTable(name = "users")] struct User { #[column(primary)] id: i32, name: String, #[column(unique)] email: Option<String> }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user } = Schema::new();
    /// let query = builder.insert(user).values([
    ///     InsertUser::new("Alice").with_email("alice@example.com"),
    ///     InsertUser::new("Bob").with_email("bob@example.com"),
    /// ]);
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
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Lists the target columns for an `INSERT ... SELECT`.
    ///
    /// Pass a tuple of the table's columns, then call
    /// [`select`](InsertBuilder::select). The column list must include every
    /// required column (one without a default), and the SELECT must produce
    /// matching types in the same order; both are checked at compile time.
    #[inline]
    pub fn columns<Columns>(
        self,
        columns: Columns,
    ) -> InsertBuilder<'a, Schema, InsertColumnsSet<Columns::Columns>, Table>
    where
        Columns: InsertTargetColumns<'a, SQLiteValue<'a>, Table>,
    {
        InsertBuilder {
            sql: self.sql.append(columns.into_target_columns_sql()),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Inserts the rows of a SELECT into every insertable column of the table.
    ///
    /// The SELECT must produce one value per insertable column, in table
    /// order, with compatible types and nullability. Its column references
    /// and aggregates are also checked. To fill only some columns, call
    /// [`columns`](Self::columns) first.
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
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
    /// # #[SQLiteTable(name = "archived_users")] struct ArchivedUser { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User, archived_user: ArchivedUser }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user, archived_user } = Schema::new();
    /// let query = builder
    ///     .insert(archived_user)
    ///     .select(builder.select((user.id, user.name)).from(user));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "archived_users" ("id", "name") SELECT "users"."id", "users"."name" FROM "users""#
    /// );
    /// ```
    #[inline]
    pub fn select<Q, R, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Table: InsertSelectTable,
        Q: IntoSelectQuery<'a, Schema, R>,
        Q::Marker: InsertSelectCompatible<'a, SQLiteValue<'a>, Table, R>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        let select = query.into_select_query().into_select_sql();
        InsertBuilder {
            sql: self
                .sql
                .append(Table::insert_columns_sql::<SQLiteValue<'a>>())
                .append(select),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Appends any SQL as the row source, without a column list.
    ///
    /// Nothing is checked: not the column count, types, nullability, column
    /// scope or aggregates. Prefer [`select`](Self::select).
    #[inline]
    pub fn select_raw<Q>(self, query: Q) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Q: ToSQL<'a, SQLiteValue<'a>>,
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

impl<'a, Schema, Table, Targets> InsertBuilder<'a, Schema, InsertColumnsSet<Targets>, Table>
where
    Table: SQLiteTable<'a> + InsertSelectTable,
{
    /// Inserts the rows of a SELECT into the columns chosen with
    /// [`columns`](InsertBuilder::columns).
    ///
    /// The SELECT must produce one value per chosen column, in the same
    /// order, with compatible types and nullability.
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
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
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
    /// # #[SQLiteTable(name = "archived_users")] struct ArchivedUser { #[column(primary)] id: i32, name: String }
    /// # #[derive(SQLiteSchema)] struct Schema { user: User, archived_user: ArchivedUser }
    /// # let builder = QueryBuilder::new::<Schema>();
    /// # let Schema { user, archived_user } = Schema::new();
    /// let query = builder
    ///     .insert(archived_user)
    ///     .columns(archived_user.name)
    ///     .select(builder.select(user.name).from(user));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "archived_users" ("name") SELECT "users"."name" FROM "users""#
    /// );
    /// ```
    #[inline]
    pub fn select<Q, R, RequiredProof, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: IntoSelectQuery<'a, Schema, R>,
        Q::Marker: PartialInsertSelectCompatible<'a, SQLiteValue<'a>, Targets>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        let select = query.into_select_query().into_select_sql();
        InsertBuilder {
            sql: self.sql.append(select),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Appends any SQL as the row source for the chosen columns.
    ///
    /// Only the column list is checked (it must include every required
    /// column). The SQL itself is not checked. Prefer
    /// [`select`](InsertBuilder::select).
    #[inline]
    pub fn select_raw<Q, RequiredProof>(
        self,
        query: Q,
    ) -> InsertBuilder<'a, Schema, InsertValuesSet, Table>
    where
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: ToSQL<'a, SQLiteValue<'a>>,
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
    /// Starts an `ON CONFLICT (target)` clause.
    ///
    /// The target can be a primary key column, a unique column, or a unique
    /// index of this table; anything else does not compile. Finish the
    /// clause with `do_nothing()` or `do_update(update_model)`. After
    /// `do_update` you may add a `where` and then
    /// [`returning`](InsertBuilder::returning).
    ///
    /// When the row source is a SELECT that ends in its FROM clause, a
    /// `WHERE true` is added before `ON CONFLICT` so `SQLite` does not parse
    /// `ON` as a join condition.
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
    /// #     pub mod sqlite {
    /// #         pub use drizzle_sqlite::*;
    /// #         #[cfg(feature = "rusqlite")]
    /// #         pub mod rusqlite { pub use ::rusqlite::{Error, Result, Row, types}; }
    /// #         #[cfg(feature = "libsql")]
    /// #         pub mod libsql { pub use ::libsql::{Row, Value}; }
    /// #         #[cfg(feature = "turso")]
    /// #         pub mod turso { pub use ::turso::{Error, IntoValue, Result, Row, Value}; }
    /// #         pub mod prelude {
    /// #             pub use drizzle_macros::{SQLiteTable, SQLiteSchema};
    /// #             pub use drizzle_sqlite::{*, attrs::*};
    /// #             pub use drizzle_core::*;
    /// #         }
    /// #     }
    /// # }
    /// # pub use _drizzle::*;
    /// # pub use const_format;
    /// fn main() {
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
    /// let query = builder
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice")])
    ///     .on_conflict(user.id)
    ///     .do_nothing();
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name") VALUES (?) ON CONFLICT ("id") DO NOTHING"#
    /// );
    ///
    /// let query = builder
    ///     .insert(user)
    ///     .values([InsertUser::new("Alice").with_email("a@example.com")])
    ///     .on_conflict(user.email)
    ///     .do_update(UpdateUser::default().with_name("Alice"));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name", "email") VALUES (?, ?) ON CONFLICT ("email") DO UPDATE SET "name" = ?"#
    /// );
    /// }
    /// ```
    pub fn on_conflict<C: ConflictTarget<T>>(self, target: C) -> OnConflictBuilder<'a, S, T> {
        let columns = target.conflict_columns();
        let target_where = target.conflict_where_clause().map(SQL::raw);
        let target_sql = SQL::join(columns.iter().map(|c| SQL::ident(*c)), Token::COMMA);
        OnConflictBuilder::new(
            crate::helpers::before_upsert(self.sql),
            ConflictColumnsTarget::new(target_sql),
        )
        .with_target_where_sql(target_where)
    }

    /// Adds `ON CONFLICT DO NOTHING` with no target, which skips a row that
    /// violates any unique or primary key constraint.
    #[must_use]
    pub fn on_conflict_do_nothing(self) -> InsertBuilder<'a, S, InsertOnConflictSet, T> {
        let conflict_sql = SQL::from_iter([Token::ON, Token::CONFLICT, Token::DO, Token::NOTHING]);
        InsertBuilder {
            sql: crate::helpers::before_upsert(self.sql).append(conflict_sql),
            schema: PhantomData,
            state: PhantomData,
            table: PhantomData,
            marker: PhantomData,
            row: PhantomData,
            grouped: PhantomData,
        }
    }

    /// Adds a RETURNING clause that reads columns of the inserted rows.
    ///
    /// Pass one column or expression, a tuple, or `()` for every column
    /// (`RETURNING *`). Only columns of the target table may be used; other
    /// tables do not compile. The row type is inferred like a SELECT's.
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
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
    /// Adds a RETURNING clause after the conflict clause. See
    /// [`returning`](InsertBuilder::returning).
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
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
    /// Adds a WHERE to `DO UPDATE SET`, so the update only runs for
    /// conflicting rows that match.
    ///
    /// Renders `ON CONFLICT (..) DO UPDATE SET .. WHERE condition`. The
    /// condition may only reference the target table.
    pub fn r#where<E, ScopeProof>(
        self,
        condition: E,
    ) -> InsertBuilder<'a, S, InsertOnConflictSet, T>
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
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

    /// Adds a RETURNING clause after `DO UPDATE SET`. See
    /// [`returning`](InsertBuilder::returning).
    #[inline]
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> ReturningBuilder<'a, S, T, Columns>
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
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
