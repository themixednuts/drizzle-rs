use crate::SQL;
use crate::expr::Nullability;
use crate::placeholder::{Placeholder, TypedPlaceholder};
use crate::prelude::Cow;
use crate::traits::{SQLParam, ToSQL};
use crate::types::Integral;

mod private {
    pub trait Sealed {}
}

/// Argument accepted by `LIMIT` and `OFFSET` clauses.
///
/// Numeric values render as SQL numeric literals, unless the dialect's value
/// type opts into bound pagination parameters via
/// [`SQLParam::pagination_param`] (`PostgreSQL` does, so `.limit(10)` renders
/// as `LIMIT $n` there, keeping SQL text stable for statement caching).
/// Placeholders render through the dialect's parameter syntax so prepared
/// statements can bind pagination values.
///
/// # Panics
///
/// Numeric arguments panic during SQL construction when they are negative or
/// too large to fit in `usize`.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a LIMIT/OFFSET argument",
    label = "expected a non-negative integer value or an integer placeholder"
)]
pub trait PaginationArg<'a, V: SQLParam + 'a>: private::Sealed {
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
