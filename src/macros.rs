#[doc(hidden)]
macro_rules! drizzle_builder_join_impl {
    () => {
        drizzle_builder_join_impl!(@natural natural, drizzle_core::InnerJoin);
        drizzle_builder_join_impl!(@natural natural_left, drizzle_core::LeftJoin);
        drizzle_builder_join_impl!(left, drizzle_core::LeftJoin);
        drizzle_builder_join_impl!(left_outer, drizzle_core::LeftJoin);
        drizzle_builder_join_impl!(@natural natural_left_outer, drizzle_core::LeftJoin);
        drizzle_builder_join_impl!(@natural natural_right, drizzle_core::RightJoin);
        drizzle_builder_join_impl!(right, drizzle_core::RightJoin);
        drizzle_builder_join_impl!(right_outer, drizzle_core::RightJoin);
        drizzle_builder_join_impl!(@natural natural_right_outer, drizzle_core::RightJoin);
        drizzle_builder_join_impl!(@natural natural_full, drizzle_core::FullJoin);
        drizzle_builder_join_impl!(full, drizzle_core::FullJoin);
        drizzle_builder_join_impl!(full_outer, drizzle_core::FullJoin);
        drizzle_builder_join_impl!(@natural natural_full_outer, drizzle_core::FullJoin);
        drizzle_builder_join_impl!(inner, drizzle_core::InnerJoin);
    };
    (@natural $type:ident, $kind:ty) => {
        paste::paste! {
            /// Adds a NATURAL join: the database matches the columns both
            /// sides share by name, so it takes a source and no ON condition.
            pub fn [<$type _join>]<J: drizzle_sqlite::helpers::JoinSource<'a>>(
                self,
                source: J,
            ) -> DrizzleBuilder<
                'd,
                Runner,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind>>::Marker, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind>>::Row, G>,
                SelectJoinSet,
            >
            where
                M: drizzle_core::JoinStep<R, J::JoinedTable, $kind>,
            {
                let builder = self.builder.[<$type _join>](source);
                DrizzleBuilder {
                    runner: self.runner,
                    builder,
                    state: ::core::marker::PhantomData,
                }
            }
        }
    };
    ($type:ident, $kind:ty) => {
        paste::paste! {
            pub fn [<$type _join>]<J: drizzle_sqlite::helpers::JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> DrizzleBuilder<
                'd,
                Runner,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>>::Marker, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>>::Row, G>,
                SelectJoinSet,
            >
            where
                M: drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>,
            {
                let builder = self.builder.[<$type _join>](arg);
                DrizzleBuilder {
                    runner: self.runner,
                    builder,
                    state: ::core::marker::PhantomData,
                }
            }
        }
    };
}

