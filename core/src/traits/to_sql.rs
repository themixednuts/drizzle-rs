//! [`ToSQL`]: rendering values and schema items as SQL fragments.

use crate::prelude::*;
use crate::{
    sql::{ColumnRef, SQL, TableRef, Token},
    traits::SQLParam,
};

#[cfg(feature = "uuid")]
use uuid::Uuid;

/// Something that renders as a [`SQL`] fragment for the value type `V`.
///
/// Columns, tables, expressions, query builders and plain Rust values all
/// implement it. Rust values (`i64`, `&str`, `Option<T>`, ...) render as
/// bound parameters, `None` renders as `NULL`, and lists (`Vec<T>`, arrays,
/// slices) render as comma-separated items. `'a` lets the fragment borrow
/// its inputs instead of copying them.
///
/// # Examples
///
/// Rendering a custom type:
///
/// ```
/// use drizzle_core::{SQL, ToSQL};
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
/// struct Now;
///
/// impl<'a> ToSQL<'a, Value> for Now {
///     fn to_sql(&self) -> SQL<'a, Value> {
///         SQL::raw("CURRENT_TIMESTAMP")
///     }
/// }
///
/// assert_eq!(Now.to_sql().sql(), "CURRENT_TIMESTAMP");
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be converted to SQL",
    label = "this type does not implement ToSQL for the current dialect",
    note = "tuples larger than the enabled arity need a larger `colN` feature (col16, col32, col64, col128, col200) on drizzle-core"
)]
pub trait ToSQL<'a, V: SQLParam> {
    /// Renders `self` as a SQL fragment.
    fn to_sql(&self) -> SQL<'a, V>;

    /// Renders `self` as a SQL fragment, consuming it.
    ///
    /// The default calls [`to_sql`](Self::to_sql). Types that already hold a
    /// fragment (such as `SQL` and `SQLExpr`) override it to avoid a clone.
    fn into_sql(self) -> SQL<'a, V>
    where
        Self: Sized,
    {
        self.to_sql()
    }
}

/// Bytes bound as one binary parameter (BLOB / `bytea`).
///
/// A `Vec<u8>` passed to [`ToSQL`] renders as a list, one parameter per
/// byte. Wrap it in `SQLBytes` (or use [`SQL::bytes`]) to bind it as a
/// single value.
///
/// # Examples
///
/// ```
/// use drizzle_core::{SQL, SQLBytes, ToSQL};
/// # use drizzle_core::{Dialect, SQLParam, SQLiteDialect};
/// # use std::borrow::Cow;
/// # #[derive(Debug, Clone, PartialEq)]
/// # struct Value(Vec<u8>);
/// # impl SQLParam for Value {
/// #     const DIALECT: Dialect = Dialect::SQLite;
/// #     type DialectMarker = SQLiteDialect;
/// # }
/// # impl From<Value> for Cow<'_, Value> {
/// #     fn from(value: Value) -> Self { Cow::Owned(value) }
/// # }
/// # impl From<&[u8]> for Value {
/// #     fn from(bytes: &[u8]) -> Self { Value(bytes.to_vec()) }
/// # }
/// # impl From<Vec<u8>> for Value {
/// #     fn from(bytes: Vec<u8>) -> Self { Value(bytes) }
/// # }
///
/// let data = vec![1_u8, 2, 3];
/// let sql: SQL<'_, Value> = SQLBytes::new(&data[..]).to_sql();
/// assert_eq!(sql.sql(), "?");
/// assert_eq!(sql.params().collect::<Vec<_>>(), [&Value(vec![1, 2, 3])]);
/// ```
#[derive(Debug, Clone)]
pub struct SQLBytes<'a>(pub Cow<'a, [u8]>);

/// Renders as the SQL literal `NULL`.
#[derive(Debug, Clone, Copy, Default)]
pub struct SQLNull;

impl<'a> SQLBytes<'a> {
    /// Wraps borrowed or owned bytes.
    #[inline]
    pub fn new(bytes: impl Into<Cow<'a, [u8]>>) -> Self {
        Self(bytes.into())
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for SQLNull {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::raw("NULL")
    }
}

impl<'a, T, V> From<&T> for SQL<'a, V>
where
    T: ToSQL<'a, V>,
    V: SQLParam,
{
    fn from(value: &T) -> Self {
        value.to_sql()
    }
}

impl<'a, V: SQLParam, T> ToSQL<'a, V> for &T
where
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        (**self).to_sql()
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for () {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::empty()
    }
}

impl<'a, V, T> ToSQL<'a, V> for Vec<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), Token::COMMA)
    }
}

impl<'a, V, T> ToSQL<'a, V> for &'a [T]
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), Token::COMMA)
    }
}

impl<'a, V, T, const N: usize> ToSQL<'a, V> for [T; N]
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::join(self.iter().map(ToSQL::to_sql), Token::COMMA)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for TableRef {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::table(*self)
    }
}

impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for ColumnRef {
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::column(*self)
    }
}

// Implement ToSQL for primitive types
impl<'a, V> ToSQL<'a, V> for &'a str
where
    V: SQLParam + 'a + From<&'a str> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
}

impl<'a, V> ToSQL<'a, V> for Box<str>
where
    V: SQLParam + 'a + From<String> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.to_string()))
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self.into_string()))
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, V> ToSQL<'a, V> for Rc<str>
where
    V: SQLParam + 'a + From<String> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.as_ref().to_string()))
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, V> ToSQL<'a, V> for Arc<str>
where
    V: SQLParam + 'a + From<String> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.as_ref().to_string()))
    }
}

