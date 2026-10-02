#![cfg(feature = "mysql")]

macro_rules! mysql_builder_constructors {
    ($runner:ty, [$($receiver:tt)*], $this:ident) => {
        /// Starts a `SELECT` query.
        ///
        /// `columns` is what to select: `()` for every column of the `FROM` table, a
        /// column, a tuple of columns and expressions, or a `FromRow` type's
        /// `::Select` marker. Follow it with `.from(..)`.
        pub fn select<'db, 'q, T>(
            $($receiver)*,
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
            DrizzleBuilder::new($this, QueryBuilder::new::<Schema>().select(columns))
        }

        /// Starts a `SELECT DISTINCT` query, which drops duplicate rows.
        pub fn select_distinct<'db, 'q, T>(
            $($receiver)*,
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
            DrizzleBuilder::new($this, QueryBuilder::new::<Schema>().select_distinct(columns))
        }

        /// Starts an `INSERT` into `table`.
        ///
        /// Follow it with `.values(..)` and `Insert*` models, or with `.select(..)`
        /// to insert a query's rows.
        pub fn insert<'db, 'q, Table>(
            $($receiver)*,
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
            DrizzleBuilder::new($this, QueryBuilder::new::<Schema>().insert(table))
        }

        /// Starts an `UPDATE` of `table`.
        ///
        /// Follow it with `.set(..)` and an `Update*` model. Without `.r#where(..)`,
        /// every row is updated.
        pub fn update<'db, 'q, Table>(
            $($receiver)*,
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
            DrizzleBuilder::new($this, QueryBuilder::new::<Schema>().update(table))
        }

        /// Starts a `DELETE` from `table`. Without `.r#where(..)`, every row is
        /// deleted.
        pub fn delete<'db, 'q, Table>(
            $($receiver)*,
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
            DrizzleBuilder::new($this, QueryBuilder::new::<Schema>().delete(table))
        }

        /// Starts a query with a common table expression: `WITH name AS (...)`.
        pub fn with<'db, 'q, C>(
            $($receiver)*,
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
            DrizzleBuilder::new($this, QueryBuilder::new::<Schema>().with(cte))
        }
    };
}

pub mod common;

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
pub(crate) mod driver_common;

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
mod introspect;

#[cfg(any(feature = "mysql-sync", feature = "mysql-async"))]
mod migration;

#[cfg(feature = "mysql-async")]
pub mod mysql_async;

#[cfg(feature = "mysql-sync")]
pub mod mysql_sync;
