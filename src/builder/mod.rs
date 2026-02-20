#[cfg(feature = "sqlite")]
#[macro_use]
pub(crate) mod sqlite;

#[cfg(feature = "postgres")]
#[macro_use]
pub(crate) mod postgres;

#[macro_export]
macro_rules! drizzle_prepare_impl {
    () => {
        impl<'a: 'b, 'b, S, Schema, State, Table, Mk, Rw>
            DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table, Mk, Rw>, State>
        where
            State: builder::ExecutableState,
        {
            /// Creates a prepared statement from this query builder.
            ///
            /// The returned statement can be executed with `.all()`, `.get()`, or
            /// `.execute()`, each taking a fixed-size array of parameter bindings.
            /// The array size is inferred from the call site and validated at runtime
            /// against the actual placeholder count.
            #[inline]
            pub fn prepare(self) -> prepared::PreparedStatement<'b> {
                prepared::PreparedStatement {
                    inner: prepare_render(self.to_sql()),
                }
            }
        }
    };
}