impl<'a, V, T> ToSQL<'a, V> for Box<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        (**self).to_sql()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, V, T> ToSQL<'a, V> for Rc<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        (**self).to_sql()
    }
}

#[cfg(any(feature = "std", feature = "alloc"))]
impl<'a, V, T> ToSQL<'a, V> for Arc<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        (**self).to_sql()
    }
}

impl<'a, V> ToSQL<'a, V> for String
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.clone()))
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
}

#[cfg(feature = "compact-str")]
impl<'a, V> ToSQL<'a, V> for compact_str::CompactString
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.clone()))
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
}

#[cfg(feature = "bytes")]
impl<'a, V> ToSQL<'a, V> for bytes::Bytes
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.clone()))
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
}

#[cfg(feature = "bytes")]
impl<'a, V> ToSQL<'a, V> for bytes::BytesMut
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.clone()))
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, V, const N: usize> ToSQL<'a, V> for arrayvec::ArrayString<N>
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(*self))
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
}

#[cfg(feature = "arrayvec")]
impl<'a, V, const N: usize> ToSQL<'a, V> for arrayvec::ArrayVec<u8, N>
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.clone()))
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
}

#[cfg(feature = "smallvec-types")]
impl<'a, V, const N: usize> ToSQL<'a, V> for smallvec::SmallVec<[u8; N]>
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(self.clone()))
    }

    fn into_sql(self) -> SQL<'a, V> {
        SQL::param(V::from(self))
    }
}

impl<'a, V> ToSQL<'a, V> for Cow<'a, str>
where
    V: SQLParam + 'a + From<&'a str> + From<String> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match self {
            Cow::Borrowed(value) => SQL::param(V::from(*value)),
            Cow::Owned(value) => SQL::param(V::from(value.clone())),
        }
    }

    fn into_sql(self) -> SQL<'a, V> {
        match self {
            Cow::Borrowed(value) => SQL::param(V::from(value)),
            Cow::Owned(value) => SQL::param(V::from(value)),
        }
    }
}

impl<'a, V> ToSQL<'a, V> for Cow<'a, [u8]>
where
    V: SQLParam + 'a + From<&'a [u8]> + From<Vec<u8>> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match self {
            Cow::Borrowed(value) => SQL::param(V::from(*value)),
            Cow::Owned(value) => SQL::param(V::from(value.clone())),
        }
    }

    fn into_sql(self) -> SQL<'a, V> {
        match self {
            Cow::Borrowed(value) => SQL::param(V::from(value)),
            Cow::Owned(value) => SQL::param(V::from(value)),
        }
    }
}

impl<'a, V> ToSQL<'a, V> for SQLBytes<'a>
where
    V: SQLParam + 'a + From<&'a [u8]> + From<Vec<u8>> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        match &self.0 {
            Cow::Borrowed(value) => SQL::param(V::from(*value)),
            Cow::Owned(value) => SQL::param(V::from(value.clone())),
        }
    }

    fn into_sql(self) -> SQL<'a, V> {
        match self.0 {
            Cow::Borrowed(value) => SQL::param(V::from(value)),
            Cow::Owned(value) => SQL::param(V::from(value)),
        }
    }
}

macro_rules! impl_tosql_param_copy {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl<'a, V> ToSQL<'a, V> for $ty
            where
                V: SQLParam + 'a + From<$ty>,
                V: Into<Cow<'a, V>>,
            {
                fn to_sql(&self) -> SQL<'a, V> {
                    SQL::param(V::from(*self))
                }
            }
        )+
    };
}

impl_tosql_param_copy!(
    i8, i16, i32, i64, f32, f64, bool, char, u8, u16, u32, u64, isize, usize
);

impl<'a, V, T> ToSQL<'a, V> for Option<T>
where
    V: SQLParam + 'a,
    T: ToSQL<'a, V>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        self.as_ref()
            .map_or_else(|| SQLNull.to_sql(), ToSQL::to_sql)
    }
}

#[cfg(feature = "uuid")]
impl<'a, V> ToSQL<'a, V> for Uuid
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(*self))
    }
}

// Date and time values bind as parameters in every dialect that stores them.
#[cfg(feature = "chrono")]
impl_tosql_param_copy!(
    chrono::NaiveDate,
    chrono::NaiveTime,
    chrono::NaiveDateTime,
    chrono::DateTime<chrono::Utc>,
    chrono::DateTime<chrono::FixedOffset>,
    chrono::Duration,
);

#[cfg(feature = "time")]
impl_tosql_param_copy!(
    time::Date,
    time::Time,
    time::PrimitiveDateTime,
    time::OffsetDateTime,
    time::Duration,
);

#[cfg(feature = "jiff")]
impl_tosql_param_copy!(
    jiff::civil::Date,
    jiff::civil::Time,
    jiff::civil::DateTime,
    jiff::Timestamp,
);

#[cfg(feature = "rust-decimal")]
impl<'a, V> ToSQL<'a, V> for rust_decimal::Decimal
where
    V: SQLParam + 'a + From<Self> + Into<Cow<'a, V>>,
{
    fn to_sql(&self) -> SQL<'a, V> {
        SQL::param(V::from(*self))
    }
}
