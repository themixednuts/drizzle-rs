// src/drizzle.rs
use drizzle_core::traits::{IsInSchema, SQLTable};
use drizzle_core::{SQL, ToSQL};
#[cfg(feature = "sqlite")]
use sqlite::SQLiteValue;
use sqlite::builder::ExecutableState;
use std::marker::PhantomData;

#[cfg(feature = "sqlite")]
use sqlite::builder::{
    delete, delete::DeleteBuilder, insert, insert::InsertBuilder, select, select::SelectBuilder,
    update, update::UpdateBuilder,
};

//------------------------------------------------------------------------------
// Drizzle - Main Connection Wrapper
//------------------------------------------------------------------------------

pub trait DrizzleState {}

pub struct DrizzleInitial;
impl DrizzleState for DrizzleInitial {}

/// Drizzle instance that provides access to the database and query builder.
#[derive(Debug, Clone)]
pub struct Drizzle<'a, Conn: 'a, Schema> {
    conn: Conn,
    #[cfg(feature = "sqlite")]
    builder: sqlite::builder::QueryBuilder<'a, Schema, sqlite::builder::BuilderInit>,
}

impl<'a, Conn: 'a, Schema> Drizzle<'a, Conn, Schema> {
    #[cfg(feature = "sqlite")]
    pub fn new(
        conn: Conn,
        builder: sqlite::builder::QueryBuilder<'a, Schema, sqlite::builder::BuilderInit>,
    ) -> Drizzle<'a, Conn, Schema> {
        Drizzle { conn, builder }
    }
}

//------------------------------------------------------------------------------
// DrizzleBuilder - Builder with Type State Pattern
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
#[derive(Debug)]
pub struct DrizzleBuilder<'a, Conn: 'a, Schema, T, State> {
    drizzle: &'a mut Drizzle<'a, Conn, Schema>,
    builder: T,
    _state: PhantomData<State>,
}

//------------------------------------------------------------------------------
// Drizzle Query Building Methods
//------------------------------------------------------------------------------

impl<'a, Conn, Schema> Drizzle<'a, Conn, Schema> {
    /// Gets a mutable reference to the underlying connection
    pub fn get_conn(&mut self) -> &mut Conn {
        &mut self.conn
    }

