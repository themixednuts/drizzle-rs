// The DELETE typestate-advancing methods are identical across all SQLite
// drivers modulo the `TransactionBuilder` struct's lifetime parameters.
// rusqlite carries `<'tx, 'conn>` because its `Transaction` borrows from a
// long-lived connection handle.

use crate::transaction::sqlite::rusqlite::TransactionBuilder;
use drizzle_sqlite::builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder};

crate::impl_tx_delete_where! {
    TransactionBuilder;
    impl['tx, 'a, 'conn, S, T];
    TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
        => TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
}
