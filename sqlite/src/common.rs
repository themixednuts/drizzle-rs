use drizzle_core::{SQL, ToSQL, traits::SQLParam};
#[cfg(feature = "serde")]
use serde::{Serialize, de::DeserializeOwned};
use std::borrow::Cow;

// Import SQLiteValue from our own module
use crate::values::SQLiteValue;

//------------------------------------------------------------------------------
// Number Type
//------------------------------------------------------------------------------

/// Numeric type that can be either an integer or a floating point value
#[derive(Clone, Copy, Debug, PartialEq, PartialOrd)]
pub enum Number {
    /// Integer value
    Integer(i64),
    /// Floating point value
    Real(f64),
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

/// Trait for enum values that can be stored in SQLite
///
/// This trait allows enums to be automatically converted to and from SQLite values.
/// The representation is determined by the #[repr] attribute on the enum:
/// - With #[repr(i8)], #[repr(u32)], etc. → stored as INTEGER
/// - Without a numeric repr → stored as TEXT
pub trait SQLiteEnum:
    Sized + std::fmt::Display + std::str::FromStr<Err = String> + Default
{
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

impl<'a, T> From<T> for SQLiteValue<'a>
where
    T: SQLiteEnum + 'a,
{
    fn from(value: T) -> Self {
        match T::ENUM_REPR {
            SQLiteEnumRepr::Text => SQLiteValue::Text(Cow::Owned(format!("{}", value))),
            SQLiteEnumRepr::Integer => SQLiteValue::Integer(value.to_integer()),
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum JoinType {
    #[default]
    Join,
    Inner,
    Left,
    Right,
    Full,
    Cross,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Join {
    pub natural: bool,
    pub join_type: JoinType,
    pub outer: bool, // only meaningful for LEFT/RIGHT/FULL
}

impl Join {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn natural(mut self) -> Self {
        self.natural = true;
        self
    }

    pub fn inner(mut self) -> Self {
        self.join_type = JoinType::Inner;
        self
    }

    pub fn left(mut self) -> Self {
        self.join_type = JoinType::Left;
        self
    }

    pub fn right(mut self) -> Self {
        self.join_type = JoinType::Right;
        self
    }

    pub fn full(mut self) -> Self {
        self.join_type = JoinType::Full;
        self
    }

    pub fn cross(mut self) -> Self {
        self.join_type = JoinType::Cross;
        self
    }

    pub fn outer(mut self) -> Self {
        self.outer = true;
        self
    }
}
impl<'a, V: SQLParam + 'a> ToSQL<'a, V> for Join {
    fn to_sql(&self) -> SQL<'a, V> {
        let mut parts = Vec::new();

        if self.natural {
            parts.push("NATURAL");
        }

        match self.join_type {
            JoinType::Inner => parts.push("INNER"),
            JoinType::Left => {
                parts.push("LEFT");
                if self.outer {
                    parts.push("OUTER");
                }
            }
            JoinType::Right => {
                parts.push("RIGHT");
                if self.outer {
                    parts.push("OUTER");
                }
            }
            JoinType::Full => {
                parts.push("FULL");
                if self.outer {
                    parts.push("OUTER");
                }
            }
            JoinType::Cross => parts.push("CROSS"),
            JoinType::Join => {}
        }

        parts.push("JOIN");
        SQL::raw(parts.join(" "))
    }
}

//------------------------------------------------------------------------------
// Tests
//------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use drizzle_rs::prelude::*;
    use procmacros::SQLiteEnum;
    use std::borrow::Cow;

    // For SQLiteEnum tests

    // Define test enums for SQLiteEnum tests
    // Text-based enum (default)
    #[derive(SQLiteEnum, Default, Debug, Clone, PartialEq)]
    enum Role {
        #[default]
        User,
        Admin,
        Moderator,
    }

    // Integer-based enum with explicit discriminants
    #[derive(SQLiteEnum, Default, Debug, Clone, PartialEq)]
    #[repr(i32)]
    enum Status {
        Active = 1,
        #[default]
        Inactive = 0,
        Banned = -1,
    }

    #[test]
    fn test_into_sqlite_value_impls() {
        assert_eq!(
            SQLiteValue::from("hello"),
            SQLiteValue::Text(Cow::Borrowed("hello"))
        );
        assert_eq!(
            SQLiteValue::from(String::from("world")),
            SQLiteValue::Text(Cow::Owned("world".to_string()))
        );
        assert_eq!(SQLiteValue::from(42i64), SQLiteValue::Integer(42));
        assert_eq!(SQLiteValue::from(123i32), SQLiteValue::Integer(123));
        assert_eq!(SQLiteValue::from(3.14f64), SQLiteValue::Real(3.14));
        assert_eq!(SQLiteValue::from(true), SQLiteValue::Integer(1));
        assert_eq!(SQLiteValue::from(false), SQLiteValue::Integer(0));
        let blob_vec: Vec<u8> = vec![1, 2, 3];
        assert_eq!(
            SQLiteValue::from(blob_vec.clone()),
            SQLiteValue::Blob(Cow::Owned(blob_vec.clone()))
        );
        let blob_slice: &[u8] = &[4, 5, 6];
        assert_eq!(
            SQLiteValue::from(blob_slice),
            SQLiteValue::Blob(Cow::Borrowed(blob_slice))
        );
        assert_eq!(SQLiteValue::from(Option::<String>::None), SQLiteValue::Null);
        assert_eq!(
            SQLiteValue::from(Some("optional")),
            SQLiteValue::Text(Cow::Borrowed("optional"))
        );
    }

    #[test]
    fn test_sqlite_enum_conversion() {
        // Test conversion for text enum
        assert_eq!(
            SQLiteValue::from(Role::Admin),
            SQLiteValue::Text(Cow::Owned("Admin".to_string()))
        );

        // Test conversion for integer enum
        assert_eq!(SQLiteValue::from(Status::Active), SQLiteValue::Integer(1));

        // Test from_sqlite_value for text enum
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Text(Cow::Borrowed("User"))),
            Some(Role::User)
        );
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Text(Cow::Borrowed("Invalid"))),
            None::<Role>
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
            None::<Status>
        ); // Default not used here
        assert_eq!(
            SQLiteEnum::from_sqlite_value(&SQLiteValue::Text(Cow::Borrowed("Active"))),
            None::<Status>
        );
    }

    // Test Into implementation for text-based enum
    #[test]
    fn test_text_enum_into() {
        let val = Role::Admin;
        let sqlite_val: SQLiteValue = val.into();
        assert_eq!(
            sqlite_val,
            SQLiteValue::Text(Cow::Owned("Admin".to_string()))
        );
    }

    // Test Into implementation for integer-based enum
    #[test]
    fn test_integer_enum_into() {
        let val = Status::Active;
        let sqlite_val: SQLiteValue = val.into();
        assert_eq!(sqlite_val, SQLiteValue::Integer(1));
    }
}
