#![allow(clippy::type_complexity)]

use core::marker::PhantomData;

use crate::drizzle_builder_join_impl;

use drizzle_core::ConflictTarget;
use drizzle_core::traits::{SQLModel, SQLTable, ToSQL};
use drizzle_sqlite::{
    builder::{
        self, CTEView, DeleteInitial, DeleteReturningSet, DeleteWhereSet, InsertColumnsSet,
        InsertDoUpdateSet, InsertInitial, InsertOnConflictSet, InsertReturningSet, InsertValuesSet,
        OnConflictBuilder, QueryBuilder, SelectFromSet, SelectGroupSet, SelectInitial,
        SelectJoinSet, SelectLimitSet, SelectOffsetSet, SelectOrderSet, SelectWhereSet,
        UpdateInitial, UpdateReturningSet, UpdateSetClauseSet, UpdateWhereSet,
        delete::DeleteBuilder,
        insert::InsertBuilder,
        select::{CompletedSelect, IntoSelect, IntoSelectQuery, SelectBuilder, SelectSetOpSet},
        update::UpdateBuilder,
    },
    common::SQLiteSchemaType,
    traits::SQLiteTable,
    values::SQLiteValue,
};

/// A query being built against a SQLite [`Drizzle`] handle.
///
/// Start one with [`Drizzle::select`], [`insert`](Drizzle::insert),
/// [`update`](Drizzle::update), or [`delete`](Drizzle::delete), chain clauses,
/// then run it with the driver's `.execute()`, `.all()`, `.get()`, or `.rows()`.
/// Each clause method is only available where it is valid SQL (for example,
/// `.having()` only after `.group_by()`). The builder implements `ToSQL`, so
/// `.to_sql().sql()` shows the SQL it will run.
#[derive(Debug)]
#[must_use = "a query builder does nothing until it runs (`.execute()`, `.all()`, `.get()`, ...)"]
pub struct DrizzleBuilder<'a, Runner, Schema, Builder, State> {
    pub(crate) runner: &'a Runner,
    pub(crate) builder: Builder,
    pub(crate) state: PhantomData<(Schema, State)>,
}

/// The `ON CONFLICT (target)` step of an insert, made by
/// `.on_conflict(target)`.
///
/// Finish it with [`do_nothing`](Self::do_nothing) or
/// [`do_update`](Self::do_update).
#[must_use = "a query builder does nothing until it runs (`.execute()`, `.all()`, `.get()`, ...)"]
pub struct DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
    runner: &'a Runner,
    builder: OnConflictBuilder<'b, Schema, Table>,
}

impl<'a, 'b, Runner, Schema, Table> DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
    /// Restricts the conflict target to rows matching `condition`, to match a
    /// partial unique index: `ON CONFLICT (cols) WHERE condition`.
    pub fn r#where<E, ScopeProof>(mut self, condition: E) -> Self
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'b, SQLiteValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        self.builder = self.builder.r#where(condition);
        self
    }

    /// Skips rows that conflict on the target: `ON CONFLICT (target) DO NOTHING`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let query = db
    ///     .insert(users)
    ///     .value(InsertUsers::new("Alex Smith", 26).with_id(1))
    ///     .on_conflict(users.id)
    ///     .do_nothing();
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("id", "name", "age") VALUES (?, ?, ?) ON CONFLICT ("id") DO NOTHING"#,
    /// );
    /// query.execute()?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn do_nothing(
        self,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.do_nothing(),
            state: PhantomData,
        }
    }

    /// Updates the existing row instead: `ON CONFLICT (target) DO UPDATE SET ...`.
    ///
    /// `set` is usually an `Update*` model. Chain `.r#where(..)` to update only
    /// some conflicting rows.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// // Upsert: insert user 1, or bump the age when the id already exists.
    /// db.insert(users)
    ///     .value(InsertUsers::new("Alex Smith", 27).with_id(1))
    ///     .on_conflict(users.id)
    ///     .do_update(UpdateUsers::default().with_age(27))
    ///     .execute()?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn do_update(
        self,
        set: impl ToSQL<'b, SQLiteValue<'b>>,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.do_update(set),
            state: PhantomData,
        }
    }
}

#[cfg(feature = "libsql")]
pub(crate) struct LibsqlCachedStatement {
    pub(crate) sql: Box<str>,
    pub(crate) statement: ::libsql::Statement,
}

#[cfg(feature = "libsql")]
pub(crate) struct LibsqlStatementCache(pub(crate) std::sync::Mutex<Option<LibsqlCachedStatement>>);

#[cfg(feature = "libsql")]
impl LibsqlStatementCache {
    pub(crate) const fn new() -> Self {
        Self(std::sync::Mutex::new(None))
    }

    /// Drops the cached statement.
    pub(crate) fn clear(&self) {
        *self.0.lock().unwrap_or_else(|err| err.into_inner()) = None;
    }

    pub(crate) fn take(&self, sql: &str) -> Option<LibsqlCachedStatement> {
        let mut cache = self.0.lock().unwrap_or_else(|err| err.into_inner());
        if cache
            .as_ref()
            .is_some_and(|cached| cached.sql.as_ref() == sql)
        {
            cache.take()
        } else {
            None
        }
    }

    /// Keeps `cached` for reuse, reset first: a statement left mid-step
    /// (after `get()` read one row) holds the connection's read transaction
    /// open, which can serve later reads a stale snapshot and blocks WAL
    /// checkpoints until the statement is reused or evicted.
    pub(crate) fn store(&self, cached: LibsqlCachedStatement) {
        cached.statement.reset();
        let mut cache = self.0.lock().unwrap_or_else(|err| err.into_inner());
        *cache = Some(cached);
    }
}

#[cfg(feature = "libsql")]
impl Clone for LibsqlStatementCache {
    fn clone(&self) -> Self {
        Self::new()
    }
}

#[cfg(feature = "libsql")]
impl Default for LibsqlStatementCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "libsql")]
impl core::fmt::Debug for LibsqlStatementCache {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("LibsqlStatementCache")
            .finish_non_exhaustive()
    }
}

/// A SQLite connection paired with its schema: the handle every query starts
/// from.
///
/// Each driver names it for its own connection type, such as
/// `drizzle::sqlite::rusqlite::Drizzle<Schema>`. Create one with
/// [`Drizzle::new`], start queries with [`select`](Self::select),
/// [`insert`](Self::insert), [`update`](Self::update), and
/// [`delete`](Self::delete), and reach the raw connection with
/// [`conn`](Self::conn).
#[derive(Debug)]
pub struct Drizzle<Conn, Schema = ()> {
    pub(crate) conn: Conn,
    pub(crate) schema: Schema,
    #[cfg(feature = "libsql")]
    pub(crate) libsql_statement_cache: LibsqlStatementCache,
}

impl<Conn: Clone, S: Clone> Clone for Drizzle<Conn, S> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            conn: self.conn.clone(),
            schema: self.schema.clone(),
            #[cfg(feature = "libsql")]
            libsql_statement_cache: self.libsql_statement_cache.clone(),
        }
    }
}

impl<Conn, Schema: Default> Drizzle<Conn, Schema> {
    /// Creates a new `Drizzle` instance over `conn`.
    ///
    /// Returns `(Drizzle, Schema)`, with the schema built by `Default`. The
    /// pattern that destructures the schema usually names its type. When
    /// nothing else names it, put the type on the call, and use `()` for a
    /// connection with no schema:
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// use drizzle::sqlite::prelude::*;
    /// use drizzle::sqlite::rusqlite::Drizzle;
    /// use rusqlite::Connection;
    ///
    /// #[SQLiteTable]
    /// struct Users {
    ///     #[column(primary)]
    ///     id: i32,
    ///     name: String,
    /// }
    ///
    /// #[derive(SQLiteSchema)]
    /// struct Schema {
    ///     users: Users,
    /// }
    ///
    /// let (db, Schema { users }) = Drizzle::new(Connection::open_in_memory()?);
    /// db.create()?;
    /// db.insert(users).values([InsertUsers::new("Alice")]).execute()?;
    ///
    /// let (db, schema) = Drizzle::<Schema>::new(Connection::open_in_memory()?);
    /// db.create()?;
    /// db.insert(schema.users).values([InsertUsers::new("Bob")]).execute()?;
    ///
    /// let (db, ()) = Drizzle::new(Connection::open_in_memory()?);
    /// # let _ = db;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[inline]
    pub fn new(conn: Conn) -> (Self, Schema) {
        let drizzle = Self {
            conn,
            schema: Schema::default(),
            #[cfg(feature = "libsql")]
            libsql_statement_cache: LibsqlStatementCache::new(),
        };
        (drizzle, Schema::default())
    }
}

