//! Bulk writes shared by the `tokio-postgres` and `postgres-sync` drivers.

use drizzle_core::traits::SQLModel;
use drizzle_postgres::traits::PostgresTable;

/// Options of a `bulk_insert`, drizzle-orm's `PostgresBulkInsertOptions`.
///
/// The default is atomic: when the rows take more than one statement, they
/// all run in one transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BulkInsertOptions {
    pub(crate) atomic: bool,
}

impl Default for BulkInsertOptions {
    fn default() -> Self {
        Self { atomic: true }
    }
}

impl BulkInsertOptions {
    /// Whether the statements run in one transaction (`true`, the default)
    /// or each commits on its own (`false`): a failure then keeps the
    /// statements before it, which loads large inputs in short transactions.
    #[must_use]
    pub const fn atomic(mut self, atomic: bool) -> Self {
        self.atomic = atomic;
        self
    }
}

/// The most bind parameters PostgreSQL takes in one statement.
const MAX_BIND_PARAMS: usize = u16::MAX as usize;

/// Splits `rows` into runs whose multi-row `VALUES` statement stays within
/// [`MAX_BIND_PARAMS`].
pub(crate) fn insert_chunks<'a, Table, T>(
    rows: impl IntoIterator<Item = Table::Insert<T>>,
) -> Vec<Vec<Table::Insert<T>>>
where
    Table: PostgresTable<'a>,
{
    let mut chunks = Vec::new();
    let mut chunk = Vec::new();
    let mut params = 0;
    for row in rows {
        let row_params = row.values().params().count();
        if !chunk.is_empty() && params + row_params > MAX_BIND_PARAMS {
            chunks.push(core::mem::take(&mut chunk));
            params = 0;
        }
        params += row_params;
        chunk.push(row);
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    chunks
}
