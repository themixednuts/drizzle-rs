// See sibling rusqlite/delete.rs — turso also carries `<'tx, 'conn>` on
// its `TransactionBuilder`.

use crate::transaction::sqlite::turso::TransactionBuilder;
use drizzle_sqlite::builder::{DeleteInitial, DeleteWhereSet, delete::DeleteBuilder};

crate::impl_tx_delete_where! {
    TransactionBuilder;
    impl['tx, 'a, 'conn, S, T];
    TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'a, S, DeleteInitial, T>, DeleteInitial>
        => TransactionBuilder<'tx, 'conn, S, DeleteBuilder<'a, S, DeleteWhereSet, T>, DeleteWhereSet>
}
