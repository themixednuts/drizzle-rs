use drizzle_core::SQLTable;

use crate::{common::MySQLSchemaType, values::MySQLValue};

/// A generated table whose SQL and values use the MySQL dialect.
pub trait MySQLTable<'a>: SQLTable<'a, MySQLSchemaType, MySQLValue<'a>> {
    /// Backtick-quoted, database-qualified identifier for const DDL generation.
    const DDL_QUALIFIED_NAME: &'static str;
}

impl<'a, T> MySQLTable<'a> for &T
where
    T: MySQLTable<'a>,
    for<'r> &'r T: SQLTable<'a, MySQLSchemaType, MySQLValue<'a>>,
{
    const DDL_QUALIFIED_NAME: &'static str = T::DDL_QUALIFIED_NAME;
}
