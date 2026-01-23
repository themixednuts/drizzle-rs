use drizzle_core::SQL;

use crate::{SQLiteSQL, values::SQLiteValue};
use drizzle_core::ToSQL;

pub fn json<'a>(value: impl ToSQL<'a, SQLiteValue<'a>>) -> SQLiteSQL<'a> {
    SQL::func("json", value.to_sql())
}

pub fn jsonb<'a>(value: impl ToSQL<'a, SQLiteValue<'a>>) -> SQLiteSQL<'a> {
    SQL::func("jsonb", value.to_sql())
}
