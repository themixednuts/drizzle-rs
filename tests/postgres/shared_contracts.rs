//! Portable ORM contracts exercised by every PostgreSQL adapter.

use drizzle::postgres::TransactionConfig;
use drizzle::postgres::prelude::*;

crate::common::crud_join::shared_crud_join_suite!(postgres, PostgresTable, PostgresSchema);
crate::common::derived::shared_derived_table_suite!(postgres, PostgresTable, PostgresSchema);
crate::common::derived::shared_lateral_derived_table_suite!(
    postgres,
    PostgresTable,
    PostgresSchema
);
crate::common::prepared::shared_prepared_statement_suite!(
    postgres,
    PostgresTable,
    PostgresSchema,
    drizzle::postgres::types::Int4
);
crate::common::rows::shared_rows_suite!(
    postgres,
    PostgresTable,
    PostgresSchema,
    TransactionConfig::default()
);
crate::common::transaction::shared_transaction_suite!(
    postgres,
    PostgresTable,
    PostgresSchema,
    TransactionConfig::default()
);