#[doc(hidden)]
macro_rules! drizzle_pg_builder_join_impl {
    () => {
        drizzle_pg_builder_join_impl!(@natural natural, drizzle_core::InnerJoin);
        drizzle_pg_builder_join_impl!(@natural natural_left, drizzle_core::LeftJoin);
        drizzle_pg_builder_join_impl!(left, drizzle_core::LeftJoin);
        drizzle_pg_builder_join_impl!(left_outer, drizzle_core::LeftJoin);
        drizzle_pg_builder_join_impl!(@natural natural_left_outer, drizzle_core::LeftJoin);
        drizzle_pg_builder_join_impl!(@natural natural_right, drizzle_core::RightJoin);
        drizzle_pg_builder_join_impl!(right, drizzle_core::RightJoin);
        drizzle_pg_builder_join_impl!(right_outer, drizzle_core::RightJoin);
        drizzle_pg_builder_join_impl!(@natural natural_right_outer, drizzle_core::RightJoin);
        drizzle_pg_builder_join_impl!(@natural natural_full, drizzle_core::FullJoin);
        drizzle_pg_builder_join_impl!(full, drizzle_core::FullJoin);
        drizzle_pg_builder_join_impl!(full_outer, drizzle_core::FullJoin);
        drizzle_pg_builder_join_impl!(@natural natural_full_outer, drizzle_core::FullJoin);
        drizzle_pg_builder_join_impl!(inner, drizzle_core::InnerJoin);
        drizzle_pg_builder_join_impl!(@lateral inner_join_lateral, drizzle_core::InnerJoin);
        drizzle_pg_builder_join_impl!(
            @lateral
            left_join_lateral,
            drizzle_core::LeftJoin,
            SelectionProof
        );
        drizzle_pg_builder_join_impl!(@cross_lateral);
    };
    (@lateral $method:ident, $kind:ty $(, $proof:ident)?) => {
        /// Adds a JOIN LATERAL clause with an ON condition.
        #[inline]
        #[allow(clippy::type_complexity)]
        pub fn $method<J $(, $proof)?>(self, arg: J) -> DrizzleBuilder<
            'd,
            Runner,
            Schema,
            SelectBuilder<
                'a,
                Schema,
                SelectJoinSet,
                J::JoinedTable,
                <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::Lateral<$kind>, J::OnSources>>::Marker,
                <M as drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::Lateral<$kind>, J::OnSources>>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            J: drizzle_core::LateralArg<'a, drizzle_postgres::values::PostgresValue<'a>>,
            M: drizzle_core::JoinStep<R, J::JoinedTable, drizzle_core::Lateral<$kind>, J::OnSources>
                $(+ drizzle_core::LeftLateralSelection<$proof>)?,
        {
            let builder = self.builder.$method(arg);
            DrizzleBuilder {
                runner: self.runner,
                builder,
                state: ::core::marker::PhantomData,
            }
        }
    };
    (@cross_lateral) => {
        /// Adds a CROSS JOIN LATERAL clause without an ON condition.
        #[inline]
        #[allow(clippy::type_complexity)]
        pub fn cross_join_lateral<Source>(
            self,
            source: Source,
        ) -> DrizzleBuilder<
            'd,
            Runner,
            Schema,
            SelectBuilder<
                'a,
                Schema,
                SelectJoinSet,
                Source::JoinedTable,
                <M as drizzle_core::JoinStep<R, Source::JoinedTable, drizzle_core::Lateral<drizzle_core::InnerJoin>>>::Marker,
                <M as drizzle_core::JoinStep<R, Source::JoinedTable, drizzle_core::Lateral<drizzle_core::InnerJoin>>>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            Source: drizzle_core::LateralSource<
                'a,
                drizzle_postgres::values::PostgresValue<'a>,
            >,
            M: drizzle_core::JoinStep<R, Source::JoinedTable, drizzle_core::Lateral<drizzle_core::InnerJoin>>,
        {
            let builder = self.builder.cross_join_lateral(source);
            DrizzleBuilder {
                runner: self.runner,
                builder,
                state: ::core::marker::PhantomData,
            }
        }
    };
    (@natural $type:ident, $kind:ty) => {
        paste::paste! {
            /// Adds a NATURAL join: the database matches the columns both
            /// sides share by name, so it takes a source and no ON condition.
            pub fn [<$type _join>]<J: drizzle_postgres::helpers::JoinSource<'a>>(
                self,
                source: J,
            ) -> DrizzleBuilder<
                'd,
                Runner,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind>>::Marker, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind>>::Row, G>,
                SelectJoinSet,
            >
            where
                M: drizzle_core::JoinStep<R, J::JoinedTable, $kind>,
            {
                let builder = self.builder.[<$type _join>](source);
                DrizzleBuilder {
                    runner: self.runner,
                    builder,
                    state: ::core::marker::PhantomData,
                }
            }
        }
    };
    ($type:ident, $kind:ty) => {
        paste::paste! {
            pub fn [<$type _join>]<J: drizzle_postgres::helpers::JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> DrizzleBuilder<
                'd,
                Runner,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>>::Marker, <M as drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>>::Row, G>,
                SelectJoinSet,
            >
            where
                M: drizzle_core::JoinStep<R, J::JoinedTable, $kind, J::OnSources>,
            {
                let builder = self.builder.[<$type _join>](arg);
                DrizzleBuilder {
                    runner: self.runner,
                    builder,
                    state: ::core::marker::PhantomData,
                }
            }
        }
    };
}

