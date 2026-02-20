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
