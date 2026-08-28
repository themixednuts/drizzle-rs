#[cfg(feature = "sqlite")]
#[macro_use]
pub mod sqlite;

#[cfg(feature = "postgres")]
#[macro_use]
pub mod postgres;

#[cfg(feature = "mysql")]
#[macro_use]
pub mod mysql;

#[macro_export]
macro_rules! drizzle_prepare_impl {
    () => {
        impl<'a: 'b, 'b, S, Schema, State, Table, Mk, Rw, Grouped>
            DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table, Mk, Rw, Grouped>, State>
        where
            State: builder::ExecutableState,
        {
            /// Creates a prepared statement from this query builder.
            ///
            /// The returned statement can be executed with `.all()`, `.get()`, or
            /// `.execute()`, each taking a fixed-size array of parameter bindings.
            /// The array size is inferred from the call site and validated at runtime
            /// against the actual placeholder count.
            ///
            /// # When to reach for this
            ///
            /// Every driver whose economics justify it now serves the plain
            /// builder path from a per-connection statement cache, so `.prepare()`
            /// is no longer the way to avoid re-parsing. Reach for it when you
            /// want *named* bindings — build the query once with
            /// [`placeholder`](drizzle_core::placeholder::Placeholder)s and bind
            /// by name at each call, in any order — or when you want to hoist SQL
            /// rendering and parameter layout out of a hot loop. For a query you
            /// simply run repeatedly, the cached default path is equivalent.
            #[inline]
            pub fn prepare(self) -> prepared::PreparedStatement<'b, Mk, Rw> {
                prepared::PreparedStatement::new(prepare_render(&self.to_sql()))
            }
        }
    };
}

/// Implements `.prepare()` on a driver's `TransactionBuilder` alias.
///
/// Pass the alias's connection lifetime when it has one (`'conn` for rusqlite,
/// turso, and both Postgres drivers); pass nothing for the aliases that carry
/// only the borrow lifetime (libsql, durable).
///
/// Expects `TransactionBuilder`, `QueryBuilder`, `builder`, `prepare_render`,
/// `ToSQL`, and the driver's `prepared` module to be in scope.
#[macro_export]
macro_rules! drizzle_tx_prepare_impl {
    ($($conn:lifetime)?) => {
        impl<'a: 'b, 'b, $($conn,)? Schema, State, Table, Mk, Rw, Grouped>
            TransactionBuilder<
                'a,
                $($conn,)?
                Schema,
                QueryBuilder<'b, Schema, State, Table, Mk, Rw, Grouped>,
                State,
            >
        where
            State: builder::ExecutableState,
        {
            /// Creates a prepared statement from this transaction's query builder.
            ///
            /// Mirrors [`prepare`](crate::drizzle_prepare_impl) on the connection
            /// runner, so a statement can be built inside a `transaction` closure
            /// instead of being hoisted outside it just to exist.
            ///
            /// The statement is detached from the transaction — executing it takes
            /// an explicit executor. On the `SQLite` drivers pass `tx.inner()` to
            /// run it inside this transaction, so its writes commit and roll back
            /// with the transaction. The `PostgreSQL` prepared executors currently
            /// take a `&Client`, so a statement built here runs on the connection
            /// once the transaction has finished; inside the transaction, prefer
            /// the builder's own `.execute()`/`.all()`/`.get()`, which serve from
            /// the connection's statement cache.
            ///
            /// Note that repetition alone is not a reason to prepare — see
            /// [`prepare`](crate::drizzle_prepare_impl) on the connection runner
            /// for when an explicit statement still earns its keep.
            #[inline]
            pub fn prepare(self) -> prepared::PreparedStatement<'b, Mk, Rw> {
                prepared::PreparedStatement::new(prepare_render(&self.to_sql()))
            }
        }
    };
}