impl<Conn, S> AsRef<Self> for Drizzle<Conn, S> {
    #[inline]
    fn as_ref(&self) -> &Self {
        self
    }
}

impl<Conn, Schema> Drizzle<Conn, Schema> {
    /// Returns the wrapped connection, for calls drizzle does not cover.
    #[inline]
    pub const fn conn(&self) -> &Conn {
        &self.conn
    }

    /// Returns the wrapped connection mutably.
    #[inline]
    pub fn conn_mut(&mut self) -> &mut Conn {
        // The caller may replace the connection; a statement cached on the old
        // one would keep running there.
        #[cfg(feature = "libsql")]
        self.libsql_statement_cache.clear();
        &mut self.conn
    }

    /// Returns the schema value this handle was created with.
    #[inline]
    pub const fn schema(&self) -> &Schema {
        &self.schema
    }

    /// Starts a `SELECT` query.
    ///
    /// `query` is what to select: `()` for every column of the `FROM` table, a
    /// column, a tuple of columns and expressions, or a `FromRow` type's
    /// `::Select` marker. Follow it with [`from`](DrizzleBuilder::from).
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::eq;
    ///
    /// // `()` selects every column into the table's `Select*` model.
    /// let everyone: Vec<SelectUsers> = db.select(()).from(users).all()?;
    ///
    /// // A column or a tuple of columns selects just those.
    /// let names: Vec<String> = db.select(users.name).from(users).all()?;
    /// let pairs: Vec<(i64, String)> = db.select((users.id, users.name)).from(users).all()?;
    ///
    /// let query = db.select(users.name).from(users).r#where(eq(users.id, 1));
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users" WHERE "users"."id" = ?"#);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "sqlite")]
    pub fn select<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<
        'a,
        Self,
        Schema,
        SelectBuilder<'b, Schema, drizzle_sqlite::builder::select::SelectInitial, (), T::Marker>,
        drizzle_sqlite::builder::select::SelectInitial,
    >
    where
        T: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
    {
        let builder = QueryBuilder::new::<Schema>().select(query);

        DrizzleBuilder {
            runner: self,
            builder,
            state: PhantomData,
        }
    }

    /// Starts a `SELECT DISTINCT` query, which drops duplicate rows.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let query = db.select_distinct(users.name).from(users);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT DISTINCT "users"."name" FROM "users""#);
    /// let names: Vec<String> = query.all()?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "sqlite")]
    pub fn select_distinct<'a, 'b, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<
        'a,
        Self,
        Schema,
        SelectBuilder<'b, Schema, drizzle_sqlite::builder::select::SelectInitial, (), T::Marker>,
        drizzle_sqlite::builder::select::SelectInitial,
    >
    where
        T: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
    {
        let builder = QueryBuilder::new::<Schema>().select_distinct(query);
        DrizzleBuilder {
            runner: self,
            builder,
            state: PhantomData,
        }
    }

    /// Starts an `INSERT` into `table`.
    ///
    /// Follow it with [`value`](DrizzleBuilder::value) or
    /// [`values`](DrizzleBuilder::values) and an `Insert*` model, or with
    /// `select(..)` to insert a query's rows.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let query = db.insert(users).value(InsertUsers::new("Dana", 41));
    /// assert_eq!(query.to_sql().sql(), r#"INSERT INTO "users" ("name", "age") VALUES (?, ?)"#);
    /// query.execute()?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "sqlite")]
    pub fn insert<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<
        'a,
        Self,
        Schema,
        InsertBuilder<'b, Schema, drizzle_sqlite::builder::insert::InsertInitial, Table>,
        drizzle_sqlite::builder::insert::InsertInitial,
    >
    where
        Table: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().insert(table);
        DrizzleBuilder {
            runner: self,
            builder,
            state: PhantomData,
        }
    }

    /// Starts an `UPDATE` of `table`.
    ///
    /// Follow it with [`set`](DrizzleBuilder::set) and an `Update*` model, then
    /// `.r#where(..)`; `.r#where(true)` updates every row.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::eq;
    ///
    /// let query = db
    ///     .update(users)
    ///     .set(UpdateUsers::default().with_age(27))
    ///     .r#where(eq(users.id, 1));
    /// assert_eq!(query.to_sql().sql(), r#"UPDATE "users" SET "age" = ? WHERE "users"."id" = ?"#);
    /// assert_eq!(query.execute()?, 1);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "sqlite")]
    pub fn update<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<
        'a,
        Self,
        Schema,
        UpdateBuilder<'b, Schema, drizzle_sqlite::builder::update::UpdateInitial, Table>,
        drizzle_sqlite::builder::update::UpdateInitial,
    >
    where
        Table: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().update(table);
        DrizzleBuilder {
            runner: self,
            builder,
            state: PhantomData,
        }
    }

    /// Starts a `DELETE` from `table`.
    ///
    /// It runs once `.r#where(..)` picks the rows; `.r#where(true)` deletes
    /// every row.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::eq;
    ///
    /// let query = db.delete(comments).r#where(eq(comments.id, 1));
    /// assert_eq!(query.to_sql().sql(), r#"DELETE FROM "comments" WHERE "comments"."id" = ?"#);
    /// assert_eq!(query.execute()?, 1);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "sqlite")]
    pub fn delete<'a, 'b, Table>(
        &'a self,
        table: Table,
    ) -> DrizzleBuilder<
        'a,
        Self,
        Schema,
        DeleteBuilder<'b, Schema, drizzle_sqlite::builder::delete::DeleteInitial, Table>,
        drizzle_sqlite::builder::delete::DeleteInitial,
    >
    where
        Table: SQLiteTable<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().delete(table);
        DrizzleBuilder {
            runner: self,
            builder,
            state: PhantomData,
        }
    }

    /// Starts a query with a common table expression: `WITH name AS (...)`.
    ///
    /// Make the CTE with [`into_cte`](DrizzleBuilder::into_cte) on a select query,
    /// then pass it here and select from it.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::gt;
    /// use drizzle::sqlite::prelude::tag;
    ///
    /// tag!(Adults, "adults");
    ///
    /// let adults = db
    ///     .select((users.id, users.name))
    ///     .from(users)
    ///     .r#where(gt(users.age, 18))
    ///     .into_cte::<Adults>();
    ///
    /// let names: Vec<(i64, String)> = db.with(&adults).select((adults.id, adults.name)).from(&adults).all()?;
    /// assert_eq!(names.len(), 2);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[cfg(feature = "sqlite")]
    pub fn with<'a, 'b, C>(
        &'a self,
        cte: &C,
    ) -> DrizzleBuilder<
        'a,
        Self,
        Schema,
        QueryBuilder<'b, Schema, builder::CTEInit>,
        builder::CTEInit,
    >
    where
        C: builder::CTEDefinition<'b>,
    {
        let builder = QueryBuilder::new::<Schema>().with(cte);
        DrizzleBuilder {
            runner: self,
            builder,
            state: PhantomData,
        }
    }
}

// =============================================================================
// Query API: DrizzleQueryBuilder
// =============================================================================

