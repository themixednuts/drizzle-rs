//! Portable ORM contracts exercised by both MySQL adapters.

use drizzle::mysql::prelude::*;

crate::common::crud_join::shared_crud_join_suite!(mysql, MySQLTable, MySQLSchema);
crate::common::conditions::shared_condition_suite!(mysql, MySQLTable, MySQLSchema, MySQLFromRow);
crate::common::derived::shared_derived_table_suite!(mysql, MySQLTable, MySQLSchema);
crate::common::derived::shared_lateral_derived_table_suite!(mysql, MySQLTable, MySQLSchema);
crate::common::prepared::shared_prepared_statement_suite!(
    mysql,
    MySQLTable,
    MySQLSchema,
    drizzle::mysql::types::Int
);
crate::common::rows::shared_rows_suite!(
    mysql,
    MySQLTable,
    MySQLSchema,
    TransactionConfig::default()
);
crate::common::transaction::shared_transaction_suite!(
    mysql,
    MySQLTable,
    MySQLSchema,
    TransactionConfig::default()
);
