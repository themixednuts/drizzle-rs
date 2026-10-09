#[cfg(feature = "sqlite")]
#[macro_use]
pub mod sqlite;

#[cfg(feature = "postgres")]
#[macro_use]
pub mod postgres;

#[cfg(feature = "mysql")]
#[macro_use]
pub mod mysql;

/// Fails a push whose drops would lose rows, as drizzle-kit's push stops to
/// ask before them. `lost` pairs each drop with the rows it holds.
#[cfg(any(
    feature = "rusqlite",
    feature = "libsql",
    feature = "turso",
    feature = "postgres-sync",
    feature = "tokio-postgres",
    feature = "hyperdrive",
    feature = "mysql-sync",
    feature = "mysql-async"
))]
pub(crate) fn refuse_data_loss(
    lost: &[(drizzle_migrations::DataLoss, i64)],
) -> drizzle_core::error::Result<()> {
    let lost: Vec<String> = lost
        .iter()
        .filter(|(_, rows)| *rows > 0)
        .map(|(drop, rows)| format!("{drop} holds {rows} row(s)"))
        .collect();
    if lost.is_empty() {
        return Ok(());
    }
    Err(drizzle_core::error::DrizzleError::Other(
        format!(
            "push would drop data, so nothing was applied: {}. Move or delete that data \
             first, or run `drizzle push`, which asks before dropping it.",
            lost.join("; ")
        )
        .into(),
    ))
}

/// Maps a relational query runner to the detached prepared-query driver marker.
///
/// Each dialect implements it for `&Drizzle<Conn, _>` (mapping to `Conn`) and
/// for its transaction references (mapping to that driver's connection type),
/// so a query prepared inside a transaction produces the same prepared query,
/// with the same SQL shape and executors, as one prepared on the database
/// handle.
#[cfg(feature = "query")]
pub trait RelationalPreparedDriver {
    /// Connection type the prepared query will execute against.
    type PreparedDriver;
}

/// Implements `.prepare()` on a driver's `DrizzleBuilder` alias.
///
/// Expects `DrizzleBuilder`, `QueryBuilder`, `builder`, `prepare_render`,
/// `ToSQL`, and the driver's `prepared` module to be in scope.
#[doc(hidden)]
#[macro_export]
macro_rules! drizzle_prepare_impl {
    () => {
        impl<'a: 'b, 'b, S, Schema, State, Table, Mk, Rw, Grouped>
            DrizzleBuilder<'a, S, QueryBuilder<'b, Schema, State, Table, Mk, Rw, Grouped>, State>
        {
            /// Renders this query once into a reusable prepared statement.
            ///
            /// Put [`placeholder`](drizzle_core::traits::SQLColumn::placeholder)s
            /// where values change between runs, then run the statement with
            /// `.execute(conn, params)`, `.all(conn, params)`, or
            /// `.get(conn, params)`, binding each placeholder by name in any
            /// order (`name.bind(value)`). A binding has the placeholder
            /// column's type, so a value of the wrong type does not compile.
            ///
            /// The PostgreSQL, libsql, and turso drivers already cache statements
            /// on the plain builder path, so repetition alone is not a reason to
            /// prepare there. Reach for it to bind by name, or to move SQL
            /// rendering out of a hot loop.
            #[inline]
            pub fn prepare(self) -> prepared::PreparedStatement<'b, Mk, Rw>
            where
                State: builder::ExecutableState,
            {
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
#[doc(hidden)]
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
        {
            /// Renders this transaction's query once into a reusable prepared
            /// statement.
            ///
            /// Works like `prepare` on the database handle's builders, so a
            /// statement can be built inside a `transaction` closure.
            ///
            /// The statement is detached from the transaction: running it takes
            /// an explicit executor. On the SQLite drivers, pass `tx.inner()` to
            /// run it inside this transaction, so its writes commit and roll
            /// back with the transaction. The PostgreSQL prepared executors take
            /// the client itself (`&mut Client` for `postgres-sync`, `&Client`
            /// for `tokio-postgres`), so a statement built here runs on the
            /// connection after the transaction ends; inside the transaction,
            /// use the builder's own `.execute()`/`.all()`/`.get()` instead.
            #[inline]
            pub fn prepare(self) -> prepared::PreparedStatement<'b, Mk, Rw>
            where
                State: builder::ExecutableState,
            {
                prepared::PreparedStatement::new(prepare_render(&self.to_sql()))
            }
        }
    };
}