/// A relational query (`db.query(table)`), which loads rows together with
/// their related rows in one SQL statement.
///
/// Add relations with [`with`](Self::with), narrow it with
/// [`r#where`](Self::r#where), [`order_by`](Self::order_by),
/// [`limit`](Self::limit), and [`offset`](Self::offset), then run it with
/// the driver's `find_many()` or `find_first()`. Each clause can be set once.
#[cfg(all(feature = "sqlite", feature = "query"))]
#[must_use = "a query builder does nothing until it runs (`.execute()`, `.all()`, `.get()`, ...)"]
pub struct DrizzleQueryBuilder<
    'db,
    'a,
    Runner,
    Schema,
    T,
    Rels = (),
    Cols = drizzle_core::query::AllColumns,
    Cl = drizzle_core::query::Clauses,
> {
    pub(crate) runner: Runner,
    pub(crate) builder: drizzle_core::query::QueryBuilder<'a, SQLiteValue<'a>, T, Rels, Cols, Cl>,
    pub(crate) _schema: PhantomData<(&'db (), Schema)>,
}

/// A relational query rendered once, made by
/// [`DrizzleQueryBuilder::prepare`].
///
/// It is detached from the connection: the driver's `find_many` and
/// `find_first` take the connection and the placeholder bindings on each
/// call.
#[cfg(all(feature = "sqlite", feature = "query"))]
#[derive(Debug, Clone)]
pub struct DrizzlePreparedQuery<'a, Driver, T, Rels, Cols> {
    pub(crate) inner: drizzle_core::prepared::PreparedStatement<'a, SQLiteValue<'a>>,
    pub(crate) _marker: PhantomData<(Driver, T, Rels, Cols)>,
}

#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'a, Driver, T, Rels, Cols> DrizzlePreparedQuery<'a, Driver, T, Rels, Cols> {
    /// Returns the rendered SQL, with the dialect's placeholders.
    #[must_use]
    pub fn sql(&self) -> &str {
        self.inner.sql()
    }

    /// Returns how many placeholder bindings each run expects.
    #[must_use]
    pub fn param_count(&self) -> usize {
        self.inner.external_param_count()
    }
}

#[cfg(all(feature = "sqlite", feature = "query"))]
impl<Driver, T, Rels, Cols> core::fmt::Display for DrizzlePreparedQuery<'_, Driver, T, Rels, Cols> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.write_str(self.sql())
    }
}

#[cfg(all(feature = "sqlite", feature = "query"))]
impl<Conn, Schema> Drizzle<Conn, Schema> {
    /// Starts a relational query on `table` (requires the `query` feature).
    ///
    /// Relations come from foreign keys: `#[column(references = Users::id)]` on
    /// `Posts::author_id` gives `users.author_posts()` (one-to-many) and
    /// `posts.author()` (many-to-one). Results nest the related rows as fields.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::eq;
    ///
    /// // Each user, with their posts.
    /// let everyone = db.query(users).with(users.author_posts()).find_many()?;
    /// assert_eq!(everyone.len(), 3);
    ///
    /// let alex = db
    ///     .query(users)
    ///     .with(users.author_posts())
    ///     .r#where(eq(users.name, "Alex Smith"))
    ///     .find_first()?
    ///     .expect("Alex exists");
    /// assert_eq!(alex.author_posts.len(), 2);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn query<'a, T>(&self, _table: T) -> DrizzleQueryBuilder<'_, 'a, &Self, Schema, T>
    where
        T: drizzle_core::query::QueryTable,
    {
        DrizzleQueryBuilder {
            runner: self,
            builder: drizzle_core::query::QueryBuilder::new(),
            _schema: PhantomData,
        }
    }
}

/// Declares which relational-query row shape a driver's connection reads.
///
/// [`DrizzleQueryBuilder::prepare`] renders SQL before any driver code runs,
/// so the connection type carries the choice:
///
/// - Native drivers (rusqlite, turso, libsql) read positional columns and
///   decode the base model with `TryFrom<&Row>`, so base columns stay plain
///   (`WRAP_BASE_JSON = false`).
/// - The Cloudflare drivers (d1, durable) receive rows as column-keyed serde
///   objects, where text is the only lossless transport (integers cross the
///   JS boundary as `f64`, and raw blob bytes don't match the hex format the
///   generated decoders expect). They read the base model from a single
///   `"__base"` JSON text column (`WRAP_BASE_JSON = true`) and decode rows
///   through [`drizzle_core::query::JsonQueryRow`].
///
/// Sealed: only driver modules in this crate implement it.
#[cfg(all(feature = "sqlite", feature = "query"))]
pub trait QueryRowFormat: private::Sealed {
    /// Whether `build_query_sql` must wrap base columns into `"__base"` JSON.
    const WRAP_BASE_JSON: bool;
}

#[cfg(all(feature = "sqlite", feature = "query"))]
pub(crate) mod private {
    /// Seals [`QueryRowFormat`](super::QueryRowFormat).
    pub trait Sealed {}
}

#[cfg(feature = "query")]
pub use crate::builder::RelationalPreparedDriver;

#[cfg(all(feature = "sqlite", feature = "query"))]
impl<Conn, Schema> RelationalPreparedDriver for &Drizzle<Conn, Schema> {
    type PreparedDriver = Conn;
}

#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'db, 'a, Runner, Schema, T, Rels, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, drizzle_core::query::AllColumns, Cl>
where
    Runner: RelationalPreparedDriver,
    Runner::PreparedDriver: QueryRowFormat,
    T: drizzle_core::query::QueryTable,
    Rels: drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
{
    /// Renders this relational query once into a reusable
    /// [`DrizzlePreparedQuery`].
    ///
    /// The SQL shape follows the connection type's [`QueryRowFormat`], so it
    /// matches what that driver's prepared `find_many`/`find_first` decode.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::SQLColumn;
    /// use drizzle::core::expr::eq;
    ///
    /// let name = users.name.placeholder("name");
    /// let by_name = db.query(users).with(users.author_posts()).r#where(eq(users.name, name)).prepare();
    ///
    /// let alex = by_name.find_many(db.conn(), [name.bind("Alex Smith")])?;
    /// let bob = by_name.find_many(db.conn(), [name.bind("Bob")])?;
    /// assert_eq!((alex.len(), bob.len()), (1, 1));
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn prepare(
        self,
    ) -> DrizzlePreparedQuery<'a, Runner::PreparedDriver, T, Rels, drizzle_core::query::AllColumns>
    {
        let builder = self.builder;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let query_sql = drizzle_core::query::build_query_sql(
            T::TABLE,
            T::COLUMN_NAMES,
            T::BLOB_COLUMNS,
            T::JSON_PROJECTIONS,
            rendered,
            builder.where_sql,
            builder.order_by_sql,
            builder.limit,
            builder.offset,
            <Runner::PreparedDriver as QueryRowFormat>::WRAP_BASE_JSON,
        );
        DrizzlePreparedQuery {
            inner: drizzle_core::prepared::prepare_render(&query_sql),
            _marker: PhantomData,
        }
    }
}

#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'db, 'a, Runner, Schema, T, Rels, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, drizzle_core::query::PartialColumns, Cl>
where
    Runner: RelationalPreparedDriver,
    T: drizzle_core::query::QueryTable,
    Rels: drizzle_core::query::RenderRelations<'a, SQLiteValue<'a>>,
{
    /// Renders this partial-column relational query once into a reusable
    /// [`DrizzlePreparedQuery`].
    pub fn prepare(
        self,
    ) -> DrizzlePreparedQuery<
        'a,
        Runner::PreparedDriver,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
    > {
        let builder = self.builder;
        let mut rendered = Vec::new();
        builder.relations.render_into(&mut rendered);
        let col_refs: Vec<&str> = builder.cols.columns;
        let query_sql = drizzle_core::query::build_query_sql(
            T::TABLE,
            &col_refs,
            T::BLOB_COLUMNS,
            T::JSON_PROJECTIONS,
            rendered,
            builder.where_sql,
            builder.order_by_sql,
            builder.limit,
            builder.offset,
            true,
        );
        DrizzlePreparedQuery {
            inner: drizzle_core::prepared::prepare_render(&query_sql),
            _marker: PhantomData,
        }
    }
}

