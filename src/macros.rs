#[doc(hidden)]
macro_rules! drizzle_builder_join_impl {
    () => {
        drizzle_builder_join_impl!(natural);
        drizzle_builder_join_impl!(natural_left);
        drizzle_builder_join_impl!(left);
        drizzle_builder_join_impl!(left_outer);
        drizzle_builder_join_impl!(natural_left_outer);
        drizzle_builder_join_impl!(natural_right);
        drizzle_builder_join_impl!(right);
        drizzle_builder_join_impl!(right_outer);
        drizzle_builder_join_impl!(natural_right_outer);
        drizzle_builder_join_impl!(natural_full);
        drizzle_builder_join_impl!(full);
        drizzle_builder_join_impl!(full_outer);
        drizzle_builder_join_impl!(natural_full_outer);
        drizzle_builder_join_impl!(inner);
        drizzle_builder_join_impl!(cross);
    };
    ($type:ident) => {
        paste::paste! {
            pub fn [<$type _join>]<U>(
                self,
                table: U,
                on_condition: impl ToSQLiteSQL<'a>,
            ) -> DrizzleBuilder<
                'a,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, T>,
                SelectJoinSet,
            >
            where
                U: IsInSchema<Schema> + SQLiteTable<'a>,
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

#[doc(hidden)]
macro_rules! transaction_builder_join_impl {
    ($($lifetimes:lifetime),*) => {
        transaction_builder_join_impl!($($lifetimes),*, natural);
        transaction_builder_join_impl!($($lifetimes),*, natural_left);
        transaction_builder_join_impl!($($lifetimes),*, left);
        transaction_builder_join_impl!($($lifetimes),*, left_outer);
        transaction_builder_join_impl!($($lifetimes),*, natural_left_outer);
        transaction_builder_join_impl!($($lifetimes),*, natural_right);
        transaction_builder_join_impl!($($lifetimes),*, right);
        transaction_builder_join_impl!($($lifetimes),*, right_outer);
        transaction_builder_join_impl!($($lifetimes),*, natural_right_outer);
        transaction_builder_join_impl!($($lifetimes),*, natural_full);
        transaction_builder_join_impl!($($lifetimes),*, full);
        transaction_builder_join_impl!($($lifetimes),*, full_outer);
        transaction_builder_join_impl!($($lifetimes),*, natural_full_outer);
        transaction_builder_join_impl!($($lifetimes),*, inner);
        transaction_builder_join_impl!($($lifetimes),*, cross);
    };
    ($($lifetimes:lifetime),*, $type:ident) => {
        paste::paste! {
            pub fn [<$type _join>]<U>(
                self,
                table: U,
                on_condition: impl ToSQLiteSQL<'a>,
            ) -> TransactionBuilder<
                $($lifetimes,)*
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, T>,
                SelectJoinSet,
            >
            where
                U: IsInSchema<Schema> + SQLiteTable<'a>,
            {
                let builder = self.builder.[<$type _join>](table, on_condition);
                TransactionBuilder {
                    transaction: self.transaction,
                    builder,
                    _phantom: PhantomData,
                }
            }
        }
    };
}
