use crate::SQL;
use crate::expr::Nullability;
use crate::placeholder::{Placeholder, TypedPlaceholder};
use crate::prelude::Cow;
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
/// # Panics
///
/// An integer argument panics when the SQL is built if it is negative or
/// does not fit in `usize`.
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
/// ```
///
/// ```should_panic
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
/// // panics: LIMIT/OFFSET value must be non-negative
/// let _: SQL<'_, Value> = (-1_i64).into_pagination_sql();
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a LIMIT/OFFSET argument",
    label = "expected a non-negative integer value or an integer placeholder"
)]
pub trait PaginationArg<'a, V: SQLParam + 'a>: private::Sealed {
    /// Renders the value as a literal or parameter.
    #[track_caller]
    fn into_pagination_sql(self) -> SQL<'a, V>;
}

/// Renders a validated pagination value either as a bound parameter (when the
/// dialect's value type opts in) or as a numeric literal.
fn pagination_value_sql<'a, V>(value: usize) -> SQL<'a, V>
where
    V: SQLParam + 'a,
{
    match V::pagination_param(value) {
        Some(param) => SQL::param(Cow::Owned(param)),
        None => SQL::number(value),
    }
}

macro_rules! impl_unsigned_pagination_arg {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}

            impl<'a, V> PaginationArg<'a, V> for $ty
            where
                V: SQLParam + 'a,
            {
                #[track_caller]
                fn into_pagination_sql(self) -> SQL<'a, V> {
                    let value =
                        usize::try_from(self).expect("LIMIT/OFFSET value must fit usize");
                    pagination_value_sql(value)
                }
            }
        )+
    };
}

macro_rules! impl_signed_pagination_arg {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl private::Sealed for $ty {}

            impl<'a, V> PaginationArg<'a, V> for $ty
            where
                V: SQLParam + 'a,
            {
                #[track_caller]
                fn into_pagination_sql(self) -> SQL<'a, V> {
                    let value = usize::try_from(self)
                        .expect("LIMIT/OFFSET value must be non-negative and fit usize");
                    pagination_value_sql(value)
                }
            }
        )+
    };
}

impl_unsigned_pagination_arg!(usize, u8, u16, u32, u64);
impl_signed_pagination_arg!(isize, i8, i16, i32, i64);

impl private::Sealed for Placeholder {}

impl<'a, V> PaginationArg<'a, V> for Placeholder
where
    V: SQLParam + 'a,
{
    #[track_caller]
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
    #[track_caller]
    fn into_pagination_sql(self) -> SQL<'a, V> {
        self.to_sql()
    }
}