    /// Creates a SELECT query builder.
    #[cfg(feature = "sqlite")]
    pub fn select<const N: usize>(
        &'a mut self,
        columns: [SQL<'a, SQLiteValue<'a>>; N],
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        SelectBuilder<'a, Schema, select::SelectInitial>,
        select::SelectInitial,
    > {
        let select_builder = self.builder.select(columns);
        DrizzleBuilder {
            drizzle: self,
            builder: select_builder,
            _state: PhantomData,
        }
    }

    /// Creates an INSERT query builder.
    #[cfg(feature = "sqlite")]
    pub fn insert<T>(
        &'a mut self,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertInitial, T>,
        insert::InsertInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a>,
    {
        let insert_builder = self.builder.insert::<T>();
        DrizzleBuilder {
            drizzle: self,
            builder: insert_builder,
            _state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    #[cfg(feature = "sqlite")]
    pub fn update<T>(
        &'a mut self,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        UpdateBuilder<'a, Schema, update::UpdateInitial, T>,
        update::UpdateInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a>,
    {
        let update_builder = self.builder.update::<T>();
        DrizzleBuilder {
            drizzle: self,
            builder: update_builder,
            _state: PhantomData,
        }
    }

    /// Creates a DELETE query builder.
    #[cfg(feature = "sqlite")]
    pub fn delete<T>(
        &'a mut self,
    ) -> DrizzleBuilder<
        'a,
        Conn,
        Schema,
        DeleteBuilder<'a, Schema, delete::DeleteInitial, T>,
        delete::DeleteInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a>,
    {
        let delete_builder = self.builder.delete::<T>();
        DrizzleBuilder {
            drizzle: self,
            builder: delete_builder,
            _state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// SELECT Builder Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, C, S>
    DrizzleBuilder<'a, C, S, SelectBuilder<'a, S, select::SelectInitial>, select::SelectInitial>
{
    pub fn from<T>(
        self,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        SelectBuilder<'a, S, select::SelectFromSet, T>,
        select::SelectFromSet,
    >
    where
        T: IsInSchema<S> + SQLTable<'a>,
    {
        let builder = self.builder.from::<T>();
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }
}

#[cfg(feature = "sqlite")]
impl<'a, C, S>
    DrizzleBuilder<'a, C, S, SelectBuilder<'a, S, select::SelectFromSet>, select::SelectFromSet>
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        SelectBuilder<'a, S, select::SelectWhereSet>,
        select::SelectWhereSet,
    > {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }

    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        SelectBuilder<'a, S, select::SelectLimitSet>,
        select::SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }

    pub fn order_by(
        self,
        expressions: Vec<(
            drizzle_core::SQL<'a, SQLiteValue<'a>>,
            drizzle_core::SortDirection,
        )>,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        SelectBuilder<'a, S, select::SelectOrderSet>,
        select::SelectOrderSet,
    > {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }
}

#[cfg(feature = "sqlite")]
impl<'a, C, S>
    DrizzleBuilder<'a, C, S, SelectBuilder<'a, S, select::SelectWhereSet>, select::SelectWhereSet>
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        SelectBuilder<'a, S, select::SelectLimitSet>,
        select::SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }

    pub fn order_by(
        self,
        expressions: Vec<(
            drizzle_core::SQL<'a, SQLiteValue<'a>>,
            drizzle_core::SortDirection,
        )>,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        SelectBuilder<'a, S, select::SelectOrderSet>,
        select::SelectOrderSet,
    > {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// INSERT Builder Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, C, S, T>
    DrizzleBuilder<'a, C, S, InsertBuilder<'a, S, insert::InsertInitial, T>, insert::InsertInitial>
{
    pub fn values(
        self,
        values: Vec<Vec<drizzle_core::SQL<'a, SQLiteValue<'a>>>>,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        InsertBuilder<'a, S, insert::InsertValuesSet, T>,
        insert::InsertValuesSet,
    > {
        let builder = self.builder.values(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }

    pub fn on_conflict(self, resolution: insert::ConflictResolution) -> Self {
        let builder = self.builder.on_conflict(resolution);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// UPDATE Builder Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, C, S, T>
    DrizzleBuilder<'a, C, S, UpdateBuilder<'a, S, update::UpdateInitial, T>, update::UpdateInitial>
{
    pub fn set(
        self,
        updates: Vec<(String, drizzle_core::SQL<'a, SQLiteValue<'a>>)>,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        UpdateBuilder<'a, S, update::UpdateSetClauseSet, T>,
        update::UpdateSetClauseSet,
    > {
        let builder = self.builder.set(updates);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }
}

#[cfg(feature = "sqlite")]
impl<'a, C, S, T>
    DrizzleBuilder<
        'a,
        C,
        S,
        UpdateBuilder<'a, S, update::UpdateSetClauseSet, T>,
        update::UpdateSetClauseSet,
    >
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        UpdateBuilder<'a, S, update::UpdateWhereSet, T>,
        update::UpdateWhereSet,
    > {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// DELETE Builder Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, C, S, T>
    DrizzleBuilder<'a, C, S, DeleteBuilder<'a, S, delete::DeleteInitial, T>, delete::DeleteInitial>
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        C,
        S,
        DeleteBuilder<'a, S, delete::DeleteWhereSet, T>,
        delete::DeleteWhereSet,
    > {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            _state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// SQL Generation Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, C, S, T, State> ToSQL<'a, SQLiteValue<'a>> for DrizzleBuilder<'a, C, S, T, State>
where
    T: ToSQL<'a, SQLiteValue<'a>>,
{
    fn to_sql(&self) -> drizzle_core::SQL<'a, SQLiteValue<'a>> {
        self.builder.to_sql()
    }
}

//------------------------------------------------------------------------------
// Execution Methods for RusQLite
//------------------------------------------------------------------------------

// Add execution methods for SELECT - SelectWhereSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, State> DrizzleBuilder<'a, ::rusqlite::Connection, S, SelectBuilder<'a, S, State>, State>
where
    State: ExecutableState,
{
    pub fn all<R>(&mut self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error: Into<::rusqlite::Error>,
    {
        self.builder.all(&mut self.drizzle.conn)
    }

    pub fn get<R>(&mut self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error: Into<::rusqlite::Error>,
    {
        self.builder.get(&mut self.drizzle.conn)
    }
}

// Add execution methods for INSERT - ValuesSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        ::rusqlite::Connection,
        S,
        InsertBuilder<'a, S, insert::InsertValuesSet, T>,
        insert::InsertValuesSet,
    >
{
    pub fn execute(&mut self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&mut self.drizzle.conn)
    }
}

// Add execution methods for INSERT - ReturningSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        ::rusqlite::Connection,
        S,
        InsertBuilder<'a, S, insert::InsertReturningSet, T>,
        insert::InsertReturningSet,
    >
{
    pub fn execute(&mut self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&mut self.drizzle.conn)
    }
}

// Add execution methods for UPDATE - SetClauseSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        ::rusqlite::Connection,
        S,
        UpdateBuilder<'a, S, update::UpdateSetClauseSet, T>,
        update::UpdateSetClauseSet,
    >
{
    pub fn execute(&mut self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&mut self.drizzle.conn)
    }
}

// Add execution methods for UPDATE - WhereSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        ::rusqlite::Connection,
        S,
        UpdateBuilder<'a, S, update::UpdateWhereSet, T>,
        update::UpdateWhereSet,
    >
{
    pub fn execute(&mut self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&mut self.drizzle.conn)
    }
}

// Add execution methods for UPDATE - ReturningSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        ::rusqlite::Connection,
        S,
        UpdateBuilder<'a, S, update::UpdateReturningSet, T>,
        update::UpdateReturningSet,
    >
{
    pub fn execute(&mut self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&mut self.drizzle.conn)
    }
}

// Add execution methods for DELETE - Initial state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        ::rusqlite::Connection,
        S,
        DeleteBuilder<'a, S, delete::DeleteInitial, T>,
        delete::DeleteInitial,
    >
{
    pub fn execute(&mut self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&mut self.drizzle.conn)
    }
}

// Add execution methods for DELETE - WhereSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        ::rusqlite::Connection,
        S,
        DeleteBuilder<'a, S, delete::DeleteWhereSet, T>,
        delete::DeleteWhereSet,
    >
{
    pub fn execute(&mut self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&mut self.drizzle.conn)
    }
}

// Add execution methods for DELETE - ReturningSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        ::rusqlite::Connection,
        S,
        DeleteBuilder<'a, S, delete::DeleteReturningSet, T>,
        delete::DeleteReturningSet,
    >
{
    pub fn execute(&mut self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&mut self.drizzle.conn)
    }
}

#[cfg(test)]
mod tests {
    use drizzle_core::expressions::conditions::eq;
    use drizzle_core::{Join, ToSQL};
    use procmacros::{SQLiteTable, drizzle, qb};
    extern crate self as drizzle_rs;

    #[cfg(feature = "rusqlite")]
    use rusqlite;

    #[SQLiteTable(name = "Users")]
    struct User {
        #[integer(primary)]
        id: i32,
        #[text]
        name: String,
        #[text]
        email: Option<String>,
    }

    #[SQLiteTable(name = "Posts")]
    struct Post {
        #[integer(primary)]
        id: i32,
        #[text]
        title: String,
    }

    #[SQLiteTable(name = "Comments")]
    struct Comment {
        #[integer(primary)]
        id: i32,
        #[text]
        content: String,
    }

    #[test]
    fn test_schema_macro() {
        // Create a schema with the User table using schema! macro
        let builder = qb!([User, Post]);

        let query = builder.select([()]).from::<User>();
        assert_eq!(query.to_sql().sql(), "SELECT * FROM Users");
    }

    #[cfg(feature = "rusqlite")]
    #[test]
    fn test_insert() {
        use drizzle_core::SQL;

        let conn = rusqlite::Connection::open_in_memory().unwrap();
        conn.execute(
            "CREATE TABLE Users (id INTEGER PRIMARY KEY, name TEXT, email TEXT)",
            [],
        )
        .unwrap();

        let mut db = drizzle!(conn, [User, Post]);
        db.insert::<User>()
            .values(sql!(name = "John Doe", email = "john@example.com"));
    }
}
