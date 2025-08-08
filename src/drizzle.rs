use drizzle_core::ToSQL;
use drizzle_core::traits::{IsInSchema, SQLTable};
use paste::paste;
use rusqlite::{Connection, params_from_iter};
use std::marker::PhantomData;

#[cfg(feature = "sqlite")]
use sqlite::{
    SQLiteValue,
    builder::{
        self, QueryBuilder,
        delete::{self, DeleteBuilder},
        insert::{self, InsertBuilder},
        select::{self, SelectBuilder},
        update::{self, UpdateBuilder},
    },
};

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
            pub fn [<$type _join>]<U>(
                self,
                table: U,
                on_condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
            ) -> DrizzleBuilder<
                'a,
                Schema,
                SelectBuilder<'a, Schema, select::SelectJoinSet, T>,
                select::SelectJoinSet,
            >
            where
                U: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
            {
                let builder = self.builder.[<$type _join>](table, on_condition);
                DrizzleBuilder {
                    drizzle: self.drizzle,
                    builder,
                    state: PhantomData,
                }
            }
        }
    };
}

//------------------------------------------------------------------------------
// Drizzle - Main Connection Wrapper
//------------------------------------------------------------------------------

pub trait DrizzleState {}

pub struct DrizzleInitial;
impl DrizzleState for DrizzleInitial {}

/// Drizzle instance that provides access to the database and query builder.
#[derive(Debug)]
pub struct Drizzle<Schema = ()> {
    #[cfg(feature = "rusqlite")]
    conn: Connection,
    _schema: PhantomData<Schema>,
}

