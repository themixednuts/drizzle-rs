#[doc(hidden)]
macro_rules! drizzle_builder_join_impl {
    () => {
        drizzle_builder_join_impl!(natural, drizzle_core::AfterJoin);
        drizzle_builder_join_impl!(natural_left, drizzle_core::AfterLeftJoin);
        drizzle_builder_join_impl!(left, drizzle_core::AfterLeftJoin);
        drizzle_builder_join_impl!(left_outer, drizzle_core::AfterLeftJoin);
        drizzle_builder_join_impl!(natural_left_outer, drizzle_core::AfterLeftJoin);
        drizzle_builder_join_impl!(natural_right, drizzle_core::AfterRightJoin);
        drizzle_builder_join_impl!(right, drizzle_core::AfterRightJoin);
        drizzle_builder_join_impl!(right_outer, drizzle_core::AfterRightJoin);
        drizzle_builder_join_impl!(natural_right_outer, drizzle_core::AfterRightJoin);
        drizzle_builder_join_impl!(natural_full, drizzle_core::AfterFullJoin);
        drizzle_builder_join_impl!(full, drizzle_core::AfterFullJoin);
        drizzle_builder_join_impl!(full_outer, drizzle_core::AfterFullJoin);
        drizzle_builder_join_impl!(natural_full_outer, drizzle_core::AfterFullJoin);
        drizzle_builder_join_impl!(inner, drizzle_core::AfterJoin);
        drizzle_builder_join_impl!(cross, drizzle_core::AfterJoin);
    };
    ($type:ident, $join_trait:path) => {
        paste::paste! {
            pub fn [<$type _join>]<J: drizzle_sqlite::helpers::JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> DrizzleBuilder<
                'd,
                Conn,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as drizzle_core::ScopePush<J::JoinedTable>>::Out, <M as $join_trait<R, J::JoinedTable>>::NewRow>,
                SelectJoinSet,
            >
            where
                M: $join_trait<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
            {
                let builder = self.builder.[<$type _join>](arg);
                DrizzleBuilder {
                    drizzle: self.drizzle,
                    builder,
                    state: PhantomData,
                }
            }
        }
    };
}

#[doc(hidden)]
macro_rules! drizzle_pg_builder_join_impl {
    () => {
        drizzle_pg_builder_join_impl!(natural, drizzle_core::AfterJoin);
        drizzle_pg_builder_join_impl!(natural_left, drizzle_core::AfterLeftJoin);
        drizzle_pg_builder_join_impl!(left, drizzle_core::AfterLeftJoin);
        drizzle_pg_builder_join_impl!(left_outer, drizzle_core::AfterLeftJoin);
        drizzle_pg_builder_join_impl!(natural_left_outer, drizzle_core::AfterLeftJoin);
        drizzle_pg_builder_join_impl!(natural_right, drizzle_core::AfterRightJoin);
        drizzle_pg_builder_join_impl!(right, drizzle_core::AfterRightJoin);
        drizzle_pg_builder_join_impl!(right_outer, drizzle_core::AfterRightJoin);
        drizzle_pg_builder_join_impl!(natural_right_outer, drizzle_core::AfterRightJoin);
        drizzle_pg_builder_join_impl!(natural_full, drizzle_core::AfterFullJoin);
        drizzle_pg_builder_join_impl!(full, drizzle_core::AfterFullJoin);
        drizzle_pg_builder_join_impl!(full_outer, drizzle_core::AfterFullJoin);
        drizzle_pg_builder_join_impl!(natural_full_outer, drizzle_core::AfterFullJoin);
        drizzle_pg_builder_join_impl!(inner, drizzle_core::AfterJoin);
        drizzle_pg_builder_join_impl!(cross, drizzle_core::AfterJoin);
    };
    ($type:ident, $join_trait:path) => {
        paste::paste! {
            pub fn [<$type _join>]<J: drizzle_postgres::helpers::JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> DrizzleBuilder<
                'd,
                DrizzleRef,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as drizzle_core::ScopePush<J::JoinedTable>>::Out, <M as $join_trait<R, J::JoinedTable>>::NewRow>,
                SelectJoinSet,
            >
            where
                M: $join_trait<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
            {
                let builder = self.builder.[<$type _join>](arg);
                DrizzleBuilder {
                    drizzle: self.drizzle,
                    builder,
                    state: PhantomData,
                }
            }
        }
    };
}

