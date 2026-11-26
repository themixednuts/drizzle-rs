#![cfg(feature = "postgres")]

macro_rules! postgres_builder_constructors {
    () => {
        /// Starts a `SELECT` query.
        ///
        /// `query` is what to select: `()` for every column of the `FROM` table, a
        /// column, a tuple of columns and expressions, or a `FromRow` type's
        /// `::Select` marker. Follow it with `.from(..)`.
        pub fn select<'a, 'b, T>(
            &'a self,
            query: T,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>, SelectInitial>
        where
            T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            let builder = QueryBuilder::new::<Schema>().select(query);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts a `SELECT DISTINCT` query, which drops duplicate rows.
        pub fn select_distinct<'a, 'b, T>(
            &'a self,
            query: T,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>, SelectInitial>
        where
            T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            let builder = QueryBuilder::new::<Schema>().select_distinct(query);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts a `SELECT DISTINCT ON (on) ...` query, which keeps the first row
        /// of each set of rows that agree on `on` (pair it with an `ORDER BY` that
        /// starts with the same expressions).
        pub fn select_distinct_on<'a, 'b, On, Columns>(
            &'a self,
            on: On,
            columns: Columns,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial, (), Columns::Marker>, SelectInitial>
        where
            On: ToSQL<'b, PostgresValue<'b>>,
            Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            let builder = QueryBuilder::new::<Schema>().select_distinct_on(on, columns);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Counts the rows of `source`: `SELECT COUNT(*) FROM source`.
        ///
        /// `source` is anything `.from(..)` accepts. Add `.r#where(..)` to count
        /// only the matching rows; `.get()` returns the count as an `i64`. The
        /// query is also a scalar subquery: compare against it, or select it as
        /// a column, where it decodes as `Option<i64>` (name it with
        /// `alias(..)`). Its condition may read the outer query's columns to
        /// count per row.
        pub fn count<'a, 'b, Source>(
            &'a self,
            source: Source,
        ) -> DrizzleBuilder<'a, Schema, crate::builder::postgres::CountSelect<'b, Schema, Source>, drizzle_postgres::builder::SelectFromSet>
        where
            Source: ToSQL<'b, PostgresValue<'b>> + drizzle_core::ScopeEntry,
            crate::builder::CountMarker<'b, PostgresValue<'b>>: drizzle_core::ResolveRow<Source>,
        {
            let builder = QueryBuilder::new::<Schema>()
                .select(drizzle_core::expr::count(()))
                .from(source);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts an `INSERT` into `table`.
        ///
        /// Follow it with `.values(..)` and `Insert*` models, or with `.select(..)`
        /// to insert a query's rows.
        pub fn insert<'a, 'b, Table>(
            &'a self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'b, Schema, InsertInitial, Table>, InsertInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().insert(table);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts an `UPDATE` of `table`.
        ///
        /// Follow it with `.set(..)` and an `Update*` model, then `.r#where(..)`;
        /// `.r#where(true)` updates every row.
        pub fn update<'a, 'b, Table>(
            &'a self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().update(table);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts a `DELETE` from `table`. It runs once `.r#where(..)` picks the
        /// rows; `.r#where(true)` deletes every row.
        pub fn delete<'a, 'b, Table>(
            &'a self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteInitial, Table>, DeleteInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().delete(table);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts a query with a common table expression: `WITH name AS (...)`.
        ///
        /// Make the CTE with `.into_cte::<Tag>()` on a select query, then pass it
        /// here and select from it.
        pub fn with<'a, 'b, C>(
            &'a self,
            cte: &C,
        ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, builder::CTEInit>, builder::CTEInit>
        where
            C: builder::CTEDefinition<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().with(cte);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }
    };
    (mut) => {
        /// Starts a `SELECT` query.
        ///
        /// `query` is what to select: `()` for every column of the `FROM` table, a
        /// column, a tuple of columns and expressions, or a `FromRow` type's
        /// `::Select` marker. Follow it with `.from(..)`.
        pub fn select<'a, 'b, T>(
            &'a mut self,
            query: T,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>, SelectInitial>
        where
            T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            let builder = QueryBuilder::new::<Schema>().select(query);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts a `SELECT DISTINCT` query, which drops duplicate rows.
        pub fn select_distinct<'a, 'b, T>(
            &'a mut self,
            query: T,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>, SelectInitial>
        where
            T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            let builder = QueryBuilder::new::<Schema>().select_distinct(query);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts a `SELECT DISTINCT ON (on) ...` query, which keeps the first row
        /// of each set of rows that agree on `on` (pair it with an `ORDER BY` that
        /// starts with the same expressions).
        pub fn select_distinct_on<'a, 'b, On, Columns>(
            &'a mut self,
            on: On,
            columns: Columns,
        ) -> DrizzleBuilder<'a, Schema, SelectBuilder<'b, Schema, SelectInitial, (), Columns::Marker>, SelectInitial>
        where
            On: ToSQL<'b, PostgresValue<'b>>,
            Columns: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            let builder = QueryBuilder::new::<Schema>().select_distinct_on(on, columns);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Counts the rows of `source`: `SELECT COUNT(*) FROM source`.
        ///
        /// `source` is anything `.from(..)` accepts. Add `.r#where(..)` to count
        /// only the matching rows; `.get()` returns the count as an `i64`. The
        /// query is also a scalar subquery: compare against it, or select it as
        /// a column, where it decodes as `Option<i64>` (name it with
        /// `alias(..)`). Its condition may read the outer query's columns to
        /// count per row.
        pub fn count<'a, 'b, Source>(
            &'a mut self,
            source: Source,
        ) -> DrizzleBuilder<'a, Schema, crate::builder::postgres::CountSelect<'b, Schema, Source>, drizzle_postgres::builder::SelectFromSet>
        where
            Source: ToSQL<'b, PostgresValue<'b>> + drizzle_core::ScopeEntry,
            crate::builder::CountMarker<'b, PostgresValue<'b>>: drizzle_core::ResolveRow<Source>,
        {
            let builder = QueryBuilder::new::<Schema>()
                .select(drizzle_core::expr::count(()))
                .from(source);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts an `INSERT` into `table`.
        ///
        /// Follow it with `.values(..)` and `Insert*` models, or with `.select(..)`
        /// to insert a query's rows.
        pub fn insert<'a, 'b, Table>(
            &'a mut self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, InsertBuilder<'b, Schema, InsertInitial, Table>, InsertInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().insert(table);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts an `UPDATE` of `table`.
        ///
        /// Follow it with `.set(..)` and an `Update*` model, then `.r#where(..)`;
        /// `.r#where(true)` updates every row.
        pub fn update<'a, 'b, Table>(
            &'a mut self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, UpdateBuilder<'b, Schema, UpdateInitial, Table>, UpdateInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().update(table);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts a `DELETE` from `table`. It runs once `.r#where(..)` picks the
        /// rows; `.r#where(true)` deletes every row.
        pub fn delete<'a, 'b, Table>(
            &'a mut self,
            table: Table,
        ) -> DrizzleBuilder<'a, Schema, DeleteBuilder<'b, Schema, DeleteInitial, Table>, DeleteInitial>
        where
            Table: PostgresTable<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().delete(table);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }

        /// Starts a query with a common table expression: `WITH name AS (...)`.
        ///
        /// Make the CTE with `.into_cte::<Tag>()` on a select query, then pass it
        /// here and select from it.
        pub fn with<'a, 'b, C>(
            &'a mut self,
            cte: &C,
        ) -> DrizzleBuilder<'a, Schema, QueryBuilder<'b, Schema, builder::CTEInit>, builder::CTEInit>
        where
            C: builder::CTEDefinition<'b>,
        {
            let builder = QueryBuilder::new::<Schema>().with(cte);
            DrizzleBuilder {
                runner: self,
                builder,
                state: ::std::marker::PhantomData,
            }
        }
    };
}

#[cfg(feature = "postgres-sync")]
pub mod postgres_sync;

// `hyperdrive` is the same driver on a different dialer — it compiles this
// module verbatim and only adds `hyperdrive::connect`.
#[cfg(any(feature = "tokio-postgres", feature = "hyperdrive"))]
pub mod tokio_postgres;

/// Cloudflare Hyperdrive connector for the [`tokio_postgres`] driver.
#[cfg(all(feature = "hyperdrive", target_arch = "wasm32"))]
pub mod hyperdrive;

#[cfg(feature = "aws-data-api")]
pub mod aws_data_api;

#[cfg(any(
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "hyperdrive"
))]
pub mod bulk;
pub mod common;
pub mod prepared_common;
pub mod rows;

/// The `SELECT COUNT(*) FROM source` query that `count(source)` starts.
pub(crate) type CountSelect<'q, Schema, Source> = drizzle_postgres::builder::select::SelectBuilder<
    'q,
    Schema,
    drizzle_postgres::builder::SelectFromSet,
    Source,
    drizzle_core::FromMarker<
        super::CountMarker<'q, drizzle_postgres::values::PostgresValue<'q>>,
        Source,
    >,
    <super::CountMarker<'q, drizzle_postgres::values::PostgresValue<'q>> as drizzle_core::ResolveRow<
        Source,
    >>::Row,
>;
