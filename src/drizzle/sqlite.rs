// Driver modules
#[cfg(feature = "rusqlite")]
pub mod rusqlite;

#[cfg(feature = "turso")]
pub mod turso;

#[cfg(feature = "libsql")]
pub mod libsql;

#[macro_export]
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
        paste::paste! {
            pub fn [<$type _join>]<U>(
                self,
                table: U,
                on_condition: drizzle_core::SQL<'a, SQLiteValue<'a>>,
            ) -> DrizzleBuilder<
                'a,
                Schema,
                SelectBuilder<'a, Schema, SelectJoinSet, T>,
                SelectJoinSet,
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
