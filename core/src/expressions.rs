pub mod conditions;

use crate::{SQL, SQLParam, ToSQL, Token};

pub fn alias<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E, alias: &'a str) -> SQL<'a, V> {
    expr.to_sql().alias(alias)
}

pub fn count<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("COUNT", expr.to_sql())
}

pub fn sum<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("SUM", expr.to_sql())
}

pub fn avg<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("AVG", expr.to_sql())
}

pub fn min<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("MIN", expr.to_sql())
}

pub fn max<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("MAX", expr.to_sql())
}

pub fn distinct<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::raw("DISTINCT").append(&expr)
}

pub fn coalesce<'a, V: SQLParam + 'a, E: ToSQL<'a, V>, D: ToSQL<'a, V>>(
    expr: E,
    default: D,
) -> SQL<'a, V> {
    SQL::func(
        "COALESCE",
        expr.to_sql().push(Token::COMMA).append(default.to_sql()),
    )
}

pub fn r#typeof<'a, V: SQLParam + 'a, E: ToSQL<'a, V>>(expr: E) -> SQL<'a, V> {
    SQL::func("TYPEOF", expr.to_sql())
}

pub fn cast<'a, V: SQLParam + 'a, E: ToSQL<'a, V>, Type: ToSQL<'a, V>>(
    expr: E,
    target_type: Type,
) -> SQL<'a, V> {
    SQL::func(
        "CAST",
        expr.to_sql().push(Token::AS).append(&target_type),
    )
}

pub fn r#in<'a, V: SQLParam + 'a, E: ToSQL<'a, V>, S: ToSQL<'a, V>>(
    expr: E,
    values: S,
) -> SQL<'a, V> {
    expr.to_sql()
        .push(Token::IN)
        .append(values.to_sql().parens())
}
