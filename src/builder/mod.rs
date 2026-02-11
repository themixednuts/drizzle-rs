#[cfg(feature = "sqlite")]
#[macro_use]
pub(crate) mod sqlite;

#[cfg(feature = "postgres")]
#[macro_use]
pub(crate) mod postgres;

#[macro_export]
macro_rules! drizzle_prepare_impl {
    () => {
        impl<'a: 'b, 'b, S, Schema, State, Table>
            DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table>, State>
        where
            State: builder::ExecutableState,
        {
            /// Creates a prepared statement that can be executed multiple times
            #[inline]
            pub fn prepare(self) -> prepared::PreparedStatement<'b> {
                let inner = prepare_render(self.to_sql());
                prepared::PreparedStatement { inner }
            }
        }
    };
}
