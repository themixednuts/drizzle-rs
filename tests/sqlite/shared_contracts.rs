//! Portable ORM contracts exercised by every SQLite adapter.

use drizzle::sqlite::{TransactionConfig, prelude::*};

crate::common::crud_join::shared_crud_join_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::conditions::shared_condition_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    SQLiteFromRow
);
crate::common::derived::shared_derived_table_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::prepared::shared_prepared_statement_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    drizzle::sqlite::types::Integer
);
crate::common::rows::shared_rows_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    TransactionConfig::Deferred
);
crate::common::transaction::shared_transaction_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    TransactionConfig::Deferred
);