#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, Cols, Cl>
{
    /// Loads a relation with each row, such as `users.posts()`.
    ///
    /// Relation handles can nest their own `.with(..)` and clauses. Call `with`
    /// again to load more relations.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// // Users with their posts, and each post with its comments.
    /// let rows = db.query(users).with(users.author_posts().with(posts.comments())).find_many()?;
    /// assert_eq!(rows[0].author_posts[0].comments.len(), 1);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn with<R, N, C, RCl>(
        self,
        handle: drizzle_core::query::RelationHandle<'a, SQLiteValue<'a>, R, N, C, RCl>,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        (
            drizzle_core::query::RelationHandle<'a, SQLiteValue<'a>, R, N, C, RCl>,
            Rels,
        ),
        Cols,
        Cl,
    >
    where
        R: drizzle_core::relation::RelationDef<Source = T> + 'static,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.with(handle),
            _schema: PhantomData,
        }
    }
}

/// WHERE is only available when no WHERE clause has been set yet.
#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, Ord, Lim>
    DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<drizzle_core::query::NoWhere, Ord, Lim>,
    >
{
    /// Filters the root rows. Combine conditions with a tuple (`AND`), `and`,
    /// or `or`; this can be called once.
    ///
    /// The condition may only read the queried table's columns.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::gt;
    ///
    /// let adults = db.query(users).r#where(gt(users.age, 18)).find_many()?;
    /// assert_eq!(adults.len(), 2);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn r#where<E, ScopeProof>(
        self,
        condition: E,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<drizzle_core::query::HasWhere, Ord, Lim>,
    >
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.r#where(condition),
            _schema: PhantomData,
        }
    }
}

/// ORDER BY is only available when no ORDER BY clause has been set yet.
#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, W, Lim>
    DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, drizzle_core::query::NoOrderBy, Lim>,
    >
{
    /// Orders the root rows. This can be called once; pass a tuple to order by
    /// several columns.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::desc;
    ///
    /// let oldest_first = db.query(users).order_by(desc(users.age)).find_many()?;
    /// assert_eq!(oldest_first[0].name, "Alice");
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn order_by<E, ScopeProof>(
        self,
        expr: E,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, drizzle_core::query::HasOrderBy, Lim>,
    >
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.order_by(expr),
            _schema: PhantomData,
        }
    }
}

/// LIMIT is only available when no LIMIT has been set yet.
#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, W, Ord>
    DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::NoLimit>,
    >
{
    /// Returns at most `n` root rows. This can be called once.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let two = db.query(users).limit(2).find_many()?;
    /// assert_eq!(two.len(), 2);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn limit<P>(
        self,
        n: P,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasLimit>,
    >
    where
        P: drizzle_core::PaginationArg<'a, SQLiteValue<'a>>,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.limit(n),
            _schema: PhantomData,
        }
    }
}

/// OFFSET requires LIMIT to have been set first.
#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'db, 'a, Runner, Schema, T, Rels, Cols, W, Ord>
    DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasLimit>,
    >
{
    /// Skips the first `n` root rows. Call [`limit`](Self::limit) first.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::asc;
    ///
    /// let rest = db.query(users).order_by(asc(users.id)).limit(10).offset(1).find_many()?;
    /// assert_eq!(rest.len(), 2);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn offset<P>(
        self,
        n: P,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        Cols,
        drizzle_core::query::Clauses<W, Ord, drizzle_core::query::HasOffset>,
    >
    where
        P: drizzle_core::PaginationArg<'a, SQLiteValue<'a>>,
    {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.offset(n),
            _schema: PhantomData,
        }
    }
}

#[cfg(all(feature = "sqlite", feature = "query"))]
impl<'db, 'a, Runner, Schema, T, Rels, Cl>
    DrizzleQueryBuilder<'db, 'a, Runner, Schema, T, Rels, drizzle_core::query::AllColumns, Cl>
where
    T: drizzle_core::query::QueryTable,
{
    /// Loads only the listed columns.
    ///
    /// Rows then use the table's `PartialSelect*` model, where every field is an
    /// `Option` and unselected ones are `None`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let names = db.query(users).columns(users.columns().name()).find_many()?;
    /// assert!(names[0].name.is_some());
    /// assert!(names[0].id.is_none()); // not selected
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn columns<S: drizzle_core::query::IntoColumnSelection>(
        self,
        selector: S,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    > {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.columns(selector),
            _schema: PhantomData,
        }
    }

    /// Loads every column except the listed ones.
    ///
    /// Rows then use the table's `PartialSelect*` model, where every field is an
    /// `Option` and omitted ones are `None`.
    pub fn omit<S: drizzle_core::query::IntoColumnSelection>(
        self,
        selector: S,
    ) -> DrizzleQueryBuilder<
        'db,
        'a,
        Runner,
        Schema,
        T,
        Rels,
        drizzle_core::query::PartialColumns,
        Cl,
    > {
        DrizzleQueryBuilder {
            runner: self.runner,
            builder: self.builder.omit(selector),
            _schema: PhantomData,
        }
    }
}

impl<'a, Runner, S, T, State> ToSQL<'a, SQLiteValue<'a>> for DrizzleBuilder<'_, Runner, S, T, State>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::sql::SQL<'a, SQLiteValue<'a>> {
        self.builder.to_sql()
    }
}

impl<'a, Runner, S, T, State> drizzle_core::expr::Expr<'a, SQLiteValue<'a>>
    for DrizzleBuilder<'_, Runner, S, T, State>
where
    T: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
{
    type SQLType = T::SQLType;
    type Nullable = T::Nullable;
    type Aggregate = T::Aggregate;
}

impl<Runner, S, T: drizzle_core::expr::SelectQuery, State> drizzle_core::expr::SelectQuery
    for DrizzleBuilder<'_, Runner, S, T, State>
{
}

impl<Runner, S, T, State> drizzle_core::expr::ExprSources
    for DrizzleBuilder<'_, Runner, S, T, State>
where
    T: drizzle_core::expr::ExprSources,
{
    type Sources = T::Sources;
}

