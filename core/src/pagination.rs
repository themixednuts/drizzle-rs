use crate::SQL;
use crate::expr::Nullability;
use crate::placeholder::{Placeholder, TypedPlaceholder};
use crate::traits::{SQLParam, ToSQL};
use crate::types::Integral;

mod private {
    pub trait Sealed {}
}

/// Argument accepted by `LIMIT` and `OFFSET` clauses.
///
/// Numeric values render as SQL numeric literals, preserving the existing
/// `.limit(10)` output. Placeholders render through the dialect's parameter
/// syntax so prepared statements can bind pagination values.
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
                    SQL::number(usize::try_from(self).expect("LIMIT/OFFSET value must fit usize"))
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
                    SQL::number(
                        usize::try_from(self)
                            .expect("LIMIT/OFFSET value must be non-negative and fit usize"),
                    )
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
