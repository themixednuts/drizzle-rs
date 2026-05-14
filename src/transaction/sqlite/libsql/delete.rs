// See sibling rusqlite/delete.rs — libsql is async with no `'conn` on its
// `TransactionBuilder`.

use crate::transaction::sqlite::libsql::TransactionBuilder;
use drizzle_sqlite::builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder};

crate::impl_tx_delete_where! {
    TransactionBuilder;
    impl['tx, 'a, S, T];
    TransactionBuilder<'tx, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
        => TransactionBuilder<'tx, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
}
