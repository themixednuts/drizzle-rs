use crate::core::{SQL, ToSQL};
use crate::{
    SQLParam,
    core::{Placeholder, PlaceholderStyle},
};
#[cfg(feature = "serde_json")]
use serde::{Serialize, de::DeserializeOwned};
use std::borrow::Cow;

// Import SQLiteValue from drivers
use drivers::SQLiteValue;

// #[cfg(feature = "rusqlite")]
// use rusqlite::types::{Value as RusqliteValue, ValueRef};

pub type Integer = i64;
pub type Real = f64;
pub type Text<'a> = Cow<'a, str>;
pub type Blob<'a> = Cow<'a, [u8]>;

// --- SQLiteValue definition and From impls moved to drivers/src/lib.rs ---

impl<'a> SQLParam for SQLiteValue<'a> {}

#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Number {
    Integer(Integer),
    Real(Real),
}

impl Default for Number {
    fn default() -> Self {
        Self::Integer(Default::default())
    }
}

impl From<i64> for Number {
    fn from(value: i64) -> Self {
        Self::Integer(value)
    }
}

impl From<f64> for Number {
    fn from(value: f64) -> Self {
        Self::Real(value)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Eq, Ord)]
pub enum SQLiteTableType {
    Table,
    View,
    Index,
    Trigger,
}

// Add SQLite-specific placeholder helpers

/// Creates a named parameter with SQLite colon syntax (:name)
pub fn named_param(name: &str) -> Placeholder<'_> {
    Placeholder::with_style(name, PlaceholderStyle::Colon)
}

/// Creates a named parameter with SQLite @ syntax (@name)
pub fn at_param(name: &str) -> Placeholder<'_> {
    Placeholder::with_style(name, PlaceholderStyle::AtSign)
}

/// Creates a named parameter with SQLite $ syntax ($name)
pub fn dollar_param(name: &str) -> Placeholder<'_> {
    Placeholder::with_style(name, PlaceholderStyle::Dollar)
}

/// Trait for types that can be converted to a SQLiteValue
///
/// This trait is similar to `Into<SQLiteValue>` but avoids orphan rule issues,
/// allowing it to be implemented for foreign types (such as user-defined enums).
pub trait IntoSQLiteValue<'a> {
    /// Convert the value to a SQLiteValue
    fn into_sqlite_value(self) -> SQLiteValue<'a>;
}

// Implement for SQLiteValue itself
impl<'a> IntoSQLiteValue<'a> for SQLiteValue<'a> {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        self // Identity conversion
    }
}

// Implement for common types (now using the imported SQLiteValue)
impl<'a> IntoSQLiteValue<'a> for &'a str {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self) // Use From impl defined in drivers
    }
}

impl<'a> IntoSQLiteValue<'a> for String {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self) // Use From impl defined in drivers
    }
}

impl<'a> IntoSQLiteValue<'a> for i64 {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for i32 {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for i16 {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for i8 {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for u32 {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for u16 {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for u8 {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for usize {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for f64 {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for bool {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a, T: IntoSQLiteValue<'a>> IntoSQLiteValue<'a> for Option<T> {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        match self {
            Some(value) => value.into_sqlite_value(),
            None => SQLiteValue::Null,
        }
    }
}

impl<'a> IntoSQLiteValue<'a> for Vec<u8> {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

impl<'a> IntoSQLiteValue<'a> for &'a [u8] {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

#[cfg(feature = "uuid")]
impl<'a> IntoSQLiteValue<'a> for uuid::Uuid {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        SQLiteValue::from(self)
    }
}

/// Trait for enum values that can be stored in SQLite
///
/// This trait allows enums to be automatically converted to and from SQLite values.
/// The representation is determined by the #[repr] attribute on the enum:
/// - With #[repr(i8)], #[repr(u32)], etc. → stored as INTEGER
/// - Without a numeric repr → stored as TEXT
pub trait SQLiteEnum: Sized + std::fmt::Display + std::str::FromStr + Default {
    /// The representation type of the enum (TEXT or INTEGER)
    const ENUM_REPR: SQLiteEnumRepr;

    /// Convert the enum to an integer (for INTEGER representation)
    fn to_integer(&self) -> i64;

    /// Try to convert from an integer to the enum (for INTEGER representation)
    fn from_integer(i: i64) -> Option<Self>;

