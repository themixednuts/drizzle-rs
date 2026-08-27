//! Portable ORM contracts exercised by every SQLite adapter.

use drizzle::sqlite::{connection::SQLiteTransactionType, prelude::*};

crate::common::crud_join::shared_crud_join_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::prepared::shared_prepared_statement_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    drizzle::sqlite::types::Integer
);
crate::common::transaction::shared_transaction_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    SQLiteTransactionType::Deferred
);
