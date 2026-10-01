//! `PostgreSQL` POSIX regular expression operators.
//!
//! | Operator | Function / method | True when the text |
//! |---|---|---|
//! | `~` | [`regex_match`] | matches the pattern (case-sensitive) |
//! | `~*` | [`regex_match_ci`] | matches the pattern (case-insensitive) |
//! | `!~` | [`regex_not_match`] | does not match the pattern (case-sensitive) |
//! | `!~*` | [`regex_not_match_ci`] | does not match the pattern (case-insensitive) |
//!
//! The left operand must be textual (`text`, `varchar`, `char` or an enum;
//! see [`Textual`](drizzle_types::Textual)). The pattern is a `&str` bound as
//! a `text` parameter. The pattern matches anywhere in the string unless it is
//! anchored with `^` or `$`. Results are NULL when the left operand is NULL.
//!
//! # Examples
//!
//! ```
//! use drizzle_core::{ToSQL, expr::raw_non_null};
//! use drizzle_postgres::expr::RegexExprExt;
//! use drizzle_postgres::values::PostgresValue;
//! use drizzle_types::postgres::types::Text;
//!
//! let sku = raw_non_null::<PostgresValue, Text>("sku");
//! let cond = sku.regex_match("^[A-Z]{3}-[0-9]+$");
//! assert_eq!(cond.to_sql().sql(), "sku ~ $1");
//! ```
//!
//! # Type safety
//!
//! ```compile_fail
//! use drizzle_core::expr::raw_non_null;
//! use drizzle_postgres::expr::regex_match;
//! use drizzle_postgres::values::PostgresValue;
//! use drizzle_types::postgres::types::Int8;
//!
//! let id = raw_non_null::<PostgresValue, Int8>("id");
//! let _ = regex_match(id, "^1"); // `int8` is not textual
//! ```

use crate::values::PostgresValue;
use drizzle_core::expr::{Expr, NonNull, SQLExpr};
use drizzle_core::scope::Arg;
use drizzle_core::sql::{SQL, SQLChunk};
use drizzle_types::postgres::types::Boolean;

/// Tests whether text matches a regular expression, case-sensitively (`~`).
///
/// The operand must be textual; see the [module docs](self) for the rules.
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
/// The operand must be textual; see the [module docs](self) for the rules.
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
/// The operand must be textual; see the [module docs](self) for the rules.
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
/// The operand must be textual; see the [module docs](self) for the rules.
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