    /// Try to convert from SQLiteValue back to the enum
    fn from_sqlite_value(value: &SQLiteValue) -> Option<Self> {
        match value {
            SQLiteValue::Text(s) => s.parse::<Self>().ok(),
            SQLiteValue::Integer(i) => Self::from_integer(*i),
            _ => None, // Ignore Blob and Real for enum conversion
        }
    }
}

/// The representation type of a SQLite enum
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SQLiteEnumRepr {
    /// Enum is stored as TEXT (using variant names)
    Text,
    /// Enum is stored as INTEGER (using discriminants)
    Integer,
}

// Implement IntoSQLiteValue for any type that implements SQLiteEnum
impl<'a, T: SQLiteEnum> IntoSQLiteValue<'a> for T {
    fn into_sqlite_value(self) -> SQLiteValue<'a> {
        match T::ENUM_REPR {
            SQLiteEnumRepr::Text => SQLiteValue::Text(Cow::Owned(format!("{}", self))),
            SQLiteEnumRepr::Integer => SQLiteValue::Integer(self.to_integer()),
        }
    }
}

// Helper function (might not be needed anymore if Into is used primarily)
// pub fn to_param<T: Into<SQLiteValue<'static>>>(value: T) -> SQLiteValue<'static> {
//     value.into()
// }

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::{SQL, ToSQL};
    use std::str::FromStr;
    // Import SQLiteValue from drivers for tests
    use drivers::SQLiteValue;

    // For SQLiteEnum tests
    use procmacros::SQLiteEnum;

    #[test]
    fn test_sqlite_placeholders() {
        // Test named parameter with colon
        let p1 = named_param("id");
        let sql1: SQL<SQLiteValue> = p1.to_sql();
        assert_eq!(sql1.0.as_ref(), ":id");

        // Test named parameter with at symbol
        let p2 = at_param("name");
        let sql2: SQL<SQLiteValue> = p2.to_sql();
        assert_eq!(sql2.0.as_ref(), "@name");

        // Test named parameter with dollar
        let p3 = dollar_param("email");
        let sql3: SQL<SQLiteValue> = p3.to_sql();
        assert_eq!(sql3.0.as_ref(), "$email");
    }

    // Define test enums for SQLiteEnum tests
    // Text-based enum (default)
    #[derive(SQLiteEnum, Debug, Clone, PartialEq)]
    enum Role {
        User,
        Admin,
        Moderator,
    }

    // Add Default implementation
    impl Default for Role {
        fn default() -> Self {
            Self::User
        }
    }

    // Integer-based enum with explicit discriminants
    #[derive(SQLiteEnum, Debug, Clone, PartialEq)]
    #[repr(i32)]
    enum Status {
        Active = 1,
        Inactive = 0,
        Banned = -1,
    }

    // Add Default implementation
    impl Default for Status {
        fn default() -> Self {
            Self::Inactive
        }
    }

    #[test]
    fn test_into_sqlite_value_impls() {
        assert_eq!(
            ("hello").into_sqlite_value(),
            SQLiteValue::Text(Cow::Borrowed("hello"))
        );
        assert_eq!(
            String::from("world").into_sqlite_value(),
            SQLiteValue::Text(Cow::Owned("world".to_string()))
        );
        assert_eq!((42i64).into_sqlite_value(), SQLiteValue::Integer(42));
        assert_eq!((123i32).into_sqlite_value(), SQLiteValue::Integer(123));
        assert_eq!((3.14f64).into_sqlite_value(), SQLiteValue::Real(3.14));
        assert_eq!(true.into_sqlite_value(), SQLiteValue::Integer(1));
        assert_eq!(false.into_sqlite_value(), SQLiteValue::Integer(0));
        let blob_vec: Vec<u8> = vec![1, 2, 3];
        assert_eq!(
            blob_vec.clone().into_sqlite_value(),
            SQLiteValue::Blob(Cow::Owned(blob_vec))
        );
        let blob_slice: &[u8] = &[4, 5, 6];
        assert_eq!(
            blob_slice.into_sqlite_value(),
            SQLiteValue::Blob(Cow::Borrowed(blob_slice))
        );
        assert_eq!(
            Option::<String>::None.into_sqlite_value(),
            SQLiteValue::Null
        );
        assert_eq!(
            Some("optional").into_sqlite_value(),
            SQLiteValue::Text(Cow::Borrowed("optional"))
        );
    }