impl<'d, 'a, Runner, Schema>
    DrizzleBuilder<'d, Runner, Schema, QueryBuilder<'a, Schema, builder::CTEInit>, builder::CTEInit>
{
    /// Starts the `SELECT` that follows the `WITH` clause. See
    /// [`Drizzle::select`].
    #[inline]
    pub fn select<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial, (), T::Marker>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Starts the `SELECT DISTINCT` that follows the `WITH` clause.
    #[inline]
    pub fn select_distinct<T>(
        self,
        query: T,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, builder::select::SelectInitial, (), T::Marker>,
        builder::select::SelectInitial,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::IntoSelectTarget,
    {
        let builder = self.builder.select_distinct(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Adds another common table expression to the `WITH` clause.
    #[inline]
    pub fn with<C>(self, cte: &C) -> Self
    where
        C: builder::CTEDefinition<'a>,
    {
        let builder = self.builder.with(cte);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'d, 'a, Runner, Schema, M>
    DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<'a, Schema, SelectInitial, (), M>,
        SelectInitial,
    >
{
    /// Sets the table (or other source) the query reads from.
    ///
    /// The source decides the row type for `select(())`, and brings its
    /// columns into scope for the rest of the query.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// // A table handle, a table alias, a CTE, or a derived table (`.alias(..)`).
    /// let query = db.select(users.name).from(users);
    /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users""#);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[inline]
    pub fn from<T>(
        self,
        table: T,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectFromSet,
            T,
            drizzle_core::FromMarker<M, T>,
            <M as drizzle_core::ResolveRow<T>>::Row,
        >,
        SelectFromSet,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>> + drizzle_core::ScopeEntry,
        M: drizzle_core::ResolveRow<T>,
    {
        let builder = self.builder.from(table);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

/// Generates select-method impl blocks for each given state type, avoiding E0592
/// overlap with insert/update/delete impls that share method names on the same
/// generic `DrizzleBuilder` type.
macro_rules! impl_select_methods {
    ($($state:ty => [$($method:ident),* $(,)?]),+ $(,)?) => {
        $(
            impl<'d, 'a, Runner, Schema, T, M, R, G>
                DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, $state, T, M, R, G>, $state>
            {
                $( impl_select_methods!(@method $method); )*
            }
        )+
    };

    // ---- individual method expansions ----

    (@method r#where) => {
        /// Adds a `WHERE` condition.
        ///
        /// Combine conditions with a tuple (`AND`), `and`/`or`, or `|`. An `Option`
        /// element of a tuple that is `None` is left out, which makes optional
        /// filters easy. Columns in the condition must come from the query's tables;
        /// this is checked when the query runs.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "rusqlite")]
        /// # fn main() -> drizzle::Result<()> {
        /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
        /// # use app::*;
        /// # use drizzle::core::ToSQL;
        /// # let (db, Schema { users, posts, comments }) = app::database()?;
        /// # let _ = (&users, &posts, &comments);
        /// use drizzle::core::expr::{eq, gt};
        ///
        /// // A tuple of conditions means AND.
        /// let query = db.select(users.id).from(users).r#where((gt(users.age, 18), eq(users.name, "Alice")));
        /// assert_eq!(
        ///     query.to_sql().sql(),
        ///     r#"SELECT "users"."id" FROM "users" WHERE ("users"."age" > ? AND "users"."name" = ?)"#,
        /// );
        /// let ids: Vec<i64> = query.all()?;
        /// # Ok(())
        /// # }
        /// # #[cfg(not(feature = "rusqlite"))]
        /// # fn main() {}
        /// ```
        #[inline]
        pub fn r#where<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectWhereSet, T, <M as drizzle_core::HasScope>::With<E::Sources>, R, G>, SelectWhereSet>
        where
            M: drizzle_core::HasScope,
            E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.r#where(condition);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method group_by) => {
        /// Adds a `GROUP BY` clause. Pass a column, an expression, or a tuple.
        ///
        /// Each column in a selected tuple must then be grouped or aggregated; this
        /// is checked when the query runs.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "rusqlite")]
        /// # fn main() -> drizzle::Result<()> {
        /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
        /// # use app::*;
        /// # use drizzle::core::ToSQL;
        /// # let (db, Schema { users, posts, comments }) = app::database()?;
        /// # let _ = (&users, &posts, &comments);
        /// use drizzle::core::expr::count;
        ///
        /// let per_author: Vec<(i64, i64)> = db
        ///     .select((posts.author_id, count(posts.id)))
        ///     .from(posts)
        ///     .group_by(posts.author_id)
        ///     .all()?;
        /// assert_eq!(per_author, vec![(1, 2)]);
        /// # Ok(())
        /// # }
        /// # #[cfg(not(feature = "rusqlite"))]
        /// # fn main() {}
        /// ```
        pub fn group_by<Gr>(
            self,
            columns: Gr,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, <M as drizzle_core::HasScope>::With<Gr::Sources>, R, Gr::Columns>, SelectGroupSet>
        where
            M: drizzle_core::HasScope,
            Gr: drizzle_core::IntoGroupBy<'a, SQLiteValue<'a>>,
        {
            let builder = self.builder.group_by(columns);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method having) => {
        /// Adds a `HAVING` condition, which filters groups after `GROUP BY`.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "rusqlite")]
        /// # fn main() -> drizzle::Result<()> {
        /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
        /// # use app::*;
        /// # use drizzle::core::ToSQL;
        /// # let (db, Schema { users, posts, comments }) = app::database()?;
        /// # let _ = (&users, &posts, &comments);
        /// use drizzle::core::expr::{count, gt};
        ///
        /// let prolific: Vec<i64> = db
        ///     .select(posts.author_id)
        ///     .from(posts)
        ///     .group_by(posts.author_id)
        ///     .having(gt(count(posts.id), 1))
        ///     .all()?;
        /// assert_eq!(prolific, vec![1]);
        /// # Ok(())
        /// # }
        /// # #[cfg(not(feature = "rusqlite"))]
        /// # fn main() {}
        /// ```
        pub fn having<E>(
            self,
            condition: E,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectGroupSet, T, <M as drizzle_core::HasScope>::With<E::Sources>, R, G>, SelectGroupSet>
        where
            M: drizzle_core::HasScope,
            E: drizzle_core::expr::Expr<'a, SQLiteValue<'a>>,
            E::SQLType: drizzle_core::types::BooleanLike,
        {
            let builder = self.builder.having(condition);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method order_by) => {
        /// Adds an `ORDER BY` clause.
        ///
        /// Pass a column (ascending), [`asc`](drizzle_core::asc)/[`desc`](drizzle_core::desc),
        /// or a tuple of them.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "rusqlite")]
        /// # fn main() -> drizzle::Result<()> {
        /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
        /// # use app::*;
        /// # use drizzle::core::ToSQL;
        /// # let (db, Schema { users, posts, comments }) = app::database()?;
        /// # let _ = (&users, &posts, &comments);
        /// use drizzle::core::{asc, desc};
        ///
        /// let query = db.select(users.name).from(users).order_by((desc(users.age), asc(users.name)));
        /// assert_eq!(
        ///     query.to_sql().sql(),
        ///     r#"SELECT "users"."name" FROM "users" ORDER BY "users"."age" DESC, "users"."name" ASC"#,
        /// );
        /// # Ok(())
        /// # }
        /// # #[cfg(not(feature = "rusqlite"))]
        /// # fn main() {}
        /// ```
        pub fn order_by<TOrderBy>(
            self,
            expressions: TOrderBy,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T, <M as drizzle_core::HasScope>::With<TOrderBy::Sources>, R, G>, SelectOrderSet>
        where
            M: drizzle_core::HasScope,
            TOrderBy: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>> + drizzle_core::expr::ExprSources,
        {
            let builder = self.builder.order_by(expressions);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method set_order_by) => {
        /// Orders a compound query (`UNION`, `INTERSECT`, ...) by its output
        /// columns.
        pub fn order_by<TOrderBy>(
            self,
            expressions: TOrderBy,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectOrderSet, T, M, R, G>, SelectOrderSet>
        where
            TOrderBy: drizzle_core::traits::ToSQL<'a, SQLiteValue<'a>>,
        {
            let builder = self.builder.order_by(expressions);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method limit) => {
        /// Returns at most `limit` rows (`LIMIT n`).
        ///
        /// Pass an integer, which is written into the SQL, or an integer
        /// placeholder, which is bound when the query runs.
        ///
        /// # Panics
        ///
        /// Panics when an integer `limit` is negative or does not fit in `usize`.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "rusqlite")]
        /// # fn main() -> drizzle::Result<()> {
        /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
        /// # use app::*;
        /// # use drizzle::core::ToSQL;
        /// # let (db, Schema { users, posts, comments }) = app::database()?;
        /// # let _ = (&users, &posts, &comments);
        /// let query = db.select(users.name).from(users).limit(2);
        /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users" LIMIT 2"#);
        /// assert_eq!(query.all::<String, _, _>()?.len(), 2);
        /// # Ok(())
        /// # }
        /// # #[cfg(not(feature = "rusqlite"))]
        /// # fn main() {}
        /// ```
        pub fn limit<P>(
            self,
            limit: P,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectLimitSet, T, M, R, G>, SelectLimitSet>
        where
            P: drizzle_core::PaginationArg<'a, SQLiteValue<'a>>,
        {
            let builder = self.builder.limit(limit);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method offset) => {
        /// Skips the first `offset` rows (`OFFSET n`).
        ///
        /// # Panics
        ///
        /// Panics when an integer `offset` is negative or does not fit in `usize`.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "rusqlite")]
        /// # fn main() -> drizzle::Result<()> {
        /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
        /// # use app::*;
        /// # use drizzle::core::ToSQL;
        /// # let (db, Schema { users, posts, comments }) = app::database()?;
        /// # let _ = (&users, &posts, &comments);
        /// let query = db.select(users.name).from(users).limit(10).offset(1);
        /// assert_eq!(query.to_sql().sql(), r#"SELECT "users"."name" FROM "users" LIMIT 10 OFFSET 1"#);
        /// # Ok(())
        /// # }
        /// # #[cfg(not(feature = "rusqlite"))]
        /// # fn main() {}
        /// ```
        pub fn offset<P>(
            self,
            offset: P,
        ) -> DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, SelectOffsetSet, T, M, R, G>, SelectOffsetSet>
        where
            P: drizzle_core::PaginationArg<'a, SQLiteValue<'a>>,
        {
            let builder = self.builder.offset(offset);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };

    (@method join) => {
        /// Adds a `JOIN` (an inner join).
        ///
        /// Pass a table to join on its foreign key to the previous table (the
        /// `FROM` table, or the table joined last), or a `(table, condition)` pair
        /// to give the `ON` condition yourself. The joined table's columns come
        /// into scope. See also `left_join`, `right_join`, `full_join`, and their
        /// `natural_` and `_outer` forms.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "rusqlite")]
        /// # fn main() -> drizzle::Result<()> {
        /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
        /// # use app::*;
        /// # use drizzle::core::ToSQL;
        /// # let (db, Schema { users, posts, comments }) = app::database()?;
        /// # let _ = (&users, &posts, &comments);
        /// use drizzle::core::expr::eq;
        ///
        /// // Pass a table to join on its foreign key ...
        /// let query = db.select((users.name, posts.title)).from(users).join(posts);
        /// assert_eq!(
        ///     query.to_sql().sql(),
        ///     r#"SELECT "users"."name", "posts"."title" FROM "users" JOIN "posts" ON "posts"."author_id" = "users"."id""#,
        /// );
        ///
        /// // ... or a `(table, condition)` pair for your own ON condition.
        /// let rows: Vec<(String, String)> = db
        ///     .select((users.name, posts.title))
        ///     .from(users)
        ///     .join((posts, eq(users.id, posts.author_id)))
        ///     .all()?;
        /// assert_eq!(rows.len(), 2);
        /// # Ok(())
        /// # }
        /// # #[cfg(not(feature = "rusqlite"))]
        /// # fn main() {}
        /// ```
        #[inline]
        pub fn join<J: drizzle_sqlite::helpers::JoinArg<'a, T, Via>, Via>(
            self,
            arg: J,
        ) -> DrizzleBuilder<
            'd,
            Runner,
            Schema,
            SelectBuilder<
                'a,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>>::Marker,
                <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::InnerJoin, J::OnSources>,
        {
            let builder = self.builder.join(arg);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }

        crate::drizzle_builder_join_impl!();

        /// Adds a `CROSS JOIN`: every row paired with every row of `arg`, with no
        /// `ON` condition.
        ///
        /// # Examples
        ///
        /// ```
        /// # #[cfg(feature = "rusqlite")]
        /// # fn main() -> drizzle::Result<()> {
        /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
        /// # use app::*;
        /// # use drizzle::core::ToSQL;
        /// # let (db, Schema { users, posts, comments }) = app::database()?;
        /// # let _ = (&users, &posts, &comments);
        /// let query = db.select((users.name, posts.title)).from(users).cross_join(posts);
        /// assert_eq!(
        ///     query.to_sql().sql(),
        ///     r#"SELECT "users"."name", "posts"."title" FROM "users" CROSS JOIN "posts""#,
        /// );
        /// # Ok(())
        /// # }
        /// # #[cfg(not(feature = "rusqlite"))]
        /// # fn main() {}
        /// ```
        #[inline]
        pub fn cross_join<Arg: drizzle_sqlite::helpers::CrossJoinArg<'a, T>>(
            self,
            arg: Arg,
        ) -> DrizzleBuilder<
            'd,
            Runner,
            Schema,
            SelectBuilder<
                'a,
                Schema,
                SelectJoinSet,
                Arg::JoinedTable,
                <M as drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>>::Marker,
                <M as drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            M: drizzle_core::JoinStep<R, Arg::JoinedTable, drizzle_core::InnerJoin, Arg::OnSources>,
        {
            let builder = self.builder.cross_join(arg);
            DrizzleBuilder { runner: self.runner, builder, state: PhantomData }
        }
    };
}

// Select method availability by state, mirroring capability trait impls:
impl_select_methods! {
    SelectFromSet  => [r#where, group_by, order_by, limit, offset, join],
    SelectJoinSet  => [r#where, group_by, order_by, join],
    SelectWhereSet => [group_by, order_by, limit],
    SelectGroupSet => [having, order_by, limit],
    SelectOrderSet => [limit],
    SelectLimitSet => [offset],
    SelectSetOpSet => [set_order_by, limit, offset],
}

//------------------------------------------------------------------------------
// IntoSelect for DrizzleBuilder
//------------------------------------------------------------------------------

impl<'a, Runner, Schema, State, T, M, R, G> IntoSelect<'a, Schema, M, R>
    for DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
    SelectBuilder<'a, Schema, State, T, M, R, G>:
        CompletedSelect<'a, Schema, R, Marker = M, Grouped = G>,
{
    type State = State;
    type Table = T;
    fn into_select(self) -> SelectBuilder<'a, Schema, State, T, M, R> {
        self.builder.into_select()
    }
}

impl<'a, Runner, Schema, State, T, M, R, G> IntoSelectQuery<'a, Schema, R>
    for DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    SelectBuilder<'a, Schema, State, T, M, R, G>:
        CompletedSelect<'a, Schema, R, Marker = M, Grouped = G>,
{
    type Marker = M;
    type Grouped = G;
    type Select = SelectBuilder<'a, Schema, State, T, M, R, G>;

    fn into_select_query(self) -> Self::Select {
        self.builder
    }
}

//------------------------------------------------------------------------------
// Set operations on DrizzleBuilder
//------------------------------------------------------------------------------

impl<'d, 'a, Runner, Schema, State, T, M, R>
    DrizzleBuilder<'d, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R>, State>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Compound>,
{
    /// Combines this query's rows with `other`'s and drops duplicates
    /// (`UNION`).
    ///
    /// Both queries must select the same row type. Use
    /// [`union_all`](Self::union_all) to keep duplicates.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::{gte, lte};
    ///
    /// let query = db
    ///     .select(users.name)
    ///     .from(users)
    ///     .r#where(lte(users.age, 18))
    ///     .union(db.select(users.name).from(users).r#where(gte(users.age, 30)));
    /// let names: Vec<String> = query.all()?;
    /// assert_eq!(names.len(), 2);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[allow(clippy::type_complexity)]
    pub fn union<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.union(other),
            state: PhantomData,
        }
    }

    /// Combines this query's rows with `other`'s, keeping duplicates
    /// (`UNION ALL`).
    #[allow(clippy::type_complexity)]
    pub fn union_all<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.union_all(other),
            state: PhantomData,
        }
    }

    /// Keeps only rows that `other` also returns (`INTERSECT`).
    #[allow(clippy::type_complexity)]
    pub fn intersect<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.intersect(other),
            state: PhantomData,
        }
    }

    /// Keeps only rows that `other` does not return (`EXCEPT`).
    #[allow(clippy::type_complexity)]
    pub fn except<M2>(
        self,
        other: impl IntoSelect<'a, Schema, M2, R>,
    ) -> DrizzleBuilder<
        'd,
        Runner,
        Schema,
        SelectBuilder<
            'a,
            Schema,
            SelectSetOpSet,
            T,
            <M as drizzle_core::SetOperand<M2>>::Combined,
            R,
        >,
        SelectSetOpSet,
    >
    where
        M: drizzle_core::SetOperand<M2>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.except(other),
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// sqlcommenter .comment() / .comment_tags() on DrizzleBuilder
//------------------------------------------------------------------------------
//
// Forwards to the inner `QueryBuilder::comment` / `comment_tags`. Because every
// select/insert/update/delete builder is a type alias for `QueryBuilder`, one
// generic impl here covers all four operation kinds.

impl<Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'_, Runner, Schema, QueryBuilder<'_, Schema, State, T, M, R, G>, State>
{
    /// Adds a free-form [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment in front of the query. See [`QueryBuilder::comment`] for how the
    /// text is escaped.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let query = db.select(users.name).from(users).comment("report");
    /// assert_eq!(query.to_sql().sql(), r#"/*report*/ SELECT "users"."name" FROM "users""#);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[inline]
    pub fn comment(self, text: impl AsRef<str>) -> Self
    where
        State: drizzle_sqlite::builder::ExecutableState,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.comment(text),
            state: PhantomData,
        }
    }

    /// Adds a key-value [sqlcommenter](https://google.github.io/sqlcommenter/)
    /// comment, such as `/*route='users'*/`, in front of the query. See
    /// [`QueryBuilder::comment_tags`] for the encoding.
    #[inline]
    pub fn comment_tags<I, K, V>(self, pairs: I) -> Self
    where
        State: drizzle_sqlite::builder::ExecutableState,
        I: IntoIterator<Item = (K, V)>,
        K: AsRef<str>,
        V: AsRef<str>,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.comment_tags(pairs),
            state: PhantomData,
        }
    }
}

impl<'a, Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Source>,
    M: drizzle_core::DerivedSelection<'a, SQLiteValue<'a>, SQLiteSchemaType, T>,
{
    /// Names this query so it can be used as a table: `(SELECT ...) AS name`.
    ///
    /// `name` is a tag type (see the `tag!` macro). `.fields()` on the result
    /// returns its output columns.
    ///
    /// # Panics
    ///
    /// Panics when two outputs of the projection have the same name. Name a
    /// computed expression with [`drizzle_core::expr::AliasExt::named`] to make
    /// each output unique.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::eq;
    /// use drizzle::sqlite::prelude::tag;
    ///
    /// tag!(Recent, "recent");
    ///
    /// // A subquery in FROM/JOIN position, with typed output columns.
    /// let recent = db.select((posts.author_id, posts.title)).from(posts).alias(Recent);
    /// let (author_id, title) = recent.fields();
    ///
    /// let rows: Vec<(String, String)> = db
    ///     .select((users.name, title))
    ///     .from(users)
    ///     .join((recent, eq(users.id, author_id)))
    ///     .all()?;
    /// assert_eq!(rows.len(), 2);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[inline]
    #[must_use]
    pub fn alias<Name, AggProof>(
        self,
        name: Name,
    ) -> drizzle_core::Derived<
        'a,
        SQLiteValue<'a>,
        Name,
        <M as drizzle_core::DerivedSelection<'a, SQLiteValue<'a>, SQLiteSchemaType, T>>::Projection,
        SelectBuilder<'a, Schema, State, T, M, R, G>,
    >
    where
        Name: drizzle_core::Tag,
        <M as drizzle_core::DerivedSelection<'a, SQLiteValue<'a>, SQLiteSchemaType, T>>::Projection:
            drizzle_core::DerivedProjection<Name>,
        M: drizzle_core::row::MarkerAggValidFor<G, AggProof>,
    {
        self.builder.alias(name)
    }
}

impl<'a, Runner, Schema, State, T, M, R, G>
    DrizzleBuilder<'_, Runner, Schema, SelectBuilder<'a, Schema, State, T, M, R, G>, State>
where
    State: drizzle_core::ClauseAllowed<drizzle_core::clause::Simple>,
    T: SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>,
{
    /// Turns this query into a common table expression named after `Tag`.
    ///
    /// Pass the result to [`Drizzle::with`] and select from it. Its columns are
    /// reachable as fields, like a table's.
    #[inline]
    pub fn into_cte<Tag: drizzle_core::Tag + 'static>(
        self,
    ) -> CTEView<
        'a,
        <T as SQLTable<'a, SQLiteSchemaType, SQLiteValue<'a>>>::Aliased<Tag>,
        SelectBuilder<'a, Schema, State, T, M, R, G>,
    > {
        self.builder.into_cte::<Tag>()
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertInitial, Table>,
        InsertInitial,
    >
{
    /// Inserts one row from an `Insert*` model.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let query = db.insert(users).value(InsertUsers::new("Dana", 41).with_email("dana@example.com"));
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name", "email", "age") VALUES (?, ?, ?)"#,
    /// );
    /// query.execute()?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[inline]
    pub fn value<T>(
        self,
        value: Table::Insert<T>,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: SQLiteTable<'b>,
        Table::Insert<T>: SQLModel<'b, SQLiteValue<'b>>,
    {
        self.values([value])
    }

    /// Inserts several rows from `Insert*` models in one statement.
    ///
    /// Every row must set the same optional fields (the same `with_*` calls);
    /// mixing them does not compile, because all rows share one column list.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// db.insert(users)
    ///     .values([InsertUsers::new("Dana", 41), InsertUsers::new("Eli", 19)])
    ///     .execute()?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[inline]
    pub fn values<T>(
        self,
        values: impl IntoIterator<Item = Table::Insert<T>>,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: SQLiteTable<'b>,
        Table::Insert<T>: SQLModel<'b, SQLiteValue<'b>>,
    {
        let builder = self.builder.values(values);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Names the columns an `INSERT ... SELECT` fills, before
    /// [`select`](Self::select).
    ///
    /// The list must include every required column (`NOT NULL` with no
    /// default); this is checked by the following `select(..)`.
    #[inline]
    pub fn columns<Columns>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertColumnsSet<Columns::Columns>, Table>,
        InsertColumnsSet<Columns::Columns>,
    >
    where
        Table: SQLiteTable<'b>,
        Columns: drizzle_core::InsertTargetColumns<'b, SQLiteValue<'b>, Table>,
    {
        let builder = self.builder.columns(columns);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Inserts the rows of a `SELECT` query: `INSERT INTO t SELECT ...`.
    ///
    /// The query's columns must match the table's insert columns in order and
    /// type; this is checked at compile time.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// // Copy every user into the same table (new ids are generated).
    /// let source = db.select((users.name, users.email, users.age)).from(users);
    /// db.insert(users)
    ///     .columns((users.name, users.email, users.age))
    ///     .select(source)
    ///     .execute()?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[inline]
    pub fn select<Q, R, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: SQLiteTable<'b> + drizzle_core::InsertSelectTable,
        Q: IntoSelectQuery<'b, Schema, R>,
        Q::Marker: drizzle_core::InsertSelectCompatible<'b, SQLiteValue<'b>, Table, R>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Inserts the rows of any SQL value, such as a raw `sql!` query, with no
    /// compile-time column checks.
    #[inline]
    pub fn select_raw<Q>(
        self,
        query: Q,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Table: SQLiteTable<'b>,
        Q: ToSQL<'b, SQLiteValue<'b>>,
    {
        let builder = self.builder.select_raw(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table, Targets>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertColumnsSet<Targets>, Table>,
        InsertColumnsSet<Targets>,
    >
where
    Table: SQLiteTable<'b> + drizzle_core::InsertSelectTable,
{
    /// Inserts the rows of a `SELECT` query into the columns named by
    /// [`columns`](Self::columns).
    ///
    /// The query's output must match those columns in order and type; this is
    /// checked at compile time.
    #[inline]
    pub fn select<Q, R, RequiredProof, ScopeProof, AggProof>(
        self,
        query: Q,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: IntoSelectQuery<'b, Schema, R>,
        Q::Marker: drizzle_core::PartialInsertSelectCompatible<'b, SQLiteValue<'b>, Targets>
            + drizzle_core::MarkerScopeValidFor<ScopeProof>
            + drizzle_core::MarkerAggValidFor<Q::Grouped, AggProof>,
    {
        let builder = self.builder.select(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }

    /// Inserts the rows of any SQL value into the columns named by
    /// [`columns`](Self::columns), with no compile-time check on the query's
    /// output.
    #[inline]
    pub fn select_raw<Q, RequiredProof>(
        self,
        query: Q,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
    where
        Targets: drizzle_core::IncludesRequired<Table::RequiredColumns, RequiredProof>,
        Q: ToSQL<'b, SQLiteValue<'b>>,
    {
        let builder = self.builder.select_raw(query);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertValuesSet, Table>,
        InsertValuesSet,
    >
where
    Table: SQLiteTable<'b>,
{
    /// Starts an `ON CONFLICT (target)` clause; finish it with
    /// [`do_nothing`](DrizzleOnConflictBuilder::do_nothing) or
    /// [`do_update`](DrizzleOnConflictBuilder::do_update).
    ///
    /// `target` is a column or tuple of columns covered by a primary key or
    /// unique constraint.
    pub fn on_conflict<C: ConflictTarget<Table>>(
        self,
        target: C,
    ) -> DrizzleOnConflictBuilder<'a, 'b, Runner, Schema, Table> {
        DrizzleOnConflictBuilder {
            runner: self.runner,
            builder: self.builder.on_conflict(target),
        }
    }

    /// Skips any row that would violate a constraint: `ON CONFLICT DO NOTHING`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let query = db.insert(users).value(InsertUsers::new("Alex Smith", 26).with_id(1)).on_conflict_do_nothing();
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("id", "name", "age") VALUES (?, ?, ?) ON CONFLICT DO NOTHING"#,
    /// );
    /// assert_eq!(query.execute()?, 0);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn on_conflict_do_nothing(
        self,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    > {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.on_conflict_do_nothing(),
            state: PhantomData,
        }
    }

    /// Returns columns of the inserted rows: `RETURNING ...`.
    ///
    /// Run it with `.all()` or `.get()` to read them.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// let query = db.insert(users).value(InsertUsers::new("Dana", 41)).returning(users.id);
    /// assert_eq!(
    ///     query.to_sql().sql(),
    ///     r#"INSERT INTO "users" ("name", "age") VALUES (?, ?) RETURNING "users"."id""#,
    /// );
    /// let id: i64 = query.get()?;
    /// assert_eq!(id, 4);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<
            'b,
            Schema,
            InsertReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        InsertReturningSet,
    >
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
{
    /// Returns columns of the inserted rows: `RETURNING ...`.
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<
            'b,
            Schema,
            InsertReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        InsertReturningSet,
    >
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertDoUpdateSet, Table>,
        InsertDoUpdateSet,
    >
{
    /// Updates only conflicting rows that match `condition`:
    /// `DO UPDATE SET ... WHERE condition`.
    pub fn r#where<E, ScopeProof>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<'b, Schema, InsertOnConflictSet, Table>,
        InsertOnConflictSet,
    >
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'b, SQLiteValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        DrizzleBuilder {
            runner: self.runner,
            builder: self.builder.r#where(condition),
            state: PhantomData,
        }
    }

    /// Returns columns of the inserted or updated rows: `RETURNING ...`.
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        InsertBuilder<
            'b,
            Schema,
            InsertReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        InsertReturningSet,
    >
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateInitial, Table>,
        UpdateInitial,
    >
where
    Table: SQLiteTable<'b>,
{
    /// Sets the columns to change, from an `Update*` model.
    ///
    /// Only the fields set with `with_*` are written.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::eq;
    ///
    /// db.update(users)
    ///     .set(UpdateUsers::default().with_age(27).with_email("alex@new.example"))
    ///     .r#where(eq(users.id, 1))
    ///     .execute()?;
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    #[inline]
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    > {
        let builder = self.builder.set(values);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateSetClauseSet, Table>,
        UpdateSetClauseSet,
    >
{
    /// Updates only rows matching `condition`.
    pub fn r#where<E, ScopeProof>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'b, SQLiteValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, Table>
    DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<'b, Schema, UpdateWhereSet, Table>,
        UpdateWhereSet,
    >
{
    /// Returns columns of the updated rows: `RETURNING ...`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::eq;
    ///
    /// let ages: Vec<i64> = db
    ///     .update(users)
    ///     .set(UpdateUsers::default().with_age(27))
    ///     .r#where(eq(users.id, 1))
    ///     .returning(users.age)
    ///     .all()?;
    /// assert_eq!(ages, vec![27]);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        UpdateBuilder<
            'b,
            Schema,
            UpdateReturningSet,
            Table,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<Table, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<Table>>::Row,
        >,
        UpdateReturningSet,
    >
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources: drizzle_core::scope::SourcesIn<drizzle_core::Cons<Table, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<Table>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, T>
    DrizzleBuilder<'a, Runner, Schema, DeleteBuilder<'b, Schema, DeleteInitial, T>, DeleteInitial>
where
    T: SQLiteTable<'b>,
{
    /// Deletes only rows matching `condition`.
    pub fn r#where<E, ScopeProof>(
        self,
        condition: E,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        DeleteBuilder<'b, Schema, DeleteWhereSet, T>,
        DeleteWhereSet,
    >
    where
        E: drizzle_core::expr::ExprSources,
        E::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        E: drizzle_core::expr::Expr<'b, SQLiteValue<'b>>,
        E::SQLType: drizzle_core::types::BooleanLike,
    {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, 'b, Runner, Schema, T>
    DrizzleBuilder<'a, Runner, Schema, DeleteBuilder<'b, Schema, DeleteWhereSet, T>, DeleteWhereSet>
{
    /// Returns columns of the deleted rows: `RETURNING ...`.
    ///
    /// # Examples
    ///
    /// ```
    /// # #[cfg(feature = "rusqlite")]
    /// # fn main() -> drizzle::Result<()> {
    /// # mod app { include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/readme/sqlite.rs")); }
    /// # use app::*;
    /// # use drizzle::core::ToSQL;
    /// # let (db, Schema { users, posts, comments }) = app::database()?;
    /// # let _ = (&users, &posts, &comments);
    /// use drizzle::core::expr::eq;
    ///
    /// let gone: Vec<String> = db
    ///     .delete(comments)
    ///     .r#where(eq(comments.id, 1))
    ///     .returning(comments.body)
    ///     .all()?;
    /// assert_eq!(gone, vec!["Nice post".to_string()]);
    /// # Ok(())
    /// # }
    /// # #[cfg(not(feature = "rusqlite"))]
    /// # fn main() {}
    /// ```
    pub fn returning<Columns, ScopeProof>(
        self,
        columns: Columns,
    ) -> DrizzleBuilder<
        'a,
        Runner,
        Schema,
        DeleteBuilder<
            'b,
            Schema,
            DeleteReturningSet,
            T,
            drizzle_core::Scoped<Columns::Marker, drizzle_core::Cons<T, drizzle_core::Nil>>,
            <Columns::Marker as drizzle_core::ResolveRow<T>>::Row,
        >,
        DeleteReturningSet,
    >
    where
        Columns: drizzle_core::expr::ExprSources,
        Columns::Sources:
            drizzle_core::scope::SourcesIn<drizzle_core::Cons<T, drizzle_core::Nil>, ScopeProof>,
        Columns: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
        Columns::Marker: drizzle_core::ResolveRow<T>,
    {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            runner: self.runner,
            builder,
            state: PhantomData,
        }
    }
}
