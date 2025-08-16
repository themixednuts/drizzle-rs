// Driver modules
#[cfg(feature = "rusqlite")]
pub mod rusqlite;

#[cfg(feature = "turso")]
pub mod turso;

#[cfg(feature = "libsql")]
pub mod libsql;

use drizzle_core::ToSQL;
use drizzle_core::traits::{IsInSchema, SQLTable};
use paste::paste;
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

// Re-export the appropriate Drizzle type based on features
#[cfg(feature = "rusqlite")]
pub use rusqlite::Drizzle;

#[cfg(feature = "turso")]
pub use turso::Drizzle;

#[cfg(feature = "libsql")]
pub use libsql::Drizzle;

//------------------------------------------------------------------------------
// DrizzleBuilder - Builder with Type State Pattern
//------------------------------------------------------------------------------

#[derive(Debug)]
pub struct DrizzleBuilder<'a, Schema, Builder, State> {
    drizzle: &'a Drizzle<Schema>,
    builder: Builder,
    state: PhantomData<(Schema, State)>,
}

// Generic prepare method for all drivers
impl<'a, S, Schema, State, Table>
    DrizzleBuilder<'a, S, QueryBuilder<'a, Schema, State, Table>, State>
where
    State: builder::ExecutableState,
{
    /// Creates a prepared statement that can be executed multiple times
    pub fn prepare(self) -> sqlite::builder::prepared::PreparedStatement<'a> {
        self.builder.prepare()
    }
}

//------------------------------------------------------------------------------
// Drizzle Query Building Methods
//------------------------------------------------------------------------------

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
        let builder = self.builder.from(table);
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

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectOrderSet, T>,
        select::SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
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
    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectOrderSet, T>,
        select::SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
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

    pub fn order_by<TOrderBy>(
        self,
        expressions: TOrderBy,
    ) -> DrizzleBuilder<
        'a,
        Schema,
        SelectBuilder<'a, Schema, select::SelectOrderSet, T>,
        select::SelectOrderSet,
    >
    where
        TOrderBy: drizzle_core::ToSQL<'a, SQLiteValue<'a>>,
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
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
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
        columns: impl ToSQL<'a, SQLiteValue<'a>>,
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
