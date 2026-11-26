use crate::SQL;
use crate::expr::Nullability;
use crate::placeholder::{Placeholder, TypedPlaceholder};
use crate::prelude::{Cow, ToString};
use crate::traits::{SQLParam, ToSQL};
use crate::types::Integral;

mod private {
    pub trait Sealed {}
}

/// A value accepted by `.limit(...)` and `.offset(...)`: an integer or an
/// integer placeholder.
///
/// Integers render as literals (`LIMIT 10`) unless the value type binds them
/// as parameters through [`SQLParam::pagination_param`]. PostgreSQL and
/// MySQL do, so the SQL text stays the same across pages and a cached
/// prepared statement can be reused. Placeholders render as parameters, so
/// a prepared statement can take the page size when it runs.
///
/// Every value is sent to the database, zero and negative ones included. A
/// value that does not fit in `usize`, such as a negative one, is written as
/// a literal: PostgreSQL and MySQL reject a negative `LIMIT` or `OFFSET`,
/// and SQLite reads a negative `LIMIT` as no limit and a negative `OFFSET`
/// as zero.
///
/// # Examples
///
/// ```
/// use drizzle_core::{PaginationArg, SQL};
/// # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
/// # use std::borrow::Cow;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct Value(i64);
/// # impl SQLParam for Value {
/// #     const DIALECT: Dialect = Dialect::SQLite;
/// #     type DialectMarker = SQLiteDialect;
/// # }
/// # impl From<Value> for Cow<'_, Value> {
/// #     fn from(value: Value) -> Self { Cow::Owned(value) }
/// # }
///
/// let limit: SQL<'_, Value> = 10_i32.into_pagination_sql();
/// assert_eq!(limit.sql(), "10");
///
/// let no_limit: SQL<'_, Value> = (-1_i64).into_pagination_sql();
/// assert_eq!(no_limit.sql(), "-1");
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a LIMIT/OFFSET argument",
    label = "expected an integer value or an integer placeholder"
)]
pub trait PaginationArg<'a, V: SQLParam + 'a>: private::Sealed {
    /// Renders the value as a literal or parameter.
    fn into_pagination_sql(self) -> SQL<'a, V>;
}

/// Renders an integer pagination value: as a bound parameter when the
/// dialect's value type opts in and the value fits `usize`, otherwise as a
/// numeric literal.
fn pagination_value_sql<'a, V, T>(value: T) -> SQL<'a, V>
where
    V: SQLParam + 'a,
    T: Copy + ToString,
    usize: TryFrom<T>,
{
    match usize::try_from(value) {
        Ok(value) => match V::pagination_param(value) {
            Some(param) => SQL::param(Cow::Owned(param)),
            None => SQL::number(value),
        },
        Err(_) => SQL::raw(value.to_string()),
    }
}

macro_rules! impl_integer_pagination_arg {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}

            impl<'a, V> PaginationArg<'a, V> for $ty
            where
                V: SQLParam + 'a,
            {
                fn into_pagination_sql(self) -> SQL<'a, V> {
                    pagination_value_sql(self)
                }
            }
        )+
    };
}

impl_integer_pagination_arg!(usize, u8, u16, u32, u64, isize, i8, i16, i32, i64);

impl private::Sealed for Placeholder {}

impl<'a, V> PaginationArg<'a, V> for Placeholder
where
    V: SQLParam + 'a,
{
    fn into_pagination_sql(self) -> SQL<'a, V> {
        self.to_sql()
    }
}

impl<T, N> private::Sealed for TypedPlaceholder<T, N>
where
    T: Integral,
    N: Nullability,
{
}

impl<'a, V, T, N> PaginationArg<'a, V> for TypedPlaceholder<T, N>
where
    V: SQLParam + 'a,
    T: Integral,
    N: Nullability,
{
    fn into_pagination_sql(self) -> SQL<'a, V> {
        self.to_sql()
    }
}