#[doc(hidden)]
macro_rules! drizzle_pg_builder_join_using_impl {
    () => {
        drizzle_pg_builder_join_using_impl!(left, drizzle_core::AfterLeftJoin);
        drizzle_pg_builder_join_using_impl!(left_outer, drizzle_core::AfterLeftJoin);
        drizzle_pg_builder_join_using_impl!(right, drizzle_core::AfterRightJoin);
        drizzle_pg_builder_join_using_impl!(right_outer, drizzle_core::AfterRightJoin);
        drizzle_pg_builder_join_using_impl!(full, drizzle_core::AfterFullJoin);
        drizzle_pg_builder_join_using_impl!(full_outer, drizzle_core::AfterFullJoin);
        drizzle_pg_builder_join_using_impl!(inner, drizzle_core::AfterJoin);

        /// JOIN USING clause (plain JOIN).
        pub fn join_using<U: drizzle_postgres::traits::PostgresTable<'a>>(
            self,
            table: U,
            columns: impl drizzle_core::ToSQL<'a, drizzle_postgres::values::PostgresValue<'a>>,
        ) -> DrizzleBuilder<
            'd,
            DrizzleRef,
            Schema,
            SelectBuilder<
                'a,
                Schema,
                SelectJoinSet,
                U,
                <M as drizzle_core::ScopePush<U>>::Out,
                <M as drizzle_core::AfterJoin<R, U>>::NewRow,
            >,
            SelectJoinSet,
        >
        where
            M: drizzle_core::AfterJoin<R, U> + drizzle_core::ScopePush<U>,
        {
            let builder = self.builder.join_using(table, columns);
            DrizzleBuilder {
                drizzle: self.drizzle,
                builder,
                state: PhantomData,
            }
        }
    };
    ($type:ident, $join_trait:path) => {
        paste::paste! {
            pub fn [<$type _join_using>]<U: drizzle_postgres::traits::PostgresTable<'a>>(
                self,
                table: U,
                columns: impl drizzle_core::ToSQL<'a, drizzle_postgres::values::PostgresValue<'a>>,
            ) -> DrizzleBuilder<
                'd,
                DrizzleRef,
                Schema,
                SelectBuilder<
                    'a,
                    Schema,
                    SelectJoinSet,
                    U,
                    <M as drizzle_core::ScopePush<U>>::Out,
                    <M as $join_trait<R, U>>::NewRow,
                >,
                SelectJoinSet,
            >
            where
                M: $join_trait<R, U> + drizzle_core::ScopePush<U>,
            {
                let builder = self.builder.[<$type _join_using>](table, columns);
                DrizzleBuilder {
                    drizzle: self.drizzle,
                    builder,
                    state: PhantomData,
                }
            }
        }
    };
}

