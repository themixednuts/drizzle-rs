use drizzle_core::{SQL, ToSQL, traits::SQLParam};

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

// Note: Generic From implementation is removed to avoid conflicts.
// The table macro will generate specific implementations using SQLiteEnumVisitor.
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

#[cfg(any(feature = "turso", feature = "libsql", feature = "rusqlite"))]
#[cfg(test)]
mod tests {
    use drizzle_rs::prelude::*;
    use std::borrow::Cow;

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
    fn test_sqlite_enum_derives() {
        // Just test that the enums compile and basic functionality works
        let role = Role::Admin;
        let status = Status::Active;

        // Test that Display works
        assert_eq!(format!("{}", role), "Admin");
        assert_eq!(format!("{}", status), "Active");

        // Test that FromStr works
        assert_eq!("User".parse::<Role>().unwrap(), Role::User);
        assert_eq!("Banned".parse::<Status>().unwrap(), Status::Banned);
    }
}
