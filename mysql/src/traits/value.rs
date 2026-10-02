//! [`DrizzleMySQLColumn`], for custom column types, and the JSON projection
//! metadata of the built-in SQL type markers.

use crate::values::{MySQLValue, OwnedMySQLValue};
use drizzle_core::error::DrizzleError;

/// A Rust type that can be stored in a `MySQL` column.
///
/// Implement it to use your own type as a field of a `#[MySQLTable]`.
/// [`SQLType`](Self::SQLType) sets which expressions the column can be used
/// in, [`SQL_TYPE`](Self::SQL_TYPE) is written into the DDL, and
/// [`decode`](Self::decode) / [`encode`](Self::encode) convert values.
/// Implementing it also gives `From<T> for MySQLValue`, which goes through
/// [`encode_owned`](Self::encode_owned).
///
/// # Examples
///
/// A `u32` stored as four big-endian bytes:
///
/// ```
/// use drizzle_core::error::DrizzleError;
/// use drizzle_mysql::traits::DrizzleMySQLColumn;
/// use drizzle_mysql::types::Binary;
/// use drizzle_mysql::values::MySQLValue;
/// use std::borrow::Cow;
///
/// struct U32Be(u32);
///
/// impl DrizzleMySQLColumn for U32Be {
///     type SQLType = Binary;
///     const SQL_TYPE: &'static str = "BINARY(4)";
///
///     fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError> {
///         let MySQLValue::Bytes(bytes) = value else {
///             return Err(DrizzleError::ConversionError("expected bytes".into()));
///         };
///         let bytes: [u8; 4] = bytes
///             .as_ref()
///             .try_into()
///             .map_err(|_| DrizzleError::ConversionError("expected 4 bytes".into()))?;
///         Ok(Self(u32::from_be_bytes(bytes)))
///     }
///
///     fn encode(&self) -> MySQLValue<'_> {
///         MySQLValue::Bytes(Cow::Owned(self.0.to_be_bytes().to_vec()))
///     }
/// }
///
/// let value = MySQLValue::from(U32Be(7));
/// assert_eq!(value, MySQLValue::Bytes(Cow::Borrowed(&[0, 0, 0, 7][..])));
/// assert_eq!(U32Be::decode(value).unwrap().0, 7);
/// assert!(U32Be::decode(MySQLValue::Int(7)).is_err());
/// ```
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a MySQL column type",
    note = "add #[derive(MySQLEnum)] for enum types, or implement DrizzleMySQLColumn"
)]
pub trait DrizzleMySQLColumn: Sized {
    /// The SQL type marker of the column, one of the markers in
    /// [`crate::types`].
    type SQLType: MySQLColumnType;

    /// The column type written into the DDL, such as `BINARY(4)`, `TEXT` or
    /// `BIGINT UNSIGNED`.
    const SQL_TYPE: &'static str;

    /// Decodes a value read from the database.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when `value` does not match
    /// this column's storage representation.
    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError>;

    /// Encodes the value as a bind parameter, borrowing from `self` where
    /// possible.
    fn encode(&self) -> MySQLValue<'_>;

    /// Encodes the value as an owned bind parameter.
    ///
    /// The default calls [`encode`](Self::encode) and copies the result.
    /// Override it when the type can move its buffer into the value instead.
    fn encode_owned(self) -> OwnedMySQLValue {
        self.encode().into_owned()
    }

    /// Decodes a cell from the JSON produced by the relational query API
    /// (`query` feature).
    ///
    /// The default converts the JSON to the value the driver would return
    /// for [`SQLType`](Self::SQLType) and calls [`decode`](Self::decode).
    /// Override it only when the JSON form differs from that.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] if the JSON does not match
    /// the column's type or [`decode`](Self::decode) rejects it.
    #[cfg(feature = "query")]
    fn decode_json(value: &serde_json::Value) -> Result<Self, DrizzleError>
    where
        Self::SQLType: MySQLColumnType,
    {
        let value = crate::driver::projected_value(
            value,
            <Self::SQLType as MySQLColumnType>::JSON_STORAGE,
        )?;
        Self::decode(value.into())
    }
}

/// MySQL JSON representation used for a typed relational projection.
#[cfg(feature = "query")]
#[doc(hidden)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MySQLJsonStorage {
    Signed,
    Unsigned,
    Double,
    Boolean,
    Text,
    SignedText,
    UnsignedText,
    FloatText,
    Binary,
    Json,
    Date,
    Time,
    DateTime,
}

/// How the relational query API projects a built-in `MySQL` SQL type marker
/// to JSON. Sealed.
#[doc(hidden)]
pub trait MySQLColumnType: drizzle_core::types::DataType + private::Sealed {
    #[cfg(feature = "query")]
    const JSON_PROJECTION: drizzle_core::query::JsonProjectionKind;
    #[cfg(feature = "query")]
    const JSON_STORAGE: MySQLJsonStorage;
}

mod private {
    pub trait Sealed {}
}

macro_rules! mysql_column_types {
    ($($type:ty => ($projection:ident, $storage:ident)),+ $(,)?) => {
        $(
            impl private::Sealed for $type {}
            impl MySQLColumnType for $type {
                #[cfg(feature = "query")]
                const JSON_PROJECTION: drizzle_core::query::JsonProjectionKind =
                    drizzle_core::query::JsonProjectionKind::$projection;
                #[cfg(feature = "query")]
                const JSON_STORAGE: MySQLJsonStorage = MySQLJsonStorage::$storage;
            }
        )+
    };
}

mysql_column_types! {
    crate::types::TinyInt => (Native, Signed),
    crate::types::SmallInt => (Native, Signed),
    crate::types::MediumInt => (Native, Signed),
    crate::types::Int => (Native, Signed),
    crate::types::TinyIntUnsigned => (Native, Unsigned),
    crate::types::SmallIntUnsigned => (Native, Unsigned),
    crate::types::MediumIntUnsigned => (Native, Unsigned),
    crate::types::IntUnsigned => (Native, Unsigned),
    crate::types::BigInt => (Text, SignedText),
    crate::types::BigIntUnsigned => (Text, UnsignedText),
    crate::types::Float => (Text, FloatText),
    crate::types::Double => (Native, Double),
    crate::types::Decimal => (Text, Text),
    crate::types::Boolean => (Native, Boolean),
    crate::types::Char => (Native, Text),
    crate::types::Varchar => (Native, Text),
    crate::types::TinyText => (Native, Text),
    crate::types::Text => (Native, Text),
    crate::types::MediumText => (Native, Text),
    crate::types::LongText => (Native, Text),
    crate::types::Binary => (TaggedHex, Binary),
    crate::types::Varbinary => (TaggedHex, Binary),
    crate::types::TinyBlob => (TaggedHex, Binary),
    crate::types::Blob => (TaggedHex, Binary),
    crate::types::MediumBlob => (TaggedHex, Binary),
    crate::types::LongBlob => (TaggedHex, Binary),
    crate::types::Json => (Native, Json),
    crate::types::Date => (Text, Date),
    crate::types::Time => (Text, Time),
    crate::types::DateTime => (Text, DateTime),
    crate::types::Timestamp => (Text, DateTime),
    crate::types::Year => (Native, Unsigned),
    crate::types::Enum => (Native, Text),
    crate::types::Set => (Native, Text),
    crate::types::Bit => (Unsigned, Unsigned),
    crate::types::Any => (Native, Json),
}