impl Drizzle {
    #[cfg(feature = "rusqlite")]
    pub const fn new<S>(conn: Connection) -> Drizzle<S> {
        Drizzle {
            conn,
            _schema: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// DrizzleBuilder - Builder with Type State Pattern
//------------------------------------------------------------------------------

#[derive(Debug)]
pub struct DrizzleBuilder<'a, Schema, Builder, State> {
    drizzle: &'a Drizzle<Schema>,
    builder: Builder,
    state: PhantomData<(Schema, State)>,
}

//------------------------------------------------------------------------------
// Drizzle Query Building Methods
//------------------------------------------------------------------------------

impl<Schema> Drizzle<Schema> {
    /// Gets a mutable reference to the underlying connection
    #[cfg(feature = "rusqlite")]
    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    #[cfg(feature = "rusqlite")]
    pub fn mut_conn(&mut self) -> &mut Connection {
        &mut self.conn
    }

    /// Creates a SELECT query builder.
    #[cfg(feature = "sqlite")]
    pub fn select<'a, T>(
        &'a self,
        query: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectInitial>,
        select::SelectInitial,
    >
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        use sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().select(query);

        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an INSERT query builder.
    #[cfg(feature = "sqlite")]
    pub fn insert<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertInitial, T>,
        insert::InsertInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>> + 'a,
    {
        use sqlite::builder::QueryBuilder;

        let builder = QueryBuilder::new::<Schema>().insert::<T>();
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates an UPDATE query builder.
    #[cfg(feature = "sqlite")]
    pub fn update<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, update::UpdateInitial, T>,
        update::UpdateInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = QueryBuilder::new::<Schema>().update::<T>();
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    /// Creates a DELETE query builder.
    #[cfg(feature = "sqlite")]
    pub fn delete<'a, T>(
        &'a self,
        table: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        DeleteBuilder<'a, Schema, delete::DeleteInitial, T>,
        delete::DeleteInitial,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = QueryBuilder::new::<Schema>().delete::<T>();
        DrizzleBuilder {
            drizzle: self,
            builder,
            state: PhantomData,
        }
    }

    #[cfg(feature = "rusqlite")]
    pub fn execute<'a, T>(&'a self, query: T) -> rusqlite::Result<usize>
    where
        T: ToSQL<'a, SQLiteValue<'a>>,
    {
        let query = query.to_sql();
        let sql = query.sql();
        let params = query.params();

        self.conn.execute(&sql, params_from_iter(params))
    }
}

//------------------------------------------------------------------------------
// SELECT Builder Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, Schema>
    DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectInitial>,
        select::SelectInitial,
    >
{
    pub fn from<T>(
        self,
        table: T,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectFromSet, T>,
        select::SelectFromSet,
    >
    where
        T: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.from::<T>();
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

#[cfg(feature = "sqlite")]
impl<'a, Schema, T>
    DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectFromSet, T>,
        select::SelectFromSet,
    >
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectWhereSet, T>,
        select::SelectWhereSet,
    > {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectLimitSet, T>,
        select::SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn order_by<TSQL, TIter>(
        self,
        expressions: TIter,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectOrderSet, T>,
        select::SelectOrderSet,
    >
    where
        TSQL: ToSQL<'a, SQLiteValue<'a>>,
        TIter: IntoIterator<Item = (TSQL, drizzle_core::OrderBy)>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn join<U>(
        self,
        table: U,
        on_condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectJoinSet, T>,
        select::SelectJoinSet,
    >
    where
        U: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.join(table, on_condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
    join_impl!();
}

#[cfg(feature = "sqlite")]
impl<'a, Schema, T>
    DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectJoinSet, T>,
        select::SelectJoinSet,
    >
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectWhereSet, T>,
        select::SelectWhereSet,
    > {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
    pub fn order_by<TSQL, TIter>(
        self,
        expressions: TIter,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectOrderSet, T>,
        select::SelectOrderSet,
    >
    where
        TSQL: ToSQL<'a, SQLiteValue<'a>>,
        TIter: IntoIterator<Item = (TSQL, drizzle_core::OrderBy)>,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn join<U>(
        self,
        table: U,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectJoinSet, T>,
        select::SelectJoinSet,
    >
    where
        U: IsInSchema<Schema> + SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.join(table, condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
    join_impl!();
}

#[cfg(feature = "sqlite")]
impl<'a, Schema, T>
    DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectWhereSet, T>,
        select::SelectWhereSet,
    >
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectLimitSet, T>,
        select::SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    pub fn order_by<TI>(
        self,
        expressions: TI,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectOrderSet, T>,
        select::SelectOrderSet,
    >
    where
        TI: IntoIterator<
            Item = (
                drizzle_core::SQL<'a, SQLiteValue<'a>>,
                drizzle_core::OrderBy,
            ),
        >,
    {
        let builder = self.builder.order_by(expressions);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, Schema, T>
    DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectLimitSet, T>,
        select::SelectLimitSet,
    >
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    pub fn offset(
        self,
        offset: usize,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectOffsetSet, T>,
        select::SelectOffsetSet,
    > {
        let builder = self.builder.offset(offset);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

impl<'a, Schema, T>
    DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectOrderSet, T>,
        select::SelectOrderSet,
    >
where
    T: SQLTable<'a, SQLiteValue<'a>>,
{
    pub fn limit(
        self,
        limit: usize,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectLimitSet, T>,
        select::SelectLimitSet,
    > {
        let builder = self.builder.limit(limit);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// INSERT Builder Implementation
//------------------------------------------------------------------------------

impl<'a, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertInitial, Table>,
        insert::InsertInitial,
    >
{
    #[cfg(feature = "sqlite")]
    pub fn values(
        self,
        values: impl IntoIterator<Item = Table::Insert>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertValuesSet, Table>,
        insert::InsertValuesSet,
    >
    where
        Table: SQLTable<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.values(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// INSERT ValuesSet State Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertValuesSet, Table>,
        insert::InsertValuesSet,
    >
where
    Table: SQLTable<'a, SQLiteValue<'a>>,
{
    /// Adds conflict resolution clause
    pub fn on_conflict<TI>(
        self,
        conflict: insert::Conflict<'a, TI>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertOnConflictSet, Table>,
        insert::InsertOnConflictSet,
    >
    where
        TI: IntoIterator,
        TI::Item: ToSQL<'a, SQLiteValue<'a>>,
    {
        let builder = self.builder.on_conflict(conflict);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }

    /// Adds RETURNING clause
    pub fn returning(
        self,
        columns: Vec<drizzle_core::SQL<'a, SQLiteValue<'a>>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertReturningSet, Table>,
        insert::InsertReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// INSERT OnConflict State Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertOnConflictSet, Table>,
        insert::InsertOnConflictSet,
    >
{
    /// Adds RETURNING clause after ON CONFLICT
    pub fn returning(
        self,
        columns: Vec<drizzle_core::SQL<'a, SQLiteValue<'a>>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        InsertBuilder<'a, Schema, insert::InsertReturningSet, Table>,
        insert::InsertReturningSet,
    > {
        let builder = self.builder.returning(columns);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// UPDATE Builder Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, update::UpdateInitial, Table>,
        update::UpdateInitial,
    >
where
    Table: SQLTable<'a, SQLiteValue<'a>>,
{
    pub fn set(
        self,
        values: Table::Update,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, update::UpdateSetClauseSet, Table>,
        update::UpdateSetClauseSet,
    > {
        let builder = self.builder.set(values);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

#[cfg(feature = "sqlite")]
impl<'a, Schema, Table>
    DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, update::UpdateSetClauseSet, Table>,
        update::UpdateSetClauseSet,
    >
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        UpdateBuilder<'a, Schema, update::UpdateWhereSet, Table>,
        update::UpdateWhereSet,
    > {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// DELETE Builder Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, S, T>
    DrizzleBuilder<'a, S, DeleteBuilder<'a, S, delete::DeleteInitial, T>, delete::DeleteInitial>
where
    T: IsInSchema<S> + SQLTable<'a, SQLiteValue<'a>>,
{
    pub fn r#where(
        self,
        condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
    ) -> DrizzleBuilder<
        'a,
        S,
        DeleteBuilder<'a, S, delete::DeleteWhereSet, T>,
        delete::DeleteWhereSet,
    > {
        let builder = self.builder.r#where(condition);
        DrizzleBuilder {
            drizzle: self.drizzle,
            builder,
            state: PhantomData,
        }
    }
}

//------------------------------------------------------------------------------
// SQL Generation Implementation
//------------------------------------------------------------------------------

#[cfg(feature = "sqlite")]
impl<'a, S, T, State> ToSQL<'a, SQLiteValue<'a>> for DrizzleBuilder<'a, S, T, State>
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
impl<'a, S, State, T> DrizzleBuilder<'a, S, SelectBuilder<'a, S, State, T>, State>
where
    State: builder::ExecutableState,
{
    pub fn all<R>(self) -> drizzle_core::error::Result<Vec<R>>
    where
        R: for<'r> TryFrom<&'r ::rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r ::rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        self.builder.all(&self.drizzle.conn)
    }

    pub fn get<R>(self) -> drizzle_core::error::Result<R>
    where
        R: for<'r> TryFrom<&'r rusqlite::Row<'r>>,
        for<'r> <R as TryFrom<&'r rusqlite::Row<'r>>>::Error:
            Into<drizzle_core::error::DrizzleError>,
    {
        self.builder.get(&self.drizzle.conn)
    }
}

// Add execution methods for INSERT - ValuesSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<'a, S, InsertBuilder<'a, S, insert::InsertValuesSet, T>, insert::InsertValuesSet>
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}

// Add execution methods for INSERT - ReturningSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        InsertBuilder<'a, S, insert::InsertReturningSet, T>,
        insert::InsertReturningSet,
    >
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}

// Add execution methods for INSERT - OnConflictSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        InsertBuilder<'a, S, insert::InsertOnConflictSet, T>,
        insert::InsertOnConflictSet,
    >
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}

// Add execution methods for UPDATE - SetClauseSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        UpdateBuilder<'a, S, update::UpdateSetClauseSet, T>,
        update::UpdateSetClauseSet,
    >
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}

// Add execution methods for UPDATE - WhereSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<'a, S, UpdateBuilder<'a, S, update::UpdateWhereSet, T>, update::UpdateWhereSet>
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}

// Add execution methods for UPDATE - ReturningSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        UpdateBuilder<'a, S, update::UpdateReturningSet, T>,
        update::UpdateReturningSet,
    >
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}

// Add execution methods for DELETE - Initial state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<'a, S, DeleteBuilder<'a, S, delete::DeleteInitial, T>, delete::DeleteInitial>
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}

// Add execution methods for DELETE - WhereSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<'a, S, DeleteBuilder<'a, S, delete::DeleteWhereSet, T>, delete::DeleteWhereSet>
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}

// Add execution methods for DELETE - ReturningSet state
#[cfg(feature = "rusqlite")]
impl<'a, S, T>
    DrizzleBuilder<
        'a,
        S,
        DeleteBuilder<'a, S, delete::DeleteReturningSet, T>,
        delete::DeleteReturningSet,
    >
{
    pub fn execute(self) -> drizzle_core::error::Result<usize> {
        self.builder.execute(&self.drizzle.conn)
    }
}
