//! Portable ORM contracts exercised by every PostgreSQL adapter.

use drizzle::postgres::prelude::*;
use drizzle_postgres::common::PostgresTransactionType;

crate::shared_crud_join_suite!(postgres, PostgresTable, PostgresSchema);
crate::shared_prepared_statement_suite!(
    postgres,
    PostgresTable,
    PostgresSchema,
    drizzle::postgres::types::Int4
);
crate::shared_transaction_suite!(
    postgres,
    PostgresTable,
    PostgresSchema,
    PostgresTransactionType::default()
);