#[doc(hidden)]
macro_rules! drizzle_pg_builder_join_using_impl {
    () => {
        drizzle_pg_builder_join_using_impl!(
            left,
            drizzle_core::LeftJoin
        );
        drizzle_pg_builder_join_using_impl!(
            left_outer,
            drizzle_core::LeftJoin
        );
        drizzle_pg_builder_join_using_impl!(
            right,
            drizzle_core::RightJoin
        );
        drizzle_pg_builder_join_using_impl!(
            right_outer,
            drizzle_core::RightJoin
        );
        drizzle_pg_builder_join_using_impl!(
            full,
            drizzle_core::FullJoin
        );
        drizzle_pg_builder_join_using_impl!(
            full_outer,
            drizzle_core::FullJoin
        );
        drizzle_pg_builder_join_using_impl!(
            inner,
            drizzle_core::InnerJoin
        );

        /// JOIN USING clause (plain JOIN).
        pub fn join_using<U: drizzle_postgres::traits::PostgresTable<'a>>(
            self,
            table: U,
            columns: impl drizzle_core::ToSQL<'a, drizzle_postgres::values::PostgresValue<'a>>,
        ) -> DrizzleBuilder<
            'd,
            Runner,
            Schema,
            SelectBuilder<
                'a,
                Schema,
                SelectJoinSet,
                U,
                <M as drizzle_core::JoinStep<R, U, drizzle_core::InnerJoin>>::Marker,
                <M as drizzle_core::JoinStep<R, U, drizzle_core::InnerJoin>>::Row,
                G,
            >,
            SelectJoinSet,
        >
        where
            M: drizzle_core::JoinStep<R, U, drizzle_core::InnerJoin>,
        {
            let builder = self.builder.join_using(table, columns);
            DrizzleBuilder {
                runner: self.runner,
                builder,
                state: ::core::marker::PhantomData,
            }
        }
    };
    ($type:ident, $kind:ty) => {
        paste::paste! {
            pub fn [<$type _join_using>]<U: drizzle_postgres::traits::PostgresTable<'a>>(
                self,
                table: U,
                columns: impl drizzle_core::ToSQL<'a, drizzle_postgres::values::PostgresValue<'a>>,
            ) -> DrizzleBuilder<
                'd,
                Runner,
                Schema,
                SelectBuilder<
                    'a,
                    Schema,
                    SelectJoinSet,
                    U,
                    <M as drizzle_core::JoinStep<R, U, $kind>>::Marker,
                    <M as drizzle_core::JoinStep<R, U, $kind>>::Row,
                    G,
                >,
                SelectJoinSet,
            >
            where
                M: drizzle_core::JoinStep<R, U, $kind>,
            {
                let builder = self.builder.[<$type _join_using>](table, columns);
                DrizzleBuilder {
                    runner: self.runner,
                    builder,
                    state: ::core::marker::PhantomData,
                }
            }
        }
    };
}

