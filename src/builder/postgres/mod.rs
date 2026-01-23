#![cfg(feature = "postgres")]

macro_rules! postgres_builder_constructors {
    () => {
        /// Creates a SELECT query builder.
        pub fn select<'a, 'b, T>(
            &'a self,
            query: T,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
        where
            T: ToSQL<'b, PostgresValue<'b>>,
        {
            let builder = QueryBuilder::new::<Schema>().select(query);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT query builder.
        pub fn select_distinct<'a, 'b, T>(
            &'a self,
            query: T,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
        where
            T: ToSQL<'b, PostgresValue<'b>>,
        {
            let builder = QueryBuilder::new::<Schema>().select_distinct(query);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT ON query builder.
        pub fn select_distinct_on<'a, 'b, On, Columns>(
            &'a self,
            on: On,
            columns: Columns,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
        where
            On: ToSQL<'b, PostgresValue<'b>>,
            Columns: ToSQL<'b, PostgresValue<'b>>,
        {
            let builder = QueryBuilder::new::<Schema>().select_distinct_on(on, columns);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates an INSERT query builder.
        pub fn insert<'a, 'b, Table>(
            &'a self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'b, Schema, InsertInitial, Table>, InsertInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().insert(table);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates an UPDATE query builder.
        pub fn update<'a, 'b, Table>(
            &'a self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().update(table);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates a DELETE query builder.
        pub fn delete<'a, 'b, Table>(
            &'a self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteInitial, Table>, DeleteInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().delete(table);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates a query with CTE (Common Table Expression).
        pub fn with<'a, 'b, C>(
            &'a self,
            cte: C,
        ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, builder::CTEInit>, builder::CTEInit>
        where
            C: builder::CTEDefinition<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().with(cte);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }
    };
    (mut) => {
        /// Creates a SELECT query builder.
        pub fn select<'a, 'b, T>(
            &'a mut self,
            query: T,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
        where
            T: ToSQL<'b, PostgresValue<'b>>,
        {
            let builder = QueryBuilder::new::<Schema>().select(query);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT query builder.
        pub fn select_distinct<'a, 'b, T>(
            &'a mut self,
            query: T,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
        where
            T: ToSQL<'b, PostgresValue<'b>>,
        {
            let builder = QueryBuilder::new::<Schema>().select_distinct(query);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT ON query builder.
        pub fn select_distinct_on<'a, 'b, On, Columns>(
            &'a mut self,
            on: On,
            columns: Columns,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial>, SelectInitial>
        where
            On: ToSQL<'b, PostgresValue<'b>>,
            Columns: ToSQL<'b, PostgresValue<'b>>,
        {
            let builder = QueryBuilder::new::<Schema>().select_distinct_on(on, columns);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates an INSERT query builder.
        pub fn insert<'a, 'b, Table>(
            &'a mut self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'b, Schema, InsertInitial, Table>, InsertInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().insert(table);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates an UPDATE query builder.
        pub fn update<'a, 'b, Table>(
            &'a mut self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().update(table);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates a DELETE query builder.
        pub fn delete<'a, 'b, Table>(
            &'a mut self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteInitial, Table>, DeleteInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().delete(table);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }

        /// Creates a query with CTE (Common Table Expression).
        pub fn with<'a, 'b, C>(
            &'a mut self,
            cte: C,
        ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, builder::CTEInit>, builder::CTEInit>
        where
            C: builder::CTEDefinition<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().with(cte);
            DrizzleBuilder {
                drizzle: self,
                builder,
                state: PhantomData,
            }
        }
    };
}

// Driver modules
#[cfg(feature = "postgres-sync")]
pub(crate) mod postgres_sync;

#[cfg(feature = "tokio-postgres")]
pub(crate) mod tokio_postgres;

pub(crate) mod common;
pub(crate) mod prepared_common;

// #[cfg(feature = "sqlx-postgres")]
// pub(crate) mod sqlx;