#[doc(hidden)]
macro_rules! transaction_builder_join_impl {
    ($($lifetimes:lifetime),*) => {
        transaction_builder_join_impl!($($lifetimes),*, natural, drizzle_core::AfterJoin);
        transaction_builder_join_impl!($($lifetimes),*, natural_left, drizzle_core::AfterLeftJoin);
        transaction_builder_join_impl!($($lifetimes),*, left, drizzle_core::AfterLeftJoin);
        transaction_builder_join_impl!($($lifetimes),*, left_outer, drizzle_core::AfterLeftJoin);
        transaction_builder_join_impl!($($lifetimes),*, natural_left_outer, drizzle_core::AfterLeftJoin);
        transaction_builder_join_impl!($($lifetimes),*, natural_right, drizzle_core::AfterRightJoin);
        transaction_builder_join_impl!($($lifetimes),*, right, drizzle_core::AfterRightJoin);
        transaction_builder_join_impl!($($lifetimes),*, right_outer, drizzle_core::AfterRightJoin);
        transaction_builder_join_impl!($($lifetimes),*, natural_right_outer, drizzle_core::AfterRightJoin);
        transaction_builder_join_impl!($($lifetimes),*, natural_full, drizzle_core::AfterFullJoin);
        transaction_builder_join_impl!($($lifetimes),*, full, drizzle_core::AfterFullJoin);
        transaction_builder_join_impl!($($lifetimes),*, full_outer, drizzle_core::AfterFullJoin);
        transaction_builder_join_impl!($($lifetimes),*, natural_full_outer, drizzle_core::AfterFullJoin);
        transaction_builder_join_impl!($($lifetimes),*, inner, drizzle_core::AfterJoin);
        transaction_builder_join_impl!($($lifetimes),*, cross, drizzle_core::AfterJoin);
    };
    ($($lifetimes:lifetime),*, $type:ident, $join_trait:path) => {
        paste::paste! {
            pub fn [<$type _join>]<J: drizzle_sqlite::helpers::JoinArg<'a, T>>(
                self,
                arg: J,
            ) -> TransactionBuilder<
                $($lifetimes,)*
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, J::JoinedTable, <M as drizzle_core::ScopePush<J::JoinedTable>>::Out, <M as $join_trait<R, J::JoinedTable>>::NewRow>,
                SelectJoinSet,
            >
            where
                M: $join_trait<R, J::JoinedTable> + drizzle_core::ScopePush<J::JoinedTable>,
            {
                let builder = self.builder.[<$type _join>](arg);
                TransactionBuilder {
                    transaction: self.transaction,
                    builder,
                    _phantom: PhantomData,
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
        pub fn select<'a, 'b, T>(
            &'a self,
            query: T,
        ) -> TransactionBuilder<
            'a,
            $($conn_lt,)*
            Schema,
            SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_sqlite::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select(query);

            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn select_distinct<'a, 'b, T>(
            &'a self,
            query: T,
        ) -> TransactionBuilder<
            'a,
            $($conn_lt,)*
            Schema,
            SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'b, SQLiteValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_sqlite::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select_distinct(query);

            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates an INSERT query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn insert<'a, Table>(
            &'a self,
            table: Table,
        ) -> TransactionBuilder<
            'a,
            $($conn_lt,)*
            Schema,
            InsertBuilder<'a, Schema, InsertInitial, Table>,
            InsertInitial,
        >
        where
            Table: SQLiteTable<'a>,
        {
            let builder = QueryBuilder::new::<Schema>().insert(table);
            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates an UPDATE query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn update<'a, Table>(
            &'a self,
            table: Table,
        ) -> TransactionBuilder<
            'a,
            $($conn_lt,)*
            Schema,
            UpdateBuilder<'a, Schema, UpdateInitial, Table>,
            UpdateInitial,
        >
        where
            Table: SQLiteTable<'a>,
        {
            let builder = QueryBuilder::new::<Schema>().update(table);
            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates a DELETE query builder within the transaction
        #[cfg(feature = "sqlite")]
        pub fn delete<'a, T>(
            &'a self,
            table: T,
        ) -> TransactionBuilder<
            'a,
            $($conn_lt,)*
            Schema,
            DeleteBuilder<'a, Schema, DeleteInitial, T>,
            DeleteInitial,
        >
        where
            T: SQLiteTable<'a>,
        {
            let builder = QueryBuilder::new::<Schema>().delete(table);
            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates a query with CTE (Common Table Expression) within the transaction
        #[cfg(feature = "sqlite")]
        pub fn with<'a, C>(
            &'a self,
            cte: C,
        ) -> TransactionBuilder<
            'a,
            $($conn_lt,)*
            Schema,
            QueryBuilder<'a, Schema, builder::CTEInit>,
            builder::CTEInit,
        >
        where
            C: builder::CTEDefinition<'a>,
        {
            let builder = QueryBuilder::new::<Schema>().with(cte);
            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }
    };
}

#[doc(hidden)]
macro_rules! postgres_transaction_constructors {
    () => {
        /// Creates a SELECT query builder within the transaction
        pub fn select<'a, 'b, T>(
            &'a self,
            query: T,
        ) -> TransactionBuilder<
            'a,
            'conn,
            Schema,
            SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_postgres::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select(query);

            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates a SELECT DISTINCT query builder within the transaction
        pub fn select_distinct<'a, 'b, T>(
            &'a self,
            query: T,
        ) -> TransactionBuilder<
            'a,
            'conn,
            Schema,
            SelectBuilder<'b, Schema, SelectInitial, (), T::Marker>,
            SelectInitial,
        >
        where
            T: ToSQL<'b, PostgresValue<'b>> + drizzle_core::IntoSelectTarget,
        {
            use drizzle_postgres::builder::QueryBuilder;

            let builder = QueryBuilder::new::<Schema>().select_distinct(query);

            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates an INSERT query builder within the transaction
        pub fn insert<'a, Table>(
            &'a self,
            table: Table,
        ) -> TransactionBuilder<
            'a,
            'conn,
            Schema,
            InsertBuilder<'a, Schema, InsertInitial, Table>,
            InsertInitial,
        >
        where
            Table: PostgresTable<'a>,
        {
            let builder = QueryBuilder::new::<Schema>().insert(table);
            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates an UPDATE query builder within the transaction
        pub fn update<'a, Table>(
            &'a self,
            table: Table,
        ) -> TransactionBuilder<
            'a,
            'conn,
            Schema,
            UpdateBuilder<'a, Schema, UpdateInitial, Table>,
            UpdateInitial,
        >
        where
            Table: PostgresTable<'a>,
        {
            let builder = QueryBuilder::new::<Schema>().update(table);
            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates a DELETE query builder within the transaction
        pub fn delete<'a, T>(
            &'a self,
            table: T,
        ) -> TransactionBuilder<
            'a,
            'conn,
            Schema,
            DeleteBuilder<'a, Schema, DeleteInitial, T>,
            DeleteInitial,
        >
        where
            T: PostgresTable<'a>,
        {
            let builder = QueryBuilder::new::<Schema>().delete(table);
            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }

        /// Creates a query with CTE (Common Table Expression) within the transaction
        pub fn with<'a, C>(
            &'a self,
            cte: C,
        ) -> TransactionBuilder<
            'a,
            'conn,
            Schema,
            QueryBuilder<'a, Schema, builder::CTEInit>,
            builder::CTEInit,
        >
        where
            C: builder::CTEDefinition<'a>,
        {
            let builder = QueryBuilder::new::<Schema>().with(cte);
            TransactionBuilder {
                transaction: self,
                builder,
                _phantom: PhantomData,
            }
        }
    };
}
