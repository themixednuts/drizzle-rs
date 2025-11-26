use drizzle_core::SQL;

use crate::{SQLiteSQL, traits::ToSQLiteSQL};

pub fn json<'a>(value: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a> {
    SQL::func("json", value.to_sql())
}

pub fn jsonb<'a>(value: impl ToSQLiteSQL<'a>) -> SQLiteSQL<'a> {
    SQL::func("jsonb", value.to_sql())
}