    #[test]
    fn test_sqlite_enum_conversion() {
        // Test IntoSQLiteValue for text enum
        assert_eq!(
            Role::Admin.into_sqlite_value(),
            SQLiteValue::Text(Cow::Owned("Admin".to_string()))
        );

        // Test IntoSQLiteValue for integer enum
        assert_eq!(Status::Active.into_sqlite_value(), SQLiteValue::Integer(1));

        // Test from_sqlite_value for text enum
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Text(Cow::Borrowed("User"))),
            Some(Role::User)
        );
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Text(Cow::Borrowed("Invalid"))),
            None
        ); // Default not used here
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Integer(123)),
            None::<Role>
        );

        // Test from_sqlite_value for integer enum
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Integer(-1)),
            Some(Status::Banned)
        );
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Integer(99)),
            None
        ); // Default not used here
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Text(Cow::Borrowed("Active"))),
            None::<Status>
        );
    }

    // Test IntoSQLiteValue implementation for text-based enum
    #[test]
    fn test_text_enum_into() {
        let val = Role::Admin;
        let sqlite_val: SQLiteValue = val.into_sqlite_value();
        assert_eq!(
            sqlite_val,
            SQLiteValue::Text(Cow::Owned("Admin".to_string()))
        );
    }

    // Test IntoSQLiteValue implementation for integer-based enum
    #[test]
    fn test_integer_enum_into() {
        let val = Status::Active;
        let sqlite_val: SQLiteValue = val.into_sqlite_value();
        assert_eq!(sqlite_val, SQLiteValue::Integer(1));
    }
}

// Add JSON helpers
#[cfg(feature = "serde_json")]
use serde_json::{Value as JsonValue, json};

#[cfg(feature = "serde_json")]
pub fn json_extract<'a, T: ToSQL<'a, SQLiteValue<'a>>>(
    column: T,
    path: &'a str,
) -> SQL<'a, SQLiteValue<'a>> {
    let column_sql = column.to_sql();
    SQL(
        Cow::Owned(format!("json_extract({}, ?)", column_sql.0)),
        [column_sql.1, vec![path.into()]].concat(),
    )
}

#[cfg(feature = "serde_json")]
pub fn json_extract_text<'a, T: ToSQL<'a, SQLiteValue<'a>>>(
    column: T,
    path: &'a str,
) -> SQL<'a, SQLiteValue<'a>> {
    let column_sql = column.to_sql();
    SQL(
        Cow::Owned(format!("json_extract({}, ?)", column_sql.0)),
        [column_sql.1, vec![path.into()]].concat(),
    )
}

#[cfg(feature = "serde_json")]
pub fn json_array_element<'a, T: ToSQL<'a, SQLiteValue<'a>>>(
    column: T,
    index: i64,
) -> SQL<'a, SQLiteValue<'a>> {
    let column_sql = column.to_sql();
    SQL(
        Cow::Owned(format!("json_extract({}, '$[?]')", column_sql.0)),
        [column_sql.1, vec![index.into()]].concat(),
    )
}

#[cfg(feature = "serde_json")]
pub fn json_blob_query<'a, T: ToSQL<'a, SQLiteValue<'a>>>(
    column: T,
    path: &'a str,
) -> SQL<'a, SQLiteValue<'a>> {
    let column_sql = column.to_sql();
    SQL(
        Cow::Owned(format!("json_extract({}, ?)", column_sql.0)),
        [column_sql.1, vec![path.into()]].concat(),
    )
}

// Add ToSQL for bool
impl<'a> ToSQL<'a, SQLiteValue<'a>> for bool {
    fn to_sql(&self) -> SQL<'a, SQLiteValue<'a>> {
        let value = if *self { 1 } else { 0 };
        SQL(Cow::Borrowed("?"), vec![SQLiteValue::Integer(value)])
    }
}

// #[cfg(feature = "rusqlite")]
// impl<'a> rusqlite::ToSql for SQLiteValue<'a> {
//     fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
//         match self {
//             SQLiteValue::Null => Ok(rusqlite::types::ToSqlOutput::Owned(RusqliteValue::Null)),
//             SQLiteValue::Integer(i) => Ok(rusqlite::types::ToSqlOutput::Owned(
//                 RusqliteValue::Integer(*i),
//             )),
//             SQLiteValue::Real(f) => {
//                 Ok(rusqlite::types::ToSqlOutput::Owned(RusqliteValue::Real(*f)))
//             }
//             SQLiteValue::Text(cow) => Ok(rusqlite::types::ToSqlOutput::Borrowed(ValueRef::Text(
//                 cow.as_bytes(),
//             ))),
//             SQLiteValue::Blob(cow) => Ok(rusqlite::types::ToSqlOutput::Borrowed(ValueRef::Blob(
//                 cow.as_ref(),
//             ))),
//         }
//     }
// }
