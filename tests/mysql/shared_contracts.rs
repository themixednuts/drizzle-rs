//! Portable ORM contracts exercised by both MySQL adapters.

use drizzle::mysql::prelude::*;

crate::shared_crud_join_suite!(mysql, MySQLTable, MySQLSchema);
crate::shared_prepared_statement_suite!(mysql, MySQLTable, MySQLSchema, drizzle::mysql::types::Int);
crate::shared_transaction_suite!(
    mysql,
    MySQLTable,
    MySQLSchema,
    MySQLTransactionConfig::default()
);