#[doc(hidden)]
macro_rules! sqlite_transaction_constructors {
    ($($conn_lt:lifetime),*) => {
        /// Creates a SELECT query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn select<'tx, 'q, T>(
            &'tx self,
            query: T,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, SQLiteValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_sqlite::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select(query);

            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn select_distinct<'tx, 'q, T>(
            &'tx self,
            query: T,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, SQLiteValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_sqlite::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select_distinct(query);

            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates an INSERT query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn insert<'tx, 'q, Table>(
            &'tx self,
            table: Table,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            InsertBuilder<'q, Schema, InsertInitial, Table>,
            InsertInitial,
        >
        where
            Table: SQLiteTable<'q>,
        {
            let builder = QueryBuilder::new::<Schema>().insert(table);
            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates an UPDATE query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn update<'tx, 'q, Table>(
            &'tx self,
            table: Table,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            UpdateBuilder<'q, Schema, UpdateInitial, Table>,
            UpdateInitial,
        >
        where
            Table: SQLiteTable<'q>,
        {
            let builder = QueryBuilder::new::<Schema>().update(table);
            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates a DELETE query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn delete<'tx, 'q, T>(
            &'tx self,
            table: T,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            DeleteBuilder<'q, Schema, DeleteInitial, T>,
            DeleteInitial,
        >
        where
            T: SQLiteTable<'q>,
        {
            let builder = QueryBuilder::new::<Schema>().delete(table);
            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates a query with CTE (Common Table Expression) within the transaction
        #[cfg(feature = "sqlite")]
        pub fn with<'tx, 'q, C>(
            &'tx self,
            cte: &C,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            QueryBuilder<'q, Schema, builder::CTEInit>,
            builder::CTEInit,
        >
        where
            C: builder::CTEDefinition<'q>,
        {
            let builder = QueryBuilder::new::<Schema>().with(cte);
            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }
    };
}

#[doc(hidden)]
macro_rules! postgres_transaction_constructors {
    ($($conn_lt:lifetime),*) => {
        /// Creates a SELECT query builder within the transaction
        pub fn select<'tx, 'q, T>(
            &'tx self,
            query: T,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, PostgresValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_postgres::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select(query);

            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT query builder within the transaction
        pub fn select_distinct<'tx, 'q, T>(
            &'tx self,
            query: T,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'q, PostgresValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_postgres::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select_distinct(query);

            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT ON query builder within the transaction
        pub fn select_distinct_on<'tx, 'q, On, Columns>(
            &'tx self,
            on: On,
            columns: Columns,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            SelectBuilder<'q, Schema, SelectInitial, (), Columns::Marker>,
            SelectInitial,
        >
        where
            On: ToSQL<'q, PostgresValue<'q>>,
            Columns: ToSQL<'q, PostgresValue<'q>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_postgres::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select_distinct_on(on, columns);

            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates an INSERT query builder within the transaction
        pub fn insert<'tx, 'q, Table>(
            &'tx self,
            table: Table,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            InsertBuilder<'q, Schema, InsertInitial, Table>,
            InsertInitial,
        >
        where
            Table: PostgresTable<'q>,
        {
            let builder = QueryBuilder::new::<Schema>().insert(table);
            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates an UPDATE query builder within the transaction
        pub fn update<'tx, 'q, Table>(
            &'tx self,
            table: Table,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            UpdateBuilder<'q, Schema, UpdateInitial, Table>,
            UpdateInitial,
        >
        where
            Table: PostgresTable<'q>,
        {
            let builder = QueryBuilder::new::<Schema>().update(table);
            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates a DELETE query builder within the transaction
        pub fn delete<'tx, 'q, T>(
            &'tx self,
            table: T,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            DeleteBuilder<'q, Schema, DeleteInitial, T>,
            DeleteInitial,
        >
        where
            T: PostgresTable<'q>,
        {
            let builder = QueryBuilder::new::<Schema>().delete(table);
            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }

        /// Creates a query with CTE (Common Table Expression) within the transaction
        pub fn with<'tx, 'q, C>(
            &'tx self,
            cte: &C,
        ) -> TransactionBuilder<
            'tx,
            $($conn_lt,)*
            Schema,
            QueryBuilder<'q, Schema, builder::CTEInit>,
            builder::CTEInit,
        >
        where
            C: builder::CTEDefinition<'q>,
        {
            let builder = QueryBuilder::new::<Schema>().with(cte);
            TransactionBuilder {
                runner: self,
                builder,
                state: ::core::marker::PhantomData,
            }
        }
    };
}
