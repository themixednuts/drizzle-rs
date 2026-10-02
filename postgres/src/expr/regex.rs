//! `PostgreSQL` POSIX regular expression operators. Documented in [`crate::expr`].

use crate::values::PostgresValue;
use drizzle_core::expr::{Expr, NonNull, SQLExpr};
use drizzle_core::scope::Arg;
use drizzle_core::sql::{SQL, SQLChunk};
use drizzle_types::postgres::types::Boolean;

/// Tests whether text matches a regular expression, case-sensitively (`~`).
///
/// `expr` must be textual (`text`, `varchar`, `char` or an enum); `pattern`
/// is bound as a `text` parameter. The result is NULL when `expr` is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::regex_match;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Text;
///
/// let name = raw_non_null::<PostgresValue, Text>("name");
/// let cond = regex_match(name, "^[A-Z]");
/// assert_eq!(cond.to_sql().sql(), "name ~ $1");
/// ```
#[allow(clippy::type_complexity)]
pub fn regex_match<'a, E>(
    expr: E,
    pattern: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, E::Aggregate, Arg<E::Nullable, E::Sources>>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: drizzle_types::Textual,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("~".into()))
            .append(SQL::param(PostgresValue::Text(pattern.into()))),
    )
}

/// Tests whether text matches a regular expression, ignoring case (`~*`).
///
/// `expr` must be textual (`text`, `varchar`, `char` or an enum); `pattern`
/// is bound as a `text` parameter. The result is NULL when `expr` is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::regex_match_ci;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Text;
///
/// let name = raw_non_null::<PostgresValue, Text>("name");
/// let cond = regex_match_ci(name, "^john");
/// assert_eq!(cond.to_sql().sql(), "name ~* $1");
/// ```
#[allow(clippy::type_complexity)]
pub fn regex_match_ci<'a, E>(
    expr: E,
    pattern: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, E::Aggregate, Arg<E::Nullable, E::Sources>>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: drizzle_types::Textual,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("~*".into()))
            .append(SQL::param(PostgresValue::Text(pattern.into()))),
    )
}

/// Tests whether text does not match a regular expression, case-sensitively (`!~`).
///
/// `expr` must be textual (`text`, `varchar`, `char` or an enum); `pattern`
/// is bound as a `text` parameter. The result is NULL when `expr` is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::regex_not_match;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Text;
///
/// let name = raw_non_null::<PostgresValue, Text>("name");
/// let cond = regex_not_match(name, "^[0-9]");
/// assert_eq!(cond.to_sql().sql(), "name !~ $1");
/// ```
#[allow(clippy::type_complexity)]
pub fn regex_not_match<'a, E>(
    expr: E,
    pattern: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, E::Aggregate, Arg<E::Nullable, E::Sources>>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: drizzle_types::Textual,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("!~".into()))
            .append(SQL::param(PostgresValue::Text(pattern.into()))),
    )
}

/// Tests whether text does not match a regular expression, ignoring case (`!~*`).
///
/// `expr` must be textual (`text`, `varchar`, `char` or an enum); `pattern`
/// is bound as a `text` parameter. The result is NULL when `expr` is NULL.
///
/// # Examples
///
/// ```
/// use drizzle_postgres::expr::regex_not_match_ci;
/// use drizzle_core::{ToSQL, expr::raw_non_null};
/// use drizzle_postgres::values::PostgresValue;
/// use drizzle_types::postgres::types::Text;
///
/// let name = raw_non_null::<PostgresValue, Text>("name");
/// let cond = regex_not_match_ci(name, "^admin");
/// assert_eq!(cond.to_sql().sql(), "name !~* $1");
/// ```
#[allow(clippy::type_complexity)]
pub fn regex_not_match_ci<'a, E>(
    expr: E,
    pattern: &'a str,
) -> SQLExpr<'a, PostgresValue<'a>, Boolean, NonNull, E::Aggregate, Arg<E::Nullable, E::Sources>>
where
    E: Expr<'a, PostgresValue<'a>>,
    E::SQLType: drizzle_types::Textual,
{
    SQLExpr::new(
        expr.to_sql()
            .push(SQLChunk::Raw("!~*".into()))
            .append(SQL::param(PostgresValue::Text(pattern.into()))),
    )
}

/// Method forms of the regex operators, available on every `PostgreSQL` expression.
///
/// Each method calls the free function of the same name and has the same operand rules.
pub trait RegexExprExt<'a>: Expr<'a, PostgresValue<'a>> + Sized {
    /// Tests whether `self` matches `pattern`, case-sensitively (`~`). See [`regex_match`].
    #[allow(clippy::type_complexity)]
    fn regex_match(
        self,
        pattern: &'a str,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        Self::Aggregate,
        Arg<Self::Nullable, Self::Sources>,
    >
    where
        Self::SQLType: drizzle_types::Textual,
    {
        regex_match(self, pattern)
    }

    /// Tests whether `self` matches `pattern`, ignoring case (`~*`). See [`regex_match_ci`].
    #[allow(clippy::type_complexity)]
    fn regex_match_ci(
        self,
        pattern: &'a str,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        Self::Aggregate,
        Arg<Self::Nullable, Self::Sources>,
    >
    where
        Self::SQLType: drizzle_types::Textual,
    {
        regex_match_ci(self, pattern)
    }

    /// Tests whether `self` does not match `pattern`, case-sensitively (`!~`). See [`regex_not_match`].
    #[allow(clippy::type_complexity)]
    fn regex_not_match(
        self,
        pattern: &'a str,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        Self::Aggregate,
        Arg<Self::Nullable, Self::Sources>,
    >
    where
        Self::SQLType: drizzle_types::Textual,
    {
        regex_not_match(self, pattern)
    }

    /// Tests whether `self` does not match `pattern`, ignoring case (`!~*`). See [`regex_not_match_ci`].
    #[allow(clippy::type_complexity)]
    fn regex_not_match_ci(
        self,
        pattern: &'a str,
    ) -> SQLExpr<
        'a,
        PostgresValue<'a>,
        Boolean,
        NonNull,
        Self::Aggregate,
        Arg<Self::Nullable, Self::Sources>,
    >
    where
        Self::SQLType: drizzle_types::Textual,
    {
        regex_not_match_ci(self, pattern)
    }
}

impl<'a, E: Expr<'a, PostgresValue<'a>>> RegexExprExt<'a> for E {}
