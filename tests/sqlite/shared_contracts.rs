//! Portable ORM contracts exercised by every SQLite adapter.

use drizzle::sqlite::{TransactionConfig, prelude::*};

crate::common::crud_join::shared_crud_join_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::conditions::shared_condition_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    SQLiteFromRow
);
crate::common::condition_list::shared_condition_list_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    SQLiteValue,
    drizzle::sqlite::types::Integer
);
crate::common::alias::shared_alias_suite!(sqlite, SQLiteTable, SQLiteSchema, SQLiteFromRow);
crate::common::delete::shared_delete_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::delete::shared_delete_returning_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::derived::shared_derived_table_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::subquery::shared_subquery_suite!(sqlite, SQLiteTable, SQLiteSchema, SQLiteFromRow);
crate::common::expressions::shared_expression_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::foreign_keys::shared_foreign_key_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::comment::shared_comment_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::wrappers::shared_wrapper_type_suite!(sqlite, SQLiteTable, SQLiteSchema);
#[cfg(feature = "arrayvec")]
crate::common::arrayvec::shared_arrayvec_suite!(sqlite, SQLiteTable, SQLiteSchema);
// Only the bundled rusqlite build can be given SQLITE_ENABLE_MATH_FUNCTIONS
// (see .cargo/config.toml), so the math-extension contract is rusqlite-only.
#[cfg(all(
    feature = "math",
    feature = "rusqlite",
    not(any(feature = "libsql", feature = "turso"))
))]
crate::common::expressions::shared_math_extension_suite!(sqlite, SQLiteTable, SQLiteSchema);
crate::common::prepared::shared_prepared_statement_suite!(
    sqlite,
    SQLiteTable,
    SQLiteSchema,
    drizzle::sqlite::types::Integer,
    TransactionConfig::Deferred
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
