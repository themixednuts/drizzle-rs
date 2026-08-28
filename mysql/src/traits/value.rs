use crate::values::{MySQLValue, OwnedMySQLValue};
use drizzle_core::error::DrizzleError;

/// Trait for custom Rust types that map to a MySQL column.
///
/// Implement this trait for wrappers that need a type-owned storage codec.
/// The table macro uses [`SQLType`](Self::SQLType) for typed expressions and
/// [`SQL_TYPE`](Self::SQL_TYPE) for DDL and schema metadata.
#[diagnostic::on_unimplemented(
    message = "`{Self}` cannot be used as a MySQL column type",
    note = "add #[derive(MySQLEnum)] for enum types, or implement DrizzleMySQLColumn"
)]
pub trait DrizzleMySQLColumn: Sized {
    /// Drizzle SQL type marker for this column.
    type SQLType: MySQLColumnType;

    /// MySQL column type, such as `BINARY(4)`, `TEXT`, or `BIGINT UNSIGNED`.
    const SQL_TYPE: &'static str;

    /// Decode a value returned by a MySQL driver.
    ///
    /// # Errors
    ///
    /// Returns [`DrizzleError::ConversionError`] when `value` does not match
    /// this column's storage representation.
    fn decode(value: MySQLValue<'_>) -> Result<Self, DrizzleError>;

    /// Encode this value for an insert, update, or comparison parameter.
    fn encode(&self) -> MySQLValue<'_>;

    /// Encode this value into owned parameter storage.
    fn encode_owned(self) -> OwnedMySQLValue {
        self.encode().into_owned()
    }

    /// Decode a value emitted by MySQL's relational JSON projection.
    ///
    /// Override this only when the projected representation intentionally
    /// differs from the binary-protocol representation.
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

/// Relational JSON metadata for built-in MySQL SQL type markers.
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
