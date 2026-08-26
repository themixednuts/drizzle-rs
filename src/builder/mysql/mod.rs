#![cfg(feature = "mysql")]

macro_rules! mysql_builder_constructors {
    ($connection:ty) => {
        /// Creates a typed `SELECT` query.
        pub fn select<'db, 'q, T>(
            &'db mut self,
            columns: T,
        ) -> DrizzleBuilder<
            'db,
            $connection,
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().select(columns))
        }

        /// Creates a typed `SELECT DISTINCT` query.
        pub fn select_distinct<'db, 'q, T>(
            &'db mut self,
            columns: T,
        ) -> DrizzleBuilder<
            'db,
            $connection,
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().select_distinct(columns))
        }

        /// Creates a typed `INSERT` query.
        pub fn insert<'db, 'q, Table>(
            &'db mut self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            $connection,
            Schema,
            InsertBuilder<'q, Schema, InsertInitial, Table>,
            InsertInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().insert(table))
        }

        /// Creates a typed `UPDATE` query.
        pub fn update<'db, 'q, Table>(
            &'db mut self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            $connection,
            Schema,
            UpdateBuilder<'q, Schema, UpdateInitial, Table>,
            UpdateInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().update(table))
        }

        /// Creates a typed `DELETE` query.
        pub fn delete<'db, 'q, Table>(
            &'db mut self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            $connection,
            Schema,
            DeleteBuilder<'q, Schema, DeleteInitial, Table>,
            DeleteInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().delete(table))
        }

        /// Starts a query with a common table expression.
        pub fn with<'db, 'q, C>(
            &'db mut self,
            cte: &C,
        ) -> DrizzleBuilder<
            'db,
            $connection,
            Schema,
            QueryBuilder<'q, Schema, builder::CTEInit>,
            builder::CTEInit,
        >
        where
            C: builder::CTEDefinition<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().with(cte))
        }
    };
}

macro_rules! mysql_shared_builder_constructors {
    ($runner:ty) => {
        pub fn select<'db, 'q, T>(
            &'db self,
            columns: T,
        ) -> DrizzleBuilder<
            'db,
            $runner,
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().select(columns))
        }

        pub fn select_distinct<'db, 'q, T>(
            &'db self,
            columns: T,
        ) -> DrizzleBuilder<
            'db,
            $runner,
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, MySQLValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().select_distinct(columns))
        }

        pub fn insert<'db, 'q, Table>(
            &'db self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            $runner,
            Schema,
            InsertBuilder<'q, Schema, InsertInitial, Table>,
            InsertInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().insert(table))
        }

        pub fn update<'db, 'q, Table>(
            &'db self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            $runner,
            Schema,
            UpdateBuilder<'q, Schema, UpdateInitial, Table>,
            UpdateInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().update(table))
        }

        pub fn delete<'db, 'q, Table>(
            &'db self,
            table: Table,
        ) -> DrizzleBuilder<
            'db,
            $runner,
            Schema,
            DeleteBuilder<'q, Schema, DeleteInitial, Table>,
            DeleteInitial,
        >
        where
            Table: MySQLTable<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().delete(table))
        }

        pub fn with<'db, 'q, C>(
            &'db self,
            cte: &C,
        ) -> DrizzleBuilder<
            'db,
            $runner,
            Schema,
            QueryBuilder<'q, Schema, builder::CTEInit>,
            builder::CTEInit,
        >
        where
            C: builder::CTEDefinition<'q>,
        {
            DrizzleBuilder::new(self, QueryBuilder::new::<Schema>().with(cte))
        }
    };
}

pub mod common;

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub(crate) mod driver_common;

#[cfg(feature = "mysql-async")]
pub mod mysql_async;

#[cfg(feature = "mysql-sync")]
pub mod mysql_sync;
